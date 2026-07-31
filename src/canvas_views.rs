use crate::app_state::TradingTerminal;
use crate::canvas_state::{CanvasId, WorkspaceId};
use crate::message::Message;
use iced::widget::{Space, button, column, container, opaque, row, stack, text};
use iced::{Element, Fill, Theme};

const CANVAS_TOOLBAR_HEIGHT: f32 = 34.0;

impl TradingTerminal {
    pub(crate) fn view_canvas(&self, id: CanvasId) -> Element<'_, Message> {
        let Some(canvas) = self.canvases.get(&id) else {
            return container(text("Canvas unavailable"))
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .into();
        };
        let workspace = WorkspaceId::Canvas(id);
        let theme = self.theme();
        let menu_open = self.add_widget_menu_open && self.add_widget_workspace == workspace;
        let chevron = if menu_open { "\u{25b4}" } else { "\u{25be}" };
        let toolbar = container(
            row![
                button(
                    row![text("Widgets").size(10), text(chevron).size(12)]
                        .spacing(5)
                        .align_y(iced::Alignment::Center),
                )
                .on_press(Message::ToggleAddWidgetMenu(workspace))
                .padding([3, 8]),
                Space::new().width(Fill),
                text(canvas.label.clone()).size(11),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .height(CANVAS_TOOLBAR_HEIGHT)
        .padding([4, 8])
        .style(|theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.strong.color.into()),
            border: iced::Border {
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
                ..Default::default()
            },
            ..Default::default()
        });

        let mut body = column![toolbar].width(Fill).height(Fill);
        if let Some(placement) = self.view_widget_placement_bar(workspace, &theme) {
            body = body.push(placement);
        }
        body = body.push(
            container(self.view_workspace_pane_grid(workspace))
                .width(Fill)
                .height(Fill)
                .padding([self.outer_widget_border_padding(), 0.0]),
        );

        let mut layers: Vec<Element<'_, Message>> = vec![body.into()];
        if menu_open {
            let can_add_income = self
                .connected_order_account_snapshot()
                .is_some_and(|(_, data)| data.is_portfolio_margin());
            layers.push(
                container(opaque(self.view_add_widget_menu_card(
                    &theme,
                    can_add_income,
                    false,
                )))
                .width(Fill)
                .height(Fill)
                .padding(iced::Padding {
                    top: CANVAS_TOOLBAR_HEIGHT,
                    right: 0.0,
                    bottom: 0.0,
                    left: 8.0,
                })
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(iced::alignment::Vertical::Top)
                .into(),
            );
        }
        if self.last_focused_workspace == workspace
            && let Some(alfred) = self.view_alfred_overlay(&theme)
        {
            layers.push(alfred);
        }

        container(stack(layers))
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            })
            .into()
    }
}
