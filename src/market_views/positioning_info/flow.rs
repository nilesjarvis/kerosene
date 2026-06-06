use super::metrics::{
    PositioningFlowData, PositioningFlowKind, PositioningFlowRow, positioning_flow_data,
    positioning_live_mark,
};
use crate::app_state::TradingTerminal;
use crate::denomination::DisplayDenominationContext;
use crate::helpers::ease_out_cubic;
use crate::message::Message;
use crate::positioning_state::PositioningInfoInstance;

use crate::wallet_views::{WalletAddressActionCell, wallet_address_action_cell};

use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::widget::{Column, Space, canvas as canvas_widget, container, responsive, row, stack};
use iced::{Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme, mouse, time};

/// Number of top movers visualized as bars. The flow view is a ranking of the
/// largest moves, so a focused cap keeps it scannable instead of endless.
const POSITIONING_FLOW_ROW_LIMIT: usize = 60;

impl TradingTerminal {
    pub(in crate::market_views::positioning_info) fn view_positioning_info_flow(
        &self,
        instance: &PositioningInfoInstance,
    ) -> Element<'_, Message> {
        let live_mark = positioning_live_mark(instance, TradingTerminal::now_ms());
        let denomination = self.display_denomination_context();
        let data = match &instance.change_data {
            Some(data) => {
                positioning_flow_data(&data.deltas, live_mark, POSITIONING_FLOW_ROW_LIMIT)
            }
            None => positioning_flow_data(&[], live_mark, POSITIONING_FLOW_ROW_LIMIT),
        };

        // Resolve trader labels against the address book before handing pure
        // data to the canvas program.
        let hovered_key = self.hovered_wallet_address_actions.clone();
        let mut chart = PositioningFlowChart::new(&data, &denomination);
        for (row, source) in chart.rows.iter_mut().zip(data.rows.iter()) {
            row.label = self.wallet_display(&source.address).primary;
            row.hover_key = format!("positioning-flow:{}:{}", instance.id, source.address);
        }
        chart.hovered_action_key = hovered_key.clone();

        let height = chart.content_height();
        let theme = self.theme();

        responsive(move |size| {
            let chart_layer: Element<'_, Message> = canvas_widget(chart.clone())
                .width(Fill)
                .height(Length::Fixed(height))
                .into();

            // The interactive label overlay only appears when the canvas shows
            // the label column, so the buttons never collide with the bars.
            if PositioningFlowChart::labels_visible(size.width)
                && let Some(overlay) =
                    build_action_overlay(chart.rows(), hovered_key.as_deref(), &theme)
            {
                container(stack![chart_layer, overlay])
                    .width(Fill)
                    .height(Length::Fixed(height))
                    .into()
            } else {
                container(chart_layer)
                    .width(Fill)
                    .height(Length::Fixed(height))
                    .into()
            }
        })
        .into()
    }
}

/// Builds the interactive trader-label overlay aligned to the canvas rows. Each
/// slot shows the resolved label and, on hover, swaps to copy/detach/ghost
/// action segments (the same widget the positioning table uses).
fn build_action_overlay(
    rows: &[PositioningFlowChartRow],
    hovered_key: Option<&str>,
    theme: &Theme,
) -> Option<Element<'static, Message>> {
    if rows.is_empty() {
        return None;
    }

    let label_left = PositioningFlowChart::label_left();
    let label_width = PositioningFlowChart::label_width();
    let row_height = PositioningFlowChart::row_height();
    let row_gap = PositioningFlowChart::row_gap();

    let mut column =
        Column::new().push(Space::new().height(Length::Fixed(PositioningFlowChart::rows_top())));

    for row in rows {
        let cell = wallet_address_action_cell(WalletAddressActionCell {
            address: row.address.clone(),
            label: row.label.clone(),
            tooltip_label: format!("Copy {}", row.address),
            hover_key: row.hover_key.clone(),
            hovered_key,
            width: label_width,
            text_size: 11,
            text_color: theme.palette().text,
        });

        let slot = row![
            Space::new().width(Length::Fixed(label_left)),
            cell,
            Space::new().width(Fill),
        ]
        .height(Length::Fixed(row_height))
        .align_y(iced::Alignment::Center);

        column = column
            .push(slot)
            .push(Space::new().height(Length::Fixed(row_gap)));
    }

    Some(column.width(Fill).into())
}

