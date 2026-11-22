# Kestrel

**Kestrel** (`Kestrel 158.0.8127.36`) is an independent, privacy-first, ultra-low-RAM desktop browser built on a custom-compiled **Ungoogled Chromium `158.0.8127.36`** fork with a Helium-inspired interface, native `kestrel://` URL scheme, built-in **Kestrel Shield** MV3 protection, and a zero-dependency **Rust** launcher & settings manager.

## Highlights

- **Custom-Compiled `Kestrel 158.0.8127.36` Runtime (`browser/patches/01`–`08`)**:
  - `01-kestrel-default-search.patch` — Sets **DuckDuckGo** as the built-in default search engine in `prepopulated_engines.json`.
  - `02-kestrel-low-ram-defaults.patch` — Enables aggressive Memory Saver (`state: 2`) by default at the browser level.
  - `03-kestrel-branding.patch` — Rebrands `IDS_PRODUCT_NAME` / `IDS_SHORT_PRODUCT_NAME` to **Kestrel**.
  - `04-kestrel-helium-ui.patch` — Helium-inspired compact 30px tab strip/omnibox, hidden single-profile avatar button, and matte-graphite (`#111317`) + Kestrel Amber (`#e09f3e`) native `ColorProvider`.
  - `05-kestrel-scheme-and-branding.patch` — Registers `kestrel://` as a standard & secure scheme with virtual URL display rewriting (`chrome://` → `kestrel://`) and bakes custom Kestrel logos into `product_logo_*` resources.
  - `06-kestrel-degoogle-helium-ui.patch` — De-Googles and de-Chromiumizes `kestrel://settings` (removing "You and Google" and Google account promos), removes the NTP extension attribution footer, and replaces `.grd`/`.grdp` product strings.
  - `07-kestrel-native-settings.patch` — Adds a native **Kestrel** settings page inside `kestrel://settings`, makes the omnibox blank on `kestrel://newtab`, registers Kestrel Shield, Kestrel New Tab, and Kestrel Theme as built-in `ComponentLoader` extensions (zero `--load-extension` CLI flags), and enables static DNR ruleset indexing for component extensions.
  - `08-kestrel-omnibox-shield-and-tabs.patch` — Excludes Kestrel Shield from the Extensions puzzle menu, adds a native Omnibox **`kKestrelShieldVectorIcon`** button right beside the Bookmark star, renders Helium-style floating rounded-rect tabs (`8px` radius, zero bottom flare), hides the tab-search chevron by default, adds live tab/toolbar customization inside `kestrel://settings`, and auto-focuses the omnibox on `Ctrl+T` New Tab.
- **Built-in Kestrel Shield (MV3)**:
  - **28,586** static Declarative Net Request (`declarativeNetRequest`) rules compiled from uAssets, EasyList, EasyPrivacy, Peter Lowe, and Fanboy Annoyances, plus **3,300** cosmetic element-hiding selectors.
  - YouTube ad-JSON pruning & anti-adblock scriptlets, tracking-parameter stripping, and anti-fingerprinting protections.
  - Verified **100%** across major adblock benchmarks:
    - `superadblocktest.com`: **482 / 482 (100%)**
    - `adblock.turtlecute.org`: **130 / 130 (100%)**
    - `getblockify.com/adblock-test`: **82 / 82 (100%)**
- **Helium-Style New Tab & `Customize Kestrel` Drawer**:
  - Clean matte-graphite New Tab (`#111317` canvas matching the seamless tab strip & toolbar) with quick-access shortcut tiles and a slide-out **`Customize Kestrel`** side drawer (appearance modes, color swatches, custom local background image, shortcut manager, and 1-click jump to `kestrel://settings`).
- **Privacy-First Defaults**:
  - **Quad9** DNS-over-HTTPS (`doh_provider: "quad9"`), WebRTC local IP leak prevention (`disable_non_proxied_udp`), 3rd-party cookies blocked, strict `--site-per-process` Site Isolation, HTTPS-First / HTTPS Upgrades, Do Not Track, Privacy Sandbox disabled, and zero Google telemetry.
