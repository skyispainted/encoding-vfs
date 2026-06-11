# encoding-vfs build script
# Set proxy environment variables
$env:HTTP_PROXY = "http://127.0.0.1:20808"
$env:HTTPS_PROXY = "http://127.0.0.1:20808"

# Use nightly Rust with full paths
$nightlyCargo = "C:\Users\HuBochao\.rustup\toolchains\nightly-x86_64-pc-windows-msvc\bin\cargo.exe"
$nightlyRustc = "C:\Users\HuBochao\.rustup\toolchains\nightly-x86_64-pc-windows-msvc\bin\rustc.exe"

Write-Host "Building encoding-vfs with Rust nightly..." -ForegroundColor Cyan
Write-Host "Rust version: " -NoNewline
& $nightlyRustc --version

$env:RUSTC = $nightlyRustc
& $nightlyCargo build --release

if ($LASTEXITCODE -eq 0) {
    # Copy git.exe to git-wrapper directory
    Copy-Item "target\release\git.exe" "git-wrapper\git.exe" -Force
    Write-Host "Copied git.exe to git-wrapper directory" -ForegroundColor Gray

    Write-Host ""
    Write-Host "Build successful!" -ForegroundColor Green
    Write-Host "Executable location: target\release\encoding-vfs.exe" -ForegroundColor Yellow
    Write-Host "Git wrapper: git-wrapper\git.exe" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Usage:" -ForegroundColor Cyan
    Write-Host "  encoding-vfs.exe -b C:\legacy-project -d X" -ForegroundColor White
    Write-Host ""
    Write-Host "Default behavior: .git directory is automatically hidden" -ForegroundColor Green
    Write-Host ""
    Write-Host "Custom hidden rules (in config file):" -ForegroundColor Cyan
    Write-Host "  [encoding.filter]" -ForegroundColor White
    Write-Host "  hidden = ['.git/', 'node_modules/', '*.tmp']" -ForegroundColor White
} else {
    Write-Host ""
    Write-Host "Build failed, please check error messages" -ForegroundColor Red
}
