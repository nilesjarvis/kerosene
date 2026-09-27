use crate::account_state::PositionsSortColumn;
use crate::app_state::TradingTerminal;
use crate::config;
use crate::message::Message;

use iced::Task;

impl TradingTerminal {
    pub(super) fn update_positions_sort(&mut self, col: PositionsSortColumn) -> Task<Message> {
        if self.positions_sort_column == col {
            self.positions_sort_direction = match self.positions_sort_direction {
                config::SortDirection::Ascending => config::SortDirection::Descending,
                config::SortDirection::Descending => config::SortDirection::Ascending,
            };
        } else {
            self.positions_sort_column = col;
            self.positions_sort_direction = col.default_direction();
        }
        Task::none()
    }

    pub(super) fn toggle_hidden_position(&mut self, coin: String) -> Task<Message> {
        let Some(account_key) = self.active_journal_account_key() else {
            return Task::none();
        };

        let now_empty = {
            let hidden = self
                .hidden_positions_by_account
                .entry(account_key.clone())
                .or_default();
            if !hidden.insert(coin.clone()) {
                hidden.remove(&coin);
            }
            hidden.is_empty()
        };
        if now_empty {
            self.hidden_positions_by_account.remove(&account_key);
            self.show_hidden_positions = false;
        }
        if self.close_menu_coin.as_deref() == Some(coin.as_str()) {
            self.close_menu_coin = None;
        }
        if !self.ghost_account_secret_ids.contains(&account_key) {
            self.persist_config();
        }
        Task::none()
    }

    pub(super) fn toggle_show_hidden_positions(&mut self) -> Task<Message> {
        self.show_hidden_positions = !self.show_hidden_positions;
        Task::none()
    }

    fn update_account_label_at_index(&mut self, index: usize, value: String) -> Task<Message> {
        let is_ghost = self.account_index_is_ghost(index);
        if let Some(profile) = self.accounts.get_mut(index) {
            profile.name = value;
            if !is_ghost {
                self.persist_config();
            }
        }
        Task::none()
    }

    pub(super) fn toggle_account_picker(&mut self) -> Task<Message> {
        let opening = !self.account_picker_open;
        if opening {
            self.close_chart_header_menus();
        } else {
            self.account_picker_rename_index = None;
        }
        self.account_picker_open = opening;
        Task::none()
    }

    pub(super) fn select_account_from_picker(&mut self, index: usize) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.switch_account_task(index)
    }

    pub(super) fn toggle_account_picker_rename(&mut self, index: usize) -> Task<Message> {
        if self.accounts.get(index).is_none() || self.account_picker_rename_index == Some(index) {
            self.account_picker_rename_index = None;
        } else {
            self.account_picker_rename_index = Some(index);
        }
        Task::none()
    }

    pub(super) fn update_account_picker_label(
        &mut self,
        index: usize,
        value: String,
    ) -> Task<Message> {
        if self.accounts.get(index).is_none() {
            self.account_picker_rename_index = None;
            return Task::none();
        }
        self.account_picker_rename_index = Some(index);
        self.update_account_label_at_index(index, value)
    }

    pub(super) fn add_ghost_wallet_from_picker(&mut self, address: String) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.ghost_wallet_task(address)
    }

    pub(super) fn forget_ghost_account_from_picker(&mut self, index: usize) -> Task<Message> {
        self.account_picker_open = false;
        self.account_picker_rename_index = None;
        self.forget_ghost_account_task(index)
    }
}
