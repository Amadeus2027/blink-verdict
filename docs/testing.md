# Testing and release gates

Daily work uses four core groups (`cargo test --locked --lib`) plus necessary compilation checks. They cover config validity and legacy compatibility, strict answer selection, structured decision / chat contracts, and endpoint transport rules. On Windows, the provider group includes one loopback WinHTTP smoke check, and one additional native PNG test checks screenshot encoding, cropped decoding and premultiplied alpha (five tests total). These are the minimum checks for replacing the HTTP and image-codec dependencies. Do not run the broad suite for routine edits.

The default Windows workflow checks formatting, runs these five tests on Windows, and builds a portable x64 executable. When the optional component or its art asset changes, it also compiles that component. It does not execute live-provider, game, endurance, or performance tests.

## Focused settings regression

For changes to settings persistence or local-model controls, manually dispatch the Windows workflow with `settings_regressions=true`, or run on Windows:

```powershell
cargo test --locked --bin blink-verdict settings_regression -- --ignored --test-threads=1
```

Two stored groups exercise About/Game Bar retention, native preset/residency-control caching, and real Credential Manager rollback against a locked configuration file (replace, delete, unchanged key, new endpoint, no previous credential and successful save). They use isolated synthetic credential targets and remove them afterward. They are excluded from daily CI and included before every release.

## Focused runtime regression

Dispatch `check.yml` with `runtime_regressions=true` when changing task ownership, rendering or DPI. It builds the real executable, runs the five core groups, then the four ignored native settings/runtime groups. To run locally on Windows:

```powershell
cargo build --locked --release
$env:BLINK_VERDICT_TEST_EXE = (Resolve-Path target/release/blink-verdict.exe).Path
cargo test --locked --bin blink-verdict regression -- --ignored --test-threads=1
```

The two runtime groups use a real Windows child that does not finish, a child that exits without a result, a subsequent real BlinkVerdict preview child, one real screenshot/native-HTTP request against a llama.cpp-shaped loopback fixture, and a blocked shortcut. The host's timer must restore task state and the Job Object must reclaim the blocked process. Rendering checks allocate a measured result surface against a 3840×2160 viewport, inspect native DIB alpha and actual layered-window hit testing/click-through at background 0/180/255 during/after edit, and send a synthetic DPI-change message to the actual settings procedure while retaining a draft. They do not emulate HDR hardware, mixed-DPI physical monitors, actual 4K display performance or game compatibility. No real provider/credentials are used. These remain outside daily CI.

## Focused Game Bar message ordering

Run `./scripts/tests/gamebar-order.ps1` on Windows, or dispatch `check.yml` with `gamebar_order_regression=true`. The script compiles the actual production `DisplayOrder.cs` and deterministically exercises delayed old hide/new show, old show after a newer hide, stale disconnect, suspended async continuation, a closed queued operation and retry. No sleeps, Game Bar installation, real provider or credentials are required. It stays outside daily CI and is included before releases. This is an ordering regression, not end-to-end Game Bar certification.

## Before every release

Use the [release preparation workflow](releases/README.md) and create a record from `docs/releases/TEMPLATE.md` with the candidate commit and actual evidence. The workflow is manual and does not expand daily CI. An unchecked box is unverified, not implicitly passed. No stable release before the focus/input and core decision gates pass on Windows.

1. `cargo fmt --all -- --check`, `cargo test --locked --lib`, `cargo build --locked --release` on Windows.
2. Build release, set `BLINK_VERDICT_TEST_EXE`, and run `cargo test --locked --bin blink-verdict -- --ignored --test-threads=1 --nocapture`. This includes the four focused settings/runtime groups plus renderer/worker lifecycle and setup-confirmation release cases (seven total). Then run `cargo test --locked --test release_providers -- --ignored`. These three grouped tests run on Windows using only a loopback mock server; cover all eleven protocol adapters / variants and both input routes, credentials in headers, default service residency and optional Ollama unloading, HTTP errors, redirect refusal, timeout and invalid model choices.
3. The release-only workflow additionally checks the real registered shortcut, PNG capture, loopback choice, busy repeat suppression, foreground/keyboard/mouse delivery on a synthetic receiver, and 60-sample hidden-fresh/hidden-after-use/visible-widget metrics. These do not certify games, live services or endurance.
4. The workflow also runs `scripts/release/llama-smoke.ps1`: checksum-verified llama-server b11669 and public stories260K weights, two schema-constrained text requests and missing-key rejection. Artifacts stay in runner temp, never the package. This does not certify model quality, user models or vision/GPU.
5. Run the focused Game Bar ordering script above, then the manual matrix below, including the optional Game Bar build if distributing it. A real API smoke test requires the maintainer's own credentials; never commit them or use them in screenshots.

