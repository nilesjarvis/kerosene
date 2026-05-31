use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::liquidations_distribution_state::{
    LiquidationDistributionData, LiquidationDistributionPoint,
};
use crate::message::Message;

use iced::widget::canvas::{self, Frame, Stroke};
use iced::widget::{Space, button, column, container, responsive, row, text};
use iced::{
    Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme, color,
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
        let denomination = self.display_denomination_context();
        let state = &self.liquidation_distribution;
        let symbol = state
            .pending_request
            .as_ref()
            .map(|request| request.display.as_str())
            .or_else(|| {
                state
                    .data
                    .as_ref()
                    .map(|data| data.request.display.as_str())
            })
            .filter(|symbol| !symbol.trim().is_empty())
            .unwrap_or(self.active_symbol_display.as_str());

        let refresh_btn = if state.loading {
            button(text("Refreshing").size(11)).padding([4, 8])
        } else {
            button(text("Refresh").size(11))
                .padding([4, 8])
                .on_press(Message::RefreshLiquidationsDistribution)
        };

        let mut content = column![
            row![
                column![
                    text("Liquidations Distribution")
                        .size(13)
                        .color(theme.palette().text),
                    text(format!("{} / USD", symbol.to_uppercase()))
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(theme.extended_palette().background.weak.text),
                ]
                .spacing(2)
                .width(Fill),
                refresh_btn,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            self.view_liquidations_distribution_legend(available_width, &theme),
        ]
        .spacing(8);

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
                    available_width,
                    &denomination,
                    &theme,
                ))
                .push(
                    container(
                        iced::widget::canvas(LiquidationsDistributionChart {
                            data: data.clone(),
                            denomination,
                        })
                        .width(Fill)
                        .height(Fill),
                    )
                    .width(Fill)
                    .height(Fill)
                    .padding(if available_width >= 520.0 {
                        [10, 12]
                    } else {
                        [8, 8]
                    })
                    .style(|theme: &Theme| container::Style {
                        background: Some(theme.extended_palette().background.weak.color.into()),
                        border: iced::Border {
                            radius: 4.0.into(),
                            width: 1.0,
                            color: Color {
                                a: 0.18,
                                ..theme.extended_palette().background.weak.text
                            },
                        },
                        ..Default::default()
                    }),
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

    fn view_liquidations_distribution_legend(
        &self,
        available_width: f32,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let compact = available_width < 520.0;
        row![
            legend_item(
                if compact { "Long" } else { "Long Liquidations" },
                color!(0xff5555),
                theme
            ),
            legend_item(
                if compact {
                    "Short"
                } else {
                    "Short Liquidations"
                },
                color!(0x50fa7b),
                theme
            ),
            legend_item(
                if compact { "Cum L" } else { "Cumulative Longs" },
                color!(0xff7777),
                theme
            ),
            legend_item(
                if compact {
                    "Cum S"
                } else {
                    "Cumulative Shorts"
                },
                color!(0x66d9a8),
                theme
            ),
        ]
        .spacing(if compact { 8 } else { 12 })
        .align_y(Alignment::Center)
        .into()
    }

    fn view_liquidations_distribution_metrics(
        &self,
        data: &LiquidationDistributionData,
        available_width: f32,
        denomination: &DisplayDenominationContext,
        theme: &Theme,
    ) -> Element<'static, Message> {
        let total = data.total_long_usd + data.total_short_usd;
        let updated = helpers::format_relative_time(data.fetched_at_ms, Self::now_ms());
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

fn legend_item(label: &'static str, color: Color, theme: &Theme) -> Element<'static, Message> {
    row![
        container(Space::new())
            .width(12.0)
            .height(8.0)
            .style(move |_| container::Style {
                background: Some(color.into()),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        text(label)
            .size(11)
            .color(theme.extended_palette().background.weak.text),
    ]
    .spacing(5)
    .align_y(Alignment::Center)
    .into()
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

// ---------------------------------------------------------------------------
// Liquidations Distribution Canvas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct LiquidationsDistributionChart {
    data: LiquidationDistributionData,
    denomination: DisplayDenominationContext,
}

impl canvas::Program<Message> for LiquidationsDistributionChart {
    type State = ();

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
) -> Vec<canvas::Geometry> {
    let mut frame = Frame::new(renderer, bounds.size());
    if data.points.is_empty() || !data.has_values() || bounds.width < 180.0 || bounds.height < 120.0
    {
        draw_empty_chart(&mut frame, theme, bounds);
        return vec![frame.into_geometry()];
    }

    let margins = ChartMargins::for_width(bounds.width);
    let plot = PlotArea::new(bounds, margins);
    if plot.width <= 0.0 || plot.height <= 0.0 {
        return vec![frame.into_geometry()];
    }

    draw_grid(&mut frame, theme, &plot);
    draw_bars(&mut frame, data, &plot, theme);
    draw_cumulative_area(
        &mut frame,
        &data.points,
        &plot,
        data.max_cumulative_usd,
        true,
        color!(0xff7777),
    );
    draw_cumulative_area(
        &mut frame,
        &data.points,
        &plot,
        data.max_cumulative_usd,
        false,
        color!(0x66d9a8),
    );
    draw_cumulative_line(
        &mut frame,
        &data.points,
        &plot,
        data.max_cumulative_usd,
        true,
        color!(0xff7777),
    );
    draw_cumulative_line(
        &mut frame,
        &data.points,
        &plot,
        data.max_cumulative_usd,
        false,
        color!(0x66d9a8),
    );
    draw_axes(&mut frame, data, denomination, theme, &plot);
    draw_current_mark(&mut frame, data, denomination, theme, &plot);
    draw_hover_state(&mut frame, data, denomination, theme, bounds, &plot, cursor);

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
}

impl PlotArea {
    fn new(bounds: Rectangle, margins: ChartMargins) -> Self {
        let left = margins.left;
        let right = (bounds.width - margins.right).max(left);
        let top = margins.top;
        let bottom = (bounds.height - margins.bottom).max(top);
        Self {
            left,
            right,
            top,
            bottom,
            width: right - left,
            height: bottom - top,
        }
    }

    fn price_to_x(self, data: &LiquidationDistributionData, price: f64) -> f32 {
        let range = data.request.max - data.request.min;
        if range <= 0.0 {
            return self.left;
        }
        let ratio = ((price - data.request.min) / range).clamp(0.0, 1.0) as f32;
        self.left + ratio * self.width
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
) {
    let max_bucket = data.max_bucket_usd.max(1.0);
    let bucket_width = (plot.width / data.points.len().max(1) as f32).max(1.0);
    let long_color = Color {
        a: 0.62,
        ..theme.palette().danger
    };
    let short_color = Color {
        a: 0.62,
        ..theme.palette().success
    };

    for point in &data.points {
        let x = plot.price_to_x(data, point.price);
        let bar_w = (bucket_width * 0.72).max(1.0);

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
            builder.move_to(Point::new(point_x(first, points, plot), plot.bottom));
        }
        for point in &visible {
            let value = if longs {
                point.cumulative_long_usd
            } else {
                point.cumulative_short_usd
            };
            builder.line_to(Point::new(
                point_x(point, points, plot),
                plot.value_to_y(value, max_cumulative),
            ));
        }
        if let Some(last) = visible.last() {
            builder.line_to(Point::new(point_x(last, points, plot), plot.bottom));
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
        let value = if longs {
            point.cumulative_long_usd
        } else {
            point.cumulative_short_usd
        };
        if value <= 0.0 {
            continue;
        }
        let p = Point::new(
            point_x(point, points, plot),
            plot.value_to_y(value, max_cumulative),
        );
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

fn point_x(
    point: &LiquidationDistributionPoint,
    points: &[LiquidationDistributionPoint],
    plot: &PlotArea,
) -> f32 {
    let Some((first, last)) = points.first().zip(points.last()) else {
        return plot.left;
    };
    let range = last.price - first.price;
    if range <= 0.0 {
        return plot.left;
    }
    let ratio = ((point.price - first.price) / range).clamp(0.0, 1.0) as f32;
    plot.left + ratio * plot.width
}

fn draw_axes(
    frame: &mut Frame,
    data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    plot: &PlotArea,
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
        let bucket_value = data.max_bucket_usd * fraction as f64;
        let cumulative_value = data.max_cumulative_usd * fraction as f64;
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
        let price = data.request.min + (data.request.max - data.request.min) * fraction;
        let x = plot.price_to_x(data, price);
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
    let x = plot.price_to_x(data, data.request.mark);
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

fn draw_hover_state(
    frame: &mut Frame,
    data: &LiquidationDistributionData,
    denomination: &DisplayDenominationContext,
    theme: &Theme,
    bounds: Rectangle,
    plot: &PlotArea,
    cursor: iced::mouse::Cursor,
) {
    let Some(cursor_pos) = cursor.position_in(bounds) else {
        return;
    };
    if cursor_pos.x < plot.left
        || cursor_pos.x > plot.right
        || cursor_pos.y < plot.top
        || cursor_pos.y > plot.bottom
    {
        return;
    }
    let Some(point) = nearest_distribution_point(data, plot, cursor_pos.x) else {
        return;
    };

    let x = point_x(point, &data.points, plot);
    let guide = canvas::Path::line(Point::new(x, plot.top), Point::new(x, plot.bottom));
    frame.stroke(
        &guide,
        Stroke::default()
            .with_color(Color {
                a: 0.22,
                ..theme.palette().text
            })
            .with_width(1.0),
    );

    for (value, max_value, color) in [
        (
            point.cumulative_long_usd,
            data.max_cumulative_usd,
            color!(0xff7777),
        ),
        (
            point.cumulative_short_usd,
            data.max_cumulative_usd,
            color!(0x66d9a8),
        ),
    ] {
        if value > 0.0 {
            let marker =
                canvas::Path::circle(Point::new(x, plot.value_to_y(value, max_value)), 2.8);
            frame.fill(&marker, color);
        }
    }

    let tooltip_width = 170.0_f32.min((plot.width - 8.0).max(126.0));
    let tooltip_height = 68.0_f32;
    let max_x = (plot.right - tooltip_width).max(plot.left);
    let max_y = (plot.bottom - tooltip_height).max(plot.top);
    let tooltip_x = if cursor_pos.x + tooltip_width + 12.0 <= plot.right {
        cursor_pos.x + 10.0
    } else {
        cursor_pos.x - tooltip_width - 10.0
    }
    .clamp(plot.left, max_x);
    let tooltip_y = (cursor_pos.y - tooltip_height / 2.0).clamp(plot.top, max_y);
    let tooltip_origin = Point::new(tooltip_x, tooltip_y);

    frame.fill_rectangle(
        tooltip_origin,
        Size::new(tooltip_width, tooltip_height),
        Color {
            a: 0.94,
            ..theme.extended_palette().background.strong.color
        },
    );
    let border = canvas::Path::rectangle(tooltip_origin, Size::new(tooltip_width, tooltip_height));
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(Color {
                a: 0.18,
                ..theme.palette().text
            })
            .with_width(1.0),
    );

    let tooltip_text = format!(
        "{}\nL {}  S {}\nCum L {}\nCum S {}",
        denomination.format_price(point.price),
        compact_denomination_value(denomination, point.long_usd),
        compact_denomination_value(denomination, point.short_usd),
        compact_denomination_value(denomination, point.cumulative_long_usd),
        compact_denomination_value(denomination, point.cumulative_short_usd),
    );
    frame.fill_text(canvas::Text {
        content: tooltip_text,
        position: Point::new(tooltip_x + 8.0, tooltip_y + 8.0),
        color: theme.palette().text,
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
    data.points.iter().min_by(|left, right| {
        let left_distance = (point_x(left, &data.points, plot) - cursor_x).abs();
        let right_distance = (point_x(right, &data.points, plot) - cursor_x).abs();
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
