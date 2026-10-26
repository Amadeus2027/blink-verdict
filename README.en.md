<p align="center"><a href="README.md" lang="zh-CN">Chinese</a> · <strong>English</strong></p>

<p align="center">
  <img src="docs/assets/lockup.png" alt="BlinkVerdict" width="530" />
</p>

<h3 align="center">An answer for the choice in front of you.</h3>

<p align="center">Press a shortcut. Let the answer appear.<br/>Use your current screen to choose from your own options, with a local or cloud model.</p>

<p align="center">
  <a href="https://github.com/Amadeus2027/blink-verdict/releases/download/v0.1.0-rc.2/blink-verdict-0.1.0-rc.2-windows-x64.zip"><strong>Download for Windows</strong></a> &nbsp;·&nbsp;
  <a href="#get-started"><strong>Get started</strong></a> &nbsp;·&nbsp;
  <a href="docs/assets/showcase.mp4"><strong>Watch the demo</strong></a> &nbsp;·&nbsp;
  <a href="#choose-your-model"><strong>Models</strong></a> &nbsp;·&nbsp;
  <a href="#privacy"><strong>Privacy</strong></a>
</p>

<p align="center"><sub>Windows 10 / 11 · x64 · v0.1.0-rc.2 preview · <a href="LICENSE">MIT</a></sub></p>

<p align="center"><a href="docs/assets/showcase.mp4"><img src="docs/assets/showcase.gif" alt="Fullscreen text, the persistent widget, word art, and model settings" width="560" /></a><br/><sub><a href="docs/assets/showcase.mp4">Watch the full 39-second film</a> · GIF: about 1 MB. Answers shown are appearance previews.</sub></p>

## When you need a choice, press once

Write a goal and list the answers the model may choose. For example, ask it to assess the current situation and choose **Attack / Retreat / Wait**. Press **Ctrl + Alt + Space** when you need a decision, and the result appears in your preferred style.

| Fullscreen text | Persistent widget |
|---|---|
| Text or transparent word art, with no background, mask, or screen dimming. | A rounded, translucent panel that keeps the latest result visible. |
| Fades away after 1.5 seconds by default; duration is adjustable. | Drag, resize, or enter exact coordinates; adjust background opacity separately. |
| For an occasional answer. | For keeping an answer in view, with optional automatic dismissal. |

Both modes share your model, goal, and options. The model may return an unknown result when evidence is insufficient. Model decisions can be wrong; check the result yourself.

## Download

