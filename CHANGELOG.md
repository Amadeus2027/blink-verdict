# Changelog

## 0.1.0 — Unreleased development preview

- Native tray host, configurable global shortcut and single-instance startup.
- Passive fullscreen and click-through widget rendering, shared theme and independent lifetimes.
- Five presentation effects and three AI-generated YES/NO art presets; per-answer custom PNG mappings.
- Native settings, custom decision task/options, provider presets and Windows Credential Manager storage.
- On-demand monitor capture, local Windows OCR, OpenAI-compatible / Responses / Anthropic / Gemini / Ollama adapters.
- Optional experimental Game Bar AppService bridge and widget source; installation and runtime validation remain Windows release gates.
- Native Jev, Perplexity Decider, Cloudflare Clef and OpenAI Decisions adapters, retaining typed choices/probabilities/confidence and explicit abstention.
- Local System One (including vLLM image input), Decider and OpenAI Decisions presets; existing five chat adapters retained. Existing configurations remain compatible.
- Numeric widget geometry, negative display coordinates, reset and borderless resize during explicit edit mode.
- WinHTTP and Windows Imaging Component replace reqwest / third-party PNG; copy only the selected built-in art cell and release request buffers before network waits. Locked package count decreases from 146 to 29.
- Portable ZIP contains only the standalone executable, quick-start and changelog; embedded art is not duplicated as loose PNG.
- Four daily core groups plus one Windows PNG smoke test; broad network tests and platform/performance acceptance remain outside default CI.

Windows interaction, live providers, exclusive-fullscreen compatibility and footprint targets are not yet validated.
