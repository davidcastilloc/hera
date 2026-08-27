#!/usr/bin/env bash
# ==============================================================================
#  🎧 HERA — Desinstalador Limpio y Completo (KISS)
#  Usage: curl -fsSL https://raw.githubusercontent.com/davidcastilloc/hera/main/uninstall.sh | bash
# ==============================================================================

set -euo pipefail

BOLD='\033[1m'
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "\n${RED}${BOLD}==> [HERA UNINSTALL] Iniciando desinstalacion completa de Hera...${NC}\n"

# 1. Detener y deshabilitar servicios SystemD
if command -v systemctl &>/dev/null; then
  echo -e "${YELLOW}[*]${NC} Deteniendo y deshabilitando servicio hera.service..."
  sudo systemctl stop hera 2>/dev/null || systemctl stop hera 2>/dev/null || true
  sudo systemctl disable hera.service 2>/dev/null || systemctl disable hera.service 2>/dev/null || true
  sudo rm -f /etc/systemd/system/hera.service 2>/dev/null || true
  sudo systemctl daemon-reload 2>/dev/null || true
fi

# 2. Matar procesos huérfanos de slskd
echo -e "${YELLOW}[*]${NC} Finalizando procesos activos..."
pkill -9 -f "slskd" 2>/dev/null || true

# 3. Remover enlaces simbolicos en ~/.local/bin
echo -e "${YELLOW}[*]${NC} Eliminando accesos directos..."
rm -f "${HOME}/.local/bin/hera-start" 2>/dev/null || true
rm -f "${HOME}/.local/bin/hera-stop" 2>/dev/null || true
rm -f "${HOME}/.local/bin/hera-status" 2>/dev/null || true
rm -f "${HOME}/.local/bin/hera-update" 2>/dev/null || true
rm -f "${HOME}/.local/bin/hera-uninstall" 2>/dev/null || true

# 4. Remover configuraciones de slskd de usuario
echo -e "${YELLOW}[*]${NC} Limpiando configuraciones residuales..."
rm -rf "${HOME}/.config/slskd" 2>/dev/null || true
rm -rf "${HOME}/.local/share/slskd" 2>/dev/null || true

# 5. Remover directorio de instalacion (~/hera)
HERA_DIR="${HOME}/hera"
if [ -d "${HERA_DIR}" ]; then
  echo -e "${YELLOW}[*]${NC} Eliminando directorio de instalacion ${HERA_DIR}..."
  rm -rf "${HERA_DIR}"
fi

echo -e "\n${GREEN}${BOLD}================================================================${NC}"
echo -e "${GREEN}${BOLD} 🎉 HERA ha sido completamente desinstalado de este sistema.   ${NC}"
echo -e "${GREEN}${BOLD}================================================================${NC}\n"
