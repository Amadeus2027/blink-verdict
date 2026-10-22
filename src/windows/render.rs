use blink_verdict::config::{parse_color, Config, DisplayMode};
use windows::{
    core::{w, Result, PCWSTR},
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
            Dxgi::Common::*,
            Gdi::*,
        },
        UI::WindowsAndMessaging::{GetClientRect, UpdateLayeredWindow, ULW_ALPHA},
    },
};

pub struct Renderer {
    target: ID2D1DCRenderTarget,
    write: IDWriteFactory,
    art: Option<(ID2D1Bitmap, D2D_RECT_F)>,
    hwnd: HWND,
    surface: Surface,
}

struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    size: SIZE,
}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bitmap);
            let _ = DeleteDC(self.dc);
        }
    }
}

impl Surface {
    unsafe fn new(width: i32, height: i32) -> Result<Self> {
        let dc = CreateCompatibleDC(None);
        if dc.is_invalid() {
            return Err(windows::core::Error::from_win32());
        }
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut bits, None, 0) {
            Ok(bitmap) => bitmap,
            Err(error) => {
                let _ = DeleteDC(dc);
                return Err(error);
            }
        };
        let old = SelectObject(dc, bitmap);
        Ok(Self {
            dc,
            bitmap,
            old,
            size: SIZE {
                cx: width,
                cy: height,
            },
        })
    }
}

fn color(value: &str) -> D2D1_COLOR_F {
    let [r, g, b] = parse_color(value).unwrap_or([17, 19, 24]);
    D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

impl Renderer {
    pub unsafe fn new(hwnd: HWND, config: &Config, text: &str) -> Result<Self> {
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect)?;
        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            // Explicit pixel units keep rendering consistent with per-monitor-aware geometry.
            dpiX: 96.0,
            dpiY: 96.0,
            ..Default::default()
        };
        // Layered windows use the supported DC/DIB + UpdateLayeredWindow path.
        let surface = Surface::new(rect.right.max(1), rect.bottom.max(1))?;
        let target = factory.CreateDCRenderTarget(&properties)?;
        target.BindDC(surface.dc, &rect)?;
        target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        let write = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let art = super::art::load(config, text).and_then(|image| {
            target
                .CreateBitmap(
                    D2D_SIZE_U {
                        width: image.width,
                        height: image.height,
                    },
                    Some(image.bgra.as_ptr() as *const _),
                    image.width * 4,
                    &D2D1_BITMAP_PROPERTIES {
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                        },
                        dpiX: 96.0,
                        dpiY: 96.0,
                    },
                )
                .ok()
                .map(|bitmap| {
                    (
                        bitmap,
                        D2D_RECT_F {
                            left: image.source[0],
                            top: image.source[1],
                            right: image.source[2],
                            bottom: image.source[3],
                        },
                    )
                })
        });
        Ok(Self {
            target,
            write,
            art,
            hwnd,
            surface,
        })
    }

    pub unsafe fn print_to(&self, dc: HDC) {
        let _ = BitBlt(
            dc,
            0,
            0,
            self.surface.size.cx,
            self.surface.size.cy,
            self.surface.dc,
            0,
            0,
            SRCCOPY,
        );
    }

    pub unsafe fn paint(
        &self,
        config: &Config,
        text: &str,
        editing: bool,
        motion: blink_verdict::animation::Motion,
    ) -> Result<()> {
        let size = self.target.GetSize();
        self.target.BeginDraw();
        let widget = config.mode == DisplayMode::Widget;
        if widget {
            self.target.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
            let mut tint = color(&config.theme.background_color);
            tint.a = 0.92;
            let background = self.target.CreateSolidColorBrush(&tint, None)?;
            let border = self.target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 0.16,
                },
                None,
            )?;
            let shape = D2D1_ROUNDED_RECT {
                rect: D2D_RECT_F {
                    left: 0.5,
                    top: 0.5,
                    right: size.width - 0.5,
                    bottom: size.height - 0.5,
                },
                radiusX: 16.0,
                radiusY: 16.0,
            };
            self.target.FillRoundedRectangle(&shape, &background);
            self.target.DrawRoundedRectangle(&shape, &border, 1.0, None);
        } else {
            self.target
                .Clear(Some(&color(&config.theme.background_color)));
        }
        let brush = self
            .target
            .CreateSolidColorBrush(&color(&config.theme.text_color), None)?;
        let margin = if widget { 18.0 } else { size.width * 0.06 };
        let top = if widget { 34.0 } else { margin };
        let bottom = if widget {
            size.height - 28.0
        } else {
            size.height - margin
        };
        let rect = D2D_RECT_F {
            left: margin,
            top,
            right: (size.width - margin).max(margin + 1.0),
            bottom: bottom.max(top + 1.0),
        };
        let font_size = if widget {
            config.theme.widget_font_size
        } else {
            config.theme.fullscreen_font_size
        } as f32;
        use windows::Foundation::Numerics::Matrix3x2;
        let cx = (rect.left + rect.right) / 2.0;
        let cy = (rect.top + rect.bottom) / 2.0;
        self.target.SetTransform(&Matrix3x2 {
            M11: motion.scale,
            M12: 0.0,
            M21: 0.0,
            M22: motion.scale,
            M31: cx * (1.0 - motion.scale),
            M32: cy * (1.0 - motion.scale) + motion.offset_y,
        });
        if let Some((bitmap, source)) = &self.art {
            let ratio = (source.right - source.left) / (source.bottom - source.top);
            let height = (font_size * 3.0)
                .min(rect.bottom - rect.top)
                .min((rect.right - rect.left) / ratio);
            let width = height * ratio;
            let destination = D2D_RECT_F {
                left: cx - width / 2.0,
                top: cy - height / 2.0,
                right: cx + width / 2.0,
                bottom: cy + height / 2.0,
            };
            self.target.DrawBitmap(
                bitmap,
                Some(&destination),
                1.0,
                D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                Some(source),
            );
        } else {
            self.text(text, &config.theme.font, font_size, rect, &brush)?;
        }
        self.target.SetTransform(&Matrix3x2 {
            M11: 1.0,
            M12: 0.0,
            M21: 0.0,
            M22: 1.0,
            M31: 0.0,
            M32: 0.0,
        });
        if widget {
            let muted = self.target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 0.65,
                    g: 0.66,
                    b: 0.70,
                    a: 1.0,
                },
                None,
            )?;
            self.text(
                "B L I N K V E R D I C T",
                &config.theme.font,
                10.0,
                D2D_RECT_F {
                    left: 12.0,
                    top: 6.0,
                    right: size.width - 12.0,
                    bottom: 27.0,
                },
                &muted,
            )?;
            let footer = if editing {
                "编辑中 · 拖动 / 缩放 · 右键完成"
            } else {
                &config.hotkey
            };
            self.text(
                footer,
                &config.theme.font,
                10.0,
                D2D_RECT_F {
                    left: 8.0,
                    top: size.height - 25.0,
                    right: size.width - 8.0,
                    bottom: size.height - 4.0,
                },
                &muted,
            )?;
        }
        self.target.EndDraw(None, None)?;
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: (motion.opacity * config.theme.opacity as f32).clamp(0.0, 255.0)
                as u8,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        UpdateLayeredWindow(
            self.hwnd,
            None,
            None,
            Some(&self.surface.size),
            self.surface.dc,
            Some(&POINT { x: 0, y: 0 }),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        )
    }

    unsafe fn text(
        &self,
        text: &str,
        family: &str,
        size: f32,
        rect: D2D_RECT_F,
        brush: &ID2D1SolidColorBrush,
    ) -> Result<()> {
        let family: Vec<u16> = family.encode_utf16().chain(Some(0)).collect();
        let content: Vec<u16> = text.encode_utf16().collect();
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        // Measure wrapped text and reduce the font if a custom result would be clipped.
        let make = |size| -> Result<IDWriteTextLayout> {
            let format = self.write.CreateTextFormat(
                PCWSTR(family.as_ptr()),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                size,
                w!("zh-CN"),
            )?;
            format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            self.write
                .CreateTextLayout(&content, &format, width, height)
        };
        let mut layout = make(size)?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        layout.GetMetrics(&mut metrics)?;
        if metrics.height > height || metrics.width > width {
            let ratio = (height / metrics.height)
                .min(width / metrics.width.max(1.0))
                .min(1.0);
            layout = make((size * ratio * 0.95).max(8.0))?;
        }
        self.target.DrawTextLayout(
            D2D_POINT_2F {
                x: rect.left,
                y: rect.top,
            },
            &layout,
            brush,
            D2D1_DRAW_TEXT_OPTIONS_CLIP,
        );
        Ok(())
    }
}
