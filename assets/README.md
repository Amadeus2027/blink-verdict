# Generated word-art atlas

`wordart.png` contains original generated YES/NO lettering for the light, minimal interface. It is transparent artwork, not an installable font or an embedded Apple typeface.

The design was reviewed against the actual [Apple font specimens](https://developer.apple.com/fonts/), [Apple typography guidance](https://developer.apple.com/design/human-interface-guidelines/typography) and [Inter display/UI specimens](https://rsms.me/inter/) before generation. The resulting styles use clear forms and readable strokes rather than hairlines, glow or decorative backgrounds.

| Column | Visible preset | Saved key retained for compatibility |
|---|---|---|
| 0 | 清晰无衬线 / Clear sans | `Ink` |
| 1 | 柔和圆角 / Soft rounded | `Cyberpunk` |
| 2 | 现代衬线 / Modern serif | `Graffiti` |

- Canvas: 1536 × 1024 RGBA, three columns, two rows; YES above NO.
- Every source cell remains 512 × 512. Native Direct2D and Game Bar use the same atlas and crop layout. No new rendering path, dependency or idle work.
- The generated source is preserved at `generated_images/exec-60a2c591-4144-4418-a544-0daf8fb6dad9.png` in the session workspace. Packaging clears RGB values only where alpha is zero and compresses the PNG; visible pixels and all alpha values were verified unchanged.
- The previous atlas is preserved verbatim as [wordart-expressive.png](archive/wordart-expressive.png). Neither executable embedding nor Game Bar content includes that archive.
- [Documentation preview](../docs/assets/wordart-preview.png) shows the new letters on a light surface for legibility in dark-mode readers. That surface is not part of the runtime atlas.
- Plain text, arbitrary user labels and custom PNGs keep their existing behavior. These monochrome presets use their image colors; text-color changes apply to plain text.
