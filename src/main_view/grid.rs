use crate::app_state::TradingTerminal;
use crate::helpers::pane_title;
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, container, pane_grid, row, text};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(super) fn view_main_pane_grid(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let focus = self.focus;
        let chart_count = self.charts.len();
        let pane_count = self.panes.iter().count();

        let pane_grid_widget = pane_grid(&self.panes, |pane, kind, _is_maximized| {
            let is_focused = focus == Some(pane);
            let content = self.view_pane_content(pane, kind, chart_count);
            let close_btn = pane_close_button(pane, pane_count);
            let controls = pane_grid::Controls::new(row![close_btn]);

            let title_theme = theme.clone();
            let title_bar = pane_grid::TitleBar::new(
                text(pane_title(kind)).size(13).font(iced::Font::MONOSPACE),
            )
            .controls(controls)
            .always_show_controls()
            .padding(4)
            .style(move |_theme: &Theme| container_style::Style {
                background: Some(
                    if is_focused {
                        title_theme.extended_palette().background.weak.color
                    } else {
                        title_theme.extended_palette().background.base.color
                    }
                    .into(),
                ),
                ..Default::default()
            });

            let content_theme = theme.clone();
            pane_grid::Content::new(content)
                .title_bar(title_bar)
                .style(move |_theme: &Theme| container_style::Style {
                    background: Some(
                        content_theme
                            .extended_palette()
                            .background
                            .strong
                            .color
                            .into(),
                    ),
                    ..Default::default()
                })
        })
        .width(Fill)
        .height(Fill)
        .spacing(1)
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

fn pane_close_button(pane: pane_grid::Pane, pane_count: usize) -> button::Button<'static, Message> {
    if pane_count > 1 {
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
