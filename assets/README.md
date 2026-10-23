# Generated word-art atlas

`wordart.png` contains original generated YES/NO lettering for the light, minimal interface. It is transparent artwork, not an installable font or an embedded Apple typeface.

The design was reviewed against the actual [Apple font specimens](https://developer.apple.com/fonts/), [Apple typography guidance](https://developer.apple.com/design/human-interface-guidelines/typography), [Liquid Glass lock-screen example](https://www.apple.com/newsroom/2025/06/apple-introduces-a-delightful-and-elegant-new-software-design/) and [Hello welcome lettering](https://www.redmondpie.com/ios-15-shows-hello-message-first-time-you-launch-it-on-iphone-or-ipad/) before generation.

The three palettes coordinate with the light interface: graphite-blue, pearl/ice-blue/lilac glass, and teal/sky-blue/lavender/coral handwriting. Glass is a prerendered visual inside the letters; it does not sample or refract the live desktop. Empty areas stay transparent.

| Column | Visible preset | Saved key retained for compatibility |
|---|---|---|
| 0 | 清晰纯色 / Clear color | `Ink` |
| 1 | 流光玻璃 / Tinted glass | `Cyberpunk` |
| 2 | 柔彩手写 / Soft script | `Graffiti` |

- Canvas: 1536 × 1024 RGBA, three columns, two rows; YES above NO.
- Every source cell remains 512 × 512. Native Direct2D and Game Bar use the same atlas and crop layout. Fixed offsets in `src/windows/art.rs` center each word; the host passes those fractions to Game Bar, which clips its source cell before translation to prevent adjacent sprites appearing. There is no runtime alpha scan, new rendering dependency or idle work.
- The generated source is preserved at `generated_images/exec-30988d9c-4148-44b3-8483-56afc801416f.png` in the session workspace. Packaging clears RGB values only where alpha is zero and compresses the PNG; visible pixels and all alpha values were verified unchanged.
- The previous atlas is preserved verbatim as [wordart-expressive.png](archive/wordart-expressive.png). Neither executable embedding nor Game Bar content includes that archive.
- [Documentation preview](../docs/assets/wordart-preview.png) shows the new letters on a light surface for legibility in dark-mode readers. That surface is not part of the runtime atlas.
- Plain text, arbitrary user labels and custom PNGs keep their existing behavior and framing. The third preset stylizes YES/NO as lowercase handwriting. Presets use their image colors; text-color changes apply to plain text.
