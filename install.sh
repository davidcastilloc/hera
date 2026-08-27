#!/usr/bin/env bash
# ==============================================================================
#  🎧 HERA — Autonomous AI Music Curator & DJ Super-Node Installer
#  Zero-Docker, Native Multi-Arch Installer (Linux x86_64, aarch64 / Raspberry Pi)
#  Usage: curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/install.sh | bash
# ==============================================================================

set -euo pipefail

# --- Colores y Estilos ---
BOLD='\033[1m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

clear 2>/dev/null || true

cat << "EOF"
  ██████╗  ██████╗  ██████╗  █████╗
  ██╔══██╗██╔═══██╗██╔══██╗██╔══██╗
  ██████╔╝██║   ██║██████╔╝███████║
  ██╔══██╗██║   ██║██╔══██╗██╔══██║
  ██║  ██║╚██████╔╝██████╔╝██║  ██║
  ╚═╝  ╚═╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═╝
  🎧 HERA AI MUSIC CURATOR & DJ SUPER-NODE
  «Todo es posible si lo imaginas: Zero-Docker, Zero-Friction»
EOF

echo -e "${CYAN}${BOLD}==> Iniciando instalacion nativa y ligera de Hera...${NC}\n"

# --- 1. Detección de Plataforma y Arquitectura ---
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

echo -e "${GREEN}[*]${NC} Sistema operativo detectado: ${BOLD}${OS}${NC}"
echo -e "${GREEN}[*]${NC} Arquitectura del procesador: ${BOLD}${ARCH}${NC}"

SLSKD_ARCH=""
RCLONE_ARCH=""

case "${ARCH}" in
  x86_64|amd64)
    SLSKD_ARCH="linux-x64"
    RCLONE_ARCH="linux-amd64"
    ;;
  aarch64|arm64)
    SLSKD_ARCH="linux-arm64"
    RCLONE_ARCH="linux-arm64"
    ;;
  armv7l|armhf|arm)
    SLSKD_ARCH="linux-arm"
    RCLONE_ARCH="linux-arm-v7"
    ;;
  *)
    echo -e "${RED}[!] Arquitectura no soportada directamente: ${ARCH}${NC}"
    exit 1
    ;;
esac

# --- 2. Directorio de Instalación ---
HERA_DIR="${HOME}/hera"
if [ -d ".git" ] && [ -f "pyproject.toml" ]; then
  HERA_DIR="$(pwd)"
  echo -e "${GREEN}[*]${NC} Ejecutando desde repositorio local: ${BOLD}${HERA_DIR}${NC}"
else
  echo -e "${GREEN}[*]${NC} Directorio de destino: ${BOLD}${HERA_DIR}${NC}"
fi

mkdir -p "${HERA_DIR}"/{bin,library,sets,quarantine/incomplete,logs,config,exports}

# --- 3. Dependencias del Sistema (FFmpeg, fpcalc, curl, git) ---
echo -e "\n${CYAN}${BOLD}==> [1/6] Verificando dependencias del sistema...${NC}"

install_sys_deps() {
  if command -v apt-get &>/dev/null; then
    echo -e "${GREEN}[*]${NC} Detectado gestor APT (Debian/Ubuntu/Raspberry Pi OS)..."
    sudo apt-get update -y || apt-get update -y || true
    sudo apt-get install -y curl unzip git ffmpeg libchromaprint-tools || apt-get install -y curl unzip git ffmpeg libchromaprint-tools
  elif command -v dnf &>/dev/null; then
    echo -e "${GREEN}[*]${NC} Detectado gestor DNF (Fedora/RHEL)..."
    sudo dnf install -y curl unzip git ffmpeg chromaprint-tools
  elif command -v pacman &>/dev/null; then
    echo -e "${GREEN}[*]${NC} Detectado gestor Pacman (Arch Linux)..."
    sudo pacman -Sy --noconfirm curl unzip git ffmpeg chromaprint
  elif command -v apk &>/dev/null; then
    echo -e "${GREEN}[*]${NC} Detectado gestor APK (Alpine)..."
    apk add --no-cache curl unzip git ffmpeg chromaprint
  else
    echo -e "${YELLOW}[!] Gestor de paquetes no reconocido. Asegurate de tener instalados: ffmpeg, fpcalc, curl, unzip, git.${NC}"
  fi
}

install_sys_deps

# --- 4. Instalador de Python Ultra-Rápido (uv) ---
echo -e "\n${CYAN}${BOLD}==> [2/6] Verificando entorno Python (uv)...${NC}"
if ! command -v uv &>/dev/null; then
  echo -e "${GREEN}[*]${NC} Instalando 'uv' (gestor de Python ultraligero sin tocar el sistema)..."
  curl -LsSf https://astral.sh/uv/install.sh | sh
  export PATH="${HOME}/.local/bin:${PATH}"
fi
echo -e "${GREEN}[OK]${NC} uv listo: $(uv --version)"

