mod editor;
mod indicator_badges;
mod header;
mod indicator_menu;
mod toolbar;

use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::ChartId;
use crate::message::Message;
use iced::widget::{button, canvas, column, container, rule, stack, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_chart(
        &self,
        chart_id: ChartId,
        _chart_count: usize,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(instance) = self.charts.get(&chart_id) else {
            return container(
                text("Chart not found")
                    .size(14)
                    .color(theme.palette().danger),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .into();
        };

        // ----- Chart editor overlay (symbol selection) -----
        if instance.editor_open {
            return self.view_chart_editor(chart_id, instance);
        }

        // ----- Empty chart (no symbol selected yet) -----
        if instance.symbol.is_empty() {
            let open_editor_btn = button(
                text("Select Symbol")
                    .size(14)
                    .center()
                    .width(Fill)
                    .color(theme.palette().text),
            )
            .on_press(Message::ChartOpenEditor(chart_id))
            .padding([8, 16])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
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
            });

            return container(open_editor_btn)
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(10)
                .into();
        }

        if self.symbol_key_is_hidden(&instance.symbol) {
            let content = column![
                text("Muted ticker")
                    .size(13)
                    .color(theme.extended_palette().background.weak.text),
                button(text("Select Symbol").size(12))
                    .on_press(Message::ChartOpenEditor(chart_id))
                    .padding([6, 12]),
            ]
            .spacing(10)
            .align_x(iced::Alignment::Center);
            return container(content)
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(10)
                .into();
        }

        // The toolbar elements are now integrated directly into the header row.

        // Determine status message overlay (if any)
        let status_overlay: Option<Element<'_, Message>> = match &instance.chart.status {
            ChartStatus::Loading if instance.chart.candles.is_empty() => {
                Some(self.loading_overlay("Loading chart data..."))
            }
            ChartStatus::Error(err) if instance.chart.candles.is_empty() => Some(
                container(
                    text(format!("Error: {err}"))
                        .size(14)
                        .color(theme.palette().danger),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .into(),
            ),
            _ => None,
        };

        // Build header + timeframe row + canvas area.
        // Always use the same 4-element column structure so the canvas
        // stays at the same widget tree position (preserving ChartState).
        {
            let header = self.view_chart_header(chart_id, instance);
            let toolbar = self.view_chart_toolbar(chart_id, instance);

            let chart_canvas: Element<'_, Message> =
                canvas(&instance.chart).width(Fill).height(Fill).into();
            let mut canvas_layers = vec![chart_canvas];
            if let Some(indicator_badges) = self.view_chart_indicator_badges(chart_id, instance) {
                canvas_layers.push(indicator_badges);
            }
            let chart_surface: Element<'_, Message> =
                stack(canvas_layers).width(Fill).height(Fill).into();

            // Always wrap the canvas in a stack to keep the widget tree
            // structure stable.
            let chart_area: Element<'_, Message> = if let Some(form) = &instance.quick_order {
                let cid = chart_id;
                self.view_quick_order_card(cid, form, chart_surface)
            } else if let Some(overlay) = status_overlay {
                stack![chart_surface, overlay]
                    .width(Fill)
                    .height(Fill)
                    .into()
            } else {
                stack![chart_surface].width(Fill).height(Fill).into()
            };
            let chart_area = container(chart_area)
                .id(Self::chart_screenshot_canvas_id(chart_id))
                .width(Fill)
                .height(Fill);

            let padded_header = container(header).width(Fill).padding([0, 4]);
            let padded_chart_area = container(chart_area)
                .width(Fill)
                .height(Fill)
                .padding([0, 4]);

            let content = column![
                padded_header,
                chart_header_separator(),
                toolbar,
                chart_header_separator(),
                padded_chart_area
            ]
            .spacing(0)
            .width(Fill)
            .height(Fill);

            let mut chart_layers: Vec<Element<'_, Message>> = vec![content.into()];

            if instance.macro_menu_open {
                chart_layers.push(self.view_macro_indicator_menu(chart_id, instance));
            }
            if self.chart_screenshot_menu_open == Some(chart_id) {
                chart_layers.push(self.view_chart_screenshot_menu(chart_id));
            }

            container(stack(chart_layers))
                .width(Fill)
                .height(Fill)
                .padding([4, 0])
                .into()
        }
    }
}

fn chart_header_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.10,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}
