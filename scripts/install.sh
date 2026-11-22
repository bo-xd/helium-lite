#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

BIN_DIR="${HOME}/.local/bin"
APP_DIR="${HOME}/.local/share/applications"
ICON_SCALABLE_DIR="${HOME}/.local/share/icons/hicolor/scalable/apps"
ICON_128_DIR="${HOME}/.local/share/icons/hicolor/128x128/apps"
DATA_DIR="${HOME}/.local/share/kestrel"
ADBLOCKER_DST="${DATA_DIR}/adblocker"
UI_DST="${DATA_DIR}/ui"
RUNTIME_BIN="${DATA_DIR}/runtime/chrome"
CFG_DIR="${XDG_CONFIG_HOME:-${HOME}/.config}/kestrel"
CACHE_DIR="${XDG_CACHE_HOME:-${HOME}/.cache}/kestrel"

SET_DEFAULT=0
SYSTEM_BROWSER=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --set-default) SET_DEFAULT=1; shift ;;
    --system-browser) SYSTEM_BROWSER=1; shift ;;
    -h|--help)
      echo "Usage: install.sh [--set-default] [--system-browser]"
      exit 0
      ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

echo "Building Kestrel launcher (Rust release)..."
cargo build --release --manifest-path "${REPO_ROOT}/launcher/Cargo.toml"

chmod +x \
  "${REPO_ROOT}/scripts/kestrel-import" \
  "${REPO_ROOT}/scripts/install.sh"

# Remove legacy helium-lite installation artifacts if present
rm -f "${BIN_DIR}/helium-lite" "${BIN_DIR}/helium-lite-settings"
rm -f "${APP_DIR}/helium-lite.desktop"
rm -rf "${XDG_CONFIG_HOME:-${HOME}/.config}/helium-lite"

mkdir -p "${BIN_DIR}" "${APP_DIR}" "${ICON_SCALABLE_DIR}" "${ICON_128_DIR}" "${DATA_DIR}" "${CFG_DIR}/profile" "${CACHE_DIR}"

install -Dm755 "${REPO_ROOT}/launcher/target/release/kestrel" "${BIN_DIR}/kestrel"
ln -sfn "${BIN_DIR}/kestrel" "${BIN_DIR}/kestrel-settings"
ln -sfn "${BIN_DIR}/kestrel" "${BIN_DIR}/kestrel-native-host"
ln -sfn "${REPO_ROOT}/scripts/kestrel-import" "${BIN_DIR}/kestrel-import"

# Install self-contained Kestrel Shield adblocker extension
rm -rf "${ADBLOCKER_DST}"
mkdir -p "${ADBLOCKER_DST}"
cp -a \
  "${REPO_ROOT}/adblocker/manifest.json" \
  "${REPO_ROOT}/adblocker/background.js" \
  "${REPO_ROOT}/adblocker/content.js" \
  "${REPO_ROOT}/adblocker/popup.html" \
  "${REPO_ROOT}/adblocker/popup.css" \
  "${REPO_ROOT}/adblocker/popup.js" \
  "${REPO_ROOT}/adblocker/options.html" \
  "${REPO_ROOT}/adblocker/options.js" \
  "${REPO_ROOT}/adblocker/rules" \
  "${REPO_ROOT}/adblocker/icons" \
  "${ADBLOCKER_DST}/"

# Install Kestrel custom theme & newtab UI if present
if [[ -d "${REPO_ROOT}/ui" ]]; then
  rm -rf "${UI_DST}"
  mkdir -p "${UI_DST}"
  cp -a "${REPO_ROOT}/ui/." "${UI_DST}/"
fi

# Install standalone Ungoogled Chromium runtime unless --system-browser is requested
if [[ "${SYSTEM_BROWSER}" -eq 0 && ! -x "${RUNTIME_BIN}" ]]; then
  "${BIN_DIR}/kestrel" install-runtime
fi

cp "${REPO_ROOT}/assets/kestrel.svg" "${ICON_SCALABLE_DIR}/kestrel.svg"
if [[ -f "${REPO_ROOT}/assets/kestrel.png" ]]; then
  cp "${REPO_ROOT}/assets/kestrel.png" "${ICON_128_DIR}/kestrel.png"
fi

sed "s|/home/bo/.local/bin|${BIN_DIR}|g" \
  "${REPO_ROOT}/launcher/kestrel.desktop" > "${APP_DIR}/kestrel.desktop"

if [[ ! -f "${CFG_DIR}/config.json" ]]; then
  cp "${REPO_ROOT}/launcher/config.default.json" "${CFG_DIR}/config.json"
  echo "Created default config: ${CFG_DIR}/config.json"
else
  echo "Keeping existing config: ${CFG_DIR}/config.json"
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "${APP_DIR}" >/dev/null 2>&1 || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "${HOME}/.local/share/icons/hicolor" >/dev/null 2>&1 || true
fi

if [[ "${SET_DEFAULT}" -eq 1 ]] && command -v xdg-settings >/dev/null 2>&1; then
  xdg-settings set default-web-browser kestrel.desktop
  echo "Set Kestrel (kestrel.desktop) as default web browser."
fi

echo "Installed:"
echo "  - ${BIN_DIR}/kestrel (native Rust binary)"
echo "  - ${BIN_DIR}/kestrel-settings -> ${BIN_DIR}/kestrel"
echo "  - ${BIN_DIR}/kestrel-import"
echo "  - ${ADBLOCKER_DST}/ (Kestrel Shield extension)"
if [[ -x "${RUNTIME_BIN}" ]]; then
  echo "  - ${RUNTIME_BIN} (standalone runtime)"
fi
echo "  - ${ICON_SCALABLE_DIR}/kestrel.svg"
echo "  - ${ICON_128_DIR}/kestrel.png"
echo "  - ${APP_DIR}/kestrel.desktop"

if [[ ":${PATH}:" != *":${BIN_DIR}:"* ]]; then
  echo
  echo "Note: ${BIN_DIR} is not in your current PATH. Add it to your shell profile:"
  echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
fi

echo
echo "Optional: install system-wide managed low-RAM Chromium policy:"
echo "  sudo install -Dm644 \"${REPO_ROOT}/policies/low-ram-policy.json\" /etc/chromium/policies/managed/low-ram-policy.json"
