use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A specialized prompt with its key, description, and template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializedPrompt {
    pub key: String,
    pub description: String,
    pub template: String,
    pub category: PromptCategory,
    pub priority: Priority,
    pub dependencies: Vec<String>, // Keys of prompts that must run before this one
    pub provides_context: Vec<String>, // Context keys this prompt provides (e.g., "framework", "language")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PromptCategory {
    StackDetection,
    TaskClassification,
    Frontend,
    Backend,
    Data,
    Security,
    Testing,
    Infrastructure,
    ErrorAnalysis,
    Documentation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

/// Registry of all specialized prompts for autonomous orchestration
pub struct PromptRegistry {
    prompts: HashMap<String, SpecializedPrompt>,
}

impl PromptRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            prompts: HashMap::new(),
        };
        registry.initialize_prompts();
        registry
    }

    fn initialize_prompts(&mut self) {
        let prompts = vec![
            SpecializedPrompt {
                key: "stack_detector".to_string(),
                description: "Analyze file tree and configuration files to detect programming language, framework, and architecture".to_string(),
                template: r#"You are a Stack Detection Specialist. Analyze this project structure and configuration:
Project files: {file_list}
Package managers: {package_managers}
Key config files: {config_files}

Respond with JSON:
{{
  "language": "typescript|rust|python|etc",
  "framework": "nextjs|react|axum|django|etc",
  "architecture": "monolith|microservices|serverless|etc",
  "features": ["ssr", "api", "auth", "database", "etc"]
}}"#.to_string(),
                category: PromptCategory::StackDetection,
                priority: Priority::Critical,
                dependencies: vec![], // No dependencies, runs first
                provides_context: vec!["language".to_string(), "framework".to_string(), "architecture".to_string()],
            },

            SpecializedPrompt {
                key: "task_classifier".to_string(),
                description: "Classify the user's request into categories like feature, bugfix, refactor, explanation".to_string(),
                template: r#"You are a Task Classification Specialist. Analyze this user request and classify it:

User request: {user_prompt}
Project stack: {project_stack}

Respond with JSON:
{{
  "task_type": "feature|bugfix|refactor|explanation|documentation",
  "complexity": "simple|medium|complex",
  "domains": ["ui", "backend", "database", "auth", "api", "testing", "etc"],
  "urgency": "low|medium|high"
}}"#.to_string(),
                category: PromptCategory::TaskClassification,
                priority: Priority::High,
                dependencies: vec![], // Can run in parallel with stack_detector
                provides_context: vec!["task_type".to_string(), "complexity".to_string(), "domains".to_string()],
            },

            SpecializedPrompt {
                key: "frontend_scanner".to_string(),
                description: "Find UI components, props, styling systems, and frontend patterns".to_string(),
                template: r#"You are a Frontend Specialist. Find relevant frontend code for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- UI components (Button, Input, Form, etc.)
- Styling systems (CSS, Tailwind, Styled Components)
- State management patterns
- Props interfaces and types

Return JSON array of relevant code snippets with paths and descriptions."#.to_string(),
                category: PromptCategory::Frontend,
                priority: Priority::High,
                dependencies: vec!["stack_detector".to_string()], // Needs framework info
                provides_context: vec!["ui_components".to_string(), "styling_system".to_string()],
            },

            SpecializedPrompt {
                key: "backend_scanner".to_string(),
                description: "Find API routes, controllers, database models, and backend patterns".to_string(),
                template: r#"You are a Backend Specialist. Find relevant backend code for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- API routes and controllers
- Database models and schemas
- Business logic functions
- Middleware and authentication

Return JSON array of relevant code snippets with paths and descriptions."#.to_string(),
                category: PromptCategory::Backend,
                priority: Priority::High,
                dependencies: vec!["stack_detector".to_string()], // Needs framework info
                provides_context: vec!["api_routes".to_string(), "database_models".to_string()],
            },

            SpecializedPrompt {
                key: "data_scanner".to_string(),
                description: "Find type definitions, interfaces, database schemas, and data models".to_string(),
                template: r#"You are a Data Specialist. Find relevant data structures and types for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- TypeScript interfaces and types
- Database schemas and models
- Validation schemas (Zod, Joi, etc.)
- Data transformation functions

Return JSON array of relevant type definitions and schemas."#.to_string(),
                category: PromptCategory::Data,
                priority: Priority::Medium,
                dependencies: vec!["stack_detector".to_string()], // Needs language/framework info
                provides_context: vec!["type_definitions".to_string(), "validation_schemas".to_string()],
            },

            SpecializedPrompt {
                key: "auth_scanner".to_string(),
                description: "Find authentication, authorization, and security-related code".to_string(),
                template: r#"You are an Authentication Specialist. Find relevant security code for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- Login/logout functions
- JWT handling and validation
- User session management
- Authorization middleware
- Password hashing and verification

Return JSON array of relevant authentication code snippets."#.to_string(),
                category: PromptCategory::Security,
                priority: Priority::Medium,
                dependencies: vec![], // Can run independently
                provides_context: vec!["auth_patterns".to_string(), "security_middleware".to_string()],
            },

            SpecializedPrompt {
                key: "api_scanner".to_string(),
                description: "Find API endpoints, HTTP handlers, and external service integrations".to_string(),
                template: r#"You are an API Specialist. Find relevant API code for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- REST API endpoints
- GraphQL resolvers
- External API integrations
- HTTP client code
- Request/response handling

Return JSON array of relevant API code snippets."#.to_string(),
                category: PromptCategory::Backend,
                priority: Priority::Medium,
                dependencies: vec!["stack_detector".to_string()], // Needs framework info
                provides_context: vec!["api_endpoints".to_string(), "external_integrations".to_string()],
            },

            SpecializedPrompt {
                key: "test_scanner".to_string(),
                description: "Find unit tests, integration tests, and testing utilities".to_string(),
                template: r#"You are a Testing Specialist. Find relevant test code for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- Unit tests for the relevant components
- Integration tests
- Test utilities and mocks
- Test configuration files

Return JSON array of relevant test files and utilities."#.to_string(),
                category: PromptCategory::Testing,
                priority: Priority::Low,
                dependencies: vec!["frontend_scanner".to_string(), "backend_scanner".to_string()], // Needs component info
                provides_context: vec!["test_files".to_string(), "test_utilities".to_string()],
            },

            SpecializedPrompt {
                key: "error_analyzer".to_string(),
                description: "Analyze error logs and find the files causing issues".to_string(),
                template: r#"You are an Error Analysis Specialist. Analyze this error and find related code:

Error: {error_message}
Project: {project_info}

Search for:
- Files mentioned in the error
- Similar error handling patterns
- Logging and error reporting code
- Exception handling blocks

Return JSON with analysis and relevant code locations."#.to_string(),
                category: PromptCategory::ErrorAnalysis,
                priority: Priority::High,
                dependencies: vec![], // Can run with just the error message
                provides_context: vec!["error_locations".to_string(), "error_patterns".to_string()],
            },

            SpecializedPrompt {
                key: "config_scanner".to_string(),
                description: "Find configuration files, environment variables, and deployment settings".to_string(),
                template: r#"You are a Configuration Specialist. Find relevant config files for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- Environment variable usage
- Configuration files (JSON, YAML, TOML)
- Docker configurations
- Build and deployment scripts

Return JSON array of relevant configuration code."#.to_string(),
                category: PromptCategory::Infrastructure,
                priority: Priority::Low,
                dependencies: vec![], // Can run independently
                provides_context: vec!["config_files".to_string(), "environment_vars".to_string()],
            },

            SpecializedPrompt {
                key: "dependency_analyzer".to_string(),
                description: "Analyze import relationships and dependency chains".to_string(),
                template: r#"You are a Dependency Analysis Specialist. Map the dependencies for this task:

Task: {user_prompt}
Starting file: {file_path}
Project: {project_info}

Trace:
- Direct imports of the target file
- Files that import the target file
- Transitive dependencies
- Circular dependency warnings

Return JSON with dependency graph and relationships."#.to_string(),
                category: PromptCategory::Infrastructure,
                priority: Priority::Medium,
                dependencies: vec!["frontend_scanner".to_string(), "backend_scanner".to_string()], // Needs file info
                provides_context: vec!["dependency_graph".to_string(), "import_chains".to_string()],
            },

            SpecializedPrompt {
                key: "security_auditor".to_string(),
                description: "Check for security vulnerabilities and authentication patterns".to_string(),
                template: r#"You are a Security Auditor. Review code for security issues:

Task: {user_prompt}
Project: {project_info}

Check for:
- Input validation and sanitization
- SQL injection vulnerabilities
- XSS protection
- Authentication bypasses
- Secure password handling
- HTTPS enforcement

Return JSON with security analysis and recommendations."#.to_string(),
                category: PromptCategory::Security,
                priority: Priority::Medium,
                dependencies: vec!["auth_scanner".to_string()], // Needs auth context
                provides_context: vec!["security_issues".to_string(), "security_recommendations".to_string()],
            },

            SpecializedPrompt {
                key: "performance_analyzer".to_string(),
                description: "Analyze code for performance bottlenecks and optimization opportunities".to_string(),
                template: r#"You are a Performance Analyst. Review code for performance issues:

Task: {user_prompt}
Project: {project_info}

Analyze:
- Database query efficiency
- Memory usage patterns
- CPU-intensive operations
- Caching opportunities
- Bottleneck identification

Return JSON with performance analysis and optimization suggestions."#.to_string(),
                category: PromptCategory::Infrastructure,
                priority: Priority::Low,
                dependencies: vec!["backend_scanner".to_string()], // Needs backend code context
                provides_context: vec!["performance_bottlenecks".to_string(), "optimization_suggestions".to_string()],
            },

            SpecializedPrompt {
                key: "documentation_scanner".to_string(),
                description: "Find documentation, READMEs, and code comments".to_string(),
                template: r#"You are a Documentation Specialist. Find relevant documentation for this task:

Task: {user_prompt}
Project: {project_info}

Search for:
- README files and documentation
- Code comments and JSDoc
- API documentation
- Usage examples

Return JSON array of relevant documentation."#.to_string(),
                category: PromptCategory::Documentation,
                priority: Priority::Low,
                dependencies: vec![], // Can run independently
                provides_context: vec!["documentation".to_string(), "code_comments".to_string()],
            },

            SpecializedPrompt {
                key: "refactor_advisor".to_string(),
                description: "Suggest refactoring opportunities and code improvements".to_string(),
                template: r#"You are a Refactoring Advisor. Analyze code for improvement opportunities:

Task: {user_prompt}
Project: {project_info}

Look for:
- Code duplication
- Complex functions to simplify
- Better abstraction opportunities
- Performance improvements
- Maintainability enhancements

Return JSON with refactoring suggestions and code examples."#.to_string(),
                category: PromptCategory::TaskClassification,
                priority: Priority::Low,
                dependencies: vec!["frontend_scanner".to_string(), "backend_scanner".to_string()], // Needs code context
                provides_context: vec!["refactoring_suggestions".to_string(), "code_improvements".to_string()],
            },
        ];

        for prompt in prompts {
            self.prompts.insert(prompt.key.clone(), prompt);
        }
    }

    pub fn get_prompt(&self, key: &str) -> Option<&SpecializedPrompt> {
        self.prompts.get(key)
    }

    pub fn get_all_prompts(&self) -> &HashMap<String, SpecializedPrompt> {
        &self.prompts
    }

    pub fn get_prompts_by_category(&self, category: &PromptCategory) -> Vec<&SpecializedPrompt> {
        self.prompts.values().filter(|p| &p.category == category).collect()
    }

    pub fn get_prompts_by_priority(&self, priority: &Priority) -> Vec<&SpecializedPrompt> {
        self.prompts.values().filter(|p| &p.priority == priority).collect()
    }

    /// Get keys of prompts that are commonly needed for different task types
    pub fn get_recommended_prompts(&self, task_type: &str) -> Vec<String> {
        match task_type {
            "feature" => vec![
                "frontend_scanner".to_string(),
                "backend_scanner".to_string(),
                "data_scanner".to_string(),
                "api_scanner".to_string(),
            ],
            "bugfix" => vec![
                "error_analyzer".to_string(),
                "test_scanner".to_string(),
                "frontend_scanner".to_string(),
                "backend_scanner".to_string(),
            ],
            "refactor" => vec![
                "refactor_advisor".to_string(),
                "dependency_analyzer".to_string(),
                "performance_analyzer".to_string(),
            ],
            "explanation" => vec![
                "documentation_scanner".to_string(),
                "frontend_scanner".to_string(),
                "data_scanner".to_string(),
            ],
            "security" => vec![
                "security_auditor".to_string(),
                "auth_scanner".to_string(),
                "config_scanner".to_string(),
            ],
            _ => vec![
                "stack_detector".to_string(),
                "frontend_scanner".to_string(),
                "backend_scanner".to_string(),
            ],
        }
    }
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}