- **Low RAM Without Throttling**:
  - `ram_profile: "ultra"` (`renderer_process_limit: 4`, `v8_max_old_space_mb: 512`), aggressive Memory Saver, background networking/prefetch suppression, and VA-API hardware video decoding/encoding (`VaapiVideoDecoder`, `VaapiVideoEncoder`, `VaapiVideoDecodeLinuxGL`) for smooth playback without CPU throttling.
- **Widevine DRM Enabled by Default**:
  - `widevine_drm: true` automatically links `WidevineCdm` for Spotify, Netflix, and other DRM streaming services.

## Directory Layout

- `browser/args.gn` & `browser/patches/01`–`08` — GN build flags and all 8 C++ patches for `Kestrel 158.0.8127.36`.
- `launcher/Cargo.toml` & `launcher/src/main.rs` — Zero-dependency native Rust binary providing `kestrel` (launcher), `kestrel-settings` (TUI + CLI settings manager), and `kestrel-native-host`.
- `launcher/config.default.json` — Default configuration template (`~/.config/kestrel/config.json`).
- `launcher/kestrel.desktop` — Desktop entry (`Name=Kestrel`, `StartupWMClass=Kestrel`, `Icon=kestrel`, HTTP/HTTPS handlers, and desktop actions).
- `adblocker/` — Built-in **Kestrel Shield** MV3 adblocker (`rules/*.json`, cosmetic selectors, YouTube scriptlets, popup UI, and pure-Rust `scripts/build_filters.rs`).
- `ui/newtab/` & `ui/theme/` — Built-in **Kestrel New Tab** (`Customize Kestrel` drawer) and seamless matte-graphite browser theme.
- `assets/kestrel.svg` & `assets/kestrel.png` — Custom Kestrel browser icons.
- `policies/low-ram-policy.json` — Chromium managed policy template for Memory Saver and telemetry/background suppression.
- `scripts/build-browser.sh` — Fetches, patches, compiles `ungoogled-chromium 158.0.8127.36`, and installs the custom runtime to `~/.local/share/kestrel/runtime/`.
- `scripts/install.sh` — Builds `launcher/` (`cargo build --release`), installs `kestrel`, `kestrel-settings`, `kestrel-import`, `adblocker/`, `ui/`, icons, and `.desktop` entry (`--set-default` supported).
- `scripts/kestrel-import` — Safe bookmark/favicon/history importer (defaults to importing from `~/.config/net.imput.helium/Default`).

## Quick Start

1. Build & install Kestrel (requires `rustc`/`cargo`, `curl`, `tar`, `xz`):
   ```bash
   ./scripts/install.sh
   # Or register Kestrel as your default web browser too:
   ./scripts/install.sh --set-default
   # Or skip downloading a runtime if using an existing runtime/system build:
   ./scripts/install.sh --system-browser
   ```
   `install.sh` compiles the Rust launcher and installs Kestrel Shield (`~/.local/share/kestrel/adblocker/`) and Kestrel UI (`~/.local/share/kestrel/ui/`). All built-in components are loaded natively via C++ `ComponentLoader` with zero `--load-extension` flags.
2. (Optional) Import bookmarks from an existing Chromium/Helium profile:
   ```bash
   kestrel-import
   # Or include Favicons and History:
   kestrel-import --all
   ```
3. Launch Kestrel:
   ```bash
   kestrel
   ```

## Configuration (`~/.config/kestrel/config.json`)

