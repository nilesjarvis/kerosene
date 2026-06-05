use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::positioning_state::{PositioningInfoId, PositioningInfoPage};

use iced::widget::{column, container, responsive, rule, text};
use iced::{Element, Fill};

mod columns;
mod controls;
mod flow;
mod metrics;
mod pages;
mod summary;
mod table;

#[cfg(test)]
use crate::positioning_state::PositioningInfoInstance;
#[cfg(test)]
use columns::*;
#[cfg(test)]
use metrics::*;
#[cfg(test)]
use table::*;

// ---------------------------------------------------------------------------
// Positioning Information View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_positioning_info(&self, id: PositioningInfoId) -> Element<'_, Message> {
        responsive(move |size| self.view_positioning_info_sized(id, size.width)).into()
    }

    fn view_positioning_info_sized(
        &self,
        id: PositioningInfoId,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(instance) = self.positioning_infos.get(&id) else {
            return container(
                text("Positioning Information instance missing")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        };

        let navigation = self.view_positioning_info_navigation(instance);
        let body = match instance.page {
            PositioningInfoPage::Positions => {
                self.view_positioning_info_positions_page(instance, available_width, &theme)
            }
            PositioningInfoPage::Change => self.view_positioning_info_change_page(instance, &theme),
        };

        container(column![navigation, rule::horizontal(1), body])
            .width(Fill)
            .height(Fill)
            .into()
    }
}

#[cfg(test)]
mod tests;
