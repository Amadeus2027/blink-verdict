use blink_verdict::config::{ArtStyle, Config};
use std::io::Cursor;

pub struct Art {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
    pub source: [f32; 4],
}

/// Decode only while a result is visible; the renderer owns and releases the GPU bitmap.
pub fn load(config: &Config, text: &str) -> Option<Art> {
    let custom = config.option_images.get(text);
    let bytes = if let Some(path) = custom {
        if std::fs::metadata(path).ok()?.len() > 8 * 1024 * 1024 {
            return None;
        }
        std::fs::read(path).ok()?
    } else {
        if config.theme.art_style == ArtStyle::Plain
            || !["YES", "NO"].contains(&text.to_ascii_uppercase().as_str())
        {
            return None;
        }
        include_bytes!("../../assets/wordart.png").to_vec()
    };
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    if reader.info().width > 4096
        || reader.info().height > 4096
        || reader.output_buffer_size() > 64 * 1024 * 1024
    {
        return None;
    }
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).ok()?;
    let channels = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        png::ColorType::GrayscaleAlpha => 2,
        png::ColorType::Grayscale => 1,
        _ => return None,
    };
    let mut bgra = Vec::with_capacity((info.width * info.height * 4) as usize);
    for pixel in buffer[..info.buffer_size()].chunks_exact(channels) {
        let (r, g, b, a) = match channels {
            4 => (pixel[0], pixel[1], pixel[2], pixel[3]),
            3 => (pixel[0], pixel[1], pixel[2], 255),
            2 => (pixel[0], pixel[0], pixel[0], pixel[1]),
            _ => (pixel[0], pixel[0], pixel[0], 255),
        };
        let premultiply = |v: u8| ((v as u16 * a as u16 + 127) / 255) as u8;
        bgra.extend_from_slice(&[premultiply(b), premultiply(g), premultiply(r), a]);
    }
    let source = if custom.is_some() {
        [0.0, 0.0, info.width as f32, info.height as f32]
    } else {
        let column = match config.theme.art_style {
            ArtStyle::Ink => 0,
            ArtStyle::Cyberpunk => 1,
            ArtStyle::Graffiti => 2,
            ArtStyle::Plain => return None,
        };
        let row = if text.eq_ignore_ascii_case("YES") {
            0
        } else {
            1
        };
        let w = info.width as f32 / 3.0;
        let h = info.height as f32 / 2.0;
        [
            column as f32 * w,
            row as f32 * h,
            (column + 1) as f32 * w,
            (row + 1) as f32 * h,
        ]
    };
    Some(Art {
        width: info.width,
        height: info.height,
        bgra,
        source,
    })
}