# --- 5. Descargar Binario Nativo de slskd (Soulseek Daemon) ---
echo -e "\n${CYAN}${BOLD}==> [3/6] Instalando demonio Soulseek P2P (slskd ${SLSKD_ARCH})...${NC}"
SLSKD_BIN="${HERA_DIR}/bin/slskd"

if [ ! -f "${SLSKD_BIN}" ]; then
  SLSKD_VERSION="0.26.0"
  SLSKD_URL="https://github.com/slskd/slskd/releases/download/${SLSKD_VERSION}/slskd-${SLSKD_VERSION}-${SLSKD_ARCH}.zip"
  echo -e "${GREEN}[*]${NC} Descargando binario oficial desde GitHub..."
  TMP_ZIP="$(mktemp /tmp/slskd_XXXXXX.zip)"
  curl -fsSL "${SLSKD_URL}" -o "${TMP_ZIP}"
  unzip -q -o "${TMP_ZIP}" -d "${HERA_DIR}/bin"
  rm -f "${TMP_ZIP}"
  chmod +x "${SLSKD_BIN}"
  echo -e "${GREEN}[OK]${NC} slskd instalado correctamente en ${SLSKD_BIN}"
else
  echo -e "${GREEN}[OK]${NC} slskd ya se encuentra instalado."
fi

# --- 6. Descargar Binario Nativo de rclone (Cloud Engine) ---
echo -e "\n${CYAN}${BOLD}==> [4/6] Instalando motor de almacenamiento en la nube (rclone)...${NC}"
RCLONE_BIN="${HERA_DIR}/bin/rclone"

if [ ! -f "${RCLONE_BIN}" ]; then
  RCLONE_VERSION="v1.69.1"
  RCLONE_URL="https://downloads.rclone.org/${RCLONE_VERSION}/rclone-${RCLONE_VERSION}-${RCLONE_ARCH}.zip"
  echo -e "${GREEN}[*]${NC} Descargando rclone (${RCLONE_ARCH})..."
  TMP_RCLONE_ZIP="$(mktemp /tmp/rclone_XXXXXX.zip)"
  curl -fsSL "${RCLONE_URL}" -o "${TMP_RCLONE_ZIP}"
  unzip -q -j -o "${TMP_RCLONE_ZIP}" "*/rclone" -d "${HERA_DIR}/bin" || true
  rm -f "${TMP_RCLONE_ZIP}"
  chmod +x "${RCLONE_BIN}" 2>/dev/null || true
  echo -e "${GREEN}[OK]${NC} rclone instalado en ${RCLONE_BIN}"
else
  echo -e "${GREEN}[OK]${NC} rclone ya se encuentra instalado."
fi

# --- 7. Clonar Código de Hera (si no existe) y Configurar Entorno ---
echo -e "\n${CYAN}${BOLD}==> [5/6] Configurando entorno y paquetes de Hera...${NC}"
cd "${HERA_DIR}"

