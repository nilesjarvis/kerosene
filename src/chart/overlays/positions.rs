use super::TradingOverlayContext;
use crate::chart::drawing::{
    AxisBadgeStyle, SegmentedHLineStyle, stroke_hline, stroke_segmented_hline,
};
use crate::chart::model::CandlestickChart;
use crate::chart::order_labels::POSITION_LABEL_HEIGHT;
use crate::chart::price_badges::{
    RIGHT_AXIS_PRIMARY_BADGE_HEIGHT, RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
    RightAxisBadgeConnectorStyle, RightAxisBadgeKind, draw_stacked_right_axis_badge,
    right_axis_line_end_x,
};
use crate::helpers::format_price;
use iced::widget::canvas;
use iced::{Color, Point, Size, alignment};

// ---------------------------------------------------------------------------
// Position Overlays
// ---------------------------------------------------------------------------

const POSITION_ENTRY_LINE_WIDTH: f32 = 2.5;
const POSITION_LABEL_X: f32 = 4.0;
const POSITION_LABEL_TEXT_X: f32 = 10.0;
const POSITION_LABEL_WIDTH: f32 = 52.0;

impl CandlestickChart {
    pub(super) fn draw_active_position_lines<PriceToY, IdxToCx>(
        &self,
        ctx: &mut TradingOverlayContext<'_, PriceToY, IdxToCx>,
    ) where
        PriceToY: Fn(f64) -> f32,
        IdxToCx: Fn(usize) -> f32,
    {
        if !should_draw_position_price_overlays(self.obscure_position_prices) {
            return;
        }

        if let Some(pos_overlay) = &self.active_position
            && ctx.price_range > 0.0
        {
            let entry_y = (ctx.price_to_y)(pos_overlay.entry_px);
            if entry_y >= -10.0 && entry_y <= ctx.price_h + 10.0 {
                let is_long = pos_overlay.szi > 0.0;
                let pos_color = if is_long {
                    Color {
                        a: 0.50,
                        ..ctx.theme.palette().success
                    }
                } else {
                    Color {
                        a: 0.50,
                        ..ctx.theme.palette().danger
                    }
                };
                let pos_color_solid = if is_long {
                    ctx.theme.palette().success
                } else {
                    ctx.theme.palette().danger
                };
                let badge_kind = RightAxisBadgeKind::PositionEntry;
                let line_end_x =
                    right_axis_line_end_x(ctx.right_axis_badges, badge_kind, entry_y, ctx.chart_w);

                stroke_hline(
                    ctx.frame,
                    line_end_x,
                    entry_y,
                    pos_color,
                    POSITION_ENTRY_LINE_WIDTH,
                );
                draw_stacked_right_axis_badge(
                    ctx.frame,
                    ctx.right_axis_badges,
                    badge_kind,
                    ctx.chart_w,
                    entry_y,
                    position_entry_badge_label(pos_overlay.entry_px, self.obscure_position_prices),
                    pos_color_solid,
                    AxisBadgeStyle {
                        char_width: 6.5,
                        padding_width: 8.0,
                        height: RIGHT_AXIS_PRIMARY_BADGE_HEIGHT,
                        text_size: 10.0,
                        text_color: Color::BLACK,
                    },
                    RightAxisBadgeConnectorStyle::Solid {
                        color: pos_color,
                        width: 1.5,
                    },
                );

                let side_label = if is_long { "LONG" } else { "SHORT" };
                let label_origin =
                    Point::new(POSITION_LABEL_X, entry_y - POSITION_LABEL_HEIGHT * 0.5);
                let label_size = Size::new(POSITION_LABEL_WIDTH, POSITION_LABEL_HEIGHT);

                let label_background_path = canvas::Path::rectangle(label_origin, label_size);
                ctx.frame.fill(&label_background_path, pos_color_solid);
                ctx.frame.fill_text(canvas::Text {
                    content: side_label.to_string(),
                    position: Point::new(POSITION_LABEL_TEXT_X, entry_y),
                    color: Color::BLACK,
                    size: iced::Pixels(9.0),
                    align_x: alignment::Horizontal::Left.into(),
                    align_y: alignment::Vertical::Center,
                    font: crate::app_fonts::monospace_font(),
                    ..canvas::Text::default()
                });
            }

            if let Some(liq_px) = pos_overlay.liquidation_px {
                let liq_y = (ctx.price_to_y)(liq_px);
                if liq_y >= -10.0 && liq_y <= ctx.price_h + 10.0 {
                    let liq_color = ctx.theme.palette().primary;
                    let liq_color_dim = Color {
                        a: 0.55,
                        ..ctx.theme.palette().primary
                    };
                    let badge_kind = RightAxisBadgeKind::PositionLiquidation;
                    let line_end_x = right_axis_line_end_x(
                        ctx.right_axis_badges,
                        badge_kind,
                        liq_y,
                        ctx.chart_w,
                    );

                    stroke_segmented_hline(
                        ctx.frame,
                        line_end_x,
                        liq_y,
                        2.0,
                        3.0,
                        liq_color_dim,
                        1.0,
                    );
                    draw_stacked_right_axis_badge(
                        ctx.frame,
                        ctx.right_axis_badges,
                        badge_kind,
                        ctx.chart_w,
                        liq_y,
                        position_liquidation_badge_label(liq_px, self.obscure_position_prices),
                        liq_color,
                        AxisBadgeStyle {
                            char_width: 6.2,
                            padding_width: 9.0,
                            height: RIGHT_AXIS_SECONDARY_BADGE_HEIGHT,
                            text_size: 8.5,
                            text_color: Color::BLACK,
                        },
                        RightAxisBadgeConnectorStyle::Segmented {
                            style: SegmentedHLineStyle {
                                segment_len: 2.0,
                                gap_len: 3.0,
                                offset: 0.0,
                                color: liq_color_dim,
                                width: 1.0,
                            },
                        },
                    );
                }
            }
        }
    }
}

fn should_draw_position_price_overlays(obscure: bool) -> bool {
    !obscure
}

fn position_entry_badge_label(entry_px: f64, obscure: bool) -> String {
    if obscure {
        "ENTRY".to_string()
    } else {
        format_price(entry_px)
    }
}

fn position_liquidation_badge_label(liq_px: f64, obscure: bool) -> String {
    if obscure {
        "LIQ".to_string()
    } else {
        format!("Liq {}", format_price(liq_px))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_price_overlays_hide_when_obscured() {
        assert!(!should_draw_position_price_overlays(true));
        assert!(should_draw_position_price_overlays(false));
    }

    #[test]
    fn position_price_labels_redact_when_obscured() {
        assert_eq!(position_entry_badge_label(12345.67, true), "ENTRY");
        assert_eq!(position_liquidation_badge_label(9800.0, true), "LIQ");
    }

    #[test]
    fn position_price_labels_show_prices_when_not_obscured() {
        assert_eq!(position_entry_badge_label(12345.67, false), "12,345.7");
        assert_eq!(
            position_liquidation_badge_label(9800.0, false),
            "Liq 9,800.0"
        );
    }
}
