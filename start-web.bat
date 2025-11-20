@echo off
set GEMINI_API_KEY=AIzaSyBv13RxXxTLZlx-UqXA_qsYY6-ecRnU2rM
REM MIOW-Context Web UI Launcher for Windows
REM Starts both backend and frontend servers

echo 🚀 Starting MIOW-Context Web Interface
echo ═══════════════════════════════════════════════

REM Check if required environment variables are set
if "%GEMINI_API_KEY%"=="" (
    echo ❌ GEMINI_API_KEY environment variable not set
    echo Please set it with: set GEMINI_API_KEY=your_key_here
    goto :error
)

REM Check if Qdrant is running
echo 📊 Checking Qdrant...
curl -s http://localhost:6333/collections >nul 2>&1
if errorlevel 1 (
    echo ❌ Qdrant is not running
    echo Please start it with: docker-compose up -d
    goto :error
)
echo ✅ Qdrant is running

REM Start backend server in background
echo 🔧 Starting Rust backend server...
start /B cargo run --features web -- serve --port 3001

REM Wait a moment for backend to start
timeout /t 3 /nobreak >nul

REM Check if backend started successfully
curl -s http://localhost:3001/api/health >nul 2>&1
if errorlevel 1 (
    echo ❌ Backend failed to start
    goto :error
)
echo ✅ Backend running at http://localhost:3001

REM Start frontend server
echo 🎨 Starting React frontend...
cd web
start /B npm run dev

REM Wait a moment for frontend to start
timeout /t 5 /nobreak >nul

echo.
echo 🎉 Both servers started successfully!
echo 🌐 Frontend: http://localhost:5173
echo 🔧 Backend API: http://localhost:3001/api
echo.
echo Press any key to stop both servers
pause >nul

goto :eof

:error
echo.
echo ❌ Failed to start servers
pause
exit /b 1
