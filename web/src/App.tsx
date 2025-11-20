import { useState, useEffect } from 'react'
import './App.css'

interface GenerateResponse {
  success: boolean
  result?: string
  error?: string
}

interface HealthResponse {
  status: string
  version: string
  qdrant_connected: boolean
  gemini_configured: boolean
}

interface ProjectSignature {
  language: string
  framework: string
  package_manager: string
  ui_library: string
  validation_library: string
  auth_library: string
  description: string
}

interface DebugContext {
  total_symbols: number
  total_files: number
  db_path: string
  collection_name: string
}

interface FileInfo {
  file_path: string
  symbol_name: string
  symbol_kind: string
  relevance_score: number
  preview: string
}

function App() {
  const [codebasePath, setCodebasePath] = useState('')
  const [userPrompt, setUserPrompt] = useState('')
  const [isLoading, setIsLoading] = useState(false)
  const [result, setResult] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [health, setHealth] = useState<HealthResponse | null>(null)
  
  // Debug info
  const [signature, setSignature] = useState<ProjectSignature | null>(null)
  const [context, setContext] = useState<DebugContext | null>(null)
  const [loadingDebug, setLoadingDebug] = useState(false)
  
  // File selection
  const [relevantFiles, setRelevantFiles] = useState<FileInfo[]>([])
  const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set())
  const [loadingFiles, setLoadingFiles] = useState(false)
  const [showFileSelection, setShowFileSelection] = useState(false)

  // Check backend health on mount
  useEffect(() => {
    checkHealth()
  }, [])

  const checkHealth = async () => {
    try {
      const response = await fetch('/api/health', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
      })
      const healthData: HealthResponse = await response.json()
      setHealth(healthData)
    } catch (err) {
      console.error('Health check failed:', err)
    }
  }

  const loadDebugInfo = async () => {
    if (!codebasePath.trim()) return
    
    setLoadingDebug(true)
    try {
      // Load signature
      const sigResponse = await fetch('/api/debug/signature', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ codebase_path: codebasePath })
      })
      const sigData = await sigResponse.json()
      if (sigData.success && sigData.signature) {
        setSignature(sigData.signature)
      }
      
      // Load context
      const ctxResponse = await fetch('/api/debug/context', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ codebase_path: codebasePath })
      })
      const ctxData = await ctxResponse.json()
      if (ctxData.success && ctxData.context) {
        setContext(ctxData.context)
      }
    } catch (err) {
      console.error('Failed to load debug info:', err)
    } finally {
      setLoadingDebug(false)
    }
  }

  const loadRelevantFiles = async () => {
    if (!codebasePath.trim() || !userPrompt.trim()) {
      setError('Please provide both codebase path and prompt')
      return
    }

    setLoadingFiles(true)
    setError(null)
    try {
      const response = await fetch('/api/files', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          codebase_path: codebasePath,
          user_prompt: userPrompt
        })
      })

      const data = await response.json()
      if (data.success && data.files) {
        setRelevantFiles(data.files)
        setShowFileSelection(true)
        setSelectedFiles(new Set()) // Reset selection
      } else {
        setError(data.error || 'Failed to load relevant files')
      }
    } catch (err) {
      setError('Failed to connect to backend. Make sure the server is running.')
      console.error('Request failed:', err)
    } finally {
      setLoadingFiles(false)
    }
  }

  const toggleFileSelection = (filePath: string) => {
    const newSelection = new Set(selectedFiles)
    if (newSelection.has(filePath)) {
      newSelection.delete(filePath)
    } else {
      newSelection.add(filePath)
    }
    setSelectedFiles(newSelection)
  }

  const handleGenerate = async () => {
    if (!codebasePath.trim() || !userPrompt.trim()) {
      setError('Please provide both codebase path and prompt')
      return
    }

    setIsLoading(true)
    setError(null)
    setResult(null)

    try {
      const response = await fetch('/api/generate', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          codebase_path: codebasePath,
          user_prompt: userPrompt
        })
      })

      const data: GenerateResponse = await response.json()

      if (data.success && data.result) {
        setResult(data.result)
      } else {
        setError(data.error || 'Unknown error occurred')
      }
    } catch (err) {
      setError('Failed to connect to backend. Make sure the server is running.')
      console.error('Request failed:', err)
    } finally {
      setIsLoading(false)
    }
  }

  const handleGenerateWithFiles = async () => {
    if (!codebasePath.trim() || !userPrompt.trim()) {
      setError('Please provide both codebase path and prompt')
      return
    }

    if (selectedFiles.size === 0) {
      setError('Please select at least one file')
      return
    }

    setIsLoading(true)
    setError(null)
    setResult(null)

    try {
      const response = await fetch('/api/generate-with-files', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          codebase_path: codebasePath,
          user_prompt: userPrompt,
          selected_files: Array.from(selectedFiles)
        })
      })

      const data: GenerateResponse = await response.json()

      if (data.success && data.result) {
        setResult(data.result)
      } else {
        setError(data.error || 'Unknown error occurred')
      }
    } catch (err) {
      setError('Failed to connect to backend. Make sure the server is running.')
      console.error('Request failed:', err)
    } finally {
      setIsLoading(false)
    }
  }

  const copyToClipboard = async () => {
    if (result) {
      try {
        await navigator.clipboard.writeText(result)
        // Could add a toast notification here
      } catch (err) {
        console.error('Failed to copy:', err)
      }
    }
  }

  return (
    <div className="app">
      <header className="header">
        <div className="logo">
          <h1>🤖 MIOW-CONTEXT</h1>
          <p>Autonomous Code Context Generation</p>
        </div>
        {health && (
          <div className="health-status">
            <div className={`status-indicator ${health.qdrant_connected ? 'connected' : 'disconnected'}`}>
              Qdrant: {health.qdrant_connected ? '🟢' : '🔴'}
            </div>
            <div className={`status-indicator ${health.gemini_configured ? 'connected' : 'disconnected'}`}>
              Gemini: {health.gemini_configured ? '🟢' : '🔴'}
            </div>
          </div>
        )}
      </header>

      <main className="main">
        <div className="input-section">
          <div className="input-group">
            <label htmlFor="codebase-path">📁 Codebase Path</label>
            <div style={{ display: 'flex', gap: '8px' }}>
              <input
                id="codebase-path"
                type="text"
                placeholder="e.g., /path/to/your/project or ./current/folder"
                value={codebasePath}
                onChange={(e) => {
                  setCodebasePath(e.target.value)
                  setSignature(null)
                  setContext(null)
                }}
                className="input-field"
                style={{ flex: 1 }}
              />
              <button
                onClick={loadDebugInfo}
                disabled={!codebasePath.trim() || loadingDebug}
                className="debug-btn"
                title="Load project signature and context info"
              >
                {loadingDebug ? '⏳' : '🔍'}
              </button>
            </div>
            <small className="input-hint">
              Absolute path or relative path to your codebase
            </small>
          </div>

          <div className="input-group">
            <label htmlFor="user-prompt">💭 Your Task/Prompt</label>
            <textarea
              id="user-prompt"
              placeholder="e.g., Add user authentication with JWT tokens to my React app"
              value={userPrompt}
              onChange={(e) => setUserPrompt(e.target.value)}
              className="textarea-field"
              rows={4}
            />
            <small className="input-hint">
              Describe what you want to implement or ask about your codebase
            </small>
          </div>

          <div style={{ display: 'flex', gap: '12px', flexWrap: 'wrap' }}>
            <button
              onClick={loadRelevantFiles}
              disabled={loadingFiles || !health?.qdrant_connected || !health?.gemini_configured}
              className="secondary-btn"
            >
              {loadingFiles ? (
                <>
                  <div className="spinner"></div>
                  Loading Files...
                </>
              ) : (
                <>
                  📋 Load Relevant Files
                </>
              )}
            </button>
            
            <button
              onClick={handleGenerate}
              disabled={isLoading || !health?.qdrant_connected || !health?.gemini_configured}
              className="generate-btn"
            >
              {isLoading ? (
                <>
                  <div className="spinner"></div>
                  Generating Context...
                </>
              ) : (
                <>
                  🚀 Generate Context
                </>
              )}
            </button>
          </div>

          {!health?.qdrant_connected && (
            <div className="warning">
              ⚠️ Qdrant database not connected. Make sure it's running: docker-compose up -d
            </div>
          )}

          {!health?.gemini_configured && (
            <div className="warning">
              ⚠️ Gemini API key not configured. Set GEMINI_API_KEY environment variable.
            </div>
          )}

          {/* Debug Info Section */}
          {(signature || context) && (
            <div className="debug-section">
              <h3>🔍 Project Debug Info</h3>
              {signature && (
                <div className="debug-card">
                  <h4>📋 Project Signature</h4>
                  <div className="debug-grid">
                    <div><strong>Language:</strong> {signature.language || 'Unknown'}</div>
                    <div><strong>Framework:</strong> {signature.framework || 'Unknown'}</div>
                    <div><strong>Package Manager:</strong> {signature.package_manager || 'Unknown'}</div>
                    <div><strong>UI Library:</strong> {signature.ui_library || 'None'}</div>
                    <div><strong>Validation:</strong> {signature.validation_library || 'None'}</div>
                    <div><strong>Auth:</strong> {signature.auth_library || 'None'}</div>
                  </div>
                  {signature.description && (
                    <div style={{ marginTop: '8px', fontSize: '0.9em', color: '#666' }}>
                      {signature.description}
                    </div>
                  )}
                </div>
              )}
              {context && (
                <div className="debug-card">
                  <h4>📊 Context Statistics</h4>
                  <div className="debug-grid">
                    <div><strong>Total Symbols:</strong> {context.total_symbols.toLocaleString()}</div>
                    <div><strong>Total Files:</strong> {context.total_files.toLocaleString()}</div>
                    <div><strong>DB Path:</strong> <code style={{ fontSize: '0.85em' }}>{context.db_path}</code></div>
                    <div><strong>Collection:</strong> <code style={{ fontSize: '0.85em' }}>{context.collection_name}</code></div>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* File Selection Section */}
          {showFileSelection && relevantFiles.length > 0 && (
            <div className="file-selection-section">
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
                <h3>📁 Relevant Files ({selectedFiles.size} selected)</h3>
                <button
                  onClick={() => {
                    setShowFileSelection(false)
                    setSelectedFiles(new Set())
                  }}
                  className="close-btn"
                >
                  ✕
                </button>
              </div>
              <div className="file-list">
                {relevantFiles.map((file, idx) => (
                  <div
                    key={idx}
                    className={`file-item ${selectedFiles.has(file.file_path) ? 'selected' : ''}`}
                    onClick={() => toggleFileSelection(file.file_path)}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                      <input
                        type="checkbox"
                        checked={selectedFiles.has(file.file_path)}
                        onChange={() => toggleFileSelection(file.file_path)}
                        onClick={(e) => e.stopPropagation()}
                      />
                      <div style={{ flex: 1 }}>
                        <div style={{ fontWeight: 'bold', color: '#333' }}>
                          {file.file_path}
                        </div>
                        <div style={{ fontSize: '0.85em', color: '#666', marginTop: '4px' }}>
                          <span style={{ backgroundColor: '#e3f2fd', padding: '2px 6px', borderRadius: '4px', marginRight: '8px' }}>
                            {file.symbol_kind}
                          </span>
                          <span>Relevance: {(file.relevance_score * 100).toFixed(1)}%</span>
                        </div>
                        <div style={{ fontSize: '0.8em', color: '#999', marginTop: '4px', fontFamily: 'monospace' }}>
                          {file.preview}...
                        </div>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
              <button
                onClick={handleGenerateWithFiles}
                disabled={isLoading || selectedFiles.size === 0 || !health?.qdrant_connected || !health?.gemini_configured}
                className="generate-btn"
                style={{ marginTop: '16px', width: '100%' }}
              >
                {isLoading ? (
                  <>
                    <div className="spinner"></div>
                    Generating with Selected Files...
                  </>
                ) : (
                  <>
                    🚀 Generate with Selected Files ({selectedFiles.size})
                  </>
                )}
              </button>
            </div>
          )}
        </div>

        <div className="output-section">
          {error && (
            <div className="error-message">
              <h3>❌ Error</h3>
              <pre>{error}</pre>
            </div>
          )}

          {result && (
            <div className="result-container">
              <div className="result-header">
                <h3>✅ Generated Context</h3>
                <button onClick={copyToClipboard} className="copy-btn">
                  📋 Copy to Clipboard
                </button>
              </div>
              <pre className="result-content">{result}</pre>
            </div>
          )}
        </div>
      </main>

      <footer className="footer">
        <div className="features">
          <div className="feature">
            <h4>🧠 Autonomous</h4>
            <p>LLM-driven analysis with no hardcoded biases</p>
          </div>
          <div className="feature">
            <h4>🔍 Smart Search</h4>
            <p>Multi-agent workers with dependency resolution</p>
          </div>
          <div className="feature">
            <h4>🎯 Context-Aware</h4>
            <p>Token-optimized prompts for any LLM</p>
          </div>
          <div className="feature">
            <h4>⚡ Fast</h4>
            <p>Gemini 2.5 Flash + Qdrant vector search</p>
          </div>
        </div>
        <div className="copyright">
          <p>Made with ❤️ for developers who want smarter code generation</p>
        </div>
      </footer>
    </div>
  )
}

export default App