// ---------------------------------------------------------------------------
// Positioning Change Flow Canvas
//
// A diverging horizontal bar chart: each trader's signed change over the
// selected timeframe extends right (more long, success color) or left (more
// short, danger color) from a center axis, scaled to the largest move in view.
// A net-flow "tug of war" header summarizes aggregate long vs short flow.
// ---------------------------------------------------------------------------

const HEADER_HEIGHT: f32 = 30.0;
const HEADER_GAP: f32 = 10.0;
const ROW_HEIGHT: f32 = 22.0;
const ROW_GAP: f32 = 3.0;
const SIDE_PADDING: f32 = 8.0;
const MIN_BAR_PX: f32 = 2.0;
const TOOLTIP_ANIMATION_EASE: f32 = 0.34;
const TOOLTIP_ANIMATION_EPSILON: f32 = 0.01;
const TOOLTIP_ANIMATION_FRAME_MS: u64 = 16;
const TOOLTIP_ANIMATION_OFFSET_PX: f32 = 5.0;

// Aggressive, stepped collapsing (mirrors the positioning column toggle): the
// bar is always shown; labels/value/tag are revealed only with real headroom.
const SHOW_VALUE_MIN_WIDTH: f32 = 240.0;
const SHOW_LABEL_MIN_WIDTH: f32 = 340.0;
const SHOW_TAG_MIN_WIDTH: f32 = 480.0;

const LABEL_WIDTH: f32 = 120.0;
const VALUE_WIDTH: f32 = 84.0;
const TAG_WIDTH: f32 = 42.0;

#[derive(Debug, Default)]
pub(in crate::market_views::positioning_info) struct PositioningFlowState {
    hovered: Option<usize>,
    tooltip_progress: f32,
}

impl PositioningFlowState {
    fn set_hovered(&mut self, hovered: Option<usize>) -> bool {
        if self.hovered == hovered {
            return false;
        }

        self.hovered = hovered;
        if hovered.is_some() {
            self.tooltip_progress = self.tooltip_progress.min(0.25);
        } else {
            self.tooltip_progress = 0.0;
        }
        true
    }

    fn tooltip_animation_active(&self) -> bool {
        self.hovered.is_some() && self.tooltip_progress < 1.0
    }

    fn advance_tooltip_animation(&mut self) {
        if self.hovered.is_none() {
            self.tooltip_progress = 0.0;
            return;
        }

        let delta = 1.0 - self.tooltip_progress;
        if delta <= TOOLTIP_ANIMATION_EPSILON {
            self.tooltip_progress = 1.0;
            return;
        }

        self.tooltip_progress =
            (self.tooltip_progress + delta * TOOLTIP_ANIMATION_EASE).clamp(0.0, 1.0);
    }

    fn tooltip_visibility(&self) -> f32 {
        if self.hovered.is_some() {
            ease_out_cubic(self.tooltip_progress)
        } else {
            0.0
        }
    }
}

