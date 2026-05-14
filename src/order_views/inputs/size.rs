use crate::app_state::TradingTerminal;
use crate::helpers::{self, format_usd};
use crate::message::Message;
use crate::signing::OrderKind;

use iced::widget::{
    Column, Space, button, canvas, row, slider, stack, text, text_input,
};
use iced::widget::canvas as canvas_widget;
use iced::{Color, Event, Fill, Length, Point, Rectangle, Renderer, Theme, mouse};

const SIZE_PRESET_MARKS: [f32; 4] = [25.0, 50.0, 75.0, 100.0];
const SIZE_PRESET_DOT_SIZE: f32 = 7.0;
const SIZE_PRESET_HIT_RADIUS: f32 = 8.0;
const SIZE_SLIDER_HEIGHT: f32 = 16.0;
const SIZE_SLIDER_HANDLE_RADIUS: f32 = 7.0;

impl TradingTerminal {
    pub(super) fn push_size_input_controls<'a>(
        &'a self,
        mut form: Column<'a, Message>,
        active_is_spot: bool,
        active_is_outcome: bool,
    ) -> (Column<'a, Message>, Option<f64>) {
        let theme = self.theme();
        let qty_placeholder = if active_is_outcome {
            "Contracts"
        } else {
            "Quantity"
        };
        let qty_input = text_input(qty_placeholder, &self.order_quantity)
            .style(helpers::text_input_style)
            .on_input(Message::OrderQuantityChanged)
            .size(13)
            .padding(6);

        let parsed_qty = parse_positive_finite(&self.order_quantity);
        let parsed_price =
            if self.order_kind == OrderKind::Limit || self.order_kind == OrderKind::Chase {
                parse_positive_finite(&self.order_price)
            } else {
                self.resolve_mid_for_symbol(&self.active_symbol)
                    .filter(|price| price.is_finite() && *price > 0.0)
            };

        let (notional_val, notional_text) = order_notional_text(
            self.order_quantity_is_usd,
            &self.active_symbol,
            parsed_qty,
            parsed_price,
        );
        let size_header = row![
            text("Size")
                .size(12)
                .color(theme.extended_palette().background.weak.text),
            Space::new().width(6.0),
            denomination_button(denomination_label(
                self.order_quantity_is_usd,
                active_is_outcome
            )),
            Space::new().width(Fill),
            text(notional_text)
                .size(11)
                .color(theme.extended_palette().background.weak.text),
        ]
        .align_y(iced::Alignment::Center);

        let percent_slider = slider(
            0.0..=100.0,
            self.order_percentage,
            Message::OrderPercentageChanged,
        )
            .width(Fill)
            .step(1.0)
            .style(|theme: &Theme, status| {
                let palette = theme.palette();
                let mut style = slider::default(theme, status);
                style.handle.background = palette.primary.into();
                style.handle.border_color = palette.primary;
                style.rail.backgrounds.0 = palette.primary.into();
                style.rail.backgrounds.1 = Color {
                    a: 0.2,
                    ..palette.text
                }
                .into();
                style
            });
        let preset_markers = canvas(SizePresetDots {
            current_pct: self.order_percentage,
        })
        .width(Fill)
        .height(Length::Fixed(SIZE_SLIDER_HEIGHT));
        let size_slider = stack![percent_slider, preset_markers].width(Fill);

        let slider_label = text(format!("{:.0}%", self.order_percentage))
            .size(10)
            .color(theme.extended_palette().background.weak.text);
        let slider_row = row![size_slider, Space::new().width(6.0), slider_label]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        form = form.push(size_header).push(qty_input).push(slider_row);
        if !active_is_spot && !active_is_outcome {
            form = form.push(
                iced::widget::checkbox(self.order_reduce_only)
                    .label("Reduce Only")
                    .on_toggle(|_| Message::ToggleReduceOnly)
                    .size(14)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced),
            );
        }

        (form, notional_val)
    }
}

fn parse_positive_finite(value: &str) -> Option<f64> {
    let parsed = helpers::parse_number(value)?;
    (parsed.is_finite() && parsed > 0.0).then_some(parsed)
}

fn order_notional_text(
    quantity_is_usd: bool,
    active_symbol: &str,
    parsed_qty: Option<f64>,
    parsed_price: Option<f64>,
) -> (Option<f64>, String) {
    let Some(parsed_qty) = parsed_qty else {
        return (None, String::new());
    };

    if quantity_is_usd {
        let coin_text = parsed_price
            .and_then(|price| {
                let coin_val = parsed_qty / price;
                (coin_val.is_finite() && coin_val > 0.0).then_some(coin_val)
            })
            .map(|coin_val| {
                let mut search_coin = active_symbol;
                if let Some((_, suffix)) = search_coin.split_once(':') {
                    search_coin = suffix;
                }
                format!("\u{2248} {coin_val:.4} {search_coin}")
            })
            .unwrap_or_default();
        (Some(parsed_qty), coin_text)
    } else {
        let Some(parsed_price) = parsed_price else {
            return (None, String::new());
        };
        let notional = parsed_qty * parsed_price;
        if !notional.is_finite() || notional <= 0.0 {
            return (None, String::new());
        }
        (
            Some(notional),
            format!("\u{2248} {}", format_usd(&format!("{notional:.2}"))),
        )
    }
}

