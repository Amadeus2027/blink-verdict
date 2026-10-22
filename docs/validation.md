# Development validation

Date: 2026-10-22. This iteration retains Rust and adds native decisions, local protocol variants, numeric widget geometry and native HTTP / PNG components.

## Performed locally

Environment: Linux with Rust stable and the x86_64-pc-windows-gnu standard library.

- `cargo check --locked --target x86_64-pc-windows-gnu --all-targets`: passed. Includes WinHTTP, WIC, settings, both display modes, and compilation of stored release regressions.
- `cargo test --locked --lib`: four platform-neutral core groups passed, including structured answers / confidence / probabilities, explicit abstention, old provider config compatibility and negative widget coordinates.
- Source/lockfile comparison: 146 locked packages before this change, 29 after removing reqwest and the PNG crate. This is a dependency count, not a RAM benchmark.
- Built-in atlas: 1536 × 1024 RGBA. Returned art pixels now cover one 512 × 512 cell (1 MiB BGRA instead of the 6 MiB full-atlas BGRA buffer); the embedded compressed PNG is borrowed rather than cloned. WIC's internal decoder allocations have not been measured.
- Default CI remains small: four groups, including one Windows-only real WinHTTP loopback smoke within the provider group, plus one Windows-only native PNG roundtrip/crop/alpha smoke test (five tests on Windows). Broad network / live-provider / game / endurance checks remain release-only.

## Windows CI

The Windows workflow runs the five core tests, builds the MSVC release executable, reports its byte size, and packages a standalone EXE with quick-start/changelog. CI results and artifact links are recorded in the implementation PR. No image asset or Game Bar source changed in this iteration, so the optional UWP build is skipped.

The prior implementation's Windows and optional Game Bar packaging workflow passed: [run 39794469489](https://github.com/Amadeus2027/blink-verdict/actions/runs/39794469489). That result predates the native transport / codec changes here.

## Unverified runtime / release gates

- Native host launch, layered-window composition, passive focus/input, interactive borderless resize, numeric geometry persistence and multi-monitor/DPI behavior.
- Real Jev / Decider / Clef / OpenAI Decisions accounts, local model weights / serving environments, OCR availability, model quality and refusal behavior. Protocol fixtures are synthetic, not captured live responses.
- Game Bar activation, positioning, hidden state and game compatibility. Previous CI packaging is not runtime evidence.
- Idle private working set, CPU, repeated-use resource stability and game anti-cheat compatibility. Hidden-host targets remain ≤30 MB private working set and <0.1% mean CPU, unmeasured.

No stable release or measured RAM claim follows from compile checks or dependency reduction. Follow `docs/testing.md` and record evidence before a release.
