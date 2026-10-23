# Native interface showcase

![Native interface showcase](assets/showcase.gif)

The showcase uses the actual Windows Rust executable. The media pipeline records native settings pages through desktop cropping of the foreground window when available (Win32 PrintWindow fallback), plus a short sequence of native overlay frames, then adds titles, cuts and fades. It does not render an HTML replica or generate a pretend screenshot.

The recorder refuses to attach to an existing resident instance and verifies ownership of the new host window. It starts the application with the explicit `--recordable-preview` flag: decision triggers are disabled and the overlay is permitted in recording. Ordinary launches retain capture exclusion. A temporary solid native window provides the backdrop; desktop crops include layered windows with `CAPTUREBLT`.

The demonstration uses isolated synthetic preferences and a sample YES result from the application's preview action. It never triggers screenshot/OCR or a model request, reads no user credentials, and does not demonstrate model reasoning quality or real game compatibility. Recording does not reliably reproduce desktop-compositor blur; actual acrylic depends on Windows/DWM availability, and unsupported environments use the opaque fallback.

The approximately 12-second GIF is embedded in the root README. [Recording workflow](https://github.com/Amadeus2027/blink-verdict/actions/runs/39829350117) · [MP4/GIF download](https://github.com/Amadeus2027/blink-verdict/actions/runs/39829350117/artifacts/12448755634). The MP4 and original frame sequence are available from the successful recording workflow's artifacts (subject to Actions retention). [Capture provenance](assets/showcase/provenance.json) records the exact source commit, Windows build, capture path and checks. The source PNGs in `docs/assets/showcase/` allow inspection without pausing the GIF.

To refresh the recording, manually run the existing **Windows build and core checks** workflow on the target branch. Push / PR events run only the usual minimal native checks. Manual dispatch instead runs the capture on Windows 2025 and editorial export on Ubuntu. The workflow:

1. Builds and runs the native application, installs an open Noto Sans CJK SC font on the capture runner only, and opens settings through the tray entry point.
2. Captures five settings pages, verifies an unfinished shortcut survives navigation and an endpoint change clears an unsaved key, and records a two-second unsaved widget preview. Checks the on-disk display mode did not change.
3. Exports a short MP4 and a looping 12 fps GIF with editorial titles / fades. Source screenshots and provenance remain beside the README asset for review.
4. Commits the generated GIF, settings screenshots and provenance to the same branch. The MP4 is available as an Actions artifact; it is not included in the portable application.

Local equivalents:

```powershell
cargo build --locked --release
./scripts/showcase/capture.ps1
```

```bash
python3 scripts/showcase/render.py showcase-frames showcase-edit
```

Windows capture requires PowerShell 7, an available desktop and GDI. Close any running BlinkVerdict instance first; run in a disposable desktop session. The script downloads its capture-only font and uses `$env:RUNNER_TEMP` for synthetic configuration (set it to a temporary directory for local use). Editorial rendering requires FFmpeg on Linux. This is a documentation workflow, not a routine test suite, a performance benchmark or a live-provider integration test. The GIF and source frames are repository documentation only; the application ZIP still contains only the EXE, quick-start and changelog.

Interaction references: [Rainmeter Manage](https://docs.rainmeter.net/manual/user-interface/manage/), [ShareX Pin to Screen](https://getsharex.com/docs/pin-to-screen), [PowerToys Always On Top](https://learn.microsoft.com/en-us/windows/powertoys/always-on-top). The software keeps its own design and passive-input requirements.
