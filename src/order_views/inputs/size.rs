use crate::app_state::TradingTerminal;
use crate::helpers::{self, format_usd};
use crate::message::Message;
use crate::signing::OrderKind;

use iced::widget::canvas as canvas_widget;
use iced::widget::{
    Column, Space, button, canvas, checkbox, container, row, slider, stack, text, text_input,
};
use iced::{Color, Event, Fill, Length, Point, Rectangle, Renderer, Size, Theme, mouse};

const SIZE_PRESET_MARKS: [f32; 4] = [25.0, 50.0, 75.0, 100.0];
const SIZE_PRESET_MARK_WIDTH: f32 = 3.0;
const SIZE_PRESET_MARK_HEIGHT: f32 = 16.0;
const SIZE_PRESET_HIT_WIDTH: f32 = 16.0;
const SIZE_AMOUNT_FIELD_HEIGHT: f32 = 29.0;
const SIZE_SLIDER_HEIGHT: f32 = SIZE_AMOUNT_FIELD_HEIGHT;
const SIZE_SLIDER_HANDLE_WIDTH: u16 = 7;
const SIZE_PERCENT_LABEL_WIDTH: f32 = 38.0;

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
        let parsed_price = if matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc) {
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
        .height(SIZE_SLIDER_HEIGHT)
        .step(1.0)
        .style(size_slider_style);
        let preset_markers = canvas(SizePresetMarks {
            current_pct: self.order_percentage,
        })
        .width(Fill)
        .height(Length::Fixed(SIZE_SLIDER_HEIGHT));
        let size_slider = stack![percent_slider, preset_markers]
            .width(Fill)
            .height(Length::Fixed(SIZE_SLIDER_HEIGHT));

        let slider_label = container(
            text(format!("{:.0}%", self.order_percentage))
                .size(12)
                .color(theme.palette().text)
                .center(),
        )
        .width(Length::Fixed(SIZE_PERCENT_LABEL_WIDTH))
        .height(Length::Fixed(SIZE_SLIDER_HEIGHT))
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 5.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        });
        let slider_row = row![size_slider, Space::new().width(6.0), slider_label]
            .spacing(4)
            .align_y(iced::Alignment::Center);

        form = form.push(size_header).push(qty_input).push(slider_row);

        let limit_selected = matches!(self.order_kind, OrderKind::Limit | OrderKind::LimitIoc);
        let mut options_row = row![].spacing(14).align_y(iced::Alignment::Center);
        let mut has_options = false;

        if !active_is_spot && !active_is_outcome {
            has_options = true;
            options_row = options_row.push(
                checkbox(self.order_reduce_only)
                    .label("Reduce Only")
                    .on_toggle(|_| Message::ToggleReduceOnly)
                    .size(14)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced),
            );
        }
        if limit_selected {
            has_options = true;
            options_row = options_row.push(
                checkbox(self.order_kind == OrderKind::LimitIoc)
                    .label("IOC")
                    .on_toggle(|enabled| {
                        Message::SetOrderKind(if enabled {
                            OrderKind::LimitIoc
                        } else {
                            OrderKind::Limit
                        })
                    })
                    .size(14)
                    .text_size(12)
                    .text_shaping(iced::widget::text::Shaping::Advanced),
            );
        }

        if has_options {
            form = form.push(options_row);
        }

        (form, notional_val)
    }
}

fn size_slider_style(theme: &Theme, status: slider::Status) -> slider::Style {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let mut active = palette.primary;
    active.a = match status {
        slider::Status::Active => 0.45,
        slider::Status::Hovered => 0.55,
        slider::Status::Dragged => 0.68,
    };

    let mut inactive = extended.background.weak.color;
    inactive.a = 0.72;

    let handle_color = match status {
        slider::Status::Active => palette.primary,
        slider::Status::Hovered => extended.primary.strong.color,
        slider::Status::Dragged => extended.primary.weak.color,
    };

    let mut style = slider::default(theme, status);
    style.rail.width = SIZE_SLIDER_HEIGHT;
    style.rail.backgrounds = (active.into(), inactive.into());
    style.rail.border = iced::Border {
        radius: 5.0.into(),
        width: 1.0,
        color: extended.background.strong.color,
    };
    style.handle.shape = slider::HandleShape::Rectangle {
        width: SIZE_SLIDER_HANDLE_WIDTH,
        border_radius: 3.0.into(),
    };
    style.handle.background = handle_color.into();
    style.handle.border_width = 0.0;
    style.handle.border_color = handle_color;
    style
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
struct SizePresetMarks {
    current_pct: f32,
}

impl canvas_widget::Program<Message> for SizePresetMarks {
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
        let position = cursor.position_in(bounds)?;

        size_preset_pct_at_position(bounds, position).map(|pct| {
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
            let center = size_preset_mark_center(bounds, pct);

            if hovered {
                let halo_origin = Point::new(center.x - 6.0, center.y - SIZE_PRESET_MARK_HEIGHT);
                let halo_size = Size::new(12.0, SIZE_PRESET_MARK_HEIGHT * 2.0);
                let halo =
                    canvas_widget::Path::rounded_rectangle(halo_origin, halo_size, 4.0.into());
                let mut halo_color = palette.primary;
                halo_color.a = if selected { 0.16 } else { 0.1 };
                frame.fill(&halo, halo_color);
            }

            let mark_height = if hovered {
                SIZE_PRESET_MARK_HEIGHT + 3.0
            } else {
                SIZE_PRESET_MARK_HEIGHT
            };
            let mark = canvas_widget::Path::rounded_rectangle(
                Point::new(
                    center.x - SIZE_PRESET_MARK_WIDTH / 2.0,
                    center.y - mark_height / 2.0,
                ),
                Size::new(SIZE_PRESET_MARK_WIDTH, mark_height),
                1.5.into(),
            );
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

            frame.fill(&mark, color);
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

fn size_preset_mark_center(bounds: Rectangle, pct: f32) -> Point {
    let handle_width = f32::from(SIZE_SLIDER_HANDLE_WIDTH);
    let rail_width = (bounds.width - handle_width).max(0.0);
    Point::new(
        handle_width / 2.0 + rail_width * pct / 100.0,
        bounds.height / 2.0,
    )
}

fn size_preset_pct_at_position(bounds: Rectangle, position: Point) -> Option<f32> {
    SIZE_PRESET_MARKS.into_iter().find(|pct| {
        let center = size_preset_mark_center(bounds, *pct);
        (position.x - center.x).abs() <= SIZE_PRESET_HIT_WIDTH / 2.0
            && position.y >= 0.0
            && position.y <= bounds.height
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
