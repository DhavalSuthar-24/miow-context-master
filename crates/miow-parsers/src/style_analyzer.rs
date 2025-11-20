use anyhow::Result;
use serde::{Deserialize, Serialize};
use miow_llm::LLMProvider;
use std::sync::Arc;

/// Style analyzer - extracts coding patterns and style information
pub struct StyleAnalyzer {
    llm: Option<Arc<Box<dyn LLMProvider>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleAnalysis {
    pub naming_convention: Vec<String>,  // ["camelCase", "PascalCase", "snake_case"]
    pub patterns: Vec<String>,            // ["Functional", "Hooks-based", "Error handling via Result"]
    pub error_handling: Vec<String>,      // ["Result<T>", "try/catch", "Option<T>"]
    pub code_samples: Vec<String>,        // Representative code snippets
}

impl StyleAnalyzer {
    /// Create new style analyzer
    pub fn new() -> Self {
        Self { llm: None }
    }
    
    /// Create with LLM for enhanced analysis
    pub fn with_llm(mut self, llm: Arc<Box<dyn LLMProvider>>) -> Self {
        self.llm = Some(llm);
        self
    }
    
    /// Analyze code style from parsed content
    pub async fn analyze(&self, code_samples: &[String], language: &str) -> Result<StyleAnalysis> {
        // If LLM is available, use it for deep analysis
        if let Some(llm) = &self.llm {
            self.analyze_with_llm(code_samples, language, llm).await
        } else {
            // Fallback to pattern-based analysis
            Ok(self.analyze_patterns(code_samples, language))
        }
    }
    
    /// LLM-powered style analysis
    async fn analyze_with_llm(
        &self,
        code_samples: &[String],
        language: &str,
        llm: &Arc<Box<dyn LLMProvider>>,
    ) -> Result<StyleAnalysis> {
        // Take first 3 samples to avoid token overload
        let samples: Vec<_> = code_samples.iter().take(3).cloned().collect();
        let combined = samples.join("\n\n---\n\n");
        
        let prompt = format!(
            r#"Analyze the following {} code samples and extract style patterns.
Return ONLY a JSON object with this structure:
{{
  "naming_convention": ["convention1", "convention2"],
  "patterns": ["pattern1", "pattern2"],
  "error_handling": ["style1", "style2"]
}}

Examples of naming: "camelCase", "snake_case", "PascalCase"
Examples of patterns: "Functional programming", "Hooks-based React", "OOP", "Trait-based"
Examples of error handling: "Result<T, E>", "try/catch", "Option<T>", "panic!"

Code samples:
{}

Return ONLY the JSON, no explanation."#,
            language, combined
        );
        
        let response = llm.generate(&prompt).await?;
        
        // Try to parse JSON response
        let clean_response = response.content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        
        match serde_json::from_str::<serde_json::Value>(clean_response) {
            Ok(json) => {
                let naming = json["naming_convention"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                
                let patterns = json["patterns"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                
                let error_handling = json["error_handling"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                
                Ok(StyleAnalysis {
                    naming_convention: naming,
                    patterns,
                    error_handling,
                    code_samples: samples,
                })
            }
            Err(_) => {
                // Fallback to pattern-based if JSON parsing fails
                Ok(self.analyze_patterns(code_samples, language))
            }
        }
    }
    
    /// Pattern-based style analysis (no LLM required)
    fn analyze_patterns(&self, code_samples: &[String], language: &str) -> StyleAnalysis {
        let mut naming_convention = Vec::new();
        let mut patterns = Vec::new();
        let mut error_handling = Vec::new();
        
        let combined = code_samples.join("\n");
        
        // Detect naming conventions
        if combined.contains("camelCase") || combined.contains("const ") && combined.contains(" = ") {
            naming_convention.push("camelCase".to_string());
        }
        if combined.contains("PascalCase") || combined.contains("class ") || combined.contains("function ") {
            naming_convention.push("PascalCase".to_string());
        }
        if combined.contains("snake_case") || combined.contains("_") {
            naming_convention.push("snake_case".to_string());
        }
        
        // Detect patterns by language
        match language {
            "TypeScript" | "JavaScript" | "TSX" => {
                if combined.contains("useState") || combined.contains("useEffect") {
                    patterns.push("Hooks-based React".to_string());
                }
                if combined.contains("=>") {
                    patterns.push("Functional programming".to_string());
                }
                if combined.contains("class ") && combined.contains("extends") {
                    patterns.push("OOP".to_string());
                }
                if combined.contains("try") && combined.contains("catch") {
                    error_handling.push("try/catch".to_string());
                }
            }
            "Rust" => {
                if combined.contains("Result<") {
                    error_handling.push("Result<T, E>".to_string());
                }
                if combined.contains("Option<") {
                    error_handling.push("Option<T>".to_string());
                }
                if combined.contains("impl ") && combined.contains("trait") {
                    patterns.push("Trait-based".to_string());
                }
                if combined.contains("struct ") {
                    patterns.push("Struct-based".to_string());
                }
            }
            "Python" => {
                if combined.contains("def ") {
                    patterns.push("Function-based".to_string());
                }
                if combined.contains("class ") {
                    patterns.push("OOP".to_string());
                }
                if combined.contains("try:") && combined.contains("except") {
                    error_handling.push("try/except".to_string());
                }
            }
            _ => {}
        }
        
        // Deduplicate
        naming_convention.sort();
        naming_convention.dedup();
        patterns.sort();
        patterns.dedup();
        error_handling.sort();
        error_handling.dedup();
        
        StyleAnalysis {
            naming_convention,
            patterns,
            error_handling,
            code_samples: code_samples.iter().take(3).cloned().collect(),
        }
    }
    
    /// Convert style analysis to tags for vector DB
    pub fn to_tags(&self, analysis: &StyleAnalysis) -> Vec<String> {
        let mut tags = Vec::new();
        
        tags.extend(analysis.naming_convention.clone());
        tags.extend(analysis.patterns.clone());
        tags.extend(analysis.error_handling.clone());
        
        tags
    }
}

impl Default for StyleAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pattern_detection_react() {
        let analyzer = StyleAnalyzer::new();
        let samples = vec![
            r#"
            const Component = () => {
                const [state, setState] = useState(0);
                return <div>{state}</div>;
            }
            "#.to_string(),
        ];
        
        let analysis = analyzer.analyze_patterns(&samples, "TypeScript");
        
        assert!(analysis.patterns.contains(&"Hooks-based React".to_string()));
        assert!(analysis.patterns.contains(&"Functional programming".to_string()));
    }
    
    #[test]
    fn test_pattern_detection_rust() {
        let analyzer = StyleAnalyzer::new();
        let samples = vec![
            r#"
            pub fn process() -> Result<String, Error> {
                Ok("success".to_string())
            }
            "#.to_string(),
        ];
        
        let analysis = analyzer.analyze_patterns(&samples, "Rust");
        
        assert!(analysis.error_handling.contains(&"Result<T, E>".to_string()));
    }
}
