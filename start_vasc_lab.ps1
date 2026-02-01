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
        Write-Warning "Trunk not found in Cargo/Bin."
        
        # Create a local tools directory
        $ToolsDir = Join-Path $PWD "tools"
        if (-not (Test-Path $ToolsDir)) {
            New-Item -ItemType Directory -Force -Path $ToolsDir | Out-Null
        }
        $env:PATH += ";$ToolsDir"
        
        if (Test-Command "trunk") {
            Write-Host "Trunk found in local tools." -ForegroundColor Green
        }
        else {
            Write-Host "Downloading pre-built Trunk binary (to avoid compilation errors)..." -ForegroundColor Yellow
             
            # URL for Trunk v0.20.1 (Stable, Windows)
            $AppveyorUrl = "https://github.com/trunk-rs/trunk/releases/download/v0.20.1/trunk-x86_64-pc-windows-msvc.zip"
            $ZipPath = Join-Path $ToolsDir "trunk.zip"
             
            try {
                # Security Protocol fix for older PS versions
                [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

                Invoke-WebRequest -Uri $AppveyorUrl -OutFile $ZipPath
                Expand-Archive -Path $ZipPath -DestinationPath $ToolsDir -Force
                Remove-Item $ZipPath
                
                # Check again
                if (-not (Test-Path (Join-Path $ToolsDir "trunk.exe"))) {
                    throw "Extraction failed."
                }
                Write-Host "Trunk installed successfully to $ToolsDir" -ForegroundColor Green
            }
            catch {
                Write-Error "Failed to download trunk: $_"
                Write-Host "Manual Install: cargo install trunk"
                Read-Host "Press Enter to exit..."
                exit 1
            }
        }
    }
}

# 2.5 Ensure WASM Target
if (Test-Command "rustup") {
    Write-Host "Checking WASM target..." -ForegroundColor Gray
    rustup target add wasm32-unknown-unknown
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
