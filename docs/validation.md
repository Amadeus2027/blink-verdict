# Development validation

Date: 2026-10-23. This iteration replaces the dark parameter panel with light grouped settings, restructures first-run and everyday actions, refreshes user-facing copy and records a quieter product film. The native Rust host, decision protocols and stored preferences remain compatible.

## Performed locally

Environment: Linux, Rust stable, x86_64-pc-windows-gnu standard library.

- `cargo check --locked --target x86_64-pc-windows-gnu --all-targets`: passed for the native UI, transparent rendering and release regression compilation.
- `cargo test --locked --lib`: four platform-neutral core groups passed. No broad release suite was run or added.
- The lockfile remains at 29 packages. No new dependency packages or API feature requirements. This is not a memory measurement.
- Interface/overlay painting remains event-driven. Both modes clear to transparent pixels; only widget mode fills a rounded pane with its own alpha. The accent-policy experiment was removed after recorded gray/black rendering defects, rather than claiming a working blur material.

## Windows evidence

Native build and capture links will be recorded here when this iteration's runs finish. The recording uses synthetic settings and sample previews, verifies retained unfinished input, clearing an unsaved key on endpoint changes, both native display modes, and text/background in the visible widget. This is limited GUI evidence on the runner, not a game or performance benchmark.

The normal workflow still runs five Windows core tests and builds/packages a portable executable. Game Bar rendering changed to match transparent fullscreen and independent widget alpha, so the optional UWP build is required for this iteration.

## Release gates still unverified

- Actual focus/input in games and other applications, direct shortcut recording with physical modifier keys, first-run Next/Enter, native color/font dialogs, drag/resize persistence, multimonitor/DPI and Windows 10 behavior.
- Consumer Windows 10/11 alpha rendering, high contrast and resource cost. The current widget provides semitransparency, without a background-blur claim.
- Live Jev, Decider, Clef, OpenAI Decisions and local services; OCR/model availability, quality, refusal and provider credentials. Fixtures and preview YES are synthetic.
- Game Bar activation and compatibility, idle private working set/CPU, endurance and anti-cheat behavior. Targets remain ≤30 MB private working set and <0.1% average CPU for the hidden host, unmeasured.

Follow [testing.md](testing.md) and record real evidence before a release. No stable-release or measured-memory claim follows from native compilation or file size.
