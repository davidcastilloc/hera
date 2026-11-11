# ==============================================================================
#  🎧 HERA — Autonomous AI Music Curator & DJ Super-Node (Windows Native Pure-Rust Setup)
#  Usage: powershell -ExecutionPolicy Bypass -File .\install.ps1
# ==============================================================================

$ErrorActionPreference = "Stop"

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host " 🎧 HERA AI MUSIC CURATOR & DJ SUPER-NODE (Windows Native Setup)" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan

$HeraDir = $PSScriptRoot
if (-not $HeraDir -or -not (Test-Path (Join-Path $HeraDir "Cargo.toml"))) {
    $Candidate = Join-Path $HOME "Git-Repos\hera"
    if (Test-Path (Join-Path $Candidate "Cargo.toml")) {
        $HeraDir = $Candidate
    } else {
        $HeraDir = Join-Path $HOME "hera"
    }
}

Write-Host "[*] Directorio de Hera: $HeraDir" -ForegroundColor Green

if (-not (Test-Path (Join-Path $HeraDir ".git"))) {
    Write-Host "[*] Clonando repositorio oficial de Hera..." -ForegroundColor Yellow
    git clone https://github.com/davidcastilloc/hera.git $HeraDir
}

Set-Location $HeraDir

# Crear carpetas base
foreach ($sub in @("bin", "library", "sets", "quarantine\incomplete", "logs", "config", "exports")) {
    $p = Join-Path $HeraDir $sub
    if (-not (Test-Path $p)) {
        New-Item -ItemType Directory -Path $p -Force | Out-Null
    }
}

# 1. Verificar o instalar Rust minimal
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "[*] Instalando Rust toolchain minimal..." -ForegroundColor Yellow
    $RustupInit = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $RustupInit
    & $RustupInit -y --profile minimal
    $env:PATH = "$HOME\.cargo\bin;$env:PATH"
}

# 2. Detectar si existe linker MSVC (link.exe) o usar GNU
$HasMsvc = $null -ne (Get-Command link.exe -ErrorAction SilentlyContinue)
if (-not $HasMsvc) {
    $ToolchainList = rustup toolchain list
    if ($ToolchainList -notmatch "x86_64-pc-windows-gnu") {
        Write-Host "[*] MSVC no detectado: instalando toolchain autónomo x86_64-pc-windows-gnu..." -ForegroundColor Yellow
        rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
        rustup default stable-x86_64-pc-windows-gnu
    }
}

# 3. Resolver target_directory dinámicamente mediante cargo metadata
$TargetDirJson = cargo metadata --manifest-path hera-rs/Cargo.toml --format-version 1 --no-deps | ConvertFrom-Json
$TargetDir = $TargetDirJson.target_directory

Write-Host "[*] Compilando hera-node.exe optimizado con SIMD nativo (-C target-cpu=native)..." -ForegroundColor Cyan
$env:CARGO_INCREMENTAL = "0"
$env:RUSTFLAGS = "-C target-cpu=native"
cargo build --manifest-path hera-rs/Cargo.toml --release --locked --bin hera-node

$SourceBin = Join-Path $TargetDir "release\hera-node.exe"
$DestBin = Join-Path $HeraDir "bin\hera-node.exe"
Copy-Item -Path $SourceBin -Destination $DestBin -Force

Write-Host "`n🎉 ¡Hera-node compilado y verificado exitosamente en Windows en: $DestBin!" -ForegroundColor Green
