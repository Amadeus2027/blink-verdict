# Development validation

Updated: 2026-10-23. Current capture/structure optimization evidence is recorded below, followed by the remaining release gates. Earlier UI and typography checks are retained as historical evidence; their media sizes do not describe the current exports. The native Rust host, decision protocols and stored preferences remain compatible.

## Performed locally

Environment: Linux, Rust stable, x86_64-pc-windows-gnu standard library.

- `cargo check --locked --target x86_64-pc-windows-gnu --all-targets`: passed for the native UI, transparent rendering and release regression compilation.
- `cargo test --locked --lib`: four platform-neutral core groups passed. No broad release suite was run or added.
- The lockfile remains at 29 packages. No new dependency packages or API feature requirements. This is not a memory measurement.
- Interface/overlay painting remains event-driven. Both modes clear to transparent pixels; only widget mode fills a rounded pane with its own alpha. The accent-policy experiment was removed after recorded gray/black rendering defects, rather than claiming a working blur material.

## Earlier UI iteration — historical Windows evidence

[Native recording run](https://github.com/Amadeus2027/blink-verdict/actions/runs/39938825934), source `a325c11`, Windows runner build 26100: five Windows core tests, release build, native capture and video export passed. The recording uses synthetic settings and sample previews, verifies retained unfinished input, clearing an unsaved key on endpoint changes, preview isolation and both native display modes. Pixel comparisons confirm fullscreen surroundings preserve the desktop and widget rounded corners are transparent.

Reviewed the captured decision/appearance/detail pages and frames from both modes: YES/NO are both visible, the background control fits, fullscreen text has no pane, and the widget has no gray block or black corners. The mobile-optimized README GIF is 640 × 400 at 10 fps, 8.1 seconds and 313,895 bytes (79.7% smaller than the original 1,549,888-byte export); the reviewed film is exactly 24 seconds and 303,159 bytes. The [MP4](assets/showcase.mp4) and GIF are retained in the repository. The README uses an 8,107-byte PNG logo for client compatibility; the original SVG remains available. Media decode, local image paths and export-script syntax were checked after compression. This is limited GUI evidence on the runner, not a game or performance benchmark.

[Windows native/package and optional UWP build](https://github.com/Amadeus2027/blink-verdict/actions/runs/39938831360) passed for source `a325c11`: five tests, native release executable **3,163,648 bytes**, unsigned Game Bar package. File size is not resident-memory usage. Final guidance/cleanup checks are linked on the PR.

The normal workflow runs five Windows core tests and builds/packages a portable executable. In that earlier iteration, Game Bar rendering changed to match transparent fullscreen and independent widget alpha, so the optional UWP build was required. Native build/package and optional-component results are available in the [PR checks](https://github.com/Amadeus2027/blink-verdict/pull/4/checks).

## Release gates still unverified

- Actual focus/input in games and other applications, direct shortcut recording with physical modifier keys, first-run Next/Enter, native color/font dialogs, drag/resize persistence, multimonitor/DPI and Windows 10 behavior.
- Consumer Windows 10/11 alpha rendering, high contrast and resource cost. The current widget provides semitransparency, without a background-blur claim.
- Live Jev, Decider, Clef, OpenAI Decisions and local services; OCR/model availability, quality, refusal and provider credentials. Fixtures and preview YES are synthetic.
- Game Bar activation and compatibility, sustained idle private working set/CPU, endurance and anti-cheat behavior. Release targets remain ≤30 MB private working set and <0.1% average CPU for the hidden host. The short runner sample below does not establish these sustained targets.

Follow [testing.md](testing.md) and record real evidence before a release. No stable-release or measured-memory claim follows from native compilation or file size.

## Minimal word-art follow-up

The new clear-color, tinted-glass and gradient-script YES/NO atlas follows a review of actual Apple typography, Liquid Glass lock-screen and Hello welcome specimens. The generated source remains in the workspace; the previous expressive atlas is archived outside embedded/package content. The runtime PNG decreased from 2,549,678 to 533,967 bytes. Packaging preserved every nontransparent RGBA pixel and every alpha value; all six 512-pixel cells contain glyphs with clear margins on a 1536 × 1024 canvas. Fixed optical offsets center each word without runtime alpha scanning; the same host-provided offsets and cell clipping are used by the optional component. Glass/gradients are prerendered artwork, not live desktop refraction. The light-backed documentation specimen is a presentation of the transparent artwork, not a screenshot of the running application.

Windows-GNU all-target compilation and four existing portable core groups passed locally. No new tests or broad release suite. The normal Windows checks also compile the optional component because its embedded artwork changed; actual small-widget/focus behavior remains part of the Windows release matrix.


## Capture and structure optimization

The capture owner now keeps the DIB alive while PNG encoding borrows its pixels, removing the former full-frame `to_vec()`. It drops GDI handles before OCR recognition/network waits; temporary OCR writer/buffer objects end after bitmap preparation. JSON response bytes are released before verdict selection. Settings are separated into state/events, page layouts and native controls; the unreachable Apply command, duplicate save/rebuild branch, save-time configuration clone and temporary event-batch vector were removed. All ten used protocols remain; no service was declared deprecated without evidence. Runtime dependencies remain unchanged.

Linux Windows-GNU all-target checking and the four existing core groups passed. The [Windows native capture run](https://github.com/Amadeus2027/blink-verdict/actions/runs/39955205509) passed its five core tests, release build and GUI checks on source `de33af3`; recorded UI navigation, transparent fullscreen/widget corners, all three native word-art previews and the local service page were reviewed. Editing-only framing changes reuse these recordings. No broad release suite was run.

The hidden release host, after closing settings and previews, was sampled 10 times on the `windows-2025` runner (NT build 26100, 4 logical CPUs, recording preview mode, no provider requests): actual interval **14.96 s**, mean private working set **4.27 MiB**, peak **5.20 MiB**, private bytes **5.68 MiB**, 240 handles. CPU accounting reported 0.0% over this short interval; that is not proof of zero CPU usage. [Raw values and source](assets/showcase/provenance.json). These are observations from this runner, not a before/after memory benchmark or a consumer-machine guarantee. The 60-second release/performance matrix remains unexecuted.

The new compact PNG identity is **21,690 bytes** and uses a glass/graphite palette on a light backing for dark-mode readers. The comprehensive film uses real native captures with editorial planar camera moves: **39 seconds**, 1280 × 800 at 24 fps, **1,918,817 bytes**. The GIF retains every chapter at 1.5× speed: **26 seconds**, 512 × 320 at 5 fps, **1,014,921 bytes**. Both exports were decoded and representative frames from every chapter reviewed, including title clearance and all three native art previews. Documentation tools/assets are excluded from executable embedding and portable packaging.
