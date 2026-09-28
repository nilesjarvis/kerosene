use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::pane_state::PaneKind;
use crate::session_data_state::{SessionDataId, SessionDataInstance, SessionDataLookback};
use iced::Task;

mod requests;
mod symbols;

#[cfg(test)]
mod tests;

impl TradingTerminal {
    pub(super) fn update_session_data_market(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AddSessionDataPane => self.add_session_data_pane(),
            Message::SessionDataSearchChanged(id, query) => {
                if let Some(instance) = self.session_data.get_mut(&id) {
                    instance.search_query = query;
                }
                Task::none()
            }
            Message::ToggleSessionDataSymbolPicker(id) => {
                if let Some(instance) = self.session_data.get_mut(&id) {
                    instance.symbol_picker_open = !instance.symbol_picker_open;
                    if instance.symbol_picker_open {
                        instance.search_query.clear();
                    }
                }
                Task::none()
            }
            Message::SessionDataSymbolSelected(id, symbol) => {
                self.select_session_data_symbol(id, symbol)
            }
            Message::SessionDataLookbackChanged(id, lookback) => {
                self.set_session_data_lookback(id, lookback)
            }
            Message::RefreshSessionData(id) => self.request_session_data_refresh(id, true),
            Message::SessionDataCandlesLoaded(request, result) => {
                self.apply_session_data_candles_loaded(request, result)
            }
            _ => Task::none(),
        }
    }

    fn add_session_data_pane(&mut self) -> Task<Message> {
        self.add_widget_menu_open = false;
        let workspace = self.add_widget_workspace;
        let Some(focus) = self.add_target_pane_in(workspace) else {
            self.push_toast(
                "Could not add Session Data: no pane is available".to_string(),
                true,
            );
            return Task::none();
        };

        let id = self.next_session_data_id;
        self.next_session_data_id = self.next_session_data_id.saturating_add(1);
        let symbol = self.visible_session_data_symbol(&self.active_symbol);
        self.session_data.insert(
            id,
            SessionDataInstance::new(id, symbol, SessionDataLookback::default()),
        );

        if self
            .add_pane_to_target(
                workspace,
                self.add_widget_axis(),
                focus,
                PaneKind::SessionData(id),
                "Session Data",
            )
            .is_none()
        {
            self.session_data.remove(&id);
            return Task::none();
        }

        self.request_session_data_refresh(id, true)
    }

    fn set_session_data_lookback(
        &mut self,
        id: SessionDataId,
        lookback: SessionDataLookback,
    ) -> Task<Message> {
        if let Some(instance) = self.session_data.get_mut(&id) {
            if instance.lookback == lookback {
                return Task::none();
            }
            instance.lookback = lookback;
            instance.error = None;
        }
        self.persist_config();
        self.request_session_data_refresh(id, true)
    }
}
