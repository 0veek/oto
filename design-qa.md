# Oto Overlay Pill Design QA

## Evidence

- Source visual truth: `/home/aveek/.codex/generated_images/019f7a55-4a5f-73c3-83bb-5f4ca4491fbc/exec-7ce4765e-c02d-447e-8e1c-89b71dd33747.png`
- Browser-rendered implementation: `/tmp/oto-overlay-reference-mode.png`
- Viewport: `1448 × 1086` CSS pixels at device scale 1
- State: Midnight theme; Listening, Processing, Inserted, and Couldn’t insert
- Full-view side-by-side comparison: `/tmp/oto-overlay-comparison.png`
- Focused Listening comparison: `/tmp/oto-overlay-focus-comparison.png`
- Eight-state interaction board: `/tmp/oto-overlay-preview.png`
- Runtime footprint: `340 × 80` CSS pixels

## Findings

- No actionable P0, P1, or P2 differences remain.
- [P3] The leading mark uses Oto’s real shipped icon rather than redrawing the concept’s generic open-ring symbol.
  - Location: circular brand pod.
  - Evidence: the source uses a simplified cyan ring; the implementation uses the existing cyan waveform-ring mark with its amber timing dot.
  - Impact: the silhouette stays faithful while brand recognition is stronger and no approximate SVG or CSS logo is introduced.
  - Classification: accepted product-asset constraint.
- [P3] The implementation is optically denser than the enlarged concept board.
  - Location: label, waveform, and action spacing.
  - Evidence: the implementation preserves the production window’s exact `340 × 80` footprint and a real 44 px action target; the concept is an enlarged visual study without runtime dimensions.
  - Impact: typography is slightly more compact, while the connected split-pod hierarchy and state readability remain intact.
  - Classification: accepted runtime constraint.

## Required Fidelity Surfaces

- Fonts and typography: Geist Variable matches the existing Oto system. Labels use a compact 15 px/600 treatment with single-line truncation for dynamic errors; hierarchy, optical weight, letter spacing, and line height were checked in the focused comparison.
- Spacing and layout rhythm: the circle-over-pill silhouette, overlap, 64 px status rail, 72 px brand pod, 44 px action, inset state icon, capsule radii, and compact vertical rhythm match the selected direction within the exact runtime footprint.
- Colors and visual tokens: all overlay colors, borders, focus treatment, shadows, success cyan, live amber, and insertion-error coral come from semantic OKLCH tokens in `tokens.css`. No gradients, glass effects, or hard-coded component colors are present.
- Image quality and asset fidelity: the leading pod uses Oto’s real raster application mark. Status and action icons come from one Tabler outline family; the audio visualization is a real canvas driven by pipeline levels. No inline SVG, emoji, placeholder icon, or approximate CSS logo is used.
- Copy and content: the visible production labels are concise and state-specific: `Listening`, `Processing`, `Inserted`, `Couldn’t insert`, and `Ready`. Actual pipeline detail remains available through the accessible label and title without crowding the live overlay.

## Accessibility and Behavior

- The component is a polite live status region with a full state label.
- Cancel and dismiss actions expose descriptive accessible names and titles.
- Keyboard focus is immediate and visible; the tested action has a 2 px focus outline and a `44 × 44` target.
- Default, hover, focus, active, disabled, loading, error, and success states are present in the preview wrapper.
- Motion is limited to state entry, live dots, waveform samples, processing blocks, and the busy spinner; all motion is disabled by the reduced-motion media query.
- Long dynamic details truncate visually and remain available to assistive technology.

## Browser Checks

- Preview URL: `http://127.0.0.1:4176/overlay-preview`
- Primary interactions tested: Cancel/Dismiss click, keyboard Tab focus, pressed state, disabled action, busy action, processing animation, success, and insertion error.
- Action behavior: clicking the default Cancel control incremented the preview’s test-action counter from 0 to 1.
- Rendered state count: 8 pills and 8 action controls, including 2 intentionally disabled/busy controls.
- Console errors on the browser preview route: none.
- Horizontal overflow at 320, 375, 544, 768, and 1200 px: none.
- Production overlay check at `340 × 80`: root bounds exactly `0, 0, 340, 80`; action target `44 × 44`; HTML and body backgrounds transparent; no horizontal or vertical overflow.

## Comparison History

1. Structure pass
   - [P2] Listening dots initially sat after the waveform instead of beneath the label, and Inserted lacked the concept’s trailing dismiss control.
   - Fixes: moved the amber live dots into the label stack and made the real action pod available across Listening, Processing, Inserted, and Error.
2. Detail pass
   - [P2] The first waveform used nine narrow samples and the processing blocks were too tall relative to the source.
   - Fixes: reduced the waveform to seven thicker samples, normalized processing blocks to compact rounded squares, and tuned label weight and spacing.
3. Asset pass
   - [P2] The first mark treatment used a small nested ring that read as a separate badge.
   - Fixes: removed the extra ring, enlarged the real Oto mark, and matched the pod surface token to the mark’s native navy field.
4. Final matched-state pass
   - Evidence: `/tmp/oto-overlay-comparison.png` and `/tmp/oto-overlay-focus-comparison.png`.
   - Result: no actionable P0, P1, or P2 findings remain. The two P3 product constraints above are accepted.

## Implementation Checklist

- [x] Connected playful split-pod silhouette in the production `340 × 80` window
- [x] Live seven-sample audio waveform and amber listening cadence
- [x] Processing, inserted, error, and dormant state treatments
- [x] Real 44 px Cancel/Dismiss control with full interaction states
- [x] Real Oto mark and one consistent vector icon family
- [x] Transparent Tauri window and transparent overlay document surface
- [x] Reduced-motion support and accessible live-state copy
- [x] Eight-state component preview and responsive checks
- [x] Svelte diagnostics, production build, and Rust/Tauri compile check

## Follow-up Polish

- If Oto later ships a transparent high-resolution brand asset, the leading mark can adopt it without changing the pod layout.

final result: passed
