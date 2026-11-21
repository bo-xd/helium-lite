#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
CFG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/helium-lite"

chmod +x \
  "${REPO_ROOT}/launcher/helium-lite" \
  "${REPO_ROOT}/launcher/helium-lite-settings" \
  "${REPO_ROOT}/scripts/helium-import.sh" \
  "${REPO_ROOT}/scripts/install.sh"

mkdir -p "${BIN_DIR}" "${APP_DIR}" "${CFG_DIR}/profile"

ln -sfn "${REPO_ROOT}/launcher/helium-lite" "${BIN_DIR}/helium-lite"
ln -sfn "${REPO_ROOT}/launcher/helium-lite-settings" "${BIN_DIR}/helium-lite-settings"

sed "s|/home/bo/.local/bin|${BIN_DIR}|g" \
  "${REPO_ROOT}/launcher/helium-lite.desktop" > "${APP_DIR}/helium-lite.desktop"

if [[ ! -f "${CFG_DIR}/config.json" ]]; then
  cp "${REPO_ROOT}/launcher/config.default.json" "${CFG_DIR}/config.json"
  echo "Created default config: ${CFG_DIR}/config.json"
else
  echo "Keeping existing config: ${CFG_DIR}/config.json"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" >/dev/null 2>&1 || true
fi

echo "Installed:"
echo "  - ${BIN_DIR}/helium-lite"
echo "  - ${BIN_DIR}/helium-lite-settings"
echo "  - ${APP_DIR}/helium-lite.desktop"

if [[ ":${PATH}:" != *":${BIN_DIR}:"* ]]; then
  echo
  echo "Note: ${BIN_DIR} is not in your current PATH. Add it to your shell profile:"
  echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
fi

echo
echo "Optional: install system-wide managed low-RAM Chromium policy:"
echo "  sudo install -Dm644 \"${REPO_ROOT}/policies/low-ram-policy.json\" /etc/chromium/policies/managed/low-ram-policy.json"