fn denomination_label(order_quantity_is_usd: bool, active_is_outcome: bool) -> &'static str {
    if active_is_outcome {
        if order_quantity_is_usd {
            "USDH"
        } else {
            "CONTRACTS"
        }
    } else if order_quantity_is_usd {
        "USD"
    } else {
        "COIN"
    }
}

fn denomination_button<'a>(label: &'static str) -> button::Button<'a, Message> {
    button(text(label).size(10).center())
        .on_press(Message::ToggleOrderDenomination)
        .padding([2, 6])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
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
}

#[derive(Debug, Clone, Copy)]
struct SizePresetDots {
    current_pct: f32,
}

impl canvas_widget::Program<Message> for SizePresetDots {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas_widget::Action<Message>> {
        let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event else {
            return None;
        };
        let Some(position) = cursor.position_in(bounds) else {
            return None;
        };

        size_preset_pct_at_position(bounds, position)
            .map(|pct| {
                canvas_widget::Action::publish(Message::OrderPercentageChanged(pct)).and_capture()
            })
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas_widget::Geometry> {
        let mut frame = canvas_widget::Frame::new(renderer, bounds.size());
        let palette = theme.palette();
        let hovered_pct = cursor
            .position_in(bounds)
            .and_then(|position| size_preset_pct_at_position(bounds, position));

        for pct in SIZE_PRESET_MARKS {
            let selected = (self.current_pct - pct).abs() < 0.5;
            let hovered = hovered_pct.is_some_and(|hovered_pct| hovered_pct == pct);
            let center = size_preset_dot_center(bounds, pct);

            if hovered {
                let mut halo_color = palette.primary;
                halo_color.a = if selected { 0.18 } else { 0.12 };
                frame.fill(
                    &canvas_widget::Path::circle(center, SIZE_PRESET_HIT_RADIUS - 1.5),
                    halo_color,
                );

                let ring = canvas_widget::Path::circle(center, SIZE_PRESET_HIT_RADIUS - 2.0);
                let mut ring_color = palette.primary;
                ring_color.a = if selected { 0.55 } else { 0.38 };
                frame.stroke(
                    &ring,
                    canvas_widget::Stroke::default()
                        .with_width(1.0)
                        .with_color(ring_color),
                );
            }

            let dot_radius = if hovered {
                SIZE_PRESET_DOT_SIZE / 2.0 + 1.0
            } else {
                SIZE_PRESET_DOT_SIZE / 2.0
            };
            let dot = canvas_widget::Path::circle(center, dot_radius);
            let mut color = if selected || hovered {
                palette.primary
            } else {
                Color {
                    a: 0.45,
                    ..palette.text
                }
            };
            if hovered && !selected {
                color.a = 0.82;
            }

            frame.fill(&dot, color);
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor
            .position_in(bounds)
            .and_then(|position| size_preset_pct_at_position(bounds, position))
            .is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn size_preset_dot_center(bounds: Rectangle, pct: f32) -> Point {
    let rail_width = (bounds.width - SIZE_SLIDER_HANDLE_RADIUS * 2.0).max(0.0);
    Point::new(
        SIZE_SLIDER_HANDLE_RADIUS + rail_width * pct / 100.0,
        bounds.height / 2.0,
    )
}

fn size_preset_pct_at_position(bounds: Rectangle, position: Point) -> Option<f32> {
    SIZE_PRESET_MARKS.into_iter().find(|pct| {
        position.distance(size_preset_dot_center(bounds, *pct)) <= SIZE_PRESET_HIT_RADIUS
    })
}

#[cfg(test)]
mod tests {
    use super::{order_notional_text, parse_positive_finite};

    #[test]
    fn size_input_parser_rejects_invalid_nonpositive_or_nonfinite_values() {
        assert_eq!(parse_positive_finite("12.5"), Some(12.5));
        assert_eq!(parse_positive_finite("0"), None);
        assert_eq!(parse_positive_finite("-1"), None);
        assert_eq!(parse_positive_finite("NaN"), None);
        assert_eq!(parse_positive_finite("bad"), None);
    }

    #[test]
    fn usd_quantity_keeps_known_notional_when_price_is_missing() {
        assert_eq!(
            order_notional_text(true, "BTC", Some(100.0), None),
            (Some(100.0), String::new())
        );
    }

    #[test]
    fn coin_quantity_requires_valid_reference_price_for_notional() {
        assert_eq!(
            order_notional_text(false, "BTC", Some(2.0), None),
            (None, String::new())
        );
        assert_eq!(
            order_notional_text(false, "BTC", Some(2.0), Some(125.0)),
            (Some(250.0), "\u{2248} $250.00".to_string())
        );
    }
}
