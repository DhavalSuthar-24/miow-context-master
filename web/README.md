# MIOW-Context Web UI

A modern React web interface for the MIOW-Context autonomous code context generation system.

## Features

- 🎯 **Simple Interface**: Paste your codebase path and describe your task
- 🤖 **Autonomous Analysis**: AI-powered code understanding with no hardcoded biases
- 🚀 **Fast Generation**: Optimized for speed with Gemini 2.5 Flash
- 📋 **Copy-Paste Ready**: Generated prompts work with ChatGPT, Claude, DeepSeek
- 🔍 **Multi-Language Support**: Works with React, Rust, Python, TypeScript, and more
- 💾 **Vector Search**: Qdrant-powered semantic code search
- 🎨 **Modern UI**: Clean, responsive design with real-time status indicators

## Quick Start

1. **Start the Backend** (in project root):
   ```bash
   # Make sure you have your API keys set
   export GEMINI_API_KEY="your_key_here"

   # Start with web feature
   cargo run --features web -- serve --port 3001
   ```

2. **Start the Frontend** (in web/ directory):
   ```bash
   cd web
   npm install
   npm run dev
   ```

3. **Open** `http://localhost:5173` in your browser

## Usage

1. **Codebase Path**: Enter the absolute or relative path to your project
   - Example: `/home/user/my-react-app` or `./current-project`

2. **Your Task**: Describe what you want to implement
   - Example: "Add user authentication with JWT tokens to my React app"
   - Example: "Create a REST API endpoint for file uploads in my Node.js backend"

3. **Generate**: Click the button to get AI-generated context
   - The system will autonomously analyze your codebase
   - Extract relevant code patterns, types, and examples
   - Generate a comprehensive prompt optimized for other LLMs

## API Endpoints

- `POST /api/generate`: Generate context from codebase path and user prompt
- `POST /api/health`: Check backend status and connectivity

## Development

```bash
cd web
npm run dev      # Development server
npm run build    # Production build
npm run preview  # Preview production build
```
