# Testing and release gates

Daily work uses four core groups (`cargo test --locked --lib`) plus necessary compilation checks. They cover config validity and legacy compatibility, strict answer selection, structured decision / chat contracts, and endpoint transport rules. On Windows, the provider group includes one loopback WinHTTP smoke check, and one additional native PNG test checks screenshot encoding, cropped decoding and premultiplied alpha (five tests total). These are the minimum checks for replacing the HTTP and image-codec dependencies. Do not run the broad suite for routine edits.

The default Windows workflow checks formatting, runs these five tests on Windows, and builds a portable x64 executable. When the optional component or its art asset changes, it also compiles that component. It does not execute live-provider, game, endurance, or performance tests.

## Before every release

Create a copy of `docs/releases/TEMPLATE.md`, record the candidate commit and actual evidence. An unchecked box is unverified, not implicitly passed. No stable release before the focus/input and core decision gates pass on Windows.

1. `cargo fmt --all -- --check`, `cargo test --locked --lib`, `cargo build --locked --release` on Windows.
2. `cargo test --locked --test release_providers -- --ignored`. These three grouped tests run on Windows using only a loopback mock server; cover all ten protocol adapters / variants and both input routes, credentials in headers, keep-alive unloading, HTTP errors, redirect refusal, timeout and invalid model choices.
3. Run the manual matrix below, including the optional Game Bar build if distributing it. A real API smoke test requires the maintainer's own credentials; never commit them or use them in screenshots.

## Windows manual matrix (stored outside daily tests)

| Area | Required scenarios | Acceptance |
|---|---|---|
| Startup | First-run model → decision setup, Enter/Next, single-click tray, Windows 10 22H2 / Windows 11, standard user, second instance, Explorer restart | One resident host, tray returns, no admin or console window |
| Shortcuts | Pause/release/resume, resume conflict, saved changes while paused, default/reassigned, auto-repeat, trigger while busy/settings/editing | Conflict reported; no duplicate requests or stuck busy state |
| Capture | Primary/secondary/negative-origin displays, 100/150/200% DPI, prior result visible | Correct foreground monitor; no self-overlay; no saved capture |
| OCR | Installed Chinese/English OCR, no OCR language, empty text, graphic-only scene | Usable text or clear tray error; no guessed graphics interpretation |
| Providers | Actual hosted and local decision services (System One, Decider, OpenAI Decisions), retained chat adapters, vision/OCR, refusal, probabilities, unreachable endpoint | Existing option or abstention; failures never become valid decisions |
| Fullscreen | Short/long/custom Chinese answer, transparent PNG, 300/1500/60000 ms, all five effects | Only glyph/art pixels cover the desktop; no pane or dimming during entry/exit; fits visible area, bounded lifetime, no focus/input changes |
| Settings | Unfinished field across page switch, invalid-input error page, preset/key isolation, Done/Cancel/window-close, four-second unsaved preview, direct shortcut recording, color/font picker, secondary pages and Back, IME and keyboard navigation | No lost drafts, unintended saves, model requests or unreadable text |
| Widget | Numeric X/Y/width/height, negative origin, reset, restart persistence, processing, error, 0/5000 ms, background alpha 0/180/255, high contrast, edit/lock, resize, unplug monitor | Background alpha leaves text unchanged and rounded corners transparent; legacy theme defaults to 180; persistent mode waits visibly; timed mode hides; fresh result timer; click-through outside edit; recovers on screen |
| Word art | Three generated presets, both YES/NO rows, arbitrary custom labels, custom RGBA PNG, missing/oversized PNG | Correct sprite, true alpha, fallback to readable text, no idle image retention |
| Games | At least one borderless and one exclusive candidate; record anti-cheat and game/version | Keyboard/mouse continue; no pause, focus steal, or process injection |
| Game Bar | Not installed, wrong family, closed/unpinned/non-click-through, both sizes/modes, background alpha 0/180/255, all effects, disconnect | Reject unsafe/unavailable display; no panel launch; host foreground unchanged |
| Secrets | Restart, separate endpoints/providers, replace/delete key, invalid config, failed save | No plaintext key in config/logs; original config survives interrupted save |
| Resource lifecycle | Capture allocation/selection/copy failure, borrowed-DIB encode, release before OCR recognition/network, repeated trigger/hide, settings open/close, cancellation by exit, monitor changes | No monotonic private/GDI/USER handle growth; idle render timers stop |

### Focus/input check

Keep a Windows editor or a test game in the foreground. Trigger decisions while typing and moving/clicking the mouse through the overlay bounds. Confirm its foreground HWND never changes during passive display; text is not lost, clicks reach the underlying app, and a game does not pause. Test tray/settings editing separately because those are explicit user interactions. Do not label a game supported from visual presence alone.

### Performance sample

Build release, close settings and let the overlay hide. Use `scripts/measure-idle.ps1` for a 60-second sample. Record OS build, hardware, logical CPU count, commit, private working set and CPU percentage. Measure the host, visible widget, and optional Game Bar process separately; local inference is outside the host budget.

Targets, not measured claims: hidden host private working set ≤30 MB and mean CPU <0.1% of total machine capacity. Private *bytes* are a separate diagnostic and must not be substituted for private working set. Compare fresh idle and after repeated use; waiting for the OS to trim memory is not a substitute for releasing resources.

## Current validation

See `docs/validation.md` for performed checks. The Linux environment cannot validate Windows compositor, Game Bar, DPI interaction, actual focus/input or game compatibility. Cross-compilation is only a build check.
