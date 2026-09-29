use super::styles::{
    agent_session_button_style, agent_session_sidebar_style, agent_sidebar_control_button_style,
    with_alpha,
};
use crate::app_fonts;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use iced::widget::{Column, Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Fill, Length};

impl TradingTerminal {
    pub(super) fn view_agent_session_sidebar(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let can_change_session = !self.agent.status.is_busy();
        let (persistence_label, persistence_color) =
            if let Some(error) = &self.agent.persistence_error {
                (helpers::ellipsized_text(error, 28), theme.palette().danger)
            } else if self.agent.persistence_in_flight || self.agent.persistence_dirty {
                (
                    "Saving locally…".to_string(),
                    theme.extended_palette().background.weak.text,
                )
            } else {
                ("Saved locally".to_string(), theme.palette().success)
            };

        if self.agent.sidebar_collapsed {
            let expand_icon = container(text("›").size(20)).center(Length::Fixed(32.0));
            let expand = button(expand_icon)
                .padding(0)
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .on_press(Message::AgentToggleSidebar)
                .style(agent_sidebar_control_button_style);
            let new_session_icon = container(text("+").size(18)).center(Length::Fixed(32.0));
            let new_session = button(new_session_icon)
                .padding(0)
                .width(Length::Fixed(32.0))
                .height(Length::Fixed(32.0))
                .on_press_maybe(can_change_session.then_some(Message::AgentNewChat))
                .style(agent_sidebar_control_button_style);

            return container(
                column![
                    new_session,
                    Space::new().height(Fill),
                    text("●").size(7).color(persistence_color),
                    expand,
                ]
                .spacing(6)
                .align_x(Alignment::Center)
                .height(Fill),
            )
            .width(Length::Fixed(52.0))
            .height(Fill)
            .padding([12, 8])
            .style(agent_session_sidebar_style)
            .into();
        }

        let collapse = button(text("‹").size(20))
            .padding([4, 9])
            .on_press(Message::AgentToggleSidebar)
            .style(agent_sidebar_control_button_style);
        let new_session = button(
            row![text("+").size(17), text("New chat").size(13)]
                .spacing(9)
                .align_y(Alignment::Center),
        )
        .padding([7, 8])
        .width(Fill)
        .on_press_maybe(can_change_session.then_some(Message::AgentNewChat))
        .style(agent_sidebar_control_button_style);

        let mut sessions = Column::new().spacing(1).width(Fill);
        for item in self.agent.session_items() {
            let count = if item.message_count == 0 {
                String::new()
            } else {
                item.message_count.to_string()
            };
            let active = item.active;
            let title_color = if active {
                theme.palette().text
            } else {
                with_alpha(theme.palette().text, 0.78)
            };
            let content = row![
                text(helpers::ellipsized_text(item.title, 25))
                    .size(12)
                    .color(title_color)
                    .width(Fill),
                text(count)
                    .size(9)
                    .font(app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Fill);
            sessions = sessions.push(
                button(content)
                    .padding([7, 8])
                    .width(Fill)
                    .on_press_maybe(
                        (can_change_session && !active)
                            .then_some(Message::AgentSelectSession(item.id)),
                    )
                    .style(move |theme, status| agent_session_button_style(theme, status, active)),
            );
        }

        let sidebar_footer = row![
            text("●").size(7).color(persistence_color),
            text(persistence_label).size(9).color(persistence_color),
            Space::new().width(Fill),
            collapse,
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        container(
            column![
                new_session,
                scrollable(sessions).height(Fill),
                sidebar_footer,
            ]
            .spacing(7)
            .height(Fill),
        )
        .width(Length::Fixed(224.0))
        .height(Fill)
        .padding([12, 8])
        .style(agent_session_sidebar_style)
        .into()
    }
}
