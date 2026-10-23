# Development validation

Date: 2026-10-23. This iteration replaces the dark parameter panel with light grouped settings, restructures first-run and everyday actions, refreshes user-facing copy and records a quieter product film. The native Rust host, decision protocols and stored preferences remain compatible.

## Performed locally

Environment: Linux, Rust stable, x86_64-pc-windows-gnu standard library.

- `cargo check --locked --target x86_64-pc-windows-gnu --all-targets`: passed for the native UI, transparent rendering and release regression compilation.
- `cargo test --locked --lib`: four platform-neutral core groups passed. No broad release suite was run or added.
- The lockfile remains at 29 packages. No new dependency packages or API feature requirements. This is not a memory measurement.
- Interface/overlay painting remains event-driven. Both modes clear to transparent pixels; only widget mode fills a rounded pane with its own alpha. The accent-policy experiment was removed after recorded gray/black rendering defects, rather than claiming a working blur material.

## Windows evidence

[Native recording run](https://github.com/Amadeus2027/blink-verdict/actions/runs/39938825934), source `a325c11`, Windows 11 build 26100: five Windows core tests, release build, native capture and video export passed. The recording uses synthetic settings and sample previews, verifies retained unfinished input, clearing an unsaved key on endpoint changes, preview isolation and both native display modes. Pixel comparisons confirm fullscreen surroundings preserve the desktop and widget rounded corners are transparent.

Reviewed the captured decision/appearance/detail pages and frames from both modes: YES/NO are both visible, the background control fits, fullscreen text has no pane, and the widget has no gray block or black corners. The README GIF is 960 × 600, 8.12 seconds and 1,549,888 bytes; the film is approximately 24 seconds. [Download media](https://github.com/Amadeus2027/blink-verdict/actions/runs/39938825934/artifacts/12495122025) (GitHub login; retained for the Actions artifact lifetime). This is limited GUI evidence on the runner, not a game or performance benchmark.

The normal workflow runs five Windows core tests and builds/packages a portable executable. Game Bar rendering changed to match transparent fullscreen and independent widget alpha, so the optional UWP build is required for this iteration. Native build/package and optional-component results are available in the [PR checks](https://github.com/Amadeus2027/blink-verdict/pull/4/checks).

## Release gates still unverified

- Actual focus/input in games and other applications, direct shortcut recording with physical modifier keys, first-run Next/Enter, native color/font dialogs, drag/resize persistence, multimonitor/DPI and Windows 10 behavior.
- Consumer Windows 10/11 alpha rendering, high contrast and resource cost. The current widget provides semitransparency, without a background-blur claim.
- Live Jev, Decider, Clef, OpenAI Decisions and local services; OCR/model availability, quality, refusal and provider credentials. Fixtures and preview YES are synthetic.
- Game Bar activation and compatibility, idle private working set/CPU, endurance and anti-cheat behavior. Targets remain ≤30 MB private working set and <0.1% average CPU for the hidden host, unmeasured.

Follow [testing.md](testing.md) and record real evidence before a release. No stable-release or measured-memory claim follows from native compilation or file size.
