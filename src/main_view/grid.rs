mod components;
mod styles;

use crate::app_state::TradingTerminal;
use crate::helpers::pane_title;
use crate::message::Message;
use crate::pane_state::PaneKind;
use components::{pane_close_button, pane_drag_ghost_body};
use iced::widget::{container, pane_grid, row, text};
use iced::{Element, Fill, Theme};
use styles::{
    PANE_BORDER_WIDTH, drag_ghost_title_color, pane_content_style, pane_drag_ghost_style,
    pane_drag_ghost_title_bar_style, pane_title_bar_style, subtle_pane_title_color,
};

// ---------------------------------------------------------------------------
// Pane Chrome
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_main_pane_grid(&self) -> Element<'_, Message> {
        let chart_count = self.charts.len();
        let pane_count = self.panes.iter().count();
        let pane_border_thickness = self.pane_border_thickness;
        let pane_corner_radius = self.pane_corner_radius;

        let pane_grid_widget = pane_grid(&self.panes, |pane, kind, _is_maximized| {
            let title = pane_title(kind);

            if self.dragging_pane == Some(pane) {
                let title_bar = pane_grid::TitleBar::new(
                    text(title)
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .style(|theme: &Theme| iced::widget::text::Style {
                            color: Some(drag_ghost_title_color(theme)),
                        }),
                )
                .padding([3, 6])
                .style(move |theme: &Theme| {
                    pane_drag_ghost_title_bar_style(theme, pane_corner_radius)
                });

                return pane_grid::Content::new(pane_drag_ghost_body())
                    .title_bar(title_bar)
                    .style(move |theme: &Theme| pane_drag_ghost_style(theme, pane_corner_radius));
            }

            let content = self.view_pane_content(pane, kind, chart_count);
            let close_btn = pane_close_button(pane, pane_count, kind.can_be_closed());
            let controls_row = if let PaneKind::Chart(chart_id) = kind {
                row![
                    self.view_chart_add_button(pane),
                    self.view_detach_chart_button(*chart_id),
                    close_btn
                ]
            } else {
                row![close_btn]
            }
            .spacing(4)
            .align_y(iced::Alignment::Center);
            let controls = pane_grid::Controls::new(controls_row);

            let title_text = text(title)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(subtle_pane_title_color(theme)),
                });
            let title_bar = pane_grid::TitleBar::new(title_text)
                .controls(controls)
                .always_show_controls()
                .padding([3, 6])
                .style(move |theme: &Theme| pane_title_bar_style(theme, pane_corner_radius));

            pane_grid::Content::new(content)
                .title_bar(title_bar)
                .style(move |theme: &Theme| pane_content_style(theme, pane_corner_radius))
        })
        .width(Fill)
        .height(Fill)
        .spacing(pane_border_thickness)
        .min_size(self.pane_grid_min_size())
        .on_resize(6, Message::PaneResized)
        .on_drag(Message::PaneDragged)
        .on_click(Message::PaneClicked)
        .style(move |theme: &Theme| {
            let palette = theme.palette();
            pane_grid::Style {
                hovered_region: pane_grid::Highlight {
                    background: palette.primary.into(),
                    border: iced::Border {
                        width: PANE_BORDER_WIDTH,
                        color: palette.primary,
                        radius: pane_corner_radius.into(),
                    },
                },
                picked_split: pane_grid::Line {
                    color: palette.primary,
                    width: pane_border_thickness,
                },
                hovered_split: pane_grid::Line {
                    color: palette.primary,
                    width: pane_border_thickness,
                },
            }
        });

        container(pane_grid_widget).width(Fill).height(Fill).into()
    }
}