**[Download Windows x64 · ZIP](https://github.com/Amadeus2027/blink-verdict/releases/download/v0.1.0-rc.2/blink-verdict-0.1.0-rc.2-windows-x64.zip)** · [Release notes and checksums](https://github.com/Amadeus2027/blink-verdict/releases/tag/v0.1.0-rc.2) · [Changelog](CHANGELOG.md)

**v0.1.0-rc.2** is a public preview. Extract the ZIP and run `blink-verdict.exe`; no installation or administrator privileges required. Automated acceptance checks passed on Windows CI. Consumer Windows hardware, games, and multi-monitor compatibility still need verification. [Acceptance record](docs/releases/0.1.0-rc.2.md).

| Requirement | Details |
|---|---|
| Target systems | Windows 10 22H2 / Windows 11, x64; physical-device validation remains pending |
| Model access | An API account or a running compatible local server; model weights are not included |
| Download verification | Compare the ZIP with `SHA256SUMS.txt` on the release page; the package includes source and file manifests |
| Code signing | No Authenticode signature yet; Windows may show an unknown publisher warning |

**Language:** this is the English product introduction. The application interface, demo captions, and supporting guides are currently in Chinese.

## Get started

1. **Connect a model.** First launch opens model setup. Select a provider, enter its model name, endpoint, and API key if required, then continue. Start a local model server separately before connecting.
2. **Define the choice.** Write what you want to decide and enter one possible answer per line. Start with YES / NO or supply 2–20 options, then finish setup.
3. **Press for an answer.** The default shortcut is **Ctrl + Alt + Space**. To change it, select the shortcut control in settings and press a new combination.

BlinkVerdict stays in the system tray. Click its icon to open settings; right-click to pause, switch display modes, adjust the widget, or quit. Repeated shortcuts during a decision do not queue extra requests. Canceling initial setup leaves decisions disabled until you complete it.

## Make the answer yours

- **Text and word art:** choose a font, size, and color. Three built-in YES / NO styles offer clean lettering, luminous glass, and soft handwritten color. Use transparent PNGs for your own answers.
- **Entrance effects:** appear instantly, fade, flash, scale in, or slide upward.
- **Timing:** keep fullscreen text visible for 0.3–60 seconds. Leave the widget visible or dismiss it automatically.
- **Widget layout:** drag, resize, or enter exact dimensions and position. Text and widget dimensions follow display scaling. Finishing an adjustment restores click-through behavior.
- **Separate opacity controls:** adjust the widget background from fully transparent to solid, independently of the text.

Appearance preview does not capture the screen, call a model, or save changes. Finish to save; cancel to discard. During layout editing, a temporary visible surface makes even a fully transparent widget draggable. The widget uses translucency; glass word art is a pre-rendered image without live background blur or refraction.

<details>
<summary>See the three built-in word art styles</summary>

![Clean lettering, luminous glass, and soft handwritten YES / NO word art](docs/assets/wordart-preview.png)

Presets retain their original colors and apply to YES / NO. Other answers use your selected font. Use plain text for freely adjustable colors. Custom transparent PNGs override built-in art; select them again if you move or delete the source files.

</details>

## Choose your model

| Connection | Implemented adapters and presets |
|---|---|
| Decision services | TypeSafe Jev, System One, Perplexity Decider, Cloudflare Clef, OpenAI Decisions |
| General models | OpenAI Chat Completions / Responses, Anthropic, Google Gemini, Ollama, and services using compatible protocols |
| Local servers | llama.cpp (llama-server), System One / vLLM, Ollama, LM Studio, and other running compatible servers |

Send a screenshot to a vision-capable model, or extract text locally with Windows OCR for a text model. **An implemented adapter does not imply live vendor certification.** The llama-server text interface has been exercised with public test weights. Other adapters primarily have synthetic and loopback coverage; real accounts, user models, and vision capabilities have separate validation limits in the release record.

Probabilities and confidence values returned by decision services are available in tray status information; they do not guarantee accuracy. Accounts, fees, and availability depend on each provider.

**Using llama.cpp?** Select its local preset, enter your server endpoint and model name, and connect. [llama-server setup guide](docs/llama-cpp.md) · [All endpoints, model names, and input capabilities](docs/models.md).

## Small while waiting. Active when asked.

Built with native Rust and Windows APIs, without a bundled browser runtime or inference engine. Screen capture, OCR, and model requests run only when you trigger a decision. Animation refresh stops when the result disappears. Each decision uses a short-lived worker process; timeouts terminate the worker and restore the shortcut. No worker remains resident while idle.

The current x64 release ZIP is about **0.97 MB**, with a **1.29 MB** executable. In 60-sample Windows runner observations, average private working set was about **2.21 MiB** while idle after use and **5.12 MiB** with the widget visible. These are observations from that environment, not guarantees for every device. Active worker and local model costs are additional. [Conditions and active-process measurements](docs/releases/0.1.0-rc.2.md).

## Privacy

- Captures the display containing the foreground application only when a decision is triggered; no continuous background capture.
- Does not save screenshots or decision history by default. No telemetry or automatic update requests.
- Cloud providers receive the current screenshot or OCR text, goal, and options. Local mode sends them to your configured local service. Each service has its own retention policy.
- API keys are stored in Windows Credential Manager, separately from ordinary settings.

## Games and display compatibility

The desktop overlay uses non-activating, click-through windows. **No game has certified compatibility yet.** Exclusive fullscreen, anti-cheat software, and protected content may prevent capture or display. Try borderless mode first. The optional [Game Bar component](docs/gamebar.md) is experimental, installed separately, and excluded from the main release ZIP.

[Compatibility records](docs/compatibility.md) · [Current release validation](docs/releases/0.1.0-rc.2.md)

## Common questions

**How do I update or remove it?** Quit from the tray and replace the program files. Settings stay at `%LOCALAPPDATA%\BlinkVerdict\config.json`. To uninstall, delete the program files. To also remove settings and keys, delete that configuration directory and Windows Credential Manager entries beginning with `BlinkVerdict/`.

**Why is there no answer?** Check the tray status and connection information. Verify the endpoint, key, model capabilities, and local server status. OCR needs the relevant Windows language pack. Try borderless mode for black or protected captures. GDI capture does not provide HDR tone mapping: when HDR or advanced color is detected, the request stops with a prompt to disable it, rather than silently using incorrect colors. [Connection guide](docs/models.md).

**Can it work offline?** Yes, with a local server and its model already available. Downloads, networking, and inference resource usage depend on that server software.

## Contribute

Use [Issues](https://github.com/Amadeus2027/blink-verdict/issues) for bugs or suggestions. Include the application version, Windows build, display scaling, reproduction steps, and redacted error details. Keep API keys and private screen content out of reports.

<details>
<summary>Build from source and validate</summary>

Install Rust stable and Visual Studio C++ Build Tools, then run on Windows:

```powershell
git clone https://github.com/Amadeus2027/blink-verdict.git
cd blink-verdict
cargo build --locked --release
.\target\release\blink-verdict.exe
```

Daily checks use the small core suite. Broader network, resource, and compatibility checks run before releases. [Testing](docs/testing.md) · [Release process](docs/releases/README.md) · [Build and device evidence](docs/validation.md).

</details>

Licensed under [MIT](LICENSE). Dependencies retain their own licenses; the release includes their notices. BlinkVerdict is not affiliated with the model providers it connects to.
