use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectSignature {
    pub language: String,
    pub framework: String,
    pub package_manager: String,
    pub ui_library: Option<String>,
    pub validation_library: Option<String>,
    pub auth_library: Option<String>,
    pub styling: Vec<String>,
    pub dependencies: HashMap<String, String>,
    pub dev_dependencies: HashMap<String, String>,
    pub features: Vec<String>,
}

impl ProjectSignature {
    pub fn detect(root_path: &Path) -> Result<Self> {
        let mut signature = ProjectSignature::default();

        // Detect package manager and parse manifests
        if let Some(package_manager) = Self::detect_package_manager(root_path)? {
            signature.package_manager = package_manager.clone();
            match package_manager.as_str() {
                "npm" | "yarn" | "pnpm" => {
                    if let Ok(package_json) = Self::parse_package_json(root_path) {
                        signature = Self::analyze_npm_package(&package_json, signature);
                    }
                }
                "cargo" => {
                    if let Ok(cargo_toml) = Self::parse_cargo_toml(root_path) {
                        signature = Self::analyze_rust_package(&cargo_toml, signature);
                    }
                }
                "pip" => {
                    // Python detection
                    signature.language = "python".to_string();
                }
                _ => {}
            }
        }

        // Detect language from file extensions
        signature.language = Self::detect_language_from_files(root_path)?;

        // Detect framework from files/config
        signature.framework = Self::detect_framework(root_path, &signature.language)?;

        // Autonomous detection will be handled by LLM in orchestrator

        // Note: Cloud, upload, and API services are detected autonomously by LLM during task planning

        // Styling detection
        signature.styling = Self::detect_styling(root_path, &signature.dependencies)?;

        // Features detection
        signature.features = Self::detect_features(root_path, &signature);

        Ok(signature)
    }

    fn detect_package_manager(root_path: &Path) -> Result<Option<String>> {
        let candidates = [
            ("package.json", "npm"),
            ("Cargo.toml", "cargo"),
            ("pyproject.toml", "pip"),
            ("requirements.txt", "pip"),
            ("yarn.lock", "yarn"),
            ("pnpm-lock.yaml", "pnpm"),
        ];

        for (file, manager) in candidates.iter() {
            if root_path.join(file).exists() {
                return Ok(Some(manager.to_string()));
            }
        }
        Ok(None)
    }

    fn parse_package_json(root_path: &Path) -> Result<Value> {
        let path = root_path.join("package.json");
        let content = fs::read_to_string(&path).context("Failed to read package.json")?;
        let parsed: Value = serde_json::from_str(&content).context("Failed to parse package.json")?;
        Ok(parsed)
    }

    fn parse_cargo_toml(root_path: &Path) -> Result<String> {
        let path = root_path.join("Cargo.toml");
        fs::read_to_string(&path).context("Failed to read Cargo.toml")
    }

    fn analyze_npm_package(package_json: &Value, mut signature: ProjectSignature) -> ProjectSignature {
        if let Some(deps) = package_json["dependencies"].as_object() {
            for (name, version) in deps {
                signature.dependencies.insert(name.clone(), version.as_str().unwrap_or("").to_string());
            }
        }
        if let Some(dev_deps) = package_json["devDependencies"].as_object() {
            for (name, version) in dev_deps {
                signature.dev_dependencies.insert(name.clone(), version.as_str().unwrap_or("").to_string());
            }
        }

        // Detect Next.js specifically
        if signature.dependencies.contains_key("next") || signature.dev_dependencies.contains_key("next") {
            signature.framework = "Next.js".to_string();
            signature.features.push("app-router".to_string()); // Assume modern Next.js
        }

        signature
    }

    fn analyze_rust_package(cargo_toml: &str, mut signature: ProjectSignature) -> ProjectSignature {
        // Parse Cargo.toml for Rust crates
        // Basic parsing - could use toml crate for better parsing
        for line in cargo_toml.lines() {
            if line.trim().starts_with('[') {
                continue;
            }
            if line.contains(" = ") {
                let parts: Vec<&str> = line.split(" = ").collect();
                if parts.len() == 2 {
                    let name = parts[0].trim().trim_end_matches('"').trim_start_matches('"');
                    let version = parts[1].trim().trim_matches('"');
                    signature.dependencies.insert(name.to_string(), version.to_string());
                }
            }
        }
        signature
    }

