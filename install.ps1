# ==============================================================================
#  🎧 HERA — Autonomous AI Music Curator & DJ Super-Node (Windows Installer & Auto-Updater)
#  Usage: powershell -ExecutionPolicy Bypass -File .\install.ps1
# ==============================================================================

$ErrorActionPreference = "Stop"

Write-Host "================================================================" -ForegroundColor Cyan
Write-Host " 🎧 HERA AI MUSIC CURATOR & DJ SUPER-NODE (Windows Auto-Setup)" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan

$HeraDir = $PSScriptRoot
if (-not $HeraDir -or -not (Test-Path (Join-Path $HeraDir "pyproject.toml"))) {
    $Candidate = Join-Path $HOME "Git-Repos\hera"
    if (Test-Path (Join-Path $Candidate "pyproject.toml")) {
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

# Verificar uv
if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    Write-Host "[*] Instalando gestor Python 'uv'..." -ForegroundColor Yellow
    powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"
}

# Crear o actualizar .venv
$VenvPy = Join-Path $HeraDir ".venv\Scripts\python.exe"
if (-not (Test-Path $VenvPy)) {
    Write-Host "[*] Creando entorno virtual .venv..." -ForegroundColor Yellow
    uv venv .venv --python 3.11
}

Write-Host "[*] Instalando/actualizando paquetes de Hera..." -ForegroundColor Green
uv pip install -e . --quiet

# Ejecutar Paso 0: Auto-Verificador y Compilador de Binario Rust (hera-node.exe) + Sincronizador de Skills
$EnsureScript = Join-Path $HeraDir "scripts\ensure_latest_hera.py"
if (Test-Path $EnsureScript) {
    Write-Host "[*] Verificando última versión de Git, compilando binario Rust (hera-node.exe) y sincronizando Skills..." -ForegroundColor Cyan
    & $VenvPy $EnsureScript
}

Write-Host "`n🎉 ¡Hera verificado y actualizado exitosamente en Windows!" -ForegroundColor Green
