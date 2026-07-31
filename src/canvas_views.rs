use crate::account_views::{AnchoredAccountMenu, AnchoredMenuLayer, MenuAlignment, MenuKind};
use crate::app_state::TradingTerminal;
use crate::canvas_state::{CanvasId, WorkspaceId};
use crate::message::Message;
use iced::widget::{Space, column, container, opaque, row, stack, text};
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
        let toolbar: Element<'_, Message> = container(
            row![
                self.widgets_dropdown_button(workspace),
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
        })
        .into();
        let menu = menu_open.then(|| {
            let can_add_income = self
                .connected_order_account_snapshot()
                .is_some_and(|(_, data)| data.is_portfolio_margin());
            AnchoredMenuLayer {
                kind: MenuKind::AddWidget,
                alignment: MenuAlignment::Start,
                content: opaque(self.view_add_widget_menu_card(&theme, can_add_income, false)),
            }
        });
        let toolbar: Element<'_, Message> = AnchoredAccountMenu::new(toolbar, menu).into();

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
