#!/usr/bin/env bash
set -euo pipefail

SOURCE_ROOT="${HOME}/.config/net.imput.helium"
SOURCE_PROFILE="Default"
TARGET_ROOT="${HOME}/.config/helium-lite/profile"
TARGET_PROFILE="Default"
MODE="bookmarks"
DRY_RUN=0

usage() {
  cat <<'EOF'
Usage: helium-import.sh [OPTIONS]

Safely import bookmarks (and optionally Favicons/History) from Helium into Helium Lite.

Options:
  --bookmarks-only   Copy Bookmarks and Bookmarks.bak only (default, safe while Helium runs)
  --all              Also copy Favicons and History (uses sqlite3 .backup if Helium is running)
  --source DIR       Helium config root (default: ~/.config/net.imput.helium)
  --target DIR       Helium Lite profile root (default: ~/.config/helium-lite/profile)
  --dry-run          Print actions without modifying files
  -h, --help         Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bookmarks-only) MODE="bookmarks"; shift ;;
    --all) MODE="all"; shift ;;
    --source) SOURCE_ROOT="$2"; shift 2 ;;
    --target) TARGET_ROOT="$2"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
  esac
done

SRC_DIR="${SOURCE_ROOT}/${SOURCE_PROFILE}"
DST_DIR="${TARGET_ROOT}/${TARGET_PROFILE}"

if [[ ! -d "${SRC_DIR}" ]]; then
  echo "Error: Helium profile directory not found at ${SRC_DIR}" >&2
  exit 1
fi

HELIUM_RUNNING=0
if [[ -L "${SOURCE_ROOT}/SingletonLock" ]] || pgrep -x "helium" >/dev/null 2>&1; then
  HELIUM_RUNNING=1
  echo "Note: Helium appears to be running (SingletonLock present)."
fi

if [[ -L "${TARGET_ROOT}/SingletonLock" ]]; then
  echo "Error: Helium Lite is currently running (${TARGET_ROOT}/SingletonLock exists). Close Helium Lite first." >&2
  exit 1
fi

TIMESTAMP="$(date +%Y%m%d-%H%M%S)"
BACKUP_DIR="${DST_DIR}/import-backup-${TIMESTAMP}"

copy_file_safe() {
  local name="$1"
  local src="${SRC_DIR}/${name}"
  local dst="${DST_DIR}/${name}"

  if [[ ! -f "${src}" ]]; then
    echo "Skip (missing): ${src}"
    return 0
  fi

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "[dry-run] Would copy ${src} -> ${dst}"
    return 0
  fi

  mkdir -p "${DST_DIR}"
  if [[ -f "${dst}" ]]; then
    mkdir -p "${BACKUP_DIR}"
    cp -a "${dst}" "${BACKUP_DIR}/${name}"
  fi
  cp -a "${src}" "${dst}"
  echo "Imported: ${name}"
}

copy_sqlite_safe() {
  local name="$1"
  local src="${SRC_DIR}/${name}"
  local dst="${DST_DIR}/${name}"

  if [[ ! -f "${src}" ]]; then
    echo "Skip (missing): ${src}"
    return 0
  fi

  if [[ "${DRY_RUN}" -eq 1 ]]; then
    echo "[dry-run] Would safely copy SQLite DB ${src} -> ${dst}"
    return 0
  fi

  mkdir -p "${DST_DIR}"
  if [[ -f "${dst}" ]]; then
    mkdir -p "${BACKUP_DIR}"
    cp -a "${dst}" "${BACKUP_DIR}/${name}"
  fi

  if [[ "${HELIUM_RUNNING}" -eq 1 ]]; then
    if command -v sqlite3 >/dev/null 2>&1; then
      sqlite3 "${src}" ".backup '${dst}'"
      echo "Imported (sqlite3 online backup): ${name}"
    else
      echo "Warning: Skipping ${name} because Helium is running and sqlite3 is not installed." >&2
    fi
  else
    cp -a "${src}" "${dst}"
    echo "Imported: ${name}"
  fi
}

echo "Source: ${SRC_DIR}"
echo "Target: ${DST_DIR}"

copy_file_safe "Bookmarks"
copy_file_safe "Bookmarks.bak"

if [[ "${MODE}" == "all" ]]; then
  copy_sqlite_safe "Favicons"
  copy_sqlite_safe "History"
fi

echo
echo "Done. Passwords/cookies are encrypted per-profile/keyring and are not copied;"
echo "if needed, export passwords as CSV from Helium settings and import in chrome://password-manager/settings."
