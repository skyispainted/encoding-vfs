# Package encoding-vfs for distribution
# Creates platform-specific archives containing encoding-vfs and git-wrapper

param(
    [switch]$Linux,
    [switch]$Windows,
    [switch]$All
)

$ErrorActionPreference = "Stop"

# Determine target platforms
$targets = @()
if ($All) {
    $targets = @("windows", "linux")
} elseif ($Linux) {
    $targets = @("linux")
} elseif ($Windows) {
    $targets = @("windows")
} else {
    # Default to current platform
    if ($IsLinux -or $env:OS -notlike "*Windows*") {
        $targets = @("linux")
    } else {
        $targets = @("windows")
    }
}

# Get version from Cargo.toml
$cargoToml = Get-Content "encoding-vfs-cli\Cargo.toml" | Out-String
$version = if ($cargoToml -match 'version\s*=\s*"([^"]+)"') { $matches[1] } else { "0.1.0" }

Write-Host "Packaging encoding-vfs v$version" -ForegroundColor Cyan
Write-Host "Targets: $($targets -join ', ')" -ForegroundColor Cyan
Write-Host ""

# Build first
Write-Host "Building..." -ForegroundColor Yellow
& .\build.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

# Create dist directory
$distDir = "dist"
if (Test-Path $distDir) {
    Remove-Item -Recurse -Force $distDir
}
New-Item -ItemType Directory -Path $distDir | Out-Null

foreach ($target in $targets) {
    Write-Host ""
    Write-Host "Packaging for $target..." -ForegroundColor Yellow

    $archiveName = "encoding-vfs-v$version-$target"

    if ($target -eq "windows") {
        # Windows package
        $pkgDir = Join-Path $distDir $archiveName
        New-Item -ItemType Directory -Path $pkgDir | Out-Null

        # Copy executables
        Copy-Item "target\release\encoding-vfs.exe" -Destination $pkgDir
        Copy-Item "git-wrapper\git.exe" -Destination $pkgDir

        # Copy scripts
        Copy-Item "install-git-wrapper.ps1" -Destination $pkgDir

        # Create README
        $readme = @"
Encoding VFS v$version (Windows)
================================

Contents:
  - encoding-vfs.exe    Main VFS program
  - git.exe             Transparent git wrapper
  - install-git-wrapper.ps1  Installation script

Quick Start:
  1. Mount a project:
     encoding-vfs.exe -b C:\your\project -d Y

  2. Install git wrapper (adds to PATH):
     .\install-git-wrapper.ps1

  3. Restart terminal and use git transparently:
     cd Y:\
     git status

For more info, see: https://github.com/your-repo/encoding-vfs
"@
        $readme | Out-File -FilePath (Join-Path $pkgDir "README.txt") -Encoding utf8

        # Create zip archive
        $zipPath = Join-Path $distDir "$archiveName.zip"
        Compress-Archive -Path "$pkgDir\*" -DestinationPath $zipPath -Force

        # Clean up temp directory
        Remove-Item -Recurse -Force $pkgDir

        Write-Host "Created: $zipPath" -ForegroundColor Green

    } elseif ($target -eq "linux") {
        # Linux package (create tar.gz structure)
        $pkgDir = Join-Path $distDir $archiveName
        New-Item -ItemType Directory -Path $pkgDir | Out-Null

        # Note: On Windows, we can't actually build Linux binaries
        # This is a placeholder structure for cross-compile or native Linux build
        $placeholder = @"
# Linux package structure

To build for Linux, run on a Linux system:
  cargo build --release

Then package:
  - target/release/encoding-vfs (main program)
  - target/release/git (git wrapper)

Or use cross-compile:
  cargo build --release --target x86_64-unknown-linux-gnu
"@
        $placeholder | Out-File -FilePath (Join-Path $pkgDir "BUILD.txt") -Encoding utf8

        Write-Host "Note: Linux binaries must be built on Linux or cross-compiled" -ForegroundColor Yellow
        Write-Host "Package structure created: $pkgDir" -ForegroundColor Yellow

        # Create tar.gz (using 7z if available, otherwise just zip)
        $tarPath = Join-Path $distDir "$archiveName.tar.gz"
        if (Get-Command 7z -ErrorAction SilentlyContinue) {
            Push-Location $distDir
            & 7z a -ttar "$archiveName.tar" $archiveName
            & 7z a -tgzip "$archiveName.tar.gz" "$archiveName.tar"
            Remove-Item "$archiveName.tar"
            Remove-Item -Recurse -Force $pkgDir
            Pop-Location
            Write-Host "Created: $tarPath" -ForegroundColor Green
        } else {
            # Fallback to zip
            $zipPath = Join-Path $distDir "$archiveName.zip"
            Compress-Archive -Path "$pkgDir\*" -DestinationPath $zipPath -Force
            Remove-Item -Recurse -Force $pkgDir
            Write-Host "Created: $zipPath (7z not found, used zip)" -ForegroundColor Yellow
        }
    }
}

Write-Host ""
Write-Host "Done! Archives in ${distDir}:" -ForegroundColor Cyan
Get-ChildItem $distDir | ForEach-Object {
    $size = [math]::Round($_.Length / 1MB, 2)
    Write-Host "  $($_.Name) ($size MB)" -ForegroundColor White
}
