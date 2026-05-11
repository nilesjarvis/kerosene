use crate::app_state::TradingTerminal;
use crate::chart_state::ChartId;
use crate::message::Message;
use arboard::{Clipboard, ImageData};
use chrono::{DateTime, Local};
use iced::advanced::widget::{Id, Operation, operation::Outcome};
use iced::widget::image::Handle as ImageHandle;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{button, column, container, image as image_widget, row, svg, text, tooltip};
use iced::{Color, ContentFit, Element, Fill, Length, Rectangle, Size, Task, Theme, window};
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ColorType, ImageBuffer, ImageEncoder, Rgba};
use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;

const CAMERA_ICON_SVG: &[u8] = br#"
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none"
     stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <path d="M14.5 4l1.6 2H20a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h3.9l1.6-2h5z"/>
  <circle cx="12" cy="13" r="4"/>
</svg>
"#;

const CHART_SCREENSHOT_MIN_EXPORT_WIDTH: u32 = 1280;
const CHART_SCREENSHOT_MIN_EXPORT_HEIGHT: u32 = 720;
const CHART_SCREENSHOT_MAX_EXPORT_EDGE: u32 = 8192;
const CHART_SCREENSHOT_MAX_EXPORT_PIXELS: u64 = 12_582_912;

// ---------------------------------------------------------------------------
// Screenshot State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct ChartScreenshotState {
    pub(crate) symbol: String,
    pub(crate) timeframe: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
    pub(crate) png: Vec<u8>,
    pub(crate) preview_handle: ImageHandle,
    pub(crate) captured_at: DateTime<Local>,
    pub(crate) default_filename: String,
}

impl fmt::Debug for ChartScreenshotState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChartScreenshotState")
            .field("symbol", &self.symbol)
            .field("timeframe", &self.timeframe)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("rgba_len", &self.rgba.len())
            .field("png_len", &self.png.len())
            .field("captured_at", &self.captured_at)
            .field("default_filename", &self.default_filename)
            .finish()
    }
}

#[derive(Debug)]
struct FindWidgetBounds {
    target: Id,
    bounds: Option<Rectangle>,
}

impl FindWidgetBounds {
    fn new(target: Id) -> Self {
        Self {
            target,
            bounds: None,
        }
    }
}

impl Operation<Option<Rectangle>> for FindWidgetBounds {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Rectangle>>)) {
        if self.bounds.is_none() {
            operate(self);
        }
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&self.target) {
            self.bounds = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<Option<Rectangle>> {
        Outcome::Some(self.bounds)
    }
}

