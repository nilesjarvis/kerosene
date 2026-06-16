use crate::api::{ExchangeSymbol, MarketType};
use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::liquidations_distribution_state::{
    LIQUIDATION_DISTRIBUTION_ZOOM_STEP, LiquidationDistributionData, LiquidationDistributionPoint,
    LiquidationDistributionZoomAnchor,
};
use crate::message::Message;

use iced::widget::canvas::{self, Frame, Stroke};
use iced::widget::{button, column, container, responsive, row, text, text_input, tooltip};
use iced::{
    Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme, color, mouse,
};

// ---------------------------------------------------------------------------
// Liquidations Distribution View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_liquidations_distribution(&self) -> Element<'_, Message> {
        responsive(move |size| self.view_liquidations_distribution_sized(size.width)).into()
    }

    fn view_liquidations_distribution_sized(&self, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = self.status_bar_now_ms;
        let denomination = self.display_denomination_context();
        let state = &self.liquidation_distribution;
        let refresh_btn = self.view_liquidations_distribution_refresh_button(state.loading);
        let mut header_actions = row![].spacing(6).align_y(Alignment::Center);
        if state.data.is_some() {
            header_actions = header_actions.push(
                self.view_liquidations_distribution_zoom_controls(state.zoom_factor(), &theme),
            );
        }
        header_actions = header_actions.push(refresh_btn);

        let header = row![
            container(self.view_liquidations_distribution_symbol_button(&theme)).width(Fill),
            header_actions,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let mut content = column![header].spacing(8);
        if state.symbol_picker_open {
            content = content.push(self.view_liquidations_distribution_symbol_dropdown(&theme));
        }

        if state.loading && state.data.is_none() {
            content = content.push(
                row![
                    self.view_spinner(18),
                    text("Loading HyperDash liquidation levels")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        if let Some(error) = &state.error {
            content = content.push(text(error.clone()).size(11).color(color!(0xff5555)));
            if state.data.is_some() {
                content = content.push(
                    text("Showing last successful snapshot")
                        .size(11)
                        .color(theme.extended_palette().background.weak.text),
                );
            }
        }

        if let Some(data) = &state.data {
            content = content
                .push(self.view_liquidations_distribution_metrics(
                    data,
                    now_ms,
                    available_width,
                    &denomination,
                    &theme,
                ))
                .push(
                    iced::widget::canvas(LiquidationsDistributionChart {
                        data: data.clone(),
                        denomination,
                        zoom: state.zoom_factor(),
                        zoom_center_price: state.zoom_center_price,
                    })
                    .width(Fill)
                    .height(Fill),
                );
        } else if !state.loading {
            content = content.push(
                container(
                    text("No liquidation distribution loaded")
                        .size(12)
                        .color(theme.extended_palette().background.weak.text),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill),
            );
        }

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .into()
    }

    fn view_liquidations_distribution_refresh_button(
        &self,
        loading: bool,
    ) -> Element<'static, Message> {
        let button = button(
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font()),
        )
        .padding([2, 7])
        .style(move |theme: &Theme, status| {
            if loading {
                subtle_liquidations_distribution_header_button(theme, button::Status::Disabled)
            } else {
                subtle_liquidations_distribution_header_button(theme, status)
            }
        });

        let button = if loading {
            button
        } else {
            button.on_press(Message::RefreshLiquidationsDistribution)
        };

        tooltip(
            button,
            text(if loading { "Refreshing" } else { "Refresh" }).size(10),
            tooltip::Position::Top,
        )
        .into()
    }

    fn view_liquidations_distribution_symbol_button(&self, theme: &Theme) -> Element<'_, Message> {
        let state = &self.liquidation_distribution;
        let selected = state.symbol.trim();
        let label = if selected.is_empty() {
            "Select market".to_string()
        } else {
            format!(
                "{} / USD",
                self.liquidation_distribution_symbol_display(selected)
            )
        };

        let mut content = row![].spacing(6).align_y(Alignment::Center);
        if !selected.is_empty()
            && let Some(icon) = helpers::symbol_icon(selected, 14, theme.palette().text)
        {
            content = content.push(icon);
        }
        content = content.push(
            text(label)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
        );
        if let Some(dex) = helpers::hip3_dex(selected) {
            content = content.push(
                text(dex.to_string())
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }
        content = content.push(
            text(if state.symbol_picker_open {
                "\u{25b2}"
            } else {
                "\u{25be}"
            })
            .size(8)
            .color(theme.extended_palette().background.weak.text),
        );

        button(content)
            .on_press(Message::ToggleLiquidationsDistributionSymbolPicker)
            .padding([2, 7])
            .style(move |theme: &Theme, status| {
                let bg = match (state.symbol_picker_open, status) {
                    (_, button::Status::Hovered) => theme.extended_palette().background.weak.color,
                    (true, _) => theme.extended_palette().background.weak.color,
                    (false, _) => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 3.0.into(),
                        width: if state.symbol_picker_open { 1.0 } else { 0.0 },
                        color: Color {
                            a: 0.35,
                            ..theme.palette().primary
                        },
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn view_liquidations_distribution_symbol_dropdown(
        &self,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let state = &self.liquidation_distribution;
        let search = text_input("Search perp market...", &state.symbol_search_query)
            .style(helpers::text_input_style)
            .on_input(Message::LiquidationsDistributionSearchChanged)
            .size(12)
            .padding([5, 8]);

        let query = state.symbol_search_query.trim().to_lowercase();
        let mut matches: Vec<&ExchangeSymbol> = self
            .exchange_symbols
            .iter()
            .filter(|symbol| symbol.market_type == MarketType::Perp)
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| liquidation_distribution_symbol_matches(symbol, &query))
            .collect();
        matches.sort_by(|a, b| {
            a.ticker
                .cmp(&b.ticker)
                .then_with(|| helpers::compare_symbol_keys_for_same_ticker(&a.key, &b.key))
        });
        matches.truncate(6);

        let mut results = column![].spacing(3);
        for symbol in matches {
            results = results.push(self.view_liquidations_distribution_symbol_row(symbol, theme));
        }

        container(column![search, results].spacing(5).padding(6))
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

    fn view_liquidations_distribution_symbol_row<'a>(
        &'a self,
        symbol: &'a ExchangeSymbol,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let sym_key = symbol.key.clone();
        let display = Self::exchange_symbol_display_name(symbol);
        let mut content = row![].spacing(6).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(&sym_key, 14, theme.palette().text) {
            content = content.push(icon);
        }
        content = content.push(
            text(display)
                .size(12)
                .color(theme.palette().text)
                .width(Fill),
        );
        if let Some(dex) = helpers::hip3_dex(&sym_key) {
            content = content.push(
                text(dex.to_string())
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        button(content)
            .on_press(Message::LiquidationsDistributionSymbolSelected(sym_key))
            .padding([4, 8])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.weak.color,
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .width(Fill)
            .into()
    }

    fn view_liquidations_distribution_zoom_controls(
        &self,
        zoom: f64,
        theme: &Theme,
    ) -> Element<'static, Message> {
        row![
            button(text("-").size(11).center())
                .padding([3, 7])
                .style(subtle_liquidations_distribution_header_button)
                .on_press(Message::LiquidationsDistributionZoomed {
                    factor: 1.0 / LIQUIDATION_DISTRIBUTION_ZOOM_STEP,
                    anchor: None,
                }),
            text(format!("{:.0}%", zoom * 100.0))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
            button(text("+").size(11).center())
                .padding([3, 7])
                .style(subtle_liquidations_distribution_header_button)
                .on_press(Message::LiquidationsDistributionZoomed {
                    factor: LIQUIDATION_DISTRIBUTION_ZOOM_STEP,
                    anchor: None,
                }),
            button(text("Reset").size(11).center())
                .padding([3, 7])
                .style(subtle_liquidations_distribution_header_button)
                .on_press(Message::ResetLiquidationsDistributionZoom),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_liquidations_distribution_metrics(
        &self,
        data: &LiquidationDistributionData,
        now_ms: u64,
        available_width: f32,
        denomination: &DisplayDenominationContext,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let total = data.total_long_usd + data.total_short_usd;
        let updated = helpers::format_relative_time(data.fetched_at_ms, now_ms);
        let first_row = row![
            metric_block("Mark", denomination.format_price(data.request.mark), theme),
            metric_block(
                "Longs",
                denomination.format_value(data.total_long_usd, 0),
                theme
            ),
            metric_block(
                "Shorts",
                denomination.format_value(data.total_short_usd, 0),
                theme
            ),
        ]
        .spacing(8)
        .width(Fill);
        let second_row = row![
            metric_block("Total", denomination.format_value(total, 0), theme),
            metric_block("Levels", data.raw_count.to_string(), theme),
            metric_block("Updated", updated, theme),
        ]
        .spacing(8)
        .width(Fill);

        if available_width < 520.0 {
            column![first_row, second_row].spacing(6).width(Fill).into()
        } else {
            row![first_row, second_row].spacing(8).width(Fill).into()
        }
    }
}

fn subtle_liquidations_distribution_header_button(
    theme: &Theme,
    status: button::Status,
) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(
            Color {
                a: 0.06,
                ..theme.palette().text
            }
            .into(),
        ),
        _ => Some(Color::TRANSPARENT.into()),
    };

    button::Style {
        background,
        text_color: theme.extended_palette().background.weak.text,
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn metric_block(label: &'static str, value: String, theme: &Theme) -> Element<'static, Message> {
    column![
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(theme.extended_palette().background.weak.text),
        text(value)
            .size(12)
            .font(crate::app_fonts::monospace_font())
            .color(theme.palette().text),
    ]
    .spacing(2)
    .width(Length::FillPortion(1))
    .into()
}

fn liquidation_distribution_symbol_matches(symbol: &ExchangeSymbol, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let display = symbol
        .display_name
        .as_deref()
        .unwrap_or(symbol.ticker.as_str())
        .to_lowercase();
    display.contains(query)
        || symbol.ticker.to_lowercase().contains(query)
        || symbol.key.to_lowercase().contains(query)
        || symbol
            .keywords
            .iter()
            .any(|keyword| keyword.to_lowercase().contains(query))
}

// ---------------------------------------------------------------------------
// Liquidations Distribution Canvas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct LiquidationsDistributionChart {
    data: LiquidationDistributionData,
    denomination: DisplayDenominationContext,
    zoom: f64,
    zoom_center_price: Option<f64>,
}

impl canvas::Program<Message> for LiquidationsDistributionChart {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) = event else {
            return None;
        };
        let dy = match delta {
            mouse::ScrollDelta::Lines { y, .. } => *y,
            mouse::ScrollDelta::Pixels { y, .. } => *y / 28.0,
        };
        if dy == 0.0 {
            return None;
        }
        let pos = cursor.position_in(bounds)?;
        let margins = ChartMargins::for_width(bounds.width);
        let domain = self.visible_price_range();
        let plot = PlotArea::new(bounds, margins, domain);
        let anchor = if plot.contains(pos) {
            Some(LiquidationDistributionZoomAnchor {
                price: plot.x_to_price(pos.x),
                fraction: ((pos.x - plot.left) / plot.width.max(1.0)) as f64,
            })
        } else {
            None
        };
        let factor = if dy > 0.0 {
            LIQUIDATION_DISTRIBUTION_ZOOM_STEP
        } else {
            1.0 / LIQUIDATION_DISTRIBUTION_ZOOM_STEP
        };

        Some(
            canvas::Action::publish(Message::LiquidationsDistributionZoomed { factor, anchor })
                .and_capture(),
        )
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        draw_distribution_chart(
            &self.data,
            &self.denomination,
            renderer,
            theme,
            bounds,
            cursor,
            self.visible_price_range(),
        )
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.position_in(bounds).is_some() {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}

impl LiquidationsDistributionChart {
    fn visible_price_range(&self) -> (f64, f64) {
        crate::liquidations_distribution_state::liquidation_distribution_visible_price_range(
            &self.data,
            self.zoom,
            self.zoom_center_price,
        )
    }
}

fn draw_distribution_chart(
    data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    renderer: &Renderer,
    theme: &Theme,
    bounds: Rectangle,
    cursor: iced::mouse::Cursor,
    price_domain: (f64, f64),
) -> Vec<canvas::Geometry> {
    let mut frame = Frame::new(renderer, bounds.size());
    if data.points.is_empty() || !data.has_values() || bounds.width < 180.0 || bounds.height < 120.0
    {
        draw_empty_chart(&mut frame, theme, bounds);
        return vec![frame.into_geometry()];
    }

    let margins = ChartMargins::for_width(bounds.width);
    let plot = PlotArea::new(bounds, margins, price_domain);
    if plot.width <= 0.0 || plot.height <= 0.0 {
        return vec![frame.into_geometry()];
    }

    let max_bucket_usd = visible_max_bucket_usd(data, &plot)
        .unwrap_or(data.max_bucket_usd)
        .max(1.0);
    let max_cumulative_usd = visible_max_cumulative_usd(data, &plot)
        .unwrap_or(data.max_cumulative_usd)
        .max(1.0);

    draw_grid(&mut frame, theme, &plot);
    frame.with_clip(plot.rectangle(), |frame| {
        draw_bars(frame, data, &plot, theme, max_bucket_usd);
        draw_cumulative_area(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            true,
            color!(0xff7777),
        );
        draw_cumulative_area(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            false,
            color!(0x66d9a8),
        );
        draw_cumulative_line(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            true,
            color!(0xff7777),
        );
        draw_cumulative_line(
            frame,
            &data.points,
            &plot,
            max_cumulative_usd,
            false,
            color!(0x66d9a8),
        );
    });
    draw_axes(
        &mut frame,
        data,
        denomination,
        theme,
        &plot,
        max_bucket_usd,
        max_cumulative_usd,
    );
    draw_current_mark(&mut frame, data, denomination, theme, &plot);
    draw_hover_state(HoverStateRenderContext {
        frame: &mut frame,
        data,
        denomination,
        theme,
        bounds,
        plot: &plot,
        cursor,
        max_cumulative_usd,
    });

    vec![frame.into_geometry()]
}

#[derive(Debug, Clone, Copy)]
struct ChartMargins {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

impl ChartMargins {
    fn for_width(width: f32) -> Self {
        if width >= 520.0 {
            Self {
                left: 58.0,
                right: 68.0,
                top: 14.0,
                bottom: 32.0,
            }
        } else {
            Self {
                left: 44.0,
                right: 48.0,
                top: 10.0,
                bottom: 28.0,
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct PlotArea {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    width: f32,
    height: f32,
    price_min: f64,
    price_max: f64,
}

impl PlotArea {
    fn new(bounds: Rectangle, margins: ChartMargins, price_domain: (f64, f64)) -> Self {
        let left = margins.left;
        let right = (bounds.width - margins.right).max(left);
        let top = margins.top;
        let bottom = (bounds.height - margins.bottom).max(top);
        let mut price_min = price_domain.0.min(price_domain.1);
        let mut price_max = price_domain.0.max(price_domain.1);
        if !price_min.is_finite() || !price_max.is_finite() || price_max <= price_min {
            price_min = 0.0;
            price_max = price_min + 1.0;
        }
        Self {
            left,
            right,
            top,
            bottom,
            width: right - left,
            height: bottom - top,
            price_min,
            price_max,
        }
    }

    fn price_to_x(self, price: f64) -> f32 {
        let range = self.price_max - self.price_min;
        if range <= 0.0 {
            return self.left;
        }
        let ratio = ((price - self.price_min) / range) as f32;
        self.left + ratio * self.width
    }

    fn x_to_price(self, x: f32) -> f64 {
        let range = self.price_max - self.price_min;
        if range <= 0.0 || self.width <= 0.0 {
            return self.price_min;
        }
        let fraction = ((x - self.left) / self.width).clamp(0.0, 1.0) as f64;
        self.price_min + range * fraction
    }

    fn contains(self, point: Point) -> bool {
        point.x >= self.left
            && point.x <= self.right
            && point.y >= self.top
            && point.y <= self.bottom
    }

    fn price_is_visible(self, price: f64) -> bool {
        price >= self.price_min && price <= self.price_max
    }

    fn rectangle(self) -> Rectangle {
        Rectangle {
            x: self.left,
            y: self.top,
            width: self.width,
            height: self.height,
        }
    }

    fn value_to_y(self, value: f64, max_value: f64) -> f32 {
        if max_value <= 0.0 {
            return self.bottom;
        }
        let ratio = (value / max_value).clamp(0.0, 1.0) as f32;
        self.bottom - ratio * self.height
    }
}

fn draw_empty_chart(frame: &mut Frame, theme: &Theme, bounds: Rectangle) {
    frame.fill_text(canvas::Text {
        content: "No liquidation levels in range".to_string(),
        position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
        color: theme.extended_palette().background.weak.text,
        size: iced::Pixels(12.0),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

fn draw_grid(frame: &mut Frame, theme: &Theme, plot: &PlotArea) {
    let grid_color = Color {
        a: 0.09,
        ..theme.palette().text
    };
    for fraction in [0.25_f32, 0.5, 0.75] {
        let y = plot.top + plot.height * fraction;
        let path = canvas::Path::line(Point::new(plot.left, y), Point::new(plot.right, y));
        frame.stroke(
            &path,
            Stroke::default().with_color(grid_color).with_width(1.0),
        );
    }
    for fraction in [0.2_f32, 0.4, 0.6, 0.8] {
        let x = plot.left + plot.width * fraction;
        let path = canvas::Path::line(Point::new(x, plot.top), Point::new(x, plot.bottom));
        frame.stroke(
            &path,
            Stroke::default().with_color(grid_color).with_width(1.0),
        );
    }
}

fn draw_bars(
    frame: &mut Frame,
    data: &LiquidationDistributionData,
    plot: &PlotArea,
    theme: &Theme,
    max_bucket: f64,
) {
    let bucket_width = visible_bucket_width(&data.points, plot);
    let long_color = Color {
        a: 0.62,
        ..theme.palette().danger
    };
    let short_color = Color {
        a: 0.62,
        ..theme.palette().success
    };

    for point in &data.points {
        let x = plot.price_to_x(point.price);
        let bar_w = (bucket_width * 0.72).max(1.0);
        if x + bar_w < plot.left || x - bar_w > plot.right {
            continue;
        }

        if point.long_usd > 0.0 {
            let y = plot.value_to_y(point.long_usd, max_bucket);
            frame.fill_rectangle(
                Point::new(x - bar_w / 2.0, y),
                iced::Size::new(bar_w, plot.bottom - y),
                long_color,
            );
        }
        if point.short_usd > 0.0 {
            let y = plot.value_to_y(point.short_usd, max_bucket);
            frame.fill_rectangle(
                Point::new(x - bar_w / 2.0, y),
                iced::Size::new(bar_w, plot.bottom - y),
                short_color,
            );
        }
    }
}

fn draw_cumulative_area(
    frame: &mut Frame,
    points: &[LiquidationDistributionPoint],
    plot: &PlotArea,
    max_cumulative: f64,
    longs: bool,
    color: Color,
) {
    let visible: Vec<_> = points
        .iter()
        .filter(|point| {
            if !plot.price_is_visible(point.price) {
                return false;
            }
            if longs {
                point.cumulative_long_usd > 0.0
            } else {
                point.cumulative_short_usd > 0.0
            }
        })
        .collect();
    if visible.len() < 2 {
        return;
    }

    let mut area_color = color;
    area_color.a = 0.12;

    let path = canvas::Path::new(|builder| {
        if let Some(first) = visible.first() {
            builder.move_to(Point::new(point_x(first, plot), plot.bottom));
        }
        for point in &visible {
            let value = if longs {
                point.cumulative_long_usd
            } else {
                point.cumulative_short_usd
            };
            builder.line_to(Point::new(
                point_x(point, plot),
                plot.value_to_y(value, max_cumulative),
            ));
        }
        if let Some(last) = visible.last() {
            builder.line_to(Point::new(point_x(last, plot), plot.bottom));
        }
        builder.close();
    });
    frame.fill(&path, area_color);
}

fn draw_cumulative_line(
    frame: &mut Frame,
    points: &[LiquidationDistributionPoint],
    plot: &PlotArea,
    max_cumulative: f64,
    longs: bool,
    color: Color,
) {
    let mut builder = canvas::path::Builder::new();
    let mut has_point = false;
    for point in points {
        if !plot.price_is_visible(point.price) {
            continue;
        }
        let value = if longs {
            point.cumulative_long_usd
        } else {
            point.cumulative_short_usd
        };
        if value <= 0.0 {
            continue;
        }
        let p = Point::new(point_x(point, plot), plot.value_to_y(value, max_cumulative));
        if has_point {
            builder.line_to(p);
        } else {
            builder.move_to(p);
            has_point = true;
        }
    }
    if !has_point {
        return;
    }

    frame.stroke(
        &builder.build(),
        Stroke::default().with_width(1.8).with_color(color),
    );
}

fn point_x(point: &LiquidationDistributionPoint, plot: &PlotArea) -> f32 {
    plot.price_to_x(point.price)
}

fn visible_bucket_width(points: &[LiquidationDistributionPoint], plot: &PlotArea) -> f32 {
    let mut previous_x: Option<f32> = None;
    let mut min_step = f32::MAX;
    for point in points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
    {
        let x = plot.price_to_x(point.price);
        if let Some(previous_x) = previous_x {
            min_step = min_step.min((x - previous_x).abs());
        }
        previous_x = Some(x);
    }

    if min_step.is_finite() {
        min_step.max(1.0)
    } else {
        (plot.width / points.len().max(1) as f32).max(1.0)
    }
}

fn visible_max_bucket_usd(data: &LiquidationDistributionData, plot: &PlotArea) -> Option<f64> {
    data.points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
        .map(|point| point.long_usd.max(point.short_usd))
        .reduce(f64::max)
}

fn visible_max_cumulative_usd(data: &LiquidationDistributionData, plot: &PlotArea) -> Option<f64> {
    data.points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
        .map(|point| point.cumulative_long_usd.max(point.cumulative_short_usd))
        .reduce(f64::max)
}

fn draw_axes(
    frame: &mut Frame,
    _data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    plot: &PlotArea,
    max_bucket_usd: f64,
    max_cumulative_usd: f64,
) {
    let axis_color = Color {
        a: 0.42,
        ..theme.palette().text
    };
    let label_color = Color {
        a: 0.68,
        ..theme.palette().text
    };
    let base = canvas::Path::new(|builder| {
        builder.move_to(Point::new(plot.left, plot.top));
        builder.line_to(Point::new(plot.left, plot.bottom));
        builder.line_to(Point::new(plot.right, plot.bottom));
        builder.move_to(Point::new(plot.right, plot.top));
        builder.line_to(Point::new(plot.right, plot.bottom));
    });
    frame.stroke(
        &base,
        Stroke::default().with_color(axis_color).with_width(1.0),
    );

    for fraction in [0.0_f32, 0.5, 1.0] {
        let y = plot.bottom - plot.height * fraction;
        let bucket_value = max_bucket_usd * fraction as f64;
        let cumulative_value = max_cumulative_usd * fraction as f64;
        frame.fill_text(canvas::Text {
            content: compact_denomination_value(denomination, bucket_value),
            position: Point::new(plot.left - 6.0, y),
            color: label_color,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
        frame.fill_text(canvas::Text {
            content: compact_denomination_value(denomination, cumulative_value),
            position: Point::new(plot.right + 6.0, y),
            color: label_color,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Left.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }

    for fraction in [0.0_f64, 0.25, 0.5, 0.75, 1.0] {
        let price = plot.price_min + (plot.price_max - plot.price_min) * fraction;
        let x = plot.price_to_x(price);
        frame.fill_text(canvas::Text {
            content: denomination.format_price(price),
            position: Point::new(x, plot.bottom + 15.0),
            color: label_color,
            size: iced::Pixels(10.0),
            align_x: iced::alignment::Horizontal::Center.into(),
            align_y: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }
}

fn draw_current_mark(
    frame: &mut Frame,
    data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    plot: &PlotArea,
) {
    if !plot.price_is_visible(data.request.mark) {
        return;
    }
    let x = plot.price_to_x(data.request.mark);
    let marker_color = theme.palette().primary;
    let mut stroke = Stroke::default().with_color(marker_color).with_width(1.5);
    stroke.line_dash = canvas::stroke::LineDash {
        segments: &[6.0, 5.0],
        offset: 0,
    };
    let line = canvas::Path::line(Point::new(x, plot.top), Point::new(x, plot.bottom));
    frame.stroke(&line, stroke);

    let price_label = denomination.format_price(data.request.mark);
    let compact = plot.width < 112.0;
    let label = if compact {
        price_label
    } else {
        format!("Current: {price_label}")
    };
    let estimated_width = label.chars().count() as f32 * 6.2 + 14.0;
    let width = estimated_width.min(plot.width.max(1.0)).max(1.0);
    let height = 20.0;
    let max_label_x = (plot.right - width).max(plot.left);
    let label_x = (x - width / 2.0).clamp(plot.left, max_label_x);
    let label_y = plot.bottom + 4.0;
    frame.fill_rectangle(
        Point::new(label_x, label_y),
        iced::Size::new(width, height),
        marker_color,
    );
    let label_border =
        canvas::Path::rectangle(Point::new(label_x, label_y), iced::Size::new(width, height));
    frame.stroke(
        &label_border,
        Stroke::default()
            .with_color(theme.extended_palette().background.strong.color)
            .with_width(1.0),
    );
    frame.fill_text(canvas::Text {
        content: label,
        position: Point::new(label_x + width / 2.0, label_y + height / 2.0),
        color: theme.palette().background,
        size: iced::Pixels(10.0),
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}

struct HoverStateRenderContext<'a> {
    frame: &'a mut Frame,
    data: &'a LiquidationDistributionData,
    denomination: &'a DisplayDenominationContext,
    theme: &'a Theme,
    bounds: Rectangle,
    plot: &'a PlotArea,
    cursor: iced::mouse::Cursor,
    max_cumulative_usd: f64,
}

fn draw_hover_state(ctx: HoverStateRenderContext<'_>) {
    let Some(cursor_pos) = ctx.cursor.position_in(ctx.bounds) else {
        return;
    };
    if cursor_pos.x < ctx.plot.left
        || cursor_pos.x > ctx.plot.right
        || cursor_pos.y < ctx.plot.top
        || cursor_pos.y > ctx.plot.bottom
    {
        return;
    }
    let Some(point) = nearest_distribution_point(ctx.data, ctx.plot, cursor_pos.x) else {
        return;
    };

    let x = point_x(point, ctx.plot);
    let guide = canvas::Path::line(Point::new(x, ctx.plot.top), Point::new(x, ctx.plot.bottom));
    ctx.frame.stroke(
        &guide,
        Stroke::default()
            .with_color(Color {
                a: 0.22,
                ..ctx.theme.palette().text
            })
            .with_width(1.0),
    );

    for (value, max_value, color) in [
        (
            point.cumulative_long_usd,
            ctx.max_cumulative_usd,
            color!(0xff7777),
        ),
        (
            point.cumulative_short_usd,
            ctx.max_cumulative_usd,
            color!(0x66d9a8),
        ),
    ] {
        if value > 0.0 {
            let marker =
                canvas::Path::circle(Point::new(x, ctx.plot.value_to_y(value, max_value)), 2.8);
            ctx.frame.fill(&marker, color);
        }
    }

    let tooltip_width = 170.0_f32.min((ctx.plot.width - 8.0).max(126.0));
    let tooltip_height = 68.0_f32;
    let max_x = (ctx.plot.right - tooltip_width).max(ctx.plot.left);
    let max_y = (ctx.plot.bottom - tooltip_height).max(ctx.plot.top);
    let tooltip_x = if cursor_pos.x + tooltip_width + 12.0 <= ctx.plot.right {
        cursor_pos.x + 10.0
    } else {
        cursor_pos.x - tooltip_width - 10.0
    }
    .clamp(ctx.plot.left, max_x);
    let tooltip_y = (cursor_pos.y - tooltip_height / 2.0).clamp(ctx.plot.top, max_y);
    let tooltip_origin = Point::new(tooltip_x, tooltip_y);

    ctx.frame.fill_rectangle(
        tooltip_origin,
        Size::new(tooltip_width, tooltip_height),
        Color {
            a: 0.94,
            ..ctx.theme.extended_palette().background.strong.color
        },
    );
    let border = canvas::Path::rectangle(tooltip_origin, Size::new(tooltip_width, tooltip_height));
    ctx.frame.stroke(
        &border,
        Stroke::default()
            .with_color(Color {
                a: 0.18,
                ..ctx.theme.palette().text
            })
            .with_width(1.0),
    );

    let tooltip_text = format!(
        "{}\nL {}  S {}\nCum L {}\nCum S {}",
        ctx.denomination.format_price(point.price),
        compact_denomination_value(ctx.denomination, point.long_usd),
        compact_denomination_value(ctx.denomination, point.short_usd),
        compact_denomination_value(ctx.denomination, point.cumulative_long_usd),
        compact_denomination_value(ctx.denomination, point.cumulative_short_usd),
    );
    ctx.frame.fill_text(canvas::Text {
        content: tooltip_text,
        position: Point::new(tooltip_x + 8.0, tooltip_y + 8.0),
        color: ctx.theme.palette().text,
        size: iced::Pixels(10.0),
        font: crate::app_fonts::monospace_font(),
        align_x: iced::alignment::Horizontal::Left.into(),
        align_y: iced::alignment::Vertical::Top,
        ..Default::default()
    });
}

fn nearest_distribution_point<'a>(
    data: &'a LiquidationDistributionData,
    plot: &PlotArea,
    cursor_x: f32,
) -> Option<&'a LiquidationDistributionPoint> {
    data.points
        .iter()
        .filter(|point| plot.price_is_visible(point.price))
        .min_by(|left, right| {
            let left_distance = (point_x(left, plot) - cursor_x).abs();
            let right_distance = (point_x(right, plot) - cursor_x).abs();
            left_distance.total_cmp(&right_distance)
        })
}

fn compact_denomination_value(denomination: &DisplayDenominationContext, usd_value: f64) -> String {
    let Some(value) = denomination.convert_usd_value(usd_value) else {
        return helpers::invalid_data_placeholder();
    };
    let sign = if value < 0.0 { "-" } else { "" };
    let number = compact_number(value.abs());
    match denomination.active_code() {
        "USD" | "EUR" => format!("{sign}{}{number}", denomination.active_symbol()),
        code => format!("{sign}{number} {code}"),
    }
}

fn compact_number(value: f64) -> String {
    if value >= 1_000_000_000.0 {
        format!("{:.1}B", value / 1_000_000_000.0)
    } else if value >= 1_000_000.0 {
        format!("{:.1}M", value / 1_000_000.0)
    } else if value >= 1_000.0 {
        format!("{:.0}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}
