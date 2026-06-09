use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::session_data_state::{
    SessionDataId, SessionDataInstance, SessionDataLookback, SessionReturnBar,
    SessionWeekdaySummary,
};

use chrono::{DateTime, Utc};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::widget::{
    Space, button, canvas as canvas_widget, column, container, responsive, row, scrollable, text,
    text_input,
};
use iced::{Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme};

const CHART_HEIGHT: f32 = 220.0;
const CHART_LEFT_PAD: f32 = 42.0;
const CHART_RIGHT_PAD: f32 = 10.0;
const CHART_TOP_PAD: f32 = 12.0;
const CHART_BOTTOM_PAD: f32 = 28.0;
const BAR_GAP: f32 = 2.0;
const TOOLTIP_WIDTH: f32 = 176.0;
const TOOLTIP_HEIGHT: f32 = 76.0;
const STATUS_ERROR_CHARS: usize = 72;
const BODY_ERROR_CHARS: usize = 140;

// ---------------------------------------------------------------------------
// Session Data View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_session_data(&self, id: SessionDataId) -> Element<'_, Message> {
        responsive(move |size| self.view_session_data_sized(id, size.width)).into()
    }

    fn view_session_data_sized(
        &self,
        id: SessionDataId,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(instance) = self.session_data.get(&id) else {
            return container(
                text("Session Data instance missing")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        };

        let header = self.view_session_data_header(instance, &theme, available_width);
        let mut content = column![header].spacing(8).padding(8);
        if instance.symbol_picker_open {
            content = content.push(self.view_session_data_symbol_dropdown(instance, &theme));
        }
        content = content.push(self.view_session_data_body(instance, &theme));

        container(scrollable(content).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_session_data_header<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        let display = self.display_name_for_symbol(&instance.symbol);
        let mut symbol_content = row![].spacing(5).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(&instance.symbol, 15, theme.palette().text) {
            symbol_content = symbol_content.push(icon);
        }
        symbol_content = symbol_content
            .push(text(display).size(12).color(theme.palette().text))
            .push(
                text(if instance.symbol_picker_open {
                    "\u{25b2}"
                } else {
                    "\u{25be}"
                })
                .size(9)
                .color(theme.extended_palette().background.weak.text),
            );

        let symbol_button = button(symbol_content)
            .on_press(Message::ToggleSessionDataSymbolPicker(instance.id))
            .padding([3, 8])
            .style(move |theme: &Theme, status| compact_button_style(theme, status, true));

        let mut lookbacks = row![].spacing(3).align_y(Alignment::Center);
        for lookback in SessionDataLookback::ALL {
            lookbacks = lookbacks.push(lookback_button(
                lookback,
                instance.lookback == lookback,
                instance.id,
            ));
        }

        let refresh_label: Element<'_, Message> = if instance.loading {
            self.view_spinner(14)
        } else {
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text)
                .into()
        };
        let refresh = button(refresh_label)
            .on_press_maybe((!instance.loading).then_some(Message::RefreshSessionData(instance.id)))
            .padding([3, 8])
            .style(move |theme: &Theme, status| compact_button_style(theme, status, false));

        let status = session_data_status(instance);
        let status_text = text(status)
            .size(10)
            .color(theme.extended_palette().background.weak.text);

        if available_width < 520.0 {
            column![
                row![symbol_button, refresh]
                    .spacing(6)
                    .align_y(Alignment::Center),
                lookbacks,
                status_text
            ]
            .spacing(6)
            .into()
        } else {
            row![
                symbol_button,
                lookbacks,
                Space::new().width(Fill),
                status_text,
                refresh
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        }
    }

    fn view_session_data_symbol_dropdown<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let search = text_input("Search perp or spot...", &instance.search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::SessionDataSearchChanged(instance.id, q))
            .size(12)
            .padding([5, 8]);

        let mut results = column![].spacing(2);
        let query = instance.search_query.trim().to_ascii_lowercase();
        for symbol in self
            .exchange_symbols
            .iter()
            .filter(|symbol| matches!(symbol.market_type, MarketType::Perp | MarketType::Spot))
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| {
                if query.is_empty() {
                    return true;
                }
                let display = Self::exchange_symbol_display_name(symbol).to_ascii_lowercase();
                symbol.key.to_ascii_lowercase().contains(&query)
                    || symbol.ticker.to_ascii_lowercase().contains(&query)
                    || display.contains(&query)
                    || symbol.category.to_ascii_lowercase().contains(&query)
            })
            .take(12)
        {
            let display = Self::exchange_symbol_display_name(symbol);
            let market = match symbol.market_type {
                MarketType::Perp => "perp",
                MarketType::Spot => "spot",
                MarketType::Outcome => "outcome",
            };
            let row = row![
                text(display).size(11).width(Fill),
                text(symbol.key.clone())
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text),
                text(market)
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            results = results.push(
                button(row)
                    .on_press(Message::SessionDataSymbolSelected(
                        instance.id,
                        symbol.key.clone(),
                    ))
                    .padding([5, 8])
                    .width(Fill)
                    .style(|theme: &Theme, status| compact_button_style(theme, status, false)),
            );
        }

        if self.exchange_symbols.is_empty() {
            results = results.push(
                text("Symbols loading")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(column![search, results].spacing(6).padding(6))
            .width(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                text_color: Some(theme.palette().text),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.weak.color,
                },
                ..Default::default()
            })
            .into()
    }

    fn view_session_data_body<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        if instance.loading && instance.bars.is_empty() {
            return container(self.view_spinner(20))
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .into();
        }

        if instance.bars.is_empty() {
            let message = instance
                .error
                .as_deref()
                .unwrap_or("No session history available");
            return container(
                text(message)
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        }

        let chart = canvas_widget(SessionDataChart {
            bars: instance.bars.clone(),
        })
        .width(Fill)
        .height(Length::Fixed(CHART_HEIGHT));

        let summary = weekday_summary_row(&instance.weekday_summaries, theme);
        let mut content = column![chart, summary].spacing(8);
        if let Some(error) = &instance.error {
            content = content.push(
                text(helpers::ellipsized_text(error, BODY_ERROR_CHARS))
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(content).width(Fill).height(Fill).into()
    }
}

fn lookback_button(
    lookback: SessionDataLookback,
    active: bool,
    id: SessionDataId,
) -> Element<'static, Message> {
    button(text(lookback.label()).size(10).center())
        .on_press(Message::SessionDataLookbackChanged(id, lookback))
        .padding([3, 7])
        .style(move |theme: &Theme, status| compact_button_style(theme, status, active))
        .into()
}

