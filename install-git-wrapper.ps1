# install-git-wrapper.ps1
# Install transparent git wrapper for encoding-vfs
# This makes git commands work transparently in VFS mounts

param(
    [switch]$Uninstall
)

$wrapperPath = "C:\projects\file-io-proxy\git-wrapper"

if ($Uninstall) {
    Write-Host "Uninstalling git wrapper..." -ForegroundColor Yellow

    # Remove from user PATH
    $currentPath = [System.Environment]::GetEnvironmentVariable('PATH', 'User')
    $pathParts = $currentPath -split ';'
    $newParts = $pathParts | Where-Object { $_ -ne $wrapperPath }
    $newPath = $newParts -join ';'

    [System.Environment]::SetEnvironmentVariable('PATH', $newPath, 'User')

    Write-Host "Git wrapper removed from PATH" -ForegroundColor Green
    Write-Host "Please restart your terminal for changes to take effect" -ForegroundColor Yellow
    exit 0
}

Write-Host "Installing transparent git wrapper for encoding-vfs..." -ForegroundColor Cyan
Write-Host ""

# Check if wrapper exists
if (-not (Test-Path "$wrapperPath\git.exe")) {
    Write-Host "Error: git.exe not found at $wrapperPath" -ForegroundColor Red
    Write-Host "Please run build.ps1 first to build the git wrapper" -ForegroundColor Yellow
    exit 1
}

# Check current PATH
$currentPath = [System.Environment]::GetEnvironmentVariable('PATH', 'User')
$pathParts = $currentPath -split ';'

if ($pathParts -contains $wrapperPath) {
    Write-Host "Git wrapper is already installed" -ForegroundColor Green
    Write-Host "Location: $wrapperPath" -ForegroundColor White
    exit 0
}

# Add to beginning of PATH (so it overrides system git)
$newPath = "$wrapperPath;$currentPath"
[System.Environment]::SetEnvironmentVariable('PATH', $newPath, 'User')

Write-Host "Git wrapper installed successfully!" -ForegroundColor Green
Write-Host ""
Write-Host "Location: $wrapperPath\git.exe" -ForegroundColor White
Write-Host "Added to: User PATH (beginning)" -ForegroundColor White
Write-Host ""
Write-Host "How it works:" -ForegroundColor Cyan
Write-Host "  - When you run 'git' in a VFS mount (e.g., Y:)" -ForegroundColor White
Write-Host "  - The wrapper reads ~/.encoding-vfs/mounts.json" -ForegroundColor White
Write-Host "  - Automatically redirects to source directory" -ForegroundColor White
Write-Host "  - Executes git command transparently" -ForegroundColor White
Write-Host ""
Write-Host "The mounts.json file is automatically maintained by encoding-vfs:" -ForegroundColor Cyan
Write-Host "  - Created when you mount a project" -ForegroundColor White
Write-Host "  - Updated when you unmount (Ctrl+C)" -ForegroundColor White
Write-Host "  - Stale entries are cleaned up on next mount" -ForegroundColor White
Write-Host ""
Write-Host "Please restart your terminal for changes to take effect" -ForegroundColor Yellow
Write-Host ""
Write-Host "Test with:" -ForegroundColor Cyan
Write-Host "  cd Y:/" -ForegroundColor White
Write-Host "  git status  # Should work transparently!" -ForegroundColor White