Configure Kestrel directly inside the browser at **`kestrel://settings`**, or interactively from the terminal:
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
kestrel-settings --toggle hide_tab_search
kestrel-settings --set tab_density=compact --set renderer_process_limit=6 --set v8_max_old_space_mb=768
```

### Feature Toggles

| Key | Default | Effect |
| --- | --- | --- |
| `search_engine` | `"duckduckgo"` | Default omnibox search engine (`duckduckgo`, `brave`, `startpage`, `google`), synced to `Default/Preferences`. |
| `ram_profile` | `"ultra"` | Ultra-low-RAM preset (`renderer_process_limit: 4`, `v8_max_old_space_mb: 512`) without throttling media/UI performance. |
| `strict_security` | `true` | Enforces `--site-per-process` Site Isolation, `HttpsUpgrades` + `HttpsFirstBalancedModeAutoEnable`, blocks 3rd-party cookies, enables Do Not Track, and disables Privacy Sandbox / WebBluetooth / WebUSB / PaymentRequest. |
| `doh_provider` | `"quad9"` | DNS-over-HTTPS provider (`quad9`, `cloudflare`, `mullvad`, `off`) written to `Default/Preferences`. |
| `webrtc_privacy` | `true` | Prevents WebRTC local IP leaks via `--force-webrtc-ip-handling-policy=disable_non_proxied_udp`. |
| `tab_sleeping` | `true` | Enables aggressive Memory Saver (`state: 2`), caps renderer processes & V8 heap, and disables background networking, telemetry, translate, speech API, prefetching, and speculation rules. |
| `hardware_accel` | `true` | Enables GPU rasterization, zero-copy, Wayland/X11 auto Ozone platform, WebGL, and VA-API hardware video decoding/encoding (`VaapiVideoDecoder`, `VaapiVideoEncoder`, `VaapiVideoDecodeLinuxGL`). |
| `widevine_drm` | `true` | Automatically links `WidevineCdm` into the Kestrel profile/runtime for DRM streaming playback. |
| `adblocker` | `true` | Controls built-in **Kestrel Shield** MV3 protection (loaded natively via `ComponentLoader`). |
| `custom_ui` | `true` | Controls built-in **Kestrel New Tab** and seamless matte-graphite theme. |
| `extension_support` | `true` | Enables Chrome Web Store `.crx` install prompts (`--extension-mime-request-handling=always-prompt-for-install`). |
| `tab_density` | `"helium"` | Floating rounded-rect tab spacing preset (`"compact"`, `"helium"`, `"standard"`). |
| `hide_tab_search` | `true` | Hides the left tab-search `v` chevron button from the tab strip. |
| `show_bookmarks_bar` | `false` | Controls the bookmarks bar (`bookmark_bar.show_on_all_tabs`), synced live from `kestrel://settings`. |

### Launcher Options

- `kestrel install-runtime` — Download/update a standalone portable Ungoogled Chromium runtime to `~/.local/share/kestrel/runtime/`.
- `kestrel install-webstore` — Download the latest `NeverDecouple/chromium-web-store` `.crx` to `~/.local/share/kestrel/extensions/` for one-click Chrome Web Store installs.
- `kestrel --auto-runtime` — Automatically download a standalone runtime on launch if none is found.
- `kestrel --dry-run` (or `--print-cmd`) — Print the resolved browser command and flags without launching.
- `kestrel settings` / `kestrel --settings` — Open `kestrel-settings`.
- `kestrel --import` — Run `kestrel-import`.

## Custom Source Build

To compile the full **Kestrel `158.0.8127.36`** runtime from source on Arch Linux (applying `browser/args.gn` and `browser/patches/01`–`08`):

1. Check required build dependencies:
   ```bash
   ./scripts/build-browser.sh --check-deps
   ```
2. Fetch, patch, compile (requires ~100GB+ disk space), and install into `~/.local/share/kestrel/runtime/`:
   ```bash
   ./scripts/build-browser.sh --jobs 10
   ```
   Use `--fetch-only` to unpack and patch the source tree without compiling.

## Managed Policy (Optional)

To enforce Memory Saver and disable background mode and reporting at the system policy level:
```bash
sudo install -Dm644 policies/low-ram-policy.json /etc/chromium/policies/managed/low-ram-policy.json
```
