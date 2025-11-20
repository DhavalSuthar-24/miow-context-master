# MiowContext - Intelligent Context Engine for Code Generation

MiowContext is an advanced context engine that helps AI agents and IDEs understand codebases better by providing comprehensive context about existing code, components, utilities, and design patterns.

## Features

- 🔍 **Intelligent Codebase Indexing**: Indexes codebases and extracts symbols, types, design tokens, constants, and schemas
- 🤖 **LLM-Powered Analysis**: Uses Google Gemini API for intent analysis and context gathering
- 🌐 **Web UI**: Modern React interface for easy codebase analysis (optional)
- 📊 **Knowledge Graph**: Stores code relationships in SQLite for fast queries
- 🔎 **Vector Search**: Semantic search using Qdrant (optional, requires Docker)
- 📝 **Context-Aware Prompts**: Generates comprehensive prompts with all relevant context
- 🎯 **Multi-Step Implementation Plans**: Creates detailed step-by-step plans from existing codebase patterns
- 🚀 **Autonomous Agents**: Multi-agent system with dependency resolution for complex analysis

## Quick Start

### Prerequisites

- Rust (latest stable)
- Docker (for Qdrant vector database - optional)
- Google Gemini API key (optional, for advanced LLM features)

### Setup

1. **Clone and build:**
   ```bash
   cargo build --release
   ```

2. **Start Qdrant (optional, for vector search):**
   ```bash
   docker-compose up -d
   ```

3. **Set environment variables:**
   ```bash
   export GEMINI_API_KEY=your_api_key_here
   export QDRANT_URL=http://localhost:6333  # Default
   export EMBEDDING_URL=http://localhost:8080/embed  # Optional, for custom embeddings
   ```

### Usage

#### CLI Usage

1. **Index a codebase:**
   ```bash
   cargo run -- index /path/to/codebase --db miow.db
   ```

2. **Generate context-aware prompt:**
   ```bash
   cargo run -- generate /path/to/codebase "make login page" --db miow.db --output prompt.txt
   ```

3. **Use the autonomous ask command:**
   ```bash
   cargo run -- ask "Add user authentication to my React app"
   ```

#### Web UI (Recommended)

For the best experience, use the web interface:

1. **Start the web servers:**
   ```bash
   # Linux/Mac
   ./start-web.sh

   # Windows
   start-web.bat
   ```

2. **Open your browser:**
   - Frontend: http://localhost:5173
   - Backend API: http://localhost:3001/api

3. **Use the interface:**
   - Enter your codebase path
   - Describe your task
   - Click "Generate Context"
   - Copy the result to any LLM (ChatGPT, Claude, etc.)

The generated prompt will include:
- All relevant existing components and helpers
- Design tokens (colors, spacing, etc.)
- Type definitions
- Constants and configuration
- Validation schemas
- Similar implementation patterns
- Step-by-step implementation plan

## Architecture

- **miow-core**: Codebase indexing and file traversal
- **miow-parsers**: Language parsers (TypeScript, Rust, Python)
- **miow-graph**: Knowledge graph storage (SQLite)
- **miow-vector**: Vector store for semantic search (Qdrant)
- **miow-llm**: LLM integration (Gemini, OpenAI)
- **miow-analyzer**: Context analysis and intent detection
- **miow-prompt**: Prompt generation with context

## Configuration

### Environment Variables

- `GEMINI_API_KEY`: Google Gemini API key (required for LLM features)
- `QDRANT_URL`: Qdrant server URL (default: http://localhost:6333)
- `EMBEDDING_URL`: Custom embedding service URL (optional)

### Docker Compose

The `docker-compose.yml` file sets up Qdrant vector database:
- REST API: http://localhost:6333
- gRPC: http://localhost:6334

## Development

```bash
# Run tests
cargo test

# Check for errors
cargo check

# Build release
cargo build --release
```

## License

MIT

