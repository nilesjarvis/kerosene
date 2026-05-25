mod anchored;

use crate::account::AccountData;
use crate::app_state::TradingTerminal;
use crate::message::Message;

use anchored::{AnchoredAccountMenu, AnchoredMenuLayer, MenuAlignment};
use iced::Element;
use iced::widget::opaque;

// ---------------------------------------------------------------------------
// Account Summary Menus
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_account_summary_with_menus<'a>(
        &'a self,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let can_add_income = self
            .account_data
            .as_ref()
            .is_some_and(AccountData::is_portfolio_margin);

        let menu = if self.account_picker_open {
            Some(AnchoredMenuLayer {
                alignment: MenuAlignment::Start,
                content: opaque(self.view_account_picker_dropdown()),
            })
        } else if self.layout_menu_open {
            Some(AnchoredMenuLayer {
                alignment: MenuAlignment::End,
                content: opaque(self.view_layout_switcher_dropdown()),
            })
        } else if self.add_widget_menu_open {
            Some(AnchoredMenuLayer {
                alignment: MenuAlignment::End,
                content: opaque(self.view_add_widget_menu_card(&theme, can_add_income)),
            })
        } else {
            None
        };

        AnchoredAccountMenu::new(content, menu).into()
    }
}
