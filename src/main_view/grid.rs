use crate::app_state::TradingTerminal;
use crate::helpers::pane_title;
use crate::message::Message;
use crate::pane_state::{MAIN_PANE_GRID_SPACING, PaneKind};
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, pane_grid, row, text};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_main_pane_grid(&self) -> Element<'_, Message> {
        let chart_count = self.charts.len();
        let pane_count = self.panes.iter().count();

        let pane_grid_widget = pane_grid(&self.panes, |pane, kind, _is_maximized| {
            let title = pane_title(kind);

            if self.dragging_pane == Some(pane) {
                let title_bar = pane_grid::TitleBar::new(
                    text(title)
                        .size(11)
                        .font(iced::Font::MONOSPACE)
                        .style(|theme: &Theme| iced::widget::text::Style {
                            color: Some(drag_ghost_title_color(theme)),
                        }),
                )
                .padding([3, 6])
                .style(pane_drag_ghost_title_bar_style);

                return pane_grid::Content::new(pane_drag_ghost_body())
                    .title_bar(title_bar)
                    .style(pane_drag_ghost_style);
            }

            let content = self.view_pane_content(pane, kind, chart_count);
            let close_btn = pane_close_button(pane, pane_count, kind.can_be_closed());
            let controls_row = if matches!(kind, PaneKind::Chart(_)) {
                row![self.view_chart_add_button(pane), close_btn]
            } else {
                row![close_btn]
            }
            .spacing(4)
            .align_y(iced::Alignment::Center);
            let controls = pane_grid::Controls::new(controls_row);

            let title_text =
                text(title)
                    .size(11)
                    .font(iced::Font::MONOSPACE)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(subtle_pane_title_color(theme)),
                    });
            let title_bar = pane_grid::TitleBar::new(title_text)
                .controls(controls)
                .always_show_controls()
                .padding([3, 6])
                .style(pane_title_bar_style);

            pane_grid::Content::new(content)
                .title_bar(title_bar)
                .style(|theme: &Theme| container_style::Style {
                    background: Some(theme.extended_palette().background.strong.color.into()),
                    ..Default::default()
                })
        })
        .width(Fill)
        .height(Fill)
        .spacing(MAIN_PANE_GRID_SPACING)
        .min_size(self.account_summary_pane_min_size())
        .on_resize(6, Message::PaneResized)
        .on_drag(Message::PaneDragged)
        .on_click(Message::PaneClicked)
        .style(|theme: &Theme| {
            let palette = theme.palette();
            pane_grid::Style {
                hovered_region: pane_grid::Highlight {
                    background: palette.primary.into(),
                    border: iced::Border {
                        width: 1.0,
                        color: palette.primary,
                        radius: 0.0.into(),
                    },
                },
                picked_split: pane_grid::Line {
                    color: palette.primary,
                    width: 2.0,
                },
                hovered_split: pane_grid::Line {
                    color: palette.primary,
                    width: 2.0,
                },
            }
        });

        container(pane_grid_widget).width(Fill).height(Fill).into()
    }
}

fn pane_drag_ghost_body() -> Element<'static, Message> {
    container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .into()
}

fn pane_drag_ghost_style(theme: &Theme) -> container_style::Style {
    let mut background = theme.palette().primary;
    background.a = 0.12;

    let mut border_color = theme.palette().primary;
    border_color.a = 0.85;

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            width: 1.0,
            color: border_color,
            radius: 2.0.into(),
        },
        ..Default::default()
    }
}

fn pane_drag_ghost_title_bar_style(theme: &Theme) -> container_style::Style {
    let mut background = theme.palette().primary;
    background.a = 0.18;

    let mut border_color = theme.palette().primary;
    border_color.a = 0.35;

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            width: 0.0,
            color: border_color,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn drag_ghost_title_color(theme: &Theme) -> Color {
    let mut color = theme.palette().text;
    color.a = 0.72;
    color
}

fn pane_title_bar_style(theme: &Theme) -> container_style::Style {
    use iced::gradient;

    let background = theme.extended_palette().background.strong.color;
    let mut separator = theme.extended_palette().background.strong.text;
    separator.a = 0.08;

    container_style::Style {
        background: Some(
            gradient::Linear::new(iced::Degrees(180.0))
                .add_stop(0.00, background)
                .add_stop(0.97, background)
                .add_stop(0.985, separator)
                .add_stop(1.00, separator)
                .into(),
        ),
        ..Default::default()
    }
}

fn subtle_pane_title_color(theme: &Theme) -> iced::Color {
    let mut color = theme.extended_palette().background.strong.text;
    color.a = 0.46;
    color
}

fn pane_close_button(
    pane: pane_grid::Pane,
    pane_count: usize,
    can_close_pane: bool,
) -> button::Button<'static, Message> {
    if pane_count > 1 && can_close_pane {
        button(text("x").size(10).center())
            .on_press(Message::ClosePane(pane))
            .padding([2, 5])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        width: 1.0,
                        color: match status {
                            button::Status::Hovered => theme.palette().danger,
                            _ => iced::Color::TRANSPARENT,
                        },
                        radius: 2.0.into(),
                    },
                    ..Default::default()
                }
            })
    } else {
        button(Space::new().width(10.0).height(10.0)).style(|_theme: &Theme, _status| {
            button::Style {
                background: None,
                ..Default::default()
            }
        })
    }
}