if [ ! -f "pyproject.toml" ]; then
  echo -e "${GREEN}[*]${NC} Clonando repositorio Hera desde GitHub..."
  git clone https://github.com/davidcastilloc/hera.git /tmp/hera_repo
  cp -r /tmp/hera_repo/* "${HERA_DIR}/" || true
  rm -rf /tmp/hera_repo
fi

# Crear entorno virtual con uv
uv venv .venv --python 3.11 2>/dev/null || uv venv .venv
export VIRTUAL_ENV="${HERA_DIR}/.venv"
export PATH="${HERA_DIR}/.venv/bin:${PATH}"

# Instalar dependencias
echo -e "${GREEN}[*]${NC} Instalando dependencias de Python..."
uv pip install -e ".[analysis]" --quiet || uv pip install -e "." --quiet

# --- 8. Generar Configuración de slskd con Puertos Abiertos y 11 Salas ---
SLSKD_CFG="${HERA_DIR}/bin/slskd.yml"
echo -e "${GREEN}[*]${NC} Generando configuracion optimizada en ${SLSKD_CFG}..."

cat << EOF > "${SLSKD_CFG}"
soulseek:
  username: "hera_dj_2026"
  password: "HeraDjGlobal2026!"
  listen_ip_address: "0.0.0.0"
  listen_port: 2234
  description: |
    🤖 HERA — AI DJ & Music Curator (https://github.com/davidcastilloc/hera)
    ✨ Repertorio Curado Profesional por Agente de Inteligencia Artificial
    🎧 Masters en FLAC y MP3 320k | BPM, Camelot Key, LUFS y Energía Calibrados
    🚀 ¡Música 100% libre y abierta para todos los DJs y amantes de la música!
    📂 Carpetas: /library (Catálogo Master) | /sets (Crates y Sets Armónicos)

rooms:
  - "electronic"
  - "House"
  - "Techno"
  - "lossless"
  - "dance"
  - "deep house"
  - "edm"
  - "French Touch"
  - "DJs"
  - "Electro"
  - "trance"

web:
  port: 5030
  authentication:
    disabled: true

directories:
  downloads: "${HERA_DIR}/quarantine"
  incomplete: "${HERA_DIR}/quarantine/incomplete"

shares:
  directories:
    - "${HERA_DIR}/library"
    - "${HERA_DIR}/sets"

transfers:
  global:
    upload:
      slots: 100
      speed_limit: 0
EOF

mkdir -p "${HOME}/.local/share/slskd" "${HOME}/.config/slskd"
cp -f "${SLSKD_CFG}" "${HOME}/.config/slskd/slskd.yml" 2>/dev/null || true

# --- 9. Scripts de Gestión del Super-Nodo ---
echo -e "\n${CYAN}${BOLD}==> [6/6] Creando comandos de administracion...${NC}"

# Script hera-start
cat << 'EOF' > "${HERA_DIR}/bin/hera-start"
#!/usr/bin/env bash
HERA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${HERA_DIR}"
export PATH="${HERA_DIR}/.venv/bin:${HERA_DIR}/bin:${PATH}"

if pgrep -f "slskd" >/dev/null; then
  echo "[-] slskd ya se encuentra en ejecucion."
else
  echo "[+] Iniciando daemon Soulseek slskd en segundo plano..."
  nohup "${HERA_DIR}/bin/slskd" --app-dir "${HERA_DIR}/bin" > "${HERA_DIR}/logs/slskd.log" 2>&1 &
  sleep 2
  echo "[OK] slskd activo (Web UI: http://localhost:5030 | Puerto P2P: 2234)"
fi
EOF
chmod +x "${HERA_DIR}/bin/hera-start"

# Script hera-stop
cat << 'EOF' > "${HERA_DIR}/bin/hera-stop"
#!/usr/bin/env bash
echo "[-] Deteniendo servicios de Hera..."
pkill -f "slskd" 2>/dev/null && echo "[OK] slskd detenido." || echo "[-] slskd no estaba en ejecucion."
EOF
chmod +x "${HERA_DIR}/bin/hera-stop"

# Script hera-status
cat << 'EOF' > "${HERA_DIR}/bin/hera-status"
#!/usr/bin/env bash
HERA_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
echo "=== ESTADO DEL NODO HERA ==="
if pgrep -f "slskd" >/dev/null; then
  PID=$(pgrep -f "slskd" | head -n1)
  echo -e "\033[0;32m[ACTIVO]\033[0m slskd corriendo con PID ${PID}"
  curl -s http://localhost:5030/api/v0/application 2>/dev/null | grep -q "version" && echo "[OK] API Web operativa en http://localhost:5030" || true
else
  echo -e "\033[0;31m[DETENIDO]\033[0m slskd no esta corriendo. Usa '${HERA_DIR}/bin/hera-start' para iniciarlo."
fi
EOF
chmod +x "${HERA_DIR}/bin/hera-status"

# Enlazar al PATH del usuario si existe ~/.local/bin
mkdir -p "${HOME}/.local/bin"
ln -sf "${HERA_DIR}/bin/hera-start" "${HOME}/.local/bin/hera-start" 2>/dev/null || true
ln -sf "${HERA_DIR}/bin/hera-stop" "${HOME}/.local/bin/hera-stop" 2>/dev/null || true
ln -sf "${HERA_DIR}/bin/hera-status" "${HOME}/.local/bin/hera-status" 2>/dev/null || true

# --- Iniciar inmediatamente el servicio ---
"${HERA_DIR}/bin/hera-start"

IP_LOCAL="$(hostname -I 2>/dev/null | awk '{print $1}' || echo "127.0.0.1")"

echo -e "\n${GREEN}${BOLD}================================================================${NC}"
echo -e "${GREEN}${BOLD} 🎉 ¡INSTALACION COMPLETADA EXITOSAMENTE! (Zero-Docker)         ${NC}"
echo -e "${GREEN}${BOLD}================================================================${NC}"
echo -e "📍 Directorio:       ${BOLD}${HERA_DIR}${NC}"
echo -e "🌐 Panel Web:        ${BOLD}http://${IP_LOCAL}:5030${NC} (o http://localhost:5030)"
echo -e "🎧 Puerto Soulseek:  ${BOLD}0.0.0.0:2234${NC} (100% abierto para la comunidad)"
echo -e "💬 Salas Auto-Join:  ${BOLD}11 salas de musica electronica y DJing${NC}"
echo -e "\n${CYAN}Comandos rapidos:${NC}"
echo -e "  • Iniciar nodo:    ${BOLD}hera-start${NC} (o ${HERA_DIR}/bin/hera-start)"
echo -e "  • Ver estado:      ${BOLD}hera-status${NC}"
echo -e "  • Detener nodo:    ${BOLD}hera-stop${NC}"
echo -e "  • CLI de Hera:     ${BOLD}${HERA_DIR}/.venv/bin/hera --help${NC}"
echo -e "${GREEN}================================================================${NC}\n"
