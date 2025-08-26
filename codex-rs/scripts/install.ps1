# AGCodex Installation Script for Windows
# This script downloads and installs AGCodex from GitHub releases
# Run with: powershell -ExecutionPolicy Bypass -File install.ps1

param(
    [string]$Version = "latest",
    [switch]$Help
)

# Configuration
$RepoOwner = "agcodex"
$RepoName = "agcodex"
$BinaryName = "agcodex"
$ConfigDir = "$env:APPDATA\agcodex"

# Color functions
function Write-Info {
    Write-Host "[INFO]" -ForegroundColor Blue -NoNewline
    Write-Host " $args"
}

function Write-Success {
    Write-Host "[SUCCESS]" -ForegroundColor Green -NoNewline
    Write-Host " $args"
}

function Write-Error {
    Write-Host "[ERROR]" -ForegroundColor Red -NoNewline
    Write-Host " $args" -ForegroundColor Red
}

function Write-Warning {
    Write-Host "[WARNING]" -ForegroundColor Yellow -NoNewline
    Write-Host " $args"
}

function Show-Help {
    @"
AGCodex Installation Script for Windows

Usage: .\install.ps1 [OPTIONS] [VERSION]

OPTIONS:
    -Help          Show this help message
    -Version       Install specific version (default: latest)

EXAMPLES:
    .\install.ps1                    # Install latest version
    .\install.ps1 -Version v1.0.0    # Install specific version
    .\install.ps1 -Help              # Show help

ENVIRONMENT VARIABLES:
    AGCODEX_INSTALL_DIR  Override installation directory
    AGCODEX_CONFIG_DIR   Override configuration directory

"@
}

# Check if running as administrator
function Test-Administrator {
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

# Detect system architecture
function Get-SystemArchitecture {
    $arch = $env:PROCESSOR_ARCHITECTURE
    
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "x86" { return "x86" }
        "ARM64" { return "aarch64" }
        default {
            Write-Error "Unsupported architecture: $arch"
            exit 1
        }
    }
}

# Get latest release version from GitHub
function Get-LatestVersion {
    try {
        $apiUrl = "https://api.github.com/repos/$RepoOwner/$RepoName/releases/latest"
        $response = Invoke-RestMethod -Uri $apiUrl -Method Get
        return $response.tag_name
    }
    catch {
        Write-Error "Failed to fetch latest version from GitHub: $_"
        exit 1
    }
}

# Download binary from GitHub releases
function Download-Binary {
    param(
        [string]$Version,
        [string]$Architecture
    )
    
    # Remove 'v' prefix if present
    $Version = $Version.TrimStart('v')
    
    $platform = "windows-$Architecture"
    $downloadUrl = "https://github.com/$RepoOwner/$RepoName/releases/download/v$Version/$BinaryName-$Version-$platform.zip"
    $tempDir = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
    
    Write-Info "Downloading AGCodex v$Version for Windows ($Architecture)..."
    
    try {
        $zipPath = Join-Path $tempDir "$BinaryName.zip"
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -ErrorAction Stop
        
        # Download and verify checksum if available
        $checksumUrl = "$downloadUrl.sha256"
        $checksumPath = "$zipPath.sha256"
        
        try {
            Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -ErrorAction Stop
            Write-Info "Verifying checksum..."
            
            $expectedChecksum = (Get-Content $checksumPath).Split(' ')[0]
            $actualChecksum = (Get-FileHash $zipPath -Algorithm SHA256).Hash
            
            if ($expectedChecksum -eq $actualChecksum) {
                Write-Success "Checksum verified"
            }
            else {
                Write-Error "Checksum verification failed!"
                Remove-Item -Recurse -Force $tempDir
                exit 1
            }
        }
        catch {
            Write-Warning "Checksum file not found, skipping verification"
        }
        
        # Extract binary
        Write-Info "Extracting binary..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force
        
        $binaryPath = Join-Path $tempDir "$BinaryName.exe"
        
        if (-not (Test-Path $binaryPath)) {
            Write-Error "Binary not found in archive"
            Remove-Item -Recurse -Force $tempDir
            exit 1
        }
        
        return $tempDir
    }
    catch {
        Write-Error "Failed to download binary: $_"
        if (Test-Path $tempDir) {
            Remove-Item -Recurse -Force $tempDir
        }
        exit 1
    }
}