fn compact_button_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let bg = match (active, status) {
        (true, _) => theme.extended_palette().background.strong.color,
        (false, button::Status::Hovered) | (false, button::Status::Pressed) => {
            theme.extended_palette().background.weak.color
        }
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.palette().text
        },
        border: iced::Border {
            radius: 3.0.into(),
            width: if active { 1.0 } else { 0.0 },
            color: if active {
                Color {
                    a: 0.42,
                    ..theme.palette().primary
                }
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
}

fn session_data_status(instance: &SessionDataInstance) -> String {
    if instance.loading {
        return "Loading".to_string();
    }
    if let Some(error) = &instance.error
        && instance.bars.is_empty()
    {
        return helpers::ellipsized_text(error, STATUS_ERROR_CHARS);
    }
    format!("{} sessions", instance.bars.len())
}

fn weekday_summary_row<'a>(
    summaries: &'a [SessionWeekdaySummary],
    theme: &Theme,
) -> Element<'a, Message> {
    let mut cells = row![].spacing(5).width(Fill);
    for summary in summaries {
        let color = helpers::signed_number_color(summary.average_return_pct, theme);
        let body = column![
            text(summary.weekday.label())
                .size(10)
                .color(theme.extended_palette().background.weak.text),
            text(helpers::format_signed_percent_value(
                summary.average_return_pct
            ))
            .size(12)
            .color(color),
            text(format!(
                "{}x  {:.0}%",
                summary.sample_count, summary.win_rate_pct
            ))
            .size(9)
            .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(2)
        .align_x(Alignment::Center);

        cells = cells.push(
            container(body)
                .width(Fill)
                .padding([6, 4])
                .style(|theme: &Theme| container::Style {
                    background: Some(theme.extended_palette().background.weak.color.into()),
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }
    cells.into()
}

#[derive(Debug, Clone)]
struct SessionDataChart {
    bars: Vec<SessionReturnBar>,
}

#[derive(Default)]
struct SessionDataChartState {
    hovered: Option<usize>,
}

impl canvas::Program<Message> for SessionDataChart {
    type State = SessionDataChartState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let next = match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                self.bar_index_at(bounds, cursor)
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => None,
            _ => return None,
        };
        if state.hovered != next {
            state.hovered = next;
            return Some(canvas::Action::request_redraw());
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        if self.bars.is_empty() || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let layout = ChartLayout::new(&self.bars, bounds.size());
        draw_axes(&mut frame, theme, &layout);
        draw_bars(&mut frame, theme, &layout, &self.bars, state.hovered);
        draw_time_labels(&mut frame, theme, &layout, &self.bars);
        if let Some(index) = state
            .hovered
            .and_then(|idx| self.bars.get(idx).map(|_| idx))
        {
            draw_tooltip(&mut frame, theme, &layout, index, &self.bars[index]);
        }

        vec![frame.into_geometry()]
    }
}

impl SessionDataChart {
    fn bar_index_at(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let pos = cursor.position_in(bounds)?;
        let layout = ChartLayout::new(&self.bars, bounds.size());
        if pos.x < layout.plot_x
            || pos.x > layout.plot_x + layout.plot_w
            || pos.y < layout.plot_y
            || pos.y > layout.plot_y + layout.plot_h
        {
            return None;
        }
        let idx = ((pos.x - layout.plot_x) / layout.step_w).floor() as usize;
        if idx >= self.bars.len() {
            return None;
        }
        let x = layout.bar_x(idx);
        (pos.x >= x && pos.x <= x + layout.bar_w).then_some(idx)
    }
}

struct ChartLayout {
    size: Size,
    plot_x: f32,
    plot_y: f32,
    plot_w: f32,
    plot_h: f32,
    zero_y: f32,
    min_return: f64,
    max_return: f64,
    bar_w: f32,
    step_w: f32,
}

impl ChartLayout {
    fn new(bars: &[SessionReturnBar], size: Size) -> Self {
        let plot_x = CHART_LEFT_PAD.min(size.width * 0.3);
        let plot_y = CHART_TOP_PAD;
        let plot_w = (size.width - plot_x - CHART_RIGHT_PAD).max(1.0);
        let plot_h = (size.height - CHART_TOP_PAD - CHART_BOTTOM_PAD).max(1.0);
        let (min_return, max_return) = padded_return_range(bars);
        let zero_y = value_to_y(0.0, min_return, max_return, plot_y, plot_h);
        let count = bars.len().max(1) as f32;
        let step_w = (plot_w / count).max(1.0);
        let bar_w = (step_w - BAR_GAP).max(1.0);
        Self {
            size,
            plot_x,
            plot_y,
            plot_w,
            plot_h,
            zero_y,
            min_return,
            max_return,
            bar_w,
            step_w,
        }
    }

    fn bar_x(&self, idx: usize) -> f32 {
        self.plot_x + idx as f32 * self.step_w + (self.step_w - self.bar_w) * 0.5
    }

    fn value_to_y(&self, value: f64) -> f32 {
        value_to_y(
            value,
            self.min_return,
            self.max_return,
            self.plot_y,
            self.plot_h,
        )
    }
}

fn padded_return_range(bars: &[SessionReturnBar]) -> (f64, f64) {
    let (mut min_value, mut max_value) = (0.0_f64, 0.0_f64);
    for value in bars.iter().map(|bar| bar.return_pct) {
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }
    if (max_value - min_value).abs() < 1e-9 {
        min_value -= 1.0;
        max_value += 1.0;
    }
    let pad = (max_value - min_value) * 0.08;
    (min_value - pad, max_value + pad)
}

fn value_to_y(value: f64, min_value: f64, max_value: f64, plot_y: f32, plot_h: f32) -> f32 {
    let span = (max_value - min_value).max(1e-9);
    (plot_y + ((max_value - value) / span) as f32 * plot_h).clamp(plot_y, plot_y + plot_h)
}

fn draw_axes(frame: &mut Frame, theme: &Theme, layout: &ChartLayout) {
    let axis_color = Color {
        a: 0.24,
        ..theme.palette().text
    };
    let zero = Path::line(
        Point::new(layout.plot_x, layout.zero_y),
        Point::new(layout.plot_x + layout.plot_w, layout.zero_y),
    );
    frame.stroke(
        &zero,
        Stroke::default().with_color(axis_color).with_width(1.0),
    );

    for value in [layout.max_return, 0.0, layout.min_return] {
        let y = layout.value_to_y(value);
        frame.fill_text(Text {
            content: helpers::format_signed_percent_value(value),
            position: Point::new(3.0, y - 6.0),
            color: theme.extended_palette().background.weak.text,
            size: iced::Pixels(9.0),
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
}

fn draw_bars(
    frame: &mut Frame,
    theme: &Theme,
    layout: &ChartLayout,
    bars: &[SessionReturnBar],
    hovered: Option<usize>,
) {
    for (idx, bar) in bars.iter().enumerate() {
        let x = layout.bar_x(idx);
        let y = layout.value_to_y(bar.return_pct);
        let top = y.min(layout.zero_y);
        let height = (y - layout.zero_y).abs().max(1.0);
        let mut color = helpers::signed_number_color(bar.return_pct, theme);
        color.a = if hovered == Some(idx) { 0.95 } else { 0.72 };
        frame.fill_rectangle(Point::new(x, top), Size::new(layout.bar_w, height), color);
    }
}

fn draw_time_labels(
    frame: &mut Frame,
    theme: &Theme,
    layout: &ChartLayout,
    bars: &[SessionReturnBar],
) {
    let target_labels = (layout.plot_w / 72.0).floor().max(2.0) as usize;
    let stride = (bars.len() / target_labels).max(1);
    let y = layout.plot_y + layout.plot_h + 12.0;
    for (idx, bar) in bars.iter().enumerate() {
        if idx != 0 && idx + 1 != bars.len() && idx % stride != 0 {
            continue;
        }
        let x = layout.bar_x(idx) + layout.bar_w * 0.5 - 16.0;
        frame.fill_text(Text {
            content: compact_date(bar.open_time),
            position: Point::new(x.clamp(layout.plot_x, layout.size.width - 34.0), y),
            color: theme.extended_palette().background.weak.text,
            size: iced::Pixels(9.0),
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
}

fn draw_tooltip(
    frame: &mut Frame,
    theme: &Theme,
    layout: &ChartLayout,
    index: usize,
    bar: &SessionReturnBar,
) {
    let point = Point::new(
        layout.bar_x(index) + layout.bar_w * 0.5,
        layout.value_to_y(bar.return_pct),
    );
    let origin = tooltip_origin(point, layout.size);
    frame.fill_rectangle(
        origin,
        Size::new(TOOLTIP_WIDTH, TOOLTIP_HEIGHT),
        Color {
            a: 0.94,
            ..theme.extended_palette().background.strong.color
        },
    );
    let border = Path::rectangle(origin, Size::new(TOOLTIP_WIDTH, TOOLTIP_HEIGHT));
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(theme.extended_palette().background.weak.color)
            .with_width(1.0),
    );

    let label = format!(
        "{} {}\nO {}  C {}\nReturn {}\nVol {}",
        full_date(bar.open_time),
        bar.weekday.label(),
        helpers::format_price(bar.open),
        helpers::format_price(bar.close),
        helpers::format_signed_percent_value(bar.return_pct),
        helpers::format_decimal_with_commas(bar.volume, 0),
    );
    frame.fill_text(Text {
        content: label,
        position: Point::new(origin.x + 7.0, origin.y + 9.0),
        color: theme.palette().text,
        size: iced::Pixels(10.0),
        font: crate::app_fonts::monospace_font(),
        ..Default::default()
    });
}

fn tooltip_origin(point: Point, size: Size) -> Point {
    let x = if point.x + TOOLTIP_WIDTH + 10.0 > size.width {
        point.x - TOOLTIP_WIDTH - 8.0
    } else {
        point.x + 8.0
    }
    .clamp(0.0, (size.width - TOOLTIP_WIDTH).max(0.0));
    let y = if point.y + TOOLTIP_HEIGHT + 10.0 > size.height {
        point.y - TOOLTIP_HEIGHT - 8.0
    } else {
        point.y + 8.0
    }
    .clamp(0.0, (size.height - TOOLTIP_HEIGHT).max(0.0));
    Point::new(x, y)
}

fn compact_date(timestamp_ms: u64) -> String {
    i64::try_from(timestamp_ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|dt| dt.format("%m-%d").to_string())
        .unwrap_or_else(|| "--".to_string())
}

fn full_date(timestamp_ms: u64) -> String {
    i64::try_from(timestamp_ms)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
