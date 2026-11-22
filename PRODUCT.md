# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Daily desktop Linux users replacing heavy Chromium or Helium builds with a fast, privacy-first, low-RAM browser that still handles everyday sites, extensions, and DRM streaming cleanly.

## Product Purpose

Provide a custom-compiled, privacy-hardened Ungoogled Chromium browser (**Kestrel**) whose New Tab page gives instant keyboard-first search/URL entry, quick-access pinned sites, and a subtle, honest protection readout without clutter or visual noise.

## Positioning

Unlike stock Chromium or bloated start pages, Kestrel ships as an independent, self-compiled browser with built-in MV3 ad/tracker blocking (Kestrel Shield), strict privacy defaults, aggressive Memory Saver, Widevine DRM support, and zero remote telemetry.

## Operating Context

- Desktop Linux (Arch Linux / Wayland) running Kestrel (`Chromium 158.0.8127.36`) locally.
- The New Tab page is a local MV3 extension (`ui/newtab/`) loaded directly by the browser on every `Ctrl+T` / new window.

## Capabilities and Constraints

- Zero remote network requests from the New Tab page.
- Minimal DOM and RAM footprint; instant focus on the search/URL input.
- Supports switching between DuckDuckGo, Google, Brave Search, and Startpage.
- Editable pinned shortcuts persisted locally in `chrome.storage.local`.

## Brand Commitments

- Name: **Kestrel**
- Custom geometric falcon/kestrel emblem (`assets/kestrel.svg`).

## Product Principles

1. **Speed and restraint over decoration** — every element on the New Tab page must earn its space.
2. **Privacy by default** — zero external fetches, zero telemetry, honest local state.
3. **Utilitarian clarity** — crisp typography and high contrast without glowing gradients or status-pill clutter.