# Get installation directory
function Get-InstallDirectory {
    # Check for environment variable override
    if ($env:AGCODEX_INSTALL_DIR) {
        return $env:AGCODEX_INSTALL_DIR
    }
    
    # Default to Program Files if admin, otherwise user's local directory
    if (Test-Administrator) {
        $installDir = "$env:ProgramFiles\AGCodex"
    }
    else {
        $installDir = "$env:LOCALAPPDATA\Programs\AGCodex"
    }
    
    # Create directory if it doesn't exist
    if (-not (Test-Path $installDir)) {
        Write-Info "Creating installation directory: $installDir"
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }
    
    return $installDir
}

# Install the binary
function Install-Binary {
    param(
        [string]$TempDir,
        [string]$InstallDir
    )
    
    $sourcePath = Join-Path $TempDir "$BinaryName.exe"
    $targetPath = Join-Path $InstallDir "$BinaryName.exe"
    
    # Check if binary already exists
    if (Test-Path $targetPath) {
        Write-Warning "AGCodex is already installed at $targetPath"
        $response = Read-Host "Do you want to replace it? (y/N)"
        if ($response -ne 'y' -and $response -ne 'Y') {
            Write-Info "Installation cancelled"
            Remove-Item -Recurse -Force $TempDir
            exit 0
        }
        
        # Stop the process if it's running
        $process = Get-Process -Name $BinaryName -ErrorAction SilentlyContinue
        if ($process) {
            Write-Info "Stopping running AGCodex process..."
            Stop-Process -Name $BinaryName -Force
            Start-Sleep -Seconds 2
        }
    }
    
    # Copy binary
    Write-Info "Installing to $targetPath..."
    try {
        Copy-Item -Path $sourcePath -Destination $targetPath -Force
        Write-Success "Binary installed successfully"
    }
    catch {
        Write-Error "Failed to install binary: $_"
        Remove-Item -Recurse -Force $TempDir
        exit 1
    }
    
    # Clean up temp directory
    Remove-Item -Recurse -Force $TempDir
}

# Add to PATH
function Add-ToPath {
    param(
        [string]$InstallDir
    )
    
    $currentPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Info "Adding $InstallDir to PATH..."
        
        try {
            $newPath = "$currentPath;$InstallDir"
            [Environment]::SetEnvironmentVariable("Path", $newPath, [EnvironmentVariableTarget]::User)
            
            # Update current session
            $env:Path = $newPath
            
            Write-Success "Added to PATH successfully"
            Write-Warning "You may need to restart your terminal for PATH changes to take effect"
        }
        catch {
            Write-Warning "Failed to add to PATH automatically: $_"
            Write-Info "Please add '$InstallDir' to your PATH manually"
        }
    }
    else {
        Write-Info "$InstallDir is already in PATH"
    }
}

# Create Start Menu shortcut
function Create-StartMenuShortcut {
    param(
        [string]$InstallDir
    )
    
    try {
        $startMenuPath = [Environment]::GetFolderPath("StartMenu")
        $shortcutPath = Join-Path $startMenuPath "Programs\AGCodex.lnk"
        $targetPath = Join-Path $InstallDir "$BinaryName.exe"
        
        Write-Info "Creating Start Menu shortcut..."
        
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut($shortcutPath)
        $shortcut.TargetPath = $targetPath
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.Description = "AGCodex - AI Coding Assistant"
        $shortcut.IconLocation = $targetPath
        $shortcut.Save()
        
        Write-Success "Start Menu shortcut created"
    }
    catch {
        Write-Warning "Failed to create Start Menu shortcut: $_"
    }
}