/// A single row prepared for rendering. Labels and tooltip text are resolved at
/// view-build time so the canvas program stays a pure function of its inputs.
#[derive(Debug, Clone)]
pub(in crate::market_views::positioning_info) struct PositioningFlowChartRow {
    pub(in crate::market_views::positioning_info) address: String,
    pub(in crate::market_views::positioning_info) label: String,
    pub(in crate::market_views::positioning_info) hover_key: String,
    pub(in crate::market_views::positioning_info) value_text: String,
    pub(in crate::market_views::positioning_info) magnitude: f64,
    pub(in crate::market_views::positioning_info) more_long: bool,
    pub(in crate::market_views::positioning_info) kind: PositioningFlowKind,
    pub(in crate::market_views::positioning_info) tooltip: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub(in crate::market_views::positioning_info) struct PositioningFlowChart {
    rows: Vec<PositioningFlowChartRow>,
    max_magnitude: f64,
    long_flow: f64,
    short_flow: f64,
    long_label: String,
    short_label: String,
    net_label: String,
    empty_text: String,
    /// Wallet-action hover key whose label is replaced by action buttons in the
    /// widget overlay; the canvas blanks that row's label to avoid overdraw.
    hovered_action_key: Option<String>,
}

impl PositioningFlowChart {
    pub(in crate::market_views::positioning_info) fn new(
        data: &PositioningFlowData,
        denomination: &DisplayDenominationContext,
    ) -> Self {
        let rows = data
            .rows
            .iter()
            .map(|row| build_chart_row(row, data.usd_scaled, denomination))
            .collect();

        let net = data.long_flow - data.short_flow;
        let (long_label, short_label, net_label) = if data.usd_scaled {
            (
                compact_usd(data.long_flow, denomination),
                compact_usd(data.short_flow, denomination),
                format!(
                    "{} {}",
                    if net >= 0.0 { "NET +" } else { "NET -" },
                    compact_usd(net.abs(), denomination)
                ),
            )
        } else {
            (
                compact_size(data.long_flow),
                compact_size(data.short_flow),
                format!(
                    "NET {}{}",
                    if net >= 0.0 { "+" } else { "-" },
                    compact_size(net.abs())
                ),
            )
        };

        let empty_text = if data.usd_scaled {
            "No measurable position changes".to_string()
        } else {
            "Awaiting live mark for USD scaling".to_string()
        };

        Self {
            rows,
            max_magnitude: data.max_magnitude,
            long_flow: data.long_flow,
            short_flow: data.short_flow,
            long_label,
            short_label,
            net_label,
            empty_text,
            hovered_action_key: None,
        }
    }

    pub(in crate::market_views::positioning_info) fn content_height(&self) -> f32 {
        let rows = self.rows.len().max(1) as f32;
        HEADER_HEIGHT + HEADER_GAP + rows * ROW_HEIGHT + (rows - 1.0).max(0.0) * ROW_GAP
    }

    /// Whether the trader label column is shown at this width (labels and the
    /// interactive action overlay only appear with real headroom).
    pub(in crate::market_views::positioning_info) fn labels_visible(width: f32) -> bool {
        width >= SHOW_LABEL_MIN_WIDTH
    }

    /// Pixel offset from the top of the chart to the first row.
    pub(in crate::market_views::positioning_info) fn rows_top() -> f32 {
        HEADER_HEIGHT + HEADER_GAP
    }

    pub(in crate::market_views::positioning_info) fn row_height() -> f32 {
        ROW_HEIGHT
    }

    pub(in crate::market_views::positioning_info) fn row_gap() -> f32 {
        ROW_GAP
    }

    pub(in crate::market_views::positioning_info) fn label_left() -> f32 {
        SIDE_PADDING
    }

    pub(in crate::market_views::positioning_info) fn label_width() -> f32 {
        LABEL_WIDTH
    }

    pub(in crate::market_views::positioning_info) fn rows(&self) -> &[PositioningFlowChartRow] {
        &self.rows
    }

