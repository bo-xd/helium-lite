# Kestrel Custom Chromium Patches

Place custom C++ / GN `.patch` files (`-p1` format against the Chromium `158.0.8127.36` source tree) in this directory (`browser/patches/*.patch`).

## How Patches Are Applied

`scripts/build-browser.sh` applies patches in this order:
1. Arch Linux compatibility patches from `ungoogled-software/ungoogled-chromium-archlinux`
2. Privacy & de-Googling patches from `ungoogled-software/ungoogled-chromium` (`158.0.8127.36-1`)
3. All `browser/patches/*.patch` files in lexical order (e.g., `01-branding.patch`, `02-memory-tuning.patch`)

## Creating a New Patch

1. Run `./scripts/build-browser.sh --fetch-only` to unpack and patch the base `ungoogled-chromium` tree under `/home/bo/Projects/kestrel-build/chromium-158.0.8127.36/`.
2. Initialize a temporary git repo or use `diff -ruN` against the unmodified file to generate a unified `-p1` patch.
3. Save it as `browser/patches/<NN>-<name>.patch`.
