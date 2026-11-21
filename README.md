# Kestrel

**Kestrel** is a **100% standalone**, independent, low-RAM desktop browser powered by Ungoogled Chromium. Cloning and running `./scripts/install.sh` gives you a complete browser with:
- A zero-dependency native **Rust** launcher & settings manager (`kestrel`, `kestrel-settings`),
- A self-contained portable Ungoogled Chromium runtime installed to `~/.local/share/kestrel/runtime/chrome` (no Flatpak or AUR required),
- A bundled Manifest V3 adblocker (**Kestrel Shield**) installed to `~/.local/share/kestrel/adblocker/`,
- Isolated profile (`~/.config/kestrel/profile`), cache (`~/.cache/kestrel`), and window class (`--class=Kestrel`),
- Aggressive tab sleeping and VA-API hardware video acceleration.

## Directory Layout

- `launcher/Cargo.toml` & `launcher/src/main.rs` — Zero-dependency native Rust binary providing both `kestrel` (launcher) and `kestrel-settings` (TUI + CLI settings manager).
- `launcher/config.default.json` — Default configuration template (`~/.config/kestrel/config.json`).
- `launcher/kestrel.desktop` — Desktop entry (`Name=Kestrel`, `StartupWMClass=Kestrel`, `Icon=kestrel`, HTTP/HTTPS handlers, and desktop actions).
- `assets/kestrel.svg` & `assets/kestrel.png` — Custom Kestrel browser icon.
- `policies/low-ram-policy.json` — Chromium managed policy template for Memory Saver and telemetry/background suppression.
- `scripts/kestrel-import` — Safe bookmark/favicon/history importer (defaults to importing from `~/.config/net.imput.helium/Default`).
- `scripts/install.sh` — Builds `cargo build --release`, installs `kestrel`, `kestrel-settings`, `kestrel-import`, icons, `.desktop` entry, and supports `--set-default`.
- `adblocker/` — Custom ultra-low-RAM Manifest V3 adblocker (**Kestrel Shield**) loaded via `--load-extension`.

## Quick Start

1. Build & install Kestrel (requires `rustc`/`cargo`, `curl`, `tar`, `xz`):
   ```bash
   ./scripts/install.sh
   # Or register Kestrel as your default web browser too:
   ./scripts/install.sh --set-default
   # Or skip downloading the standalone runtime if using system/Flatpak Ungoogled Chromium:
   ./scripts/install.sh --system-browser
   ```
   By default, `install.sh` compiles the Rust launcher, copies Kestrel Shield to `~/.local/share/kestrel/adblocker/`, and runs `kestrel install-runtime` to fetch the latest portable Ungoogled Chromium runtime into `~/.local/share/kestrel/runtime/chrome`.
3. (Optional) Import bookmarks from an existing Chromium/Helium profile:
   ```bash
   kestrel-import
   # Or include Favicons and History:
   kestrel-import --all
   ```
4. Launch Kestrel:
   ```bash
   kestrel
   ```

## Configuration (`~/.config/kestrel/config.json`)

Manage settings interactively with:
```bash
kestrel-settings
# or:
kestrel settings
# or:
kestrel --settings
```

Non-interactive CLI examples:
```bash
kestrel-settings --list
kestrel-settings --toggle widevine_drm
kestrel-settings --set renderer_process_limit=6 --set v8_max_old_space_mb=768
```

### Feature Toggles

| Key | Default | Effect |
| --- | --- | --- |
| `search_engine` | `"duckduckgo"` | Default omnibox search engine (`duckduckgo`, `brave`, `startpage`, `google`), auto-configured in `Web Data` so address-bar queries never fall back to `No Search` (`http://<query>/`). |
| `tab_sleeping` | `true` | Enables Memory Saver (`HighEfficiencyModeAvailable`, `MemorySaverModeAvailable`), caps renderer processes (`--renderer-process-limit`), caps V8 heap (`--js-flags=--max-old-space-size`), and disables background networking, telemetry, prefetching, and speculation rules. |
| `hardware_accel` | `true` | Enables GPU rasterization, zero-copy, Wayland/X11 auto Ozone platform, WebGL, and VA-API hardware video decoding/encoding (`VaapiVideoDecoder`, `VaapiVideoEncoder`, `VaapiVideoDecodeLinuxGL`). |
| `widevine_drm` | `false` | Symlinks `WidevineCdm` (from `~/.local/share/kestrel/runtime/WidevineCdm`, `/usr/lib/chromium/WidevineCdm`, `~/.config/sh.cider.genten/WidevineCdm`, or Helium) into the Kestrel profile when enabled, and removes the symlink when disabled. |
| `adblocker` | `true` | Loads `adblocker/` via `--load-extension`. |
| `extension_support` | `true` | Enables Chrome Web Store `.crx` install prompts (`--extension-mime-request-handling=always-prompt-for-install`). When `false`, disables other extensions while keeping the built-in adblocker if enabled. |

### Launcher Options

- `kestrel install-runtime` — Download/update the standalone portable Ungoogled Chromium runtime to `~/.local/share/kestrel/runtime/`.
- `kestrel --auto-runtime` — Automatically download the standalone runtime on launch if none is found.
- `kestrel --dry-run` (or `--print-cmd`) — Print the resolved browser command and flags without launching.
- `kestrel settings` / `kestrel --settings` — Open `kestrel-settings`.
- `kestrel --import` — Run `kestrel-import`.

## Custom Source Build (Optional)

To compile your own stripped-down, low-RAM **Ungoogled Chromium `158.0.8127.36-1`** runtime from source on Arch Linux (with `browser/args.gn` and any custom patches in `browser/patches/*.patch`):

1. Check required build dependencies:
   ```bash
   ./scripts/build-browser.sh --check-deps
   ```
2. Fetch, patch, compile (requires ~100GB+ under `/home/bo/Projects/kestrel-build`), and install into `~/.local/share/kestrel/runtime/`:
   ```bash
   ./scripts/build-browser.sh --jobs 10
   ```
   Use `--fetch-only` to unpack and patch the source tree without compiling.

## Managed Policy (Optional)

To enforce Memory Saver and disable background mode and reporting at the policy level:
```bash
sudo install -Dm644 policies/low-ram-policy.json /etc/chromium/policies/managed/low-ram-policy.json
```
