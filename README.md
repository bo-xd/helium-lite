# Helium Lite

A low-RAM **Ungoogled Chromium** launcher, configuration suite, and custom MV3 adblocker designed to replace heavier Chromium setups while preserving VA-API video acceleration, tab sleeping, and easy profile migration from Helium.

## Directory Layout

- `launcher/helium-lite` — Main launcher script; reads `~/.config/helium-lite/config.json` and builds tuned Chromium flags.
- `launcher/helium-lite-settings` — Interactive CLI/TUI (and scriptable CLI) to toggle features on/off.
- `launcher/config.default.json` — Default configuration template.
- `launcher/helium-lite.desktop` — Desktop entry for application menus.
- `policies/low-ram-policy.json` — Chromium managed policy template for Memory Saver and telemetry/background suppression.
- `scripts/helium-import.sh` — Safe bookmark/favicon/history importer from Helium (`~/.config/net.imput.helium/Default`).
- `scripts/install.sh` — Installs CLI symlinks into `~/.local/bin`, the `.desktop` entry, and initializes `~/.config/helium-lite/config.json`.
- `adblocker/` — Custom ultra-low-RAM Manifest V3 adblocker loaded via `--load-extension`.

## Quick Start

1. Install Ungoogled Chromium (e.g., on Arch Linux: `flatpak install flathub io.github.ungoogled_software.ungoogled_chromium` or `yay -S ungoogled-chromium-bin`). When using Flatpak, `helium-lite` automatically passes `--filesystem` flags for `~/.config/helium-lite`, `adblocker/`, and `WidevineCdm`.
2. Run the installer:
   ```bash
   ./scripts/install.sh
   ```
3. (Optional) Import bookmarks from Helium:
   ```bash
   ./scripts/helium-import.sh
   # Or include Favicons and History:
   ./scripts/helium-import.sh --all
   ```
4. Launch the browser:
   ```bash
   helium-lite
   ```

## Configuration (`~/.config/helium-lite/config.json`)

Manage settings interactively with:
```bash
helium-lite-settings
# or:
helium-lite --settings
```

Non-interactive CLI examples:
```bash
helium-lite-settings --list
helium-lite-settings --toggle widevine_drm
helium-lite-settings --set renderer_process_limit=6 --set v8_max_old_space_mb=768
```

### Feature Toggles

| Key | Default | Effect |
| --- | --- | --- |
| `tab_sleeping` | `true` | Enables Memory Saver (`HighEfficiencyModeAvailable`, `MemorySaverModeAvailable`), caps renderer processes (`--renderer-process-limit`), caps V8 heap (`--js-flags=--max-old-space-size`), and disables background networking, telemetry, prefetching, and speculation rules. |
| `hardware_accel` | `true` | Enables GPU rasterization, zero-copy, Wayland/X11 auto Ozone platform, WebGL, and VA-API hardware video decoding/encoding (`VaapiVideoDecoder`, `VaapiVideoEncoder`, `VaapiVideoDecodeLinuxGL`). |
| `widevine_drm` | `false` | Symlinks `WidevineCdm` (from `/usr/lib/chromium/WidevineCdm`, `~/.config/sh.cider.genten/WidevineCdm`, or Helium) into the Helium Lite profile when enabled, and removes the symlink when disabled. |
| `adblocker` | `true` | Loads `adblocker/` via `--load-extension`. |
| `extension_support` | `true` | Enables Chrome Web Store `.crx` install prompts (`--extension-mime-request-handling=always-prompt-for-install`). When `false`, disables other extensions while keeping the built-in adblocker if enabled. |

### Launcher Options

- `helium-lite --dry-run` (or `--print-cmd`) — Print the resolved browser command and flags without launching.
- `helium-lite --settings` — Open `helium-lite-settings`.
- `helium-lite --import` — Run `scripts/helium-import.sh`.

## Managed Policy (Optional)

To enforce Memory Saver and disable background mode and reporting at the policy level:
```bash
sudo install -Dm644 policies/low-ram-policy.json /etc/chromium/policies/managed/low-ram-policy.json
```
