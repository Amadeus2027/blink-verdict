# Native interface showcase

The showcase uses the actual Windows Rust executable. The media pipeline records native settings pages through Win32 PrintWindow and a short sequence of native overlay frames, then adds titles, cuts and fades. It does not render an HTML replica or generate a pretend screenshot.

The demonstration uses isolated synthetic preferences and a sample YES result from the application's preview action. It never triggers screenshot/OCR or a model request, reads no user credentials, and does not demonstrate model reasoning quality or real game compatibility. Offscreen recording does not reliably include desktop-compositor blur; actual acrylic depends on Windows/DWM availability, and unsupported environments use the opaque fallback.

To refresh the recording, manually run the existing **Windows build and core checks** workflow on the target branch. Push / PR events run only the usual minimal native checks. Manual dispatch instead runs the capture on Windows 2025 and editorial export on Ubuntu. The workflow:

1. Builds and runs the native application, installs an open Noto Sans SC font on the capture runner only, and opens settings through the tray entry point.
2. Captures five settings pages, verifies an unfinished shortcut survives navigation, and records a two-second unsaved widget preview. Checks the on-disk display mode did not change.
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

Windows capture requires PowerShell, an available desktop and GDI. Editorial rendering requires FFmpeg on Linux. This is a documentation workflow, not a routine test suite, a performance benchmark or a live-provider integration test. The GIF and source frames are repository documentation only; the application ZIP still contains only the EXE, quick-start and changelog.

Interaction references: [Rainmeter Manage](https://docs.rainmeter.net/manual/user-interface/manage/), [ShareX Pin to Screen](https://getsharex.com/docs/pin-to-screen), [PowerToys Always On Top](https://learn.microsoft.com/en-us/windows/powertoys/always-on-top). The software keeps its own design and passive-input requirements.