// ---------------------------------------------------------------------------
// Update
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn update_chart_screenshot(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenChartScreenshot(chart_id) => {
                if self
                    .charts
                    .get(&chart_id)
                    .is_none_or(|instance| instance.chart.candles.is_empty())
                {
                    self.push_toast(
                        "Chart screenshot unavailable: no visible candles".to_string(),
                        true,
                    );
                    return Task::none();
                }

                let target = Self::chart_screenshot_canvas_id(chart_id);
                return iced::advanced::widget::operate(FindWidgetBounds::new(target))
                    .map(move |bounds| Message::ChartScreenshotBoundsResolved(chart_id, bounds));
            }
            Message::ChartScreenshotBoundsResolved(chart_id, Some(bounds)) => {
                let Some(main_window_id) = self.main_window_id else {
                    self.push_toast(
                        "Chart screenshot unavailable: main window not ready".to_string(),
                        true,
                    );
                    return Task::none();
                };

                let Some(instance) = self.charts.get(&chart_id) else {
                    self.push_toast(
                        "Chart screenshot unavailable: chart not found".to_string(),
                        true,
                    );
                    return Task::none();
                };

                let symbol = instance.symbol_display.clone();
                let timeframe = instance.interval.label().to_string();
                let label_style = chart_screenshot_label_style(&self.theme());

                return window::screenshot(main_window_id).map(move |screenshot| {
                    Message::ChartScreenshotCaptured(
                        chart_id,
                        build_chart_screenshot(
                            symbol.clone(),
                            timeframe.clone(),
                            label_style,
                            bounds,
                            screenshot,
                        ),
                    )
                });
            }
            Message::ChartScreenshotBoundsResolved(_, None) => {
                self.push_toast(
                    "Chart screenshot unavailable: chart area was not visible".to_string(),
                    true,
                );
            }
            Message::ChartScreenshotCaptured(_chart_id, result) => match result {
                Ok(state) => {
                    self.chart_screenshot = Some(state);
                    if let Some(id) = self.chart_screenshot_window_id {
                        return window::gain_focus(id);
                    }

                    let settings = window::Settings {
                        size: Size::new(720.0, 560.0),
                        ..window::Settings::default()
                    };
                    let (id, task) = window::open(settings);
                    self.chart_screenshot_window_id = Some(id);
                    return task.map(Message::WindowOpened);
                }
                Err(err) => {
                    self.push_toast(format!("Chart screenshot failed: {err}"), true);
                }
            },
            Message::CopyChartScreenshot => {
                let Some(state) = self.chart_screenshot.clone() else {
                    self.push_toast("No chart screenshot to copy".to_string(), true);
                    return Task::none();
                };

                return Task::perform(
                    async move { copy_chart_screenshot_to_clipboard(state).map_err(|e| e.to_string()) },
                    Message::ChartScreenshotCopied,
                );
            }
            Message::ChartScreenshotCopied(result) => match result {
                Ok(()) => self.push_toast("Chart image copied to clipboard".to_string(), false),
                Err(err) => self.push_toast(format!("Chart image copy failed: {err}"), true),
            },
            Message::SaveChartScreenshot => {
                let Some(state) = self.chart_screenshot.clone() else {
                    self.push_toast("No chart screenshot to save".to_string(), true);
                    return Task::none();
                };

                return Task::perform(
                    save_chart_screenshot_png(state),
                    Message::ChartScreenshotSaved,
                );
            }
            Message::ChartScreenshotSaved(result) => match result {
                Ok(Some(path)) => {
                    self.push_toast(format!("Chart image saved to {}", path.display()), false)
                }
                Ok(None) => {}
                Err(err) => self.push_toast(format!("Chart image save failed: {err}"), true),
            },
            Message::CloseChartScreenshotWindow => {
                if let Some(id) = self.chart_screenshot_window_id {
                    return window::close(id);
                }
            }
            _ => {}
        }

        Task::none()
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_chart_screenshot_window(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(state) = &self.chart_screenshot else {
            let content = column![
                text("No chart screenshot available")
                    .size(14)
                    .color(theme.palette().text),
                chart_screenshot_button("Close", Message::CloseChartScreenshotWindow),
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center);

            return container(content)
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(16)
                .into();
        };

        let preview = image_widget(state.preview_handle.clone())
            .content_fit(ContentFit::Contain)
            .width(Fill)
            .height(Fill);

        let metadata = text(format!(
            "{} {}  {}x{}  {}",
            state.symbol,
            state.timeframe,
            state.width,
            state.height,
            state.captured_at.format("%Y-%m-%d %H:%M:%S")
        ))
        .size(11)
        .font(iced::Font::MONOSPACE)
        .color(theme.extended_palette().background.weak.text);

        let actions = row![
            chart_screenshot_button("Copy Image", Message::CopyChartScreenshot),
            chart_screenshot_button("Save PNG", Message::SaveChartScreenshot),
            chart_screenshot_button("Close", Message::CloseChartScreenshotWindow),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let content = column![
            row![
                text("Chart Screenshot")
                    .size(16)
                    .color(theme.palette().text),
                iced::widget::Space::new().width(Fill),
                metadata,
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
            container(preview)
                .width(Fill)
                .height(Length::Fill)
                .style(|theme: &Theme| {
                    let ext = theme.extended_palette();
                    container::Style {
                        background: Some(ext.background.base.color.into()),
                        border: iced::Border {
                            color: ext.background.strong.color,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                }),
            actions,
        ]
        .spacing(12);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(14)
            .into()
    }

    pub(crate) fn view_chart_screenshot_button(
        &self,
        chart_id: ChartId,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let icon = svg(SvgHandle::from_memory(CAMERA_ICON_SVG))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(|theme: &Theme, _status| svg::Style {
                color: Some(theme.palette().text),
            });

        tooltip(
            button(icon)
                .on_press(Message::OpenChartScreenshot(chart_id))
                .padding([3, 6])
                .style(|theme: &Theme, status| {
                    let bg = match status {
                        button::Status::Hovered => theme.extended_palette().background.strong.color,
                        _ => iced::Color::TRANSPARENT,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: theme.palette().text,
                        border: iced::Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
            text("Capture chart")
                .size(10)
                .font(iced::Font::MONOSPACE)
                .color(theme.palette().text),
            tooltip::Position::Top,
        )
        .into()
    }

    pub(crate) fn chart_screenshot_canvas_id(chart_id: ChartId) -> Id {
        Id::from(format!("chart_screenshot_canvas_{chart_id}"))
    }
}

fn chart_screenshot_button(label: &'static str, msg: Message) -> Element<'static, Message> {
    button(text(label).size(12).center())
        .on_press(msg)
        .padding([6, 12])
        .style(|theme: &Theme, status| {
            let ext = theme.extended_palette();
            let bg = match status {
                button::Status::Hovered => ext.background.strong.color,
                _ => ext.background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

// ---------------------------------------------------------------------------
// Image Processing
// ---------------------------------------------------------------------------

fn build_chart_screenshot(
    symbol: String,
    timeframe: String,
    label_style: ChartScreenshotLabelStyle,
    logical_bounds: Rectangle,
    screenshot: window::Screenshot,
) -> Result<ChartScreenshotState, String> {
    let region = physical_crop_region(logical_bounds, screenshot.size, screenshot.scale_factor)?;
    let cropped = screenshot
        .crop(region)
        .map_err(|err| format!("crop failed: {err}"))?;
    let (width, height, mut rgba) = upscale_chart_screenshot(
        cropped.size.width,
        cropped.size.height,
        cropped.rgba.as_ref().to_vec(),
    )?;
    draw_ticker_label(&mut rgba, width, height, &symbol, &timeframe, label_style);
    let png = encode_png_rgba(width, height, &rgba)?;
    let preview_handle = ImageHandle::from_rgba(width, height, rgba.clone());
    let captured_at = Local::now();
    let default_filename = chart_screenshot_filename(&symbol, &timeframe, captured_at);

    Ok(ChartScreenshotState {
        symbol,
        timeframe,
        width,
        height,
        rgba,
        png,
        preview_handle,
        captured_at,
        default_filename,
    })
}

fn upscale_chart_screenshot(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> Result<(u32, u32, Vec<u8>), String> {
    let expected_len = width as usize * height as usize * 4;
    if rgba.len() != expected_len {
        return Err("captured image buffer had an unexpected size".to_string());
    }

    let Some((target_width, target_height)) = chart_screenshot_export_dimensions(width, height)
    else {
        return Ok((width, height, rgba));
    };

    let source = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_vec(width, height, rgba)
        .ok_or_else(|| "captured image buffer had an unexpected size".to_string())?;
    let resized =
        image::imageops::resize(&source, target_width, target_height, FilterType::Lanczos3);

    Ok((target_width, target_height, resized.into_raw()))
}

fn chart_screenshot_export_dimensions(width: u32, height: u32) -> Option<(u32, u32)> {
    if width == 0 || height == 0 {
        return None;
    }

    let width_scale = CHART_SCREENSHOT_MIN_EXPORT_WIDTH as f64 / width as f64;
    let height_scale = CHART_SCREENSHOT_MIN_EXPORT_HEIGHT as f64 / height as f64;
    let requested_scale = width_scale.max(height_scale).max(1.0);
    if requested_scale <= 1.0 {
        return None;
    }

    let edge_scale = CHART_SCREENSHOT_MAX_EXPORT_EDGE as f64 / width.max(height) as f64;
    let pixel_scale =
        (CHART_SCREENSHOT_MAX_EXPORT_PIXELS as f64 / (width as f64 * height as f64)).sqrt();
    let max_scale = edge_scale.max(1.0).min(pixel_scale.max(1.0));
    let scale = requested_scale.min(max_scale);
    if scale <= 1.0 {
        return None;
    }

    let target_width = ((width as f64 * scale).round() as u32).max(width);
    let target_height = ((height as f64 * scale).round() as u32).max(height);
    if target_width == width && target_height == height {
        None
    } else {
        Some((target_width, target_height))
    }
}

fn physical_crop_region(
    logical_bounds: Rectangle,
    screenshot_size: Size<u32>,
    scale_factor: f32,
) -> Result<Rectangle<u32>, String> {
    if !logical_bounds.width.is_finite()
        || !logical_bounds.height.is_finite()
        || !logical_bounds.x.is_finite()
        || !logical_bounds.y.is_finite()
        || !scale_factor.is_finite()
        || scale_factor <= 0.0
    {
        return Err("invalid chart bounds".to_string());
    }

    let left = (logical_bounds.x * scale_factor).floor().max(0.0);
    let top = (logical_bounds.y * scale_factor).floor().max(0.0);
    let right = ((logical_bounds.x + logical_bounds.width) * scale_factor)
        .ceil()
        .min(screenshot_size.width as f32);
    let bottom = ((logical_bounds.y + logical_bounds.height) * scale_factor)
        .ceil()
        .min(screenshot_size.height as f32);

    if right <= left || bottom <= top {
        return Err("chart area was outside the screenshot".to_string());
    }

    Ok(Rectangle {
        x: left as u32,
        y: top as u32,
        width: (right - left) as u32,
        height: (bottom - top) as u32,
    })
}

fn encode_png_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let expected_len = width as usize * height as usize * 4;
    if rgba.len() != expected_len {
        return Err("captured image buffer had an unexpected size".to_string());
    }

    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(rgba, width, height, ColorType::Rgba8.into())
        .map_err(|err| err.to_string())?;
    Ok(png)
}

#[derive(Debug, Clone, Copy)]
struct ChartScreenshotLabelStyle {
    background: [u8; 4],
    border: [u8; 4],
    accent: [u8; 4],
    text: [u8; 4],
}

fn chart_screenshot_label_style(theme: &Theme) -> ChartScreenshotLabelStyle {
    let palette = theme.palette();
    let extended = theme.extended_palette();

    ChartScreenshotLabelStyle {
        background: color_to_rgba(extended.background.weak.color, 230),
        border: color_to_rgba(extended.background.strong.color, 145),
        accent: color_to_rgba(palette.primary, 210),
        text: color_to_rgba(palette.text, 248),
    }
}

fn color_to_rgba(color: Color, alpha: u8) -> [u8; 4] {
    [
        color_to_u8(color.r),
        color_to_u8(color.g),
        color_to_u8(color.b),
        alpha,
    ]
}

fn color_to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn draw_ticker_label(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    symbol: &str,
    timeframe: &str,
    style: ChartScreenshotLabelStyle,
) {
    if width < 72 || height < 28 || rgba.len() != width as usize * height as usize * 4 {
        return;
    }

    let scale = if width < 360 || height < 220 { 1 } else { 2 };
    let padding_x = 6 * scale;
    let padding_y = 5 * scale;
    let x = 8 * scale;
    let y = 8 * scale;
    let available_width = width.saturating_sub(x + padding_x * 2 + 4);
    let text = ticker_label_text(symbol, timeframe, available_width, scale);
    if text.is_empty() {
        return;
    }

    let text_w = bitmap_text_width(&text, scale);
    let text_h = BITMAP_GLYPH_HEIGHT * scale;
    let accent_w = 2 * scale;
    let accent_gap = 3 * scale;
    let label_w = text_w + padding_x * 2 + accent_w + accent_gap;
    let label_h = text_h + padding_y * 2;
    if x + label_w >= width || y + label_h >= height {
        return;
    }

    fill_rect(
        rgba,
        width,
        height,
        Rect {
            x,
            y,
            width: label_w,
            height: label_h,
        },
        style.background,
    );
    stroke_rect(
        rgba,
        width,
        height,
        Rect {
            x,
            y,
            width: label_w,
            height: label_h,
        },
        style.border,
    );
    fill_rect(
        rgba,
        width,
        height,
        Rect {
            x: x + 1,
            y: y + 1,
            width: accent_w,
            height: label_h.saturating_sub(2),
        },
        style.accent,
    );
    draw_bitmap_text(
        rgba,
        width,
        height,
        PixelPoint {
            x: x + padding_x + accent_w + accent_gap,
            y: y + padding_y,
        },
        scale,
        &text,
        style.text,
    );
}

#[derive(Debug, Clone, Copy)]
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Copy)]
struct PixelPoint {
    x: u32,
    y: u32,
}

fn fill_rect(rgba: &mut [u8], width: u32, height: u32, rect: Rect, color: [u8; 4]) {
    let max_x = rect.x.saturating_add(rect.width).min(width);
    let max_y = rect.y.saturating_add(rect.height).min(height);
    for y in rect.y..max_y {
        for x in rect.x..max_x {
            blend_pixel(rgba, width, x, y, color);
        }
    }
}

fn stroke_rect(rgba: &mut [u8], width: u32, height: u32, rect: Rect, color: [u8; 4]) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }

    let right = rect.x.saturating_add(rect.width).saturating_sub(1);
    let bottom = rect.y.saturating_add(rect.height).saturating_sub(1);
    for x in rect.x..=right.min(width.saturating_sub(1)) {
        if rect.y < height {
            blend_pixel(rgba, width, x, rect.y, color);
        }
        if bottom < height {
            blend_pixel(rgba, width, x, bottom, color);
        }
    }
    for y in rect.y..=bottom.min(height.saturating_sub(1)) {
        if rect.x < width {
            blend_pixel(rgba, width, rect.x, y, color);
        }
        if right < width {
            blend_pixel(rgba, width, right, y, color);
        }
    }
}

fn draw_bitmap_text(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    origin: PixelPoint,
    scale: u32,
    text: &str,
    color: [u8; 4],
) {
    let mut cursor_x = origin.x;
    for ch in text.chars() {
        let glyph = bitmap_glyph(ch);
        for (row_idx, row) in glyph.iter().enumerate() {
            for col in 0..BITMAP_GLYPH_WIDTH {
                if *row & (1 << (BITMAP_GLYPH_WIDTH - 1 - col)) == 0 {
                    continue;
                }
                let px = cursor_x + col * scale;
                let py = origin.y + row_idx as u32 * scale;
                fill_rect(
                    rgba,
                    width,
                    height,
                    Rect {
                        x: px,
                        y: py,
                        width: scale,
                        height: scale,
                    },
                    color,
                );
            }
        }
        cursor_x += (BITMAP_GLYPH_WIDTH + 1) * scale;
    }
}

const BITMAP_GLYPH_WIDTH: u32 = 5;
const BITMAP_GLYPH_HEIGHT: u32 = 7;

fn bitmap_text_width(text: &str, scale: u32) -> u32 {
    let count = text.chars().count() as u32;
    if count == 0 {
        0
    } else {
        ((BITMAP_GLYPH_WIDTH + 1) * count - 1) * scale
    }
}

fn ticker_label_text(symbol: &str, timeframe: &str, available_width: u32, scale: u32) -> String {
    let max_chars = ((available_width / ((BITMAP_GLYPH_WIDTH + 1) * scale)).max(1)) as usize;
    let symbol = sanitize_label_component(symbol);
    let timeframe = sanitize_label_component(timeframe);

    if symbol.is_empty() {
        return truncate_chars(&timeframe, max_chars);
    }
    if timeframe.is_empty() {
        return truncate_chars(&symbol, max_chars);
    }

    let full = format!("{symbol} {timeframe}");
    if full.chars().count() <= max_chars {
        return full;
    }

    let timeframe_len = timeframe.chars().count();
    if max_chars <= timeframe_len {
        return truncate_chars(&timeframe, max_chars);
    }

    let symbol_max = max_chars.saturating_sub(timeframe_len + 1);
    format!("{} {}", truncate_chars(&symbol, symbol_max), timeframe)
        .trim()
        .to_string()
}

fn sanitize_label_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            let upper = ch.to_ascii_uppercase();
            if is_bitmap_glyph_supported(upper) {
                upper
            } else if ch.is_whitespace() {
                ' '
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect::<String>()
}

fn is_bitmap_glyph_supported(ch: char) -> bool {
    matches!(
        ch,
        'A'..='Z' | '0'..='9' | '/' | ':' | '-' | '_' | '.' | ' '
    )
}

fn blend_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, color: [u8; 4]) {
    let idx = (y as usize * width as usize + x as usize) * 4;
    if idx + 3 >= rgba.len() {
        return;
    }

    let alpha = color[3] as u16;
    let inv_alpha = 255 - alpha;
    for channel in 0..3 {
        rgba[idx + channel] =
            ((color[channel] as u16 * alpha + rgba[idx + channel] as u16 * inv_alpha) / 255) as u8;
    }
    rgba[idx + 3] = 255;
}

fn bitmap_glyph(ch: char) -> [u32; 7] {
    match ch {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10011, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        ':' => [
            0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '_' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111,
        ],
        '.' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b01100, 0b01100,
        ],
        ' ' => [0; 7],
        _ => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b00100, 0b00000, 0b00100,
        ],
    }
}

fn copy_chart_screenshot_to_clipboard(state: ChartScreenshotState) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;
    clipboard
        .set_image(ImageData {
            width: state.width as usize,
            height: state.height as usize,
            bytes: Cow::Owned(state.rgba),
        })
        .map_err(|err| err.to_string())
}

async fn save_chart_screenshot_png(state: ChartScreenshotState) -> Result<Option<PathBuf>, String> {
    let path = rfd::AsyncFileDialog::new()
        .add_filter("PNG image", &["png"])
        .set_file_name(state.default_filename)
        .save_file()
        .await;

    let Some(path) = path else {
        return Ok(None);
    };

    std::fs::write(path.path(), state.png).map_err(|err| err.to_string())?;
    Ok(Some(path.path().to_path_buf()))
}

fn chart_screenshot_filename(
    symbol: &str,
    timeframe: &str,
    captured_at: DateTime<Local>,
) -> String {
    format!(
        "kerosene-{}-{}-{}.png",
        sanitize_filename_part(symbol),
        sanitize_filename_part(timeframe),
        captured_at.format("%Y%m%d-%H%M%S")
    )
}

fn sanitize_filename_part(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "chart".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn physical_crop_region_scales_and_clamps_logical_bounds() {
        let region = physical_crop_region(
            Rectangle {
                x: 10.25,
                y: 5.5,
                width: 100.25,
                height: 50.25,
            },
            Size::new(220, 120),
            2.0,
        )
        .expect("crop region");

        assert_eq!(region.x, 20);
        assert_eq!(region.y, 11);
        assert_eq!(region.width, 200);
        assert_eq!(region.height, 101);
    }

    #[test]
    fn physical_crop_region_rejects_invalid_values() {
        let err = physical_crop_region(
            Rectangle {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 20.0,
            },
            Size::new(100, 100),
            1.0,
        )
        .expect_err("zero-width crop should fail");

        assert!(err.contains("outside") || err.contains("invalid"));
    }

    #[test]
    fn encode_png_rgba_rejects_wrong_buffer_size() {
        let err = encode_png_rgba(2, 2, &[0; 4]).expect_err("wrong size");
        assert!(err.contains("unexpected size"));
    }

    #[test]
    fn encode_png_rgba_produces_png_header() {
        let rgba = vec![255; 2 * 2 * 4];
        let png = encode_png_rgba(2, 2, &rgba).expect("png");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn chart_screenshot_export_dimensions_upscale_small_charts() {
        assert_eq!(
            chart_screenshot_export_dimensions(320, 180),
            Some((1280, 720))
        );
        assert_eq!(
            chart_screenshot_export_dimensions(500, 300),
            Some((1280, 768))
        );
    }

    #[test]
    fn chart_screenshot_export_dimensions_preserve_large_charts() {
        assert_eq!(chart_screenshot_export_dimensions(1600, 900), None);
        assert_eq!(chart_screenshot_export_dimensions(1280, 720), None);
    }

    #[test]
    fn chart_screenshot_export_dimensions_cap_extreme_shapes() {
        let (width, height) =
            chart_screenshot_export_dimensions(4000, 100).expect("extreme chart still upscales");

        assert_eq!(width, CHART_SCREENSHOT_MAX_EXPORT_EDGE);
        assert_eq!(height, 205);
    }

    #[test]
    fn upscale_chart_screenshot_resizes_rgba_buffer() {
        let rgba = vec![255; 320 * 180 * 4];
        let (width, height, upscaled) =
            upscale_chart_screenshot(320, 180, rgba).expect("upscaled screenshot");

        assert_eq!((width, height), (1280, 720));
        assert_eq!(upscaled.len(), 1280 * 720 * 4);
    }

    #[test]
    fn upscale_chart_screenshot_rejects_wrong_buffer_size() {
        let err = upscale_chart_screenshot(10, 10, vec![0; 4]).expect_err("wrong size");
        assert!(err.contains("unexpected size"));
    }

    #[test]
    fn chart_screenshot_filename_sanitizes_symbol_and_timeframe() {
        let at = Local.with_ymd_and_hms(2026, 5, 11, 15, 30, 0).unwrap();
        assert_eq!(
            chart_screenshot_filename("UBTC/USDC:PERP", "1H", at),
            "kerosene-UBTC-USDC-PERP-1H-20260511-153000.png"
        );
    }

    #[test]
    fn ticker_label_text_sanitizes_and_truncates_to_available_width() {
        assert_eq!(
            ticker_label_text("ubtc/usdc:perp", "1H", 132, 1),
            "UBTC/USDC:PERP 1H"
        );
        assert_eq!(ticker_label_text("kPEPE@dex", "15m", 54, 1), "KPEPE 15M");
        assert_eq!(ticker_label_text("verylongticker", "1D", 48, 1), "VERYL 1D");
    }

    #[test]
    fn draw_ticker_label_mutates_top_left_pixels() {
        let width = 160;
        let height = 80;
        let mut rgba = vec![0; width as usize * height as usize * 4];

        draw_ticker_label(&mut rgba, width, height, "BTC", "1H", test_label_style());

        assert!(rgba.iter().any(|value| *value != 0));
        let untouched_bottom_right =
            ((height as usize - 1) * width as usize + width as usize - 1) * 4;
        assert_eq!(
            &rgba[untouched_bottom_right..untouched_bottom_right + 4],
            &[0, 0, 0, 0]
        );
    }

    #[test]
    fn draw_ticker_label_ignores_tiny_or_malformed_images() {
        let mut tiny = vec![0; 12];
        draw_ticker_label(&mut tiny, 2, 2, "BTC", "1H", test_label_style());
        assert_eq!(tiny, vec![0; 12]);

        let mut wrong_len = vec![0; 10];
        draw_ticker_label(&mut wrong_len, 160, 80, "BTC", "1H", test_label_style());
        assert_eq!(wrong_len, vec![0; 10]);
    }

    fn test_label_style() -> ChartScreenshotLabelStyle {
        ChartScreenshotLabelStyle {
            background: [8, 12, 18, 218],
            border: [255, 255, 255, 62],
            accent: [80, 250, 123, 210],
            text: [245, 248, 250, 245],
        }
    }
}