    fn detect_language_from_files(root_path: &Path) -> Result<String> {
        let mut counts = HashMap::new();
        let extensions = vec![".ts", ".tsx", ".js", ".jsx", ".rs", ".py", ".go", ".java"];

        for ext in extensions {
            let pattern = format!("**/*{}", ext);
            let _glob_path = root_path.join(&pattern.replace("**/", ""));
            // Simple count - could use walkdir for accuracy
            if root_path.join(format!("src/*{}", ext)).exists() || root_path.join(format!("lib/*{}", ext)).exists() {
                *counts.entry(ext).or_insert(0) += 1;
            }
        }

        // Determine primary language
        if *counts.get(&".ts").unwrap_or(&0) + *counts.get(&".tsx").unwrap_or(&0) > 0 {
            Ok("typescript".to_string())
        } else if *counts.get(&".rs").unwrap_or(&0) > 0 {
            Ok("rust".to_string())
        } else if *counts.get(&".py").unwrap_or(&0) > 0 {
            Ok("python".to_string())
        } else {
            Ok("unknown".to_string())
        }
    }

    fn detect_framework(root_path: &Path, language: &str) -> Result<String> {
        match language {
            "typescript" => {
                if root_path.join("next.config.js").exists() || root_path.join("next.config.mjs").exists() {
                    Ok("Next.js".to_string())
                } else if root_path.join("vite.config.ts").exists() {
                    Ok("Vite + React".to_string())
                } else if root_path.join("app").exists() {
                    Ok("Next.js App Router".to_string())
                } else {
                    Ok("React".to_string())
                }
            }
            "rust" => {
                if root_path.join("Cargo.toml").exists() {
                    // Check for web frameworks
                    if fs::read_to_string(root_path.join("Cargo.toml"))?
                        .contains("actix-web") || fs::read_to_string(root_path.join("Cargo.toml"))?.contains("axum") {
                        Ok("Rust Web".to_string())
                    } else {
                        Ok("Rust CLI".to_string())
                    }
                } else {
                    Ok("unknown".to_string())
                }
            }
            _ => Ok("unknown".to_string()),
        }
    }

    fn detect_ui_library(root_path: &Path, dependencies: &HashMap<String, String>) -> Option<String> {
        // Check dependencies first
        let ui_indicators = vec![
            ("@shadcn/ui", "shadcn/ui"),
            ("headlessui", "Headless UI"),
            ("@headlessui/react", "Headless UI"),
            ("@radix-ui", "Radix UI"),
            ("mantine", "Mantine"),
            ("chakra-ui", "Chakra UI"),
            ("antd", "Ant Design"),
            ("material-ui", "Material-UI"),
        ];

        for (dep, name) in ui_indicators {
            if dependencies.contains_key(dep) {
                return Some(name.to_string());
            }
        }

        // Fallback: scan for usage patterns
        let common_components = vec!["InputBox", "Button", "Form", "Modal", "Dialog"];
        for component in common_components {
            if Self::scan_for_component_usage(root_path, component).is_some() {
                return Some(format!("Custom UI (uses {})", component));
            }
        }

        None
    }

    fn detect_validation_library(dependencies: &HashMap<String, String>) -> Option<String> {
        let validation_indicators = vec![
            ("zod", "Zod"),
            ("yup", "Yup"),
            ("joi", "Joi"),
            ("class-validator", "Class Validator"),
            ("react-hook-form", "React Hook Form + Zod/Yup"),
        ];

        for (dep, name) in validation_indicators {
            if dependencies.contains_key(dep) {
                return Some(name.to_string());
            }
        }

        None
    }

    fn detect_auth_library(dependencies: &HashMap<String, String>) -> Option<String> {
        let auth_indicators = vec![
            ("next-auth", "NextAuth.js"),
            ("@auth0/auth0-react", "Auth0"),
            ("@supabase/auth-helpers-nextjs", "Supabase Auth"),
            ("firebase", "Firebase Auth"),
            ("jsonwebtoken", "JWT"),
        ];

        for (dep, name) in auth_indicators {
            if dependencies.contains_key(dep) {
                return Some(name.to_string());
            }
        }

        None
    }

    fn detect_styling(root_path: &Path, dependencies: &HashMap<String, String>) -> Result<Vec<String>> {
        let mut styling = Vec::new();

        // Check dependencies
        if dependencies.contains_key("tailwindcss") || root_path.join("tailwind.config.js").exists() {
            styling.push("Tailwind CSS".to_string());
        }
        if dependencies.contains_key("styled-components") {
            styling.push("Styled Components".to_string());
        }
        if dependencies.contains_key("emotion") {
            styling.push("Emotion".to_string());
        }
        if dependencies.contains_key("sass") || dependencies.contains_key("node-sass") {
            styling.push("Sass/SCSS".to_string());
        }
        if dependencies.contains_key("css-modules") {
            styling.push("CSS Modules".to_string());
        }

        // Scan for CSS files
        if root_path.join("styles").exists() || root_path.join("src/styles").exists() {
            styling.push("CSS".to_string());
        }

        Ok(styling)
    }

