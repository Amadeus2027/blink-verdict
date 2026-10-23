# Compatibility record

This file records observed behavior, not inferred support. No game has been verified yet.

| Environment | Capture | Native fullscreen | Native widget | Game Bar fullscreen / widget | Focus / input |
|---|---|---|---|---|---|
| Windows runner NT 26100, synthetic receiver | PNG to loopback passed | Native alpha check passed | Native alpha check passed | Build only; runtime unverified | Foreground, SendInput keyboard/mouse passed |
| Windows 10 22H2 desktop | Unverified | Unverified | Unverified | Unverified | Unverified |
| Windows 11 desktop | Unverified | Unverified | Unverified | Unverified | Unverified |
| Borderless games | Unverified | Unverified | Unverified | Unverified | Unverified |
| Exclusive-fullscreen games | Unverified; GDI may return black | Not guaranteed | Not guaranteed | Experimental, unverified | Unverified |

For each tested game record the game/version, Windows build, GPU/driver, display mode, DPI, anti-cheat, screenshot result, overlay behavior, and whether keyboard/mouse/focus were preserved. A visible overlay alone does not count as passing.

Do not test protected games using injection or disable anti-cheat to obtain a passing result. Prefer borderless mode if capture or passive presentation is unavailable.

Runner evidence: [0.1.0-rc.1 acceptance](releases/0.1.0-rc.1.md). This uses ordinary host startup for desktop shortcut/input tests and a separate recording-mode capture for native alpha. It does not certify consumer Windows or games.