## Windows manual matrix (stored outside daily tests)

| Area | Required scenarios | Acceptance |
|---|---|---|
| Startup | First-run model → decision setup, Enter/Next, visible shortcut-conflict prompt, single-click tray, Windows 10 22H2 / Windows 11, standard user, second instance, Explorer restart | One resident host, tray returns, no admin or console window |
| Shortcuts | Pause/release/resume, resume conflict, saved changes while paused, default/reassigned, auto-repeat, trigger while busy/settings/editing | Conflict reported; no duplicate requests or stuck busy state |
| Capture | Primary/secondary/negative-origin displays, 100/150/200% DPI, prior result visible | Correct foreground monitor; no self-overlay; no saved capture; HDR/advanced-color capture refused with explanation |
| OCR | Installed Chinese/English OCR, no OCR language, empty text, graphic-only scene | Usable text or clear tray error; no guessed graphics interpretation |
| Providers | Actual hosted and local decision services (System One, Decider, OpenAI Decisions), retained chat adapters, vision/OCR, refusal, probabilities, unreachable endpoint | Existing option or abstention; failures never become valid decisions |
| Fullscreen | Short/long/custom Chinese answer, transparent PNG, 300/1500/60000 ms, all five effects; 4K short answer and long wrapped text | Only glyph/art pixels cover the desktop; no pane or dimming during entry/exit; fits visible area, bounded lifetime, no focus/input changes |
| Settings | Unfinished field across page switch, invalid-input error page, preset/key isolation, Done/Cancel/window-close, four-second unsaved preview, direct shortcut recording, color/font picker, secondary pages and Back, IME and keyboard navigation | No lost drafts across navigation or DPI changes, unintended saves, model requests or unreadable text |
| Widget | Numeric X/Y/width/height, negative origin, reset, restart persistence, processing, error, 0/5000 ms, background alpha 0/180/255 including edit hit testing, high contrast, edit/lock, resize, unplug monitor | Background alpha leaves text unchanged and rounded corners transparent; legacy theme defaults to 180; persistent mode waits visibly; timed mode hides; fresh result timer; click-through outside edit; recovers on screen |
| Word art | Three generated presets, both YES/NO rows, arbitrary custom labels, custom RGBA PNG, missing/oversized PNG | Correct sprite, true alpha, fallback to readable text, no idle image retention |
| Games | At least one borderless and one exclusive candidate; record anti-cheat and game/version | Keyboard/mouse continue; no pause, focus steal, or process injection |
| Game Bar | Not installed, wrong family, closed/unpinned/non-click-through, both sizes/modes, background alpha 0/180/255, all effects, disconnect | Reject unsafe/unavailable display; no panel launch; host foreground unchanged |
| Secrets | Restart, separate endpoints/providers, replace/delete key, invalid config, failed save | No plaintext key in config/logs; original config survives interrupted save |
| Worker recovery | Block/suspend child, unexpected child exit, lost result notification, timeout during OCR/network/IPC, immediate retry, host exit during request | Host leaves busy state, no late-result replacement, child Job Object reclaimed; no idle task timer/worker |
| Resource lifecycle | Capture allocation/selection/copy failure, borrowed-DIB encode, release before OCR recognition/network, repeated trigger/hide, settings open/close, cancellation by exit, monitor changes | No monotonic private/GDI/USER handle growth; idle render timers stop |

### Focus/input check

Keep a Windows editor or a test game in the foreground. Trigger decisions while typing and moving/clicking the mouse through the overlay bounds. Confirm its foreground HWND never changes during passive display; text is not lost, clicks reach the underlying app, and a game does not pause. Test tray/settings editing separately because those are explicit user interactions. Do not label a game supported from visual presence alone.

### Performance sample

Build release, close settings and let the overlay hide. Use `scripts/measure-idle.ps1` for a 60-second sample. Record OS build, hardware, logical CPU count, commit, private working set and CPU percentage. Measure the host, visible widget, and optional Game Bar process separately; local inference is outside the host budget.

Targets, not measured claims: hidden host private working set ≤30 MB and mean CPU <0.1% of total machine capacity. Private *bytes* are a separate diagnostic and must not be substituted for private working set. Compare fresh idle and after repeated use; waiting for the OS to trim memory is not a substitute for releasing resources.

## Current validation

See `docs/validation.md` for performed checks. Linux cross-compilation is only a build check. Windows CI adds real desktop compositor/input evidence on its runner, but does not certify physical mixed-DPI/HDR devices, installed Game Bar or real games. Under a no-physical-machine release, passing these gates permits a prerelease with explicit limits; it does not check off the remaining manual matrix.
