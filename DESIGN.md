# Design System — Kestrel

## Visual World

Restrained Swiss/Braun dark-room instrument: matte graphite surfaces, 1px hairline rules, crisp system-sans typography, and a single kestrel-amber (`#e09f3e`) focus/active accent. Zero gradients, zero drop-shadow glows, zero rounded status-pill clutter.

## Tokens

### Color
- `--bg`: `#111317` (matte graphite ground; browser frame, omnibox, NTP background)
- `--surface`: `#181b21` (recessed input, toolbar, shortcut cell)
- `--border`: `#232730` (1px hairline dividers)
- `--text`: `#e6e9ef` (primary ink)
- `--muted`: `#8b93a1` (secondary ink, placeholders, inactive tabs)
- `--accent`: `#e09f3e` (kestrel-amber focus border, caret, active engine underline, crest)

### Typography
- **Font stack**: `-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif`
- **Tabular figures**: `font-variant-numeric: tabular-nums` on protection readouts

### Elevation & Radii
- Elevation via 1px `#232730` hairline borders only (no box-shadows).
- Input/grid radius: `4px`–`6px`.

### Browser Surfaces
- `::selection` themed with `#e09f3e` / `#111317`
- `caret-color: #e09f3e`
- Custom dark scrollbar and `:focus-visible` ring
