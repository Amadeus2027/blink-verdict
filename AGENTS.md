# BlinkVerdict development rules

- Follow the Andrej Karpathy guidelines: state consequential assumptions, prefer the simplest working solution, make surgical changes, and verify concrete outcomes.
- Windows 10/11 native Rust application. Keep the resident process small; no webview, bundled model, idle capture, polling, or continuous OCR.
- Never activate an overlay, intercept game input, inject into another process, or claim untested exclusive-fullscreen compatibility.
- Keep fullscreen and widget modes visually consistent. Lock and click-through the widget outside explicit editing.
- Word art is an image asset, not a general-purpose font. Preserve original generated files; load result art only on demand. Keep custom text editable with native fonts.
- Maintain README using the clear brand header, preview, quick-start, capability tables and collapsible technical detail pattern from https://github.com/tt-a1i/archify. Use independent assets and only substantiated claims.
- Daily verification: compile affected targets and run only the small core test suite or a directly relevant regression. Do not add tests that merely restate implementation.
- Keep broad compatibility, endurance, performance, and live-provider tests in the release checklist, outside default CI. Run them before each release and record evidence; unexecuted checks stay unverified.
- Never commit API keys, screenshots, local configuration, or build artifacts. Store secrets in Windows Credential Manager.
- Do not add unrelated features, speculative abstractions, automatic game hooks, or background services.
- Report Linux/cross-compilation checks separately from actual Windows/game validation.