    fn row_index_at(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let pos = cursor.position_in(bounds)?;
        let rows_top = HEADER_HEIGHT + HEADER_GAP;
        if pos.y < rows_top {
            return None;
        }
        let offset = pos.y - rows_top;
        let stride = ROW_HEIGHT + ROW_GAP;
        let index = (offset / stride).floor() as usize;
        if index < self.rows.len() && (offset - index as f32 * stride) <= ROW_HEIGHT {
            Some(index)
        } else {
            None
        }
    }
}

impl canvas::Program<Message> for PositioningFlowChart {
    type State = PositioningFlowState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        if let iced::Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            if state.tooltip_animation_active() {
                state.advance_tooltip_animation();
                if state.tooltip_animation_active() {
                    return Some(canvas::Action::request_redraw_at(
                        *now + time::Duration::from_millis(TOOLTIP_ANIMATION_FRAME_MS),
                    ));
                }
            }
            return None;
        }

        let next = match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                self.row_index_at(bounds, cursor)
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => None,
            _ => return None,
        };
        if state.set_hovered(next) {
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
        self.draw_header(&mut frame, theme, bounds.width);

        if self.rows.is_empty() {
            frame.fill_text(Text {
                content: self.empty_text.clone(),
                position: Point::new(bounds.width / 2.0, HEADER_HEIGHT + HEADER_GAP + 24.0),
                color: theme.extended_palette().background.weak.text,
                size: iced::Pixels(12.0),
                align_x: Horizontal::Center.into(),
                align_y: Vertical::Center,
                ..Default::default()
            });
            return vec![frame.into_geometry()];
        }

        let layout = FlowLayout::new(bounds.width);
        for (index, row) in self.rows.iter().enumerate() {
            let top = HEADER_HEIGHT + HEADER_GAP + index as f32 * (ROW_HEIGHT + ROW_GAP);
            // The widget overlay draws action buttons over this row's label when
            // its wallet actions are hovered, so the canvas omits the label.
            let actions_hovered =
                self.hovered_action_key.as_deref() == Some(row.hover_key.as_str());
            self.draw_row(
                &mut frame,
                theme,
                &layout,
                row,
                top,
                state.hovered == Some(index),
                actions_hovered,
            );
        }

        // Center axis line spanning the rows.
        let rows_top = HEADER_HEIGHT + HEADER_GAP;
        let rows_bottom = self.content_height().min(bounds.height);
        let axis = Path::line(
            Point::new(layout.center_x, rows_top),
            Point::new(layout.center_x, rows_bottom),
        );
        frame.stroke(
            &axis,
            Stroke::default()
                .with_color(axis_color(theme))
                .with_width(1.0),
        );

        if let Some(index) = state.hovered
            && let Some(row) = self.rows.get(index)
        {
            draw_tooltip(
                &mut frame,
                theme,
                bounds,
                &layout,
                row,
                index,
                state.tooltip_visibility(),
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.hovered.is_some() {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

struct FlowLayout {
    show_value: bool,
    show_tag: bool,
    plot_right: f32,
    center_x: f32,
    half_width: f32,
}

impl FlowLayout {
    fn new(width: f32) -> Self {
        let show_value = width >= SHOW_VALUE_MIN_WIDTH;
        let show_label = width >= SHOW_LABEL_MIN_WIDTH;
        let show_tag = width >= SHOW_TAG_MIN_WIDTH;

        let label_w = if show_label { LABEL_WIDTH } else { 0.0 };
        let value_w = if show_value { VALUE_WIDTH } else { 0.0 };
        let tag_w = if show_tag { TAG_WIDTH } else { 0.0 };

        let plot_left = SIDE_PADDING + label_w;
        let plot_right = (width - SIDE_PADDING - value_w - tag_w).max(plot_left + 2.0 * MIN_BAR_PX);
        let center_x = (plot_left + plot_right) / 2.0;
        let half_width = (plot_right - plot_left) / 2.0;

        Self {
            show_value,
            show_tag,
            plot_right,
            center_x,
            half_width,
        }
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

impl PositioningFlowChart {
    fn draw_header(&self, frame: &mut Frame, theme: &Theme, width: f32) {
        let palette = theme.palette();
        let long_color = palette.success;
        let short_color = palette.danger;
        let muted = theme.extended_palette().background.weak.text;

        let left = SIDE_PADDING;
        let right = (width - SIDE_PADDING).max(left + 2.0);
        let track_w = right - left;
        let y = 4.0;
        let bar_h = 12.0;

        let total = self.long_flow + self.short_flow;
        let long_frac = if total > 0.0 {
            (self.long_flow / total) as f32
        } else {
            0.5
        };
        let split_x = left + track_w * long_frac;

        // Track background.
        frame.fill_rectangle(
            Point::new(left, y),
            Size::new(track_w, bar_h),
            faint(muted, 0.10),
        );
        // Short portion (left) and long portion (right).
        frame.fill_rectangle(
            Point::new(left, y),
            Size::new((split_x - left).max(0.0), bar_h),
            faint(short_color, 0.55),
        );
        frame.fill_rectangle(
            Point::new(split_x, y),
            Size::new((right - split_x).max(0.0), bar_h),
            faint(long_color, 0.55),
        );

        // Side labels and net readout below the track.
        let label_y = y + bar_h + 9.0;
        frame.fill_text(Text {
            content: format!("Shorts {}", self.short_label),
            position: Point::new(left, label_y),
            color: short_color,
            size: iced::Pixels(10.0),
            align_x: Horizontal::Left.into(),
            align_y: Vertical::Center,
            ..Default::default()
        });
        frame.fill_text(Text {
            content: self.net_label.clone(),
            position: Point::new((left + right) / 2.0, label_y),
            color: theme.palette().text,
            size: iced::Pixels(10.0),
            align_x: Horizontal::Center.into(),
            align_y: Vertical::Center,
            ..Default::default()
        });
        frame.fill_text(Text {
            content: format!("Longs {}", self.long_label),
            position: Point::new(right, label_y),
            color: long_color,
            size: iced::Pixels(10.0),
            align_x: Horizontal::Right.into(),
            align_y: Vertical::Center,
            ..Default::default()
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_row(
        &self,
        frame: &mut Frame,
        theme: &Theme,
        layout: &FlowLayout,
        row: &PositioningFlowChartRow,
        top: f32,
        hovered: bool,
        actions_hovered: bool,
    ) {
        let palette = theme.palette();
        let center_y = top + ROW_HEIGHT / 2.0;
        let color = if row.more_long {
            palette.success
        } else {
            palette.danger
        };

        // While the trader actions are hovered the overlay shows the action
        // pill (with its own surface), so the canvas skips the row highlight to
        // avoid stacking box-on-box; a plain cursor hover gets a faint wash.
        if hovered && !actions_hovered {
            frame.fill_rectangle(
                Point::new(SIDE_PADDING / 2.0, top),
                Size::new(layout.plot_right + 200.0, ROW_HEIGHT),
                faint(palette.text, 0.06),
            );
        }

        // Diverging bar.
        let frac = if self.max_magnitude > 0.0 {
            (row.magnitude / self.max_magnitude) as f32
        } else {
            0.0
        };
        let bar_len =
            (frac * layout.half_width).max(if row.magnitude > 0.0 { MIN_BAR_PX } else { 0.0 });
        let bar_h = ROW_HEIGHT - 8.0;
        let bar_y = center_y - bar_h / 2.0;
        if row.more_long {
            frame.fill_rectangle(
                Point::new(layout.center_x, bar_y),
                Size::new(bar_len, bar_h),
                faint(color, 0.85),
            );
        } else {
            frame.fill_rectangle(
                Point::new(layout.center_x - bar_len, bar_y),
                Size::new(bar_len, bar_h),
                faint(color, 0.85),
            );
        }

        // The trader label (and its hover action buttons) live in the widget
        // overlay stacked above the canvas, so it is never drawn here.

        if layout.show_value {
            frame.fill_text(Text {
                content: row.value_text.clone(),
                position: Point::new(layout.plot_right + 6.0, center_y),
                color,
                size: iced::Pixels(11.0),
                align_x: Horizontal::Left.into(),
                align_y: Vertical::Center,
                ..Default::default()
            });
        }

        if layout.show_tag {
            frame.fill_text(Text {
                content: row.kind.label().to_string(),
                position: Point::new(layout.plot_right + 6.0 + VALUE_WIDTH, center_y),
                color: kind_color(row.kind, theme),
                size: iced::Pixels(10.0),
                align_x: Horizontal::Left.into(),
                align_y: Vertical::Center,
                ..Default::default()
            });
        }
    }
}

fn draw_tooltip(
    frame: &mut Frame,
    theme: &Theme,
    bounds: Rectangle,
    layout: &FlowLayout,
    row: &PositioningFlowChartRow,
    index: usize,
    visibility: f32,
) {
    let visibility = visibility.clamp(0.0, 1.0);
    if row.tooltip.is_empty() || visibility <= 0.0 {
        return;
    }
    let line_h = 14.0;
    let pad = 8.0;
    let title_h = 16.0;
    let label_w: f32 = 64.0;
    let value_w: f32 = 96.0;
    let box_w = pad * 2.0 + label_w + value_w;
    let box_h = pad * 2.0 + title_h + row.tooltip.len() as f32 * line_h;

    let row_top = HEADER_HEIGHT + HEADER_GAP + index as f32 * (ROW_HEIGHT + ROW_GAP);
    let mut y = row_top + ROW_HEIGHT + 2.0;
    if y + box_h > bounds.height {
        y = (row_top - box_h - 2.0).max(0.0);
    }
    y = (y + (1.0 - visibility) * TOOLTIP_ANIMATION_OFFSET_PX)
        .clamp(0.0, (bounds.height - box_h).max(0.0));
    let x = (layout.center_x - box_w / 2.0).clamp(2.0, (bounds.width - box_w - 2.0).max(2.0));

    frame.fill_rectangle(
        Point::new(x, y),
        Size::new(box_w, box_h),
        scale_alpha(theme.extended_palette().background.strong.color, visibility),
    );
    let border = Path::rectangle(Point::new(x, y), Size::new(box_w, box_h));
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(scale_alpha(
                theme.extended_palette().background.weak.color,
                visibility,
            ))
            .with_width(1.0),
    );

    frame.fill_text(Text {
        content: row.label.clone(),
        position: Point::new(x + pad, y + pad),
        color: scale_alpha(theme.palette().text, visibility),
        size: iced::Pixels(11.0),
        align_x: Horizontal::Left.into(),
        align_y: Vertical::Top,
        ..Default::default()
    });

    let mut line_y = y + pad + title_h;
    for (label, value) in &row.tooltip {
        frame.fill_text(Text {
            content: label.clone(),
            position: Point::new(x + pad, line_y),
            color: scale_alpha(theme.extended_palette().background.weak.text, visibility),
            size: iced::Pixels(10.0),
            align_x: Horizontal::Left.into(),
            align_y: Vertical::Top,
            ..Default::default()
        });
        frame.fill_text(Text {
            content: value.clone(),
            position: Point::new(x + box_w - pad, line_y),
            color: scale_alpha(theme.palette().text, visibility),
            size: iced::Pixels(10.0),
            align_x: Horizontal::Right.into(),
            align_y: Vertical::Top,
            ..Default::default()
        });
        line_y += line_h;
    }
}

// ---------------------------------------------------------------------------
// Colors & helpers
// ---------------------------------------------------------------------------

fn faint(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn scale_alpha(color: Color, scale: f32) -> Color {
    Color {
        a: color.a * scale.clamp(0.0, 1.0),
        ..color
    }
}

fn axis_color(theme: &Theme) -> Color {
    faint(theme.extended_palette().background.weak.text, 0.35)
}

fn kind_color(kind: PositioningFlowKind, theme: &Theme) -> Color {
    match kind {
        PositioningFlowKind::Flip => theme.palette().primary,
        PositioningFlowKind::Add | PositioningFlowKind::Cut => {
            theme.extended_palette().background.weak.text
        }
    }
}

fn build_chart_row(
    row: &PositioningFlowRow,
    usd_scaled: bool,
    denomination: &DisplayDenominationContext,
) -> PositioningFlowChartRow {
    let more_long = row.delta_size >= 0.0;
    let value_text = if usd_scaled {
        match row.delta_usd {
            Some(usd) => signed_compact_usd(usd, denomination),
            None => signed_compact_size(row.delta_size),
        }
    } else {
        signed_compact_size(row.delta_size)
    };

    let mut tooltip: Vec<(String, String)> = Vec::with_capacity(5);
    tooltip.push(("Kind".to_string(), row.kind.label().to_string()));
    tooltip.push((
        "Prev".to_string(),
        row.previous_size
            .map(signed_size)
            .unwrap_or_else(|| "-".to_string()),
    ));
    tooltip.push(("Now".to_string(), signed_size(row.current_size)));
    tooltip.push(("Change".to_string(), signed_size(row.delta_size)));
    if usd_scaled {
        if let Some(usd) = row.delta_usd {
            tooltip.push((
                "Change $".to_string(),
                denomination.format_signed_value(usd, 0),
            ));
        }
        if let Some(usd) = row.current_usd {
            tooltip.push((
                "Now $".to_string(),
                denomination.format_signed_value(usd, 0),
            ));
        }
    }

    PositioningFlowChartRow {
        address: row.address.clone(),
        label: row.address.clone(),
        hover_key: String::new(),
        value_text,
        magnitude: row.magnitude(),
        more_long,
        kind: row.kind,
        tooltip,
    }
}

fn signed_size(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let sign = if value > 0.0 {
        "+"
    } else if value < 0.0 {
        "-"
    } else {
        ""
    };
    format!("{sign}{}", crate::helpers::format_size(value.abs()))
}

fn signed_compact_size(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    let sign = if value > 0.0 {
        "+"
    } else if value < 0.0 {
        "-"
    } else {
        ""
    };
    format!("{sign}{}", compact_number(value.abs()))
}

fn compact_size(value: f64) -> String {
    if !value.is_finite() {
        return "-".to_string();
    }
    compact_number(value.abs())
}

fn signed_compact_usd(value: f64, denomination: &DisplayDenominationContext) -> String {
    let Some(display) = denomination.convert_usd_value(value) else {
        return "-".to_string();
    };
    let sign = if display > 0.0 {
        "+"
    } else if display < 0.0 {
        "-"
    } else {
        ""
    };
    denomination.format_active_amount(sign, compact_number(display.abs()))
}

fn compact_usd(value: f64, denomination: &DisplayDenominationContext) -> String {
    let Some(display) = denomination.convert_usd_value(value) else {
        return "-".to_string();
    };
    denomination.format_active_amount("", compact_number(display.abs()))
}

fn compact_number(value: f64) -> String {
    let value = value.abs();
    if value >= 1_000_000_000.0 {
        trim_zeros(format!("{:.1}", value / 1_000_000_000.0)) + "b"
    } else if value >= 1_000_000.0 {
        trim_zeros(format!("{:.1}", value / 1_000_000.0)) + "m"
    } else if value >= 1_000.0 {
        trim_zeros(format!("{:.1}", value / 1_000.0)) + "k"
    } else {
        trim_zeros(format!("{value:.2}"))
    }
}

fn trim_zeros(value: String) -> String {
    if let Some((whole, frac)) = value.split_once('.') {
        let frac = frac.trim_end_matches('0');
        if frac.is_empty() {
            whole.to_string()
        } else {
            format!("{whole}.{frac}")
        }
    } else {
        value
    }
}