# Setup configuration directory
function Setup-Configuration {
    Write-Info "Setting up configuration directory..."
    
    # Override with environment variable if set
    if ($env:AGCODEX_CONFIG_DIR) {
        $script:ConfigDir = $env:AGCODEX_CONFIG_DIR
    }
    
    # Create directories
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    New-Item -ItemType Directory -Path "$ConfigDir\agents" -Force | Out-Null
    New-Item -ItemType Directory -Path "$ConfigDir\history" -Force | Out-Null
    New-Item -ItemType Directory -Path "$ConfigDir\cache" -Force | Out-Null
    
    # Create default config if it doesn't exist
    $configFile = Join-Path $ConfigDir "config.toml"
    
    if (-not (Test-Path $configFile)) {
        Write-Info "Creating default configuration..."
        
        @'
# AGCodex Configuration File
# This file is automatically created during installation
# Edit this file to customize your AGCodex settings

[model]
provider = "openai"
name = "gpt-4"
temperature = 0.7
max_tokens = 4096

[tui]
theme = "dark"
history_limit = 100

[modes]
default = "build"

[search]
intelligence = "medium"  # light, medium, hard
chunk_size = 512

[cache]
enabled = true
max_size_mb = 500

[security]
sandbox_enabled = true
approval_mode = "auto"
'@ | Out-File -FilePath $configFile -Encoding UTF8
        
        Write-Success "Default configuration created"
    }
    else {
        Write-Info "Configuration file already exists, skipping..."
    }
}

# Verify installation
function Test-Installation {
    param(
        [string]$InstallDir
    )
    
    $binaryPath = Join-Path $InstallDir "$BinaryName.exe"
    
    if (Test-Path $binaryPath) {
        try {
            $version = & $binaryPath --version 2>$null
            if ($version) {
                Write-Success "AGCodex $version installed successfully!"
            }
            else {
                Write-Success "AGCodex installed successfully!"
            }
            
            Write-Info "Installation directory: $InstallDir"
            Write-Info "Configuration directory: $ConfigDir"
            Write-Info "Run 'agcodex --help' to get started"
        }
        catch {
            Write-Warning "Installation completed but could not verify version"
        }
    }
    else {
        Write-Error "Installation verification failed"
        exit 1
    }
}

# Main installation flow
function Main {
    Write-Host "╔═══════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "║     AGCodex Installation Script       ║" -ForegroundColor Cyan
    Write-Host "╚═══════════════════════════════════════╝" -ForegroundColor Cyan
    Write-Host ""
    
    # Check Windows version
    $osVersion = [System.Environment]::OSVersion.Version
    if ($osVersion.Major -lt 10) {
        Write-Warning "Windows 10 or later is recommended for optimal performance"
    }
    
    # Detect architecture
    $architecture = Get-SystemArchitecture
    Write-Info "Detected architecture: $architecture"
    
    # Get version to install
    if ($Version -eq "latest") {
        $Version = Get-LatestVersion
        Write-Info "Latest version: $Version"
    }
    else {
        Write-Info "Installing specified version: $Version"
    }
    
    # Download binary
    $tempDir = Download-Binary -Version $Version -Architecture $architecture
    
    # Get installation directory
    $installDir = Get-InstallDirectory
    
    # Install binary
    Install-Binary -TempDir $tempDir -InstallDir $installDir
    
    # Add to PATH
    Add-ToPath -InstallDir $installDir
    
    # Create Start Menu shortcut
    Create-StartMenuShortcut -InstallDir $installDir
    
    # Setup configuration
    Setup-Configuration
    
    # Verify installation
    Test-Installation -InstallDir $installDir
    
    Write-Host ""
    Write-Host "╔═══════════════════════════════════════╗" -ForegroundColor Green
    Write-Host "║    Installation Complete! 🎉          ║" -ForegroundColor Green
    Write-Host "╚═══════════════════════════════════════╝" -ForegroundColor Green
}

# Script entry point
if ($Help) {
    Show-Help
    exit 0
}

# Check for updates to PowerShell
if ($PSVersionTable.PSVersion.Major -lt 5) {
    Write-Warning "PowerShell 5.0 or later is recommended for this installer"
    $response = Read-Host "Continue anyway? (y/N)"
    if ($response -ne 'y' -and $response -ne 'Y') {
        exit 0
    }
}

# Run main installation
try {
    Main
}
catch {
    Write-Error "Installation failed: $_"
    exit 1
}