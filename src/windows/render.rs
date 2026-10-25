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
        UI::{HiDpi::GetDpiForWindow, WindowsAndMessaging::*},
    },
};

pub struct Renderer {
    target: ID2D1DCRenderTarget,
    write: IDWriteFactory,
    art: Option<(ID2D1Bitmap, D2D_RECT_F, [f32; 2])>,
    layout: Option<IDWriteTextLayout>,
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
        if old.is_invalid() {
            let error = windows::core::Error::from_win32();
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(dc);
            return Err(error);
        }
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
    pub unsafe fn new(hwnd: HWND, config: &Config, text: &str, bounds: RECT) -> Result<Self> {
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
        let dpi = GetDpiForWindow(hwnd).max(96) as f32;
        let scale = dpi / 96.0;
        let image = super::art::load(config, text);
        let mut rect = RECT::default();
        GetClientRect(hwnd, &mut rect)?;
        let layout = if config.mode == DisplayMode::Fullscreen {
            let max_width = (bounds.right - bounds.left).max(1) as f32 / scale;
            let max_height = (bounds.bottom - bounds.top).max(1) as f32 / scale;
            let (width, height, layout) = if let Some(image) = &image {
                let ratio =
                    (image.source[2] - image.source[0]) / (image.source[3] - image.source[1]);
                let height = (config.theme.fullscreen_font_size as f32 * 3.0)
                    .min((max_height - 96.0).max(1.0))
                    .min((max_width - 32.0).max(1.0) / ratio);
                (
                    height * ratio * (1.0 + 2.0 * image.offset[0].abs()) + 32.0,
                    height * (1.0 + 2.0 * image.offset[1].abs()) + 96.0,
                    None,
                )
            } else {
                let layout = text_layout(
                    &write,
                    text,
                    &config.theme.font,
                    config.theme.fullscreen_font_size as f32,
                    (max_width * 0.88 - 32.0).max(1.0),
                    (max_height * 0.88 - 96.0).max(1.0),
                )?;
                let mut metrics = DWRITE_TEXT_METRICS::default();
                layout.GetMetrics(&mut metrics)?;
                (
                    metrics.widthIncludingTrailingWhitespace + 32.0,
                    metrics.height + 96.0,
                    Some(layout),
                )
            };
            // Allocate/composite only the centered result bounds, including slide/AA padding.
            // The rest of the monitor has no overlay surface at all.
            let width = (width * scale)
                .ceil()
                .max(1.0)
                .min((bounds.right - bounds.left).max(1) as f32) as i32;
            let height = (height * scale)
                .ceil()
                .max(1.0)
                .min((bounds.bottom - bounds.top).max(1) as f32) as i32;
            SetWindowRgn(hwnd, HRGN::default(), false);
            SetWindowPos(
                hwnd,
                None,
                bounds.left + (bounds.right - bounds.left - width) / 2,
                bounds.top + (bounds.bottom - bounds.top - height) / 2,
                width,
                height,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )?;
            rect = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            layout
        } else {
            let radius = (48.0 * scale).round() as i32;
            let region = CreateRoundRectRgn(0, 0, rect.right, rect.bottom, radius, radius);
            if SetWindowRgn(hwnd, region, false) == 0 {
                let _ = DeleteObject(region);
            }
            None
        };

        let properties = D2D1_RENDER_TARGET_PROPERTIES {
            r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            // Fonts, padding and motion use DIPs; the DIB remains physical pixels.
            dpiX: dpi,
            dpiY: dpi,
            ..Default::default()
        };
        // Layered windows use the supported DC/DIB + UpdateLayeredWindow path.
        let surface = Surface::new(rect.right.max(1), rect.bottom.max(1))?;
        let target = factory.CreateDCRenderTarget(&properties)?;
        target.BindDC(surface.dc, &rect)?;
        target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        let art = image.and_then(|image| {
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
                        image.offset,
                    )
                })
        });
        Ok(Self {
            target,
            write,
            art,
            layout,
            hwnd,
            surface,
        })
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
        // Clear every pixel in both modes. Fullscreen owns no pane: the desktop
        // remains visible around text and transparent PNGs, including during fades.
        self.target.Clear(Some(&D2D1_COLOR_F {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        }));
        if widget && (editing || config.theme.widget_background_opacity > 0) {
            let mut tint = color(&config.theme.background_color);
            // A temporary edit surface keeps every interior pixel hittable even at 0%.
            tint.a = config
                .theme
                .widget_background_opacity
                .max(if editing { 48 } else { 0 }) as f32
                / 255.0;
            let background = self.target.CreateSolidColorBrush(&tint, None)?;
            let border = self.target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: tint.a * 0.68,
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
                radiusX: 24.0,
                radiusY: 24.0,
            };
            self.target.FillRoundedRectangle(&shape, &background);
            self.target.DrawRoundedRectangle(&shape, &border, 1.0, None);
        }
        let brush = self
            .target
            .CreateSolidColorBrush(&color(&config.theme.text_color), None)?;
        let margin = if widget { 18.0 } else { 16.0 };
        let top = if widget { 12.0 } else { 48.0 };
        let bottom = if widget {
            size.height - 28.0
        } else {
            size.height - 48.0
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
        if let Some((bitmap, source, offset)) = &self.art {
            let ratio = (source.right - source.left) / (source.bottom - source.top);
            let height = (font_size * 3.0)
                .min(rect.bottom - rect.top)
                .min((rect.right - rect.left) / ratio);
            let width = height * ratio;
            let cx = cx + offset[0] * width;
            let cy = cy + offset[1] * height;
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
        } else if let Some(layout) = &self.layout {
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics)?;
            self.target.DrawTextLayout(
                D2D_POINT_2F {
                    x: (size.width - metrics.widthIncludingTrailingWhitespace) / 2.0 - metrics.left,
                    y: (size.height - metrics.height) / 2.0 - metrics.top,
                },
                layout,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
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
                    r: 0.48,
                    g: 0.48,
                    b: 0.50,
                    a: 1.0,
                },
                None,
            )?;
            let footer = if editing {
                "右键完成调整"
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
        let layout = text_layout(
            &self.write,
            text,
            family,
            size,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )?;
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

unsafe fn text_layout(
    write: &IDWriteFactory,
    text: &str,
    family: &str,
    size: f32,
    width: f32,
    height: f32,
) -> Result<IDWriteTextLayout> {
    let family: Vec<u16> = family.encode_utf16().chain(Some(0)).collect();
    let content: Vec<u16> = text.encode_utf16().collect();
    // Measure wrapped text and reduce the font if a custom result would be clipped.
    let make = |size| -> Result<IDWriteTextLayout> {
        let format = write.CreateTextFormat(
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
        write.CreateTextLayout(&content, &format, width, height)
    };
    let mut layout = make(size)?;
    let mut metrics = DWRITE_TEXT_METRICS::default();
    layout.GetMetrics(&mut metrics)?;
    if metrics.height > height || metrics.width > width {
        // Wrapping changes nonlinearly with size. Search measured layouts rather
        // than applying one ratio and an 8-DIP floor that can still clip long labels.
        let (mut low, mut high) = (1.0, size);
        layout = make(low)?;
        for _ in 0..8 {
            let middle = (low + high) / 2.0;
            let candidate = make(middle)?;
            candidate.GetMetrics(&mut metrics)?;
            if metrics.height <= height && metrics.width <= width {
                low = middle;
                layout = candidate;
            } else {
                high = middle;
            }
        }
    }
    Ok(layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::windows::*;

    #[test]
    #[ignore = "targeted Windows regression: bounded 4K surface, native edit alpha and DPI draft retention"]
    fn runtime_regression_surface_and_dpi() {
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let mut fixture = TestApp::new();
            let app = &mut fixture.0;
            let bounds = RECT {
                left: 0,
                top: 0,
                right: 3840,
                bottom: 2160,
            };
            for text in [
                "YES",
                "NO",
                "这是一个较长的自定义中文答案",
                "第一行\n第二行",
            ] {
                let renderer = Renderer::new(app.overlay, &app.config, text, bounds).unwrap();
                let pixels = renderer.surface.size.cx * renderer.surface.size.cy;
                println!(
                    "4K viewport result {text:?}: {} x {} pixels, {} DIB bytes",
                    renderer.surface.size.cx,
                    renderer.surface.size.cy,
                    pixels * 4
                );
                assert!(
                    pixels < 3840 * 2160 / 2,
                    "short result still allocates the full monitor"
                );
                renderer
                    .paint(
                        &app.config,
                        text,
                        false,
                        blink_verdict::animation::sample(
                            blink_verdict::config::Animation::Slide,
                            0.11,
                            None,
                        ),
                    )
                    .unwrap();
                let mut rect = RECT::default();
                GetWindowRect(app.overlay, &mut rect).unwrap();
                assert!(((rect.left + rect.right) - 3840).abs() <= 1);
                assert!(((rect.top + rect.bottom) - 2160).abs() <= 1);
            }
            for dpi in [96, 144, 192] {
                assert_eq!(display::pixels(320, dpi), (320 * dpi / 96) as i32);
                assert_eq!(display::logical(display::pixels(320, dpi), dpi), 320);
            }
            app.config.mode = DisplayMode::Widget;
            SetWindowPos(
                app.overlay,
                None,
                10,
                10,
                320,
                140,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .unwrap();
            for opacity in [0, 180, 255] {
                app.config.theme.widget_background_opacity = opacity;
                let renderer = Renderer::new(app.overlay, &app.config, "YES", bounds).unwrap();
                let mut dib = DIBSECTION::default();
                assert_ne!(
                    GetObjectW(
                        renderer.surface.bitmap,
                        std::mem::size_of_val(&dib) as i32,
                        Some((&mut dib as *mut DIBSECTION).cast())
                    ),
                    0
                );
                for editing in [false, true] {
                    renderer
                        .paint(
                            &app.config,
                            "YES",
                            editing,
                            blink_verdict::animation::sample(
                                blink_verdict::config::Animation::None,
                                1.0,
                                None,
                            ),
                        )
                        .unwrap();
                    let pixels = std::slice::from_raw_parts(
                        dib.dsBm.bmBits.cast::<u8>(),
                        (320 * 140 * 4) as usize,
                    );
                    let alpha = pixels[(70 * 320 + 8) * 4 + 3];
                    if opacity == 0 && !editing {
                        assert_eq!(alpha, 0);
                    } else {
                        assert!(alpha > 0);
                    }
                }
                assert_eq!(app.config.theme.widget_background_opacity, opacity);
                let foreground = GetForegroundWindow();
                app.command(CMD_EDIT);
                let mut actual = RECT::default();
                GetWindowRect(app.overlay, &mut actual).unwrap();
                let point = POINT {
                    x: actual.left + 6,
                    y: (actual.top + actual.bottom) / 2,
                };
                assert_eq!(
                    WindowFromPoint(point),
                    app.overlay,
                    "edit pane did not receive native hit testing at alpha {opacity}"
                );
                let packed = ((point.y as u32 & 0xffff) << 16) | (point.x as u32 & 0xffff);
                assert_eq!(
                    SendMessageW(
                        app.overlay,
                        WM_NCHITTEST,
                        WPARAM(0),
                        LPARAM(packed as isize)
                    )
                    .0,
                    HTLEFT as isize
                );
                assert_eq!(GetForegroundWindow(), foreground);
                app.command(CMD_EDIT);
                assert_ne!(
                    WindowFromPoint(point),
                    app.overlay,
                    "locked widget stopped passing clicks through"
                );
                assert_eq!(app.config.theme.widget_background_opacity, opacity);
            }
            app.config.mode = DisplayMode::Fullscreen;
            settings::open(app);
            SendMessageW(app.settings, WM_COMMAND, WPARAM(211), LPARAM(0));
            let control = GetDlgItem(app.settings, 313).unwrap();
            SetWindowTextW(control, w!("unsaved-model-name")).unwrap();
            let mut suggested = RECT::default();
            GetWindowRect(app.settings, &mut suggested).unwrap();
            let foreground = GetForegroundWindow();
            SendMessageW(
                app.settings,
                WM_DPICHANGED,
                WPARAM(192 | (192 << 16)),
                LPARAM((&suggested as *const RECT) as isize),
            );
            let control = GetDlgItem(app.settings, 313).unwrap();
            let mut value = [0u16; 64];
            let count = GetWindowTextW(control, &mut value);
            assert_eq!(
                String::from_utf16_lossy(&value[..count as usize]),
                "unsaved-model-name"
            );
            assert_eq!(GetForegroundWindow(), foreground);
            let mut actual = RECT::default();
            GetWindowRect(app.settings, &mut actual).unwrap();
            let work =
                display::monitor_info(MonitorFromWindow(app.settings, MONITOR_DEFAULTTONEAREST))
                    .rcWork;
            assert!(
                actual.left >= work.left
                    && actual.top >= work.top
                    && actual.right <= work.right
                    && actual.bottom <= work.bottom
            );
        }
    }
    #[test]
    #[ignore = "release-only: repeated renderer/art lifetime and bounded GDI/USER resources"]
    fn release_surface_lifecycle() {
        unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap();
            struct Com;
            impl Drop for Com {
                fn drop(&mut self) {
                    unsafe {
                        CoUninitialize();
                    }
                }
            }
            let _com = Com;
            let mut fixture = TestApp::new();
            let app = &mut fixture.0;
            let write: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap();
            for text in ["长".repeat(80), "W".repeat(80)] {
                let layout =
                    text_layout(&write, &text, &app.config.theme.font, 120.0, 124.0, 40.0).unwrap();
                let mut metrics = DWRITE_TEXT_METRICS::default();
                layout.GetMetrics(&mut metrics).unwrap();
                assert!(
                    metrics.height <= 40.0 && metrics.width <= 124.0,
                    "long answer clips in minimum widget"
                );
                println!(
                    "minimum-widget 80-character layout: {:.2} x {:.2} DIPs",
                    metrics.width, metrics.height
                );
            }
            let bounds = RECT {
                left: -3840,
                top: -200,
                right: 0,
                bottom: 1960,
            };
            let mut run = |index: usize| {
                app.config.mode = if index.is_multiple_of(2) {
                    DisplayMode::Fullscreen
                } else {
                    DisplayMode::Widget
                };
                app.config.theme.art_style = [
                    blink_verdict::config::ArtStyle::Plain,
                    blink_verdict::config::ArtStyle::Ink,
                    blink_verdict::config::ArtStyle::Cyberpunk,
                    blink_verdict::config::ArtStyle::Graffiti,
                ][(index / 2) % 4];
                let text = if index.is_multiple_of(2) { "YES" } else { "NO" };
                let renderer = Renderer::new(app.overlay, &app.config, text, bounds).unwrap();
                for effect in [
                    blink_verdict::config::Animation::None,
                    blink_verdict::config::Animation::Fade,
                    blink_verdict::config::Animation::Flash,
                    blink_verdict::config::Animation::Pop,
                    blink_verdict::config::Animation::Slide,
                ] {
                    for elapsed in [0.0, 0.1, 1.0] {
                        renderer
                            .paint(
                                &app.config,
                                text,
                                false,
                                blink_verdict::animation::sample(effect, elapsed, Some(0.2)),
                            )
                            .unwrap();
                    }
                }
            };
            for index in 0..16 {
                run(index);
            }
            let process = GetCurrentProcess();
            let baseline_gdi = GetGuiResources(process, GR_GDIOBJECTS);
            let baseline_user = GetGuiResources(process, GR_USEROBJECTS);
            assert!(
                baseline_gdi > 0 && baseline_user > 0,
                "GUI counters unavailable"
            );
            let started = Instant::now();
            for index in 0..128 {
                run(index);
            }
            let final_gdi = GetGuiResources(process, GR_GDIOBJECTS);
            let final_user = GetGuiResources(process, GR_USEROBJECTS);
            println!(
                "release surfaces: 128 cycles, 1920 frames, {:.2}s; GDI {} -> {}, USER {} -> {}",
                started.elapsed().as_secs_f32(),
                baseline_gdi,
                final_gdi,
                baseline_user,
                final_user
            );
            assert!(
                final_gdi <= baseline_gdi + 4,
                "GDI handles grew over repeated render/art cycles"
            );
            assert!(
                final_user <= baseline_user + 2,
                "USER handles grew over repeated render/art cycles"
            );
        }
    }
}
