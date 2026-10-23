use blink_verdict::{
    config::{ArtStyle, Config},
    imaging::decode_png,
};
use std::borrow::Cow;

pub struct Art {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    pub source: [f32; 4],
    pub offset: [f32; 2],
}

/// Fixed optical centers for the generated atlas. Fractions of a 512px cell;
/// custom images keep their own framing. No runtime alpha scan is needed.
pub fn atlas_offset(config: &Config, text: &str) -> [f32; 2] {
    if config.option_images.contains_key(text) {
        return [0.0, 0.0];
    }
    let column = match config.theme.art_style {
        ArtStyle::Ink => 0,
        ArtStyle::Cyberpunk => 1,
        ArtStyle::Graffiti => 2,
        ArtStyle::Plain => return [0.0, 0.0],
    };
    let pixels = if text.eq_ignore_ascii_case("YES") {
        [[-11.0, -36.5], [0.5, -39.0], [21.0, -86.0]][column]
    } else if text.eq_ignore_ascii_case("NO") {
        [[-12.5, 56.0], [-5.5, 54.0], [12.0, 44.5]][column]
    } else {
        return [0.0, 0.0];
    };
    [pixels[0] / 512.0, pixels[1] / 512.0]
}

/// Decode only while a result is visible; no full-atlas pixel buffer or embedded PNG copy.
pub fn load(config: &Config, text: &str) -> Option<Art> {
    let custom = config.option_images.get(text);
    let bytes: Cow<'_, [u8]> = if let Some(path) = custom {
        // Limit the actual read too, in case the file changes after inspecting its metadata.
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .ok()?
            .take(8 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > 8 * 1024 * 1024 {
            return None;
        }
        Cow::Owned(bytes)
    } else {
        if config.theme.art_style == ArtStyle::Plain
            || !(text.eq_ignore_ascii_case("YES") || text.eq_ignore_ascii_case("NO"))
        {
            return None;
        }
        Cow::Borrowed(include_bytes!("../../assets/wordart.png"))
    };
    let cell = if custom.is_some() {
        None
    } else {
        let column = match config.theme.art_style {
            ArtStyle::Ink => 0,
            ArtStyle::Cyberpunk => 1,
            ArtStyle::Graffiti => 2,
            ArtStyle::Plain => return None,
        };
        Some((
            column,
            if text.eq_ignore_ascii_case("YES") {
                0
            } else {
                1
            },
        ))
    };
    let pixels = decode_png(&bytes, cell).ok()?;
    Some(Art {
        width: pixels.width,
        height: pixels.height,
        source: [0.0, 0.0, pixels.width as f32, pixels.height as f32],
        offset: atlas_offset(config, text),
        bgra: pixels.bgra,
    })
}
