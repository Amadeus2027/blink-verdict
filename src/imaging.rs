//! Windows Imaging Component codecs. Factories, streams and pixels live only for one operation.
use windows::{
    core::Result,
    Win32::{
        Foundation::E_FAIL,
        Graphics::Imaging::*,
        System::Com::{StructuredStorage::CreateStreamOnHGlobal, *},
    },
};

pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

/// PNG decode with optional atlas cell selection (column, row). Copy only the selected cell.
pub fn decode_png(bytes: &[u8], cell: Option<(u32, u32)>) -> Result<Pixels> {
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let stream = factory.CreateStream()?;
        // The borrowed bytes outlive the decoder and its stream.
        stream.InitializeFromMemory(bytes)?;
        let decoder = factory.CreateDecoderFromStream(
            &stream,
            std::ptr::null(),
            WICDecodeMetadataCacheOnDemand,
        )?;
        if decoder.GetContainerFormat()? != GUID_ContainerFormatPng {
            return Err(E_FAIL.into());
        }
        let frame = decoder.GetFrame(0)?;
        let (mut width, mut height) = (0, 0);
        frame.GetSize(&mut width, &mut height)?;
        if width == 0 || height == 0 || width > 4096 || height > 4096 {
            return Err(E_FAIL.into());
        }
        let rect = if let Some((column, row)) = cell {
            if column >= 3 || row >= 2 || width % 3 != 0 || height % 2 != 0 {
                return Err(E_FAIL.into());
            }
            width /= 3;
            height /= 2;
            WICRect {
                X: (column * width) as i32,
                Y: (row * height) as i32,
                Width: width as i32,
                Height: height as i32,
            }
        } else {
            WICRect {
                X: 0,
                Y: 0,
                Width: width as i32,
                Height: height as i32,
            }
        };
        let converter = factory.CreateFormatConverter()?;
        converter.Initialize(
            &frame,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
        let mut bgra = vec![0; (width * height * 4) as usize];
        converter.CopyPixels(&rect, width * 4, &mut bgra)?;
        Ok(Pixels {
            width,
            height,
            bgra,
        })
    }
}

/// GDI pixels are BGRX; ignore their undefined alpha, without an intermediate RGB vector.
pub fn encode_screen_png(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>> {
    if width == 0
        || height == 0
        || width > 1600
        || height > 1600
        || pixels.len() != (width * height * 4) as usize
    {
        return Err(E_FAIL.into());
    }
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)?;
        let source = factory.CreateBitmapFromMemory(
            width,
            height,
            &GUID_WICPixelFormat32bppBGR,
            width * 4,
            pixels,
        )?;
        let stream = CreateStreamOnHGlobal(None, true)?;
        let encoder = factory.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null())?;
        encoder.Initialize(&stream, WICBitmapEncoderNoCache)?;
        let mut frame = None;
        let mut options = None;
        encoder.CreateNewFrame(&mut frame, &mut options)?;
        let frame = frame.ok_or(E_FAIL)?;
        frame.Initialize(options.as_ref())?;
        frame.SetSize(width, height)?;
        let mut format = GUID_WICPixelFormat24bppBGR;
        frame.SetPixelFormat(&mut format)?;
        frame.WriteSource(&source, std::ptr::null())?;
        frame.Commit()?;
        encoder.Commit()?;
        let mut info = STATSTG::default();
        stream.Stat(&mut info, STATFLAG_NONAME)?;
        if info.cbSize > 16 * 1024 * 1024 {
            return Err(E_FAIL.into());
        }
        let mut bytes = vec![0; info.cbSize as usize];
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        let mut read = 0;
        stream
            .Read(
                bytes.as_mut_ptr().cast(),
                bytes.len() as u32,
                Some(&mut read),
            )
            .ok()?;
        if read as usize != bytes.len() {
            return Err(E_FAIL.into());
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_png_roundtrip_and_atlas_crop() {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok().unwrap();
        }
        let source = [0, 0, 255, 0, 0, 255, 0, 0, 255, 0, 0, 0].repeat(2);
        let png = encode_screen_png(3, 2, &source).unwrap();
        let all = decode_png(&png, None).unwrap();
        assert_eq!((all.width, all.height), (3, 2));
        assert_eq!(&all.bgra[..4], &[0, 0, 255, 255]);
        let green = decode_png(&png, Some((1, 1))).unwrap();
        assert_eq!((green.width, green.height), (1, 1));
        assert_eq!(green.bgra, [0, 255, 0, 255]);
        assert!(decode_png(b"invalid image", None).is_err());
        let art = decode_png(include_bytes!("../assets/wordart.png"), Some((0, 0))).unwrap();
        assert_eq!((art.width, art.height), (512, 512));
        assert!(art.bgra.chunks_exact(4).any(|p| p[3] == 0));
        assert!(art.bgra.chunks_exact(4).any(|p| p[3] > 0));
        assert!(art
            .bgra
            .chunks_exact(4)
            .all(|p| p[..3].iter().all(|v| *v <= p[3])));
        unsafe {
            CoUninitialize();
        }
    }
}
