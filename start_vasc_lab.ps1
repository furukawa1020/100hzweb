# VASC-LAB System Launcher
Write-Host "Starting VASC-LAB System..." -ForegroundColor Cyan

# Function to check command existence
function Test-Command ($cmd) {
    return [bool](Get-Command $cmd -ErrorAction SilentlyContinue)
}

# 1. Check Python
if (-not (Test-Command "python")) {
    Write-Error "Python not found. Please install Python."
    Read-Host "Press Enter to exit..."
    exit 1
}

# 2. Check/Install Trunk
if (-not (Test-Command "trunk")) {
    # Try adding common cargo path first
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    
    if (-not (Test-Command "trunk")) {
        Write-Warning "Trunk not found."
        if (Test-Command "cargo") {
            Write-Host "Cargo found. Attempting to install Trunk automatically..." -ForegroundColor Yellow
            Write-Host "(This make take 1-2 minutes to compile)" -ForegroundColor Gray
            cargo install trunk
            
            # Re-check
            if (-not (Test-Command "trunk")) {
                Write-Error "Trunk install failed. Please run 'cargo install trunk' manually."
                Read-Host "Press Enter to exit..."
                exit 1
            }
        }
        else {
            Write-Error "Cargo (Rust) not found. Please install Rust first: https://rustup.rs/"
            Read-Host "Press Enter to exit..."
            exit 1
        }
    }
}

# 3. Start Relay Server (in new window)
Write-Host "Launching Relay Server..." -ForegroundColor Green
Start-Process powershell -ArgumentList "-NoExit", "-Command", "& { python relay_server/relay_server.py }"

# 4. Start Frontend (in new window)
Write-Host "Launching Frontend..." -ForegroundColor Green
Start-Process powershell -ArgumentList "-NoExit", "-Command", "& { cd frontend; trunk serve --open }"

Write-Host "System Launching..."
Write-Host "1. Relay Server window should appear."
Write-Host "2. Frontend window should appear and open Browser."

Read-Host "Press Enter to close this launcher..."
