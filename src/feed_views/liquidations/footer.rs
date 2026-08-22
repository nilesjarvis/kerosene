use crate::app_state::TradingTerminal;
use crate::helpers::section_separator;
use crate::message::Message;

use iced::widget::{Column, container};
use iced::{Element, Fill};

use super::LIQUIDATIONS_CONTENT_HORIZONTAL_PADDING;

mod chart;
mod summary;

impl TradingTerminal {
    pub(crate) fn view_liquidations_bottom_content(&self, now_ms: u64) -> Element<'_, Message> {
        let mut bottom_content = Column::new().spacing(0).width(Fill);

        if self.liquidation_chart_enabled {
            bottom_content =
                bottom_content
                    .push(section_separator())
                    .push(liquidations_footer_section(
                        self.view_liquidations_chart(now_ms),
                    ));
        }

        if self.liquidation_summary_enabled {
            bottom_content =
                bottom_content
                    .push(section_separator())
                    .push(liquidations_footer_section(
                        self.view_liquidations_summary(now_ms),
                    ));
        }

        bottom_content.into()
    }
}

fn liquidations_footer_section<'a>(
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(content)
        .padding(iced::Padding {
            top: 8.0,
            right: LIQUIDATIONS_CONTENT_HORIZONTAL_PADDING,
            bottom: 8.0,
            left: LIQUIDATIONS_CONTENT_HORIZONTAL_PADDING,
        })
        .width(Fill)
        .into()
}