    fn detect_features(root_path: &Path, signature: &ProjectSignature) -> Vec<String> {
        let mut features = Vec::new();

        // TypeScript features
        if signature.language == "typescript" {
            features.push("TypeScript".to_string());
            if root_path.join("tsconfig.json").exists() {
                features.push("Strict Type Checking".to_string());
            }
        }

        // Framework features
        match signature.framework.as_str() {
            "Next.js" => {
                features.push("Server-Side Rendering".to_string());
                features.push("Static Site Generation".to_string());
                if root_path.join("app").exists() {
                    features.push("App Router".to_string());
                } else if root_path.join("pages").exists() {
                    features.push("Pages Router".to_string());
                }
            }
            "React" => {
                features.push("Client-Side Rendering".to_string());
            }
            _ => {}
        }

        // UI and validation features
        if let Some(ui) = &signature.ui_library {
            features.push(format!("UI: {}", ui));
        }
        if let Some(val) = &signature.validation_library {
            features.push(format!("Validation: {}", val));
        }
        if let Some(auth) = &signature.auth_library {
            features.push(format!("Auth: {}", auth));
        }

        // Styling features
        for style in &signature.styling {
            features.push(format!("Styling: {}", style));
        }

        features
    }

    fn scan_for_component_usage(root_path: &Path, component_name: &str) -> Option<PathBuf> {
        // Simple scan - could be enhanced with git grep or tree-sitter
        let pattern = format!("{}(", component_name); // Usage like InputBox(props)
        let search_paths = vec!["src", "components", "app", "."];

        for search_path in search_paths {
            let full_path = root_path.join(search_path);
            if full_path.exists() {
                // This is a simple placeholder - in production, use git grep or walkdir
                if let Ok(content) = fs::read_dir(&full_path) {
                    for entry in content.flatten() {
                        if let Ok(file_content) = fs::read_to_string(entry.path()) {
                            if file_content.contains(&pattern) {
                                return Some(entry.path());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Get a human-readable description of the project signature
    pub fn to_description(&self) -> String {
        let mut parts = vec![];
        if !self.language.is_empty() {
            parts.push(format!("Language: {}", self.language));
        }
        if !self.framework.is_empty() {
            parts.push(format!("Framework: {}", self.framework));
        }
        if !self.package_manager.is_empty() {
            parts.push(format!("Package Manager: {}", self.package_manager));
        }
        if let Some(ref ui) = self.ui_library {
            parts.push(format!("UI Library: {}", ui));
        }
        if let Some(ref val) = self.validation_library {
            parts.push(format!("Validation: {}", val));
        }
        parts.join(", ")
    }

    /// Get the dominant language (alias for language field)
    pub fn dominant_language(&self) -> &str {
        &self.language
    }

    /// Get question templates based on detected project characteristics
    pub fn get_question_templates(&self) -> Vec<String> {
        let mut questions = vec![];

        // Language-specific questions
        match self.language.as_str() {
            "typescript" | "javascript" => {
                questions.push("What React components are used for UI?".to_string());
                questions.push("What TypeScript types are defined?".to_string());
                questions.push("What utility functions are available?".to_string());
            }
            "rust" => {
                questions.push("What structs and enums are defined?".to_string());
                questions.push("What functions are available?".to_string());
                questions.push("What traits are implemented?".to_string());
            }
            "python" => {
                questions.push("What classes are defined?".to_string());
                questions.push("What functions are available?".to_string());
                questions.push("What modules are imported?".to_string());
            }
            _ => {
                questions.push("What components are available?".to_string());
                questions.push("What types are defined?".to_string());
            }
        }

        // Framework-specific questions
        if self.framework.contains("React") {
            questions.push("What React hooks are used?".to_string());
        }
        if self.framework.contains("Next.js") {
            questions.push("What Next.js pages or API routes exist?".to_string());
        }
        if self.framework.contains("NestJS") {
            questions.push("What NestJS controllers and services exist?".to_string());
        }

        // Validation library questions
        if let Some(ref val_lib) = self.validation_library {
            if val_lib == "Zod" {
                questions.push("What Zod schemas are defined?".to_string());
            }
        }

        questions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_package_manager() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("package.json"), r#"{}"#).unwrap();
        let result = ProjectSignature::detect_package_manager(temp_dir.path()).unwrap();
        assert_eq!(result, Some("npm".to_string()));
    }

    #[test]
    fn test_detect_nextjs() {
        let temp_dir = tempfile::tempdir().unwrap();
        fs::write(temp_dir.path().join("package.json"), r#"{"dependencies":{"next":"14.0.0"}}"#).unwrap();
        let package_json = ProjectSignature::parse_package_json(temp_dir.path()).unwrap();
        let signature = ProjectSignature::analyze_npm_package(&package_json, ProjectSignature::default());
        assert_eq!(signature.framework, "Next.js".to_string());
    }
}
