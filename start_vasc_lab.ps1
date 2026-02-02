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

# 2.5 Ensure Correct Toolchain (Fix for missing VS Build Tools)
if (Test-Command "rustup") {
    Write-Host "Checking Rust Environment..." -ForegroundColor Gray
    
    # Check if link.exe exists (MSVC requirement)
    if (-not (Test-Command "link.exe")) {
        Write-Warning "Visual Studio Build Tools (link.exe) not found."
        Write-Host "Switching to GNU toolchain to avoid massive download..." -ForegroundColor Yellow
        
        # Install GNU toolchain
        rustup toolchain install stable-x86_64-pc-windows-gnu
        rustup default stable-x86_64-pc-windows-gnu
        
        Write-Host "Toolchain switched to GNU. Retrying setup..." -ForegroundColor Green
    }

    # Ensure WASM Target
    Write-Host "Ensuring WASM target..." -ForegroundColor Gray
    rustup target add wasm32-unknown-unknown
}

# 3. Start Relay Server (in new window)
Write-Host "Launching Relay Server..." -ForegroundColor Green
Start-Process powershell -ArgumentList "-NoExit", "-Command", "& { python relay_server/relay_server.py }"

# 4. Build and Start Frontend
Write-Host "Building Frontend (Diagnostic Mode)..." -ForegroundColor Cyan
Write-Host "Please wait. If this fails, we will see the error here." -ForegroundColor Gray

# FIX: Move build artifacts out of OneDrive to prevent "Access Denied" / Linker errors
$TempBuildDir = Join-Path $env:TEMP "vasc_lab_build"
Write-Host "Redirecting build artifacts to: $TempBuildDir" -ForegroundColor Gray
$env:CARGO_TARGET_DIR = $TempBuildDir

# Use local trunk if exists
$TrunkCmd = if (Test-Path "tools\trunk.exe") { "..\tools\trunk.exe" } else { "trunk" }

Set-Location frontend
try {
    # 0. Kill stale trunk processes (Fix for Access Denied)
    Get-Process trunk -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500

    # 1. Force clean dist directory
    if (Test-Path "dist") {
        Write-Host "Cleaning dist directory..." -ForegroundColor Gray
        Remove-Item -Path "dist" -Recurse -Force -ErrorAction SilentlyContinue
        # Retry once if failed
        if (Test-Path "dist") {
            Start-Sleep -Milliseconds 1000
            Remove-Item -Path "dist" -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    # Run build synchronously to check for errors
    if (Test-Path "..\tools\trunk.exe") {
        & "..\tools\trunk.exe" build
    }
    else {
        trunk build
    }
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Build Successful! Launching Server..." -ForegroundColor Green
        Start-Process powershell -ArgumentList "-NoExit", "-Command", "& { $TrunkCmd serve --address 127.0.0.1 --port 8080 --open }"
    }
    else {
        Write-Error "Build Failed. Please check the error messages above."
        Read-Host "Press Enter to exit..."
        exit 1
    }
}
catch {
    Write-Error "An error occurred during build: $_"
    Read-Host "Press Enter to exit..."
    exit 1
}
finally {
    Set-Location ..
}

Write-Host "System Launching..."
Write-Host "1. Relay Server window should appear."
Write-Host "2. Browser should open."

Read-Host "Press Enter to close this launcher..."
