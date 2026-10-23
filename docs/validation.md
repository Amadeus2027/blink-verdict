# Development validation

Date: 2026-10-22. This iteration retains Rust and refines the native settings / widget workflow, with sidebar glass, readable opaque form content, rounded controls and a recorded Windows showcase. Dedicated decision APIs, local service protocols and lightweight native HTTP / PNG components remain intact.

## Performed locally

Environment: Linux with Rust stable and the x86_64-pc-windows-gnu standard library.

- `cargo check --locked --target x86_64-pc-windows-gnu --all-targets`: passed. Includes WinHTTP, WIC, settings, both display modes, and compilation of stored release regressions.
- `cargo test --locked --lib`: four platform-neutral core groups passed, including structured answers / confidence / probabilities, explicit abstention, old provider config compatibility and negative widget coordinates.
- The current lockfile remains at 29 packages; the preceding transport / codec refactor reduced it from 146 by removing reqwest and the PNG crate. This UI iteration adds no runtime dependency. This is a dependency count, not a RAM benchmark.
- Built-in atlas: 1536 × 1024 RGBA. Returned art pixels now cover one 512 × 512 cell (1 MiB BGRA instead of the 6 MiB full-atlas BGRA buffer); the embedded compressed PNG is borrowed rather than cloned. WIC's internal decoder allocations have not been measured.
- Default CI remains small: four groups, including one Windows-only real WinHTTP loopback smoke within the provider group, plus one Windows-only native PNG roundtrip/crop/alpha smoke test (five tests on Windows). Broad network / live-provider / game / endurance checks remain release-only.

## Windows CI

The Windows workflow runs the five core tests, builds the MSVC release executable, reports its byte size, and packages a standalone EXE with quick-start/changelog. CI results and artifact links are recorded in the implementation PR. No application word-art asset or Game Bar source changed in this iteration, so the optional UWP build is skipped. Documentation captures are separate from application assets.

The native UI code at `b8cf50e` passed [Windows CI run 39828111044](https://github.com/Amadeus2027/blink-verdict/actions/runs/39828111044): five core tests and the MSVC release build. The executable is **3,127,296 bytes (2.98 MiB)**, compared with the preceding implementation’s 3,097,088 bytes; no runtime dependencies were added. This measures file size only, not resident memory. [Recording run 39829350117](https://github.com/Amadeus2027/blink-verdict/actions/runs/39829350117) captures the same native code at `4f0ca38`, with a static GDI-compatible capture font, and exports the MP4/GIF. The manual recording checks actual GUI launch, all five settings pages, unfinished-field retention, clearing an unsaved key on endpoint change, and a visible native widget preview that does not save draft mode. These checks use synthetic data and zero provider requests; they do not establish game compatibility.

## Unverified runtime / release gates

- Passive focus/input in real apps/games, interactive borderless resize, numeric geometry persistence, multi-monitor/DPI behavior, Windows 10 material fallback and consumer Windows 11 acrylic appearance. The limited CI recording confirms launch/settings pages/preview on its Windows Server runner, not this wider matrix.
- Real Jev / Decider / Clef / OpenAI Decisions accounts, local model weights / serving environments, OCR availability, model quality and refusal behavior. Protocol fixtures are synthetic, not captured live responses.
- Game Bar activation, positioning, hidden state and game compatibility. Previous CI packaging is not runtime evidence.
- Idle private working set, CPU, repeated-use resource stability and game anti-cheat compatibility. Hidden-host targets remain ≤30 MB private working set and <0.1% mean CPU, unmeasured.

No stable release or measured RAM claim follows from compile checks or dependency reduction. Follow `docs/testing.md` and record evidence before a release.
