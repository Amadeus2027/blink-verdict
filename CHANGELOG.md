# Changelog

## 0.1.0-rc.1 — Candidate in preparation

- Release-only Windows validation, real desktop shortcut/capture/input checks, hidden/visible resource samples, allowlisted ZIP, source manifest, SHA-256 and third-party license texts.
- MIT project license; clearer download, upgrade, privacy and compatibility documentation.
- Reject a failed overlay bitmap selection and release the allocated GDI objects.

The candidate is not a stable release. Live services, consumer Windows/game compatibility and long-duration acceptance remain unverified.

### Included development work

- Capture owns its GDI frame through RAII and encodes from a borrowed slice, avoiding an extra full-screen pixel vector. Release capture handles before OCR recognition/network waits; release raw response bytes after JSON parsing.
- Separate settings state/events, page layouts and native controls. Remove the unreachable Apply command and its duplicate save/rebuild path; retain every used local/API adapter and saved protocol key.
- Coordinated glass/graphite brand lockup and a complete product film with eased perspective moves, native word-art previews, local/API connection views and a short hidden-host resource sample.

- Replace the expressive YES/NO presets with clear graphite-blue, softly tinted glass and welcome-style gradient script lettering, informed by actual Apple typography, Liquid Glass and Hello examples. Preserve saved preset keys and archive the previous artwork outside packaged assets.

- Light grouped settings inspired by the supplied ChatGPT iOS references: three main sections, white grouped rows, outline icons, compact navigation and quiet window chrome.
- First-run model → decision setup; click-and-press shortcut recording, color pickers, secondary pages for advanced options, and direct widget layout editing.
- Done saves and closes, Cancel discards, and the window close button offers to save unfinished work. Draft fields survive navigation; the four-second preview reads no screen and calls no model.
- Text-only transparent fullscreen output; only the widget has a rounded pane, with independent 0–100% background opacity. The optional Game Bar component follows the same rules.
- Remove the accent-policy experiment after native capture exposed gray panes/black corners. Keep correct per-pixel alpha, light defaults and existing preferences; no webview, frame polling or new runtime package.
- Product-focused interface, tray, README and portable copy. The original UI film was superseded by the comprehensive 39-second film and 26-second README preview.

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
