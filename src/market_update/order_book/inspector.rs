use crate::app_state::TradingTerminal;
use crate::market_state::OrderBookSymbolMode;
use crate::market_state::book_inspector::{BookInspectorState, InspectorEvent};
use crate::message::Message;
use iced::{Size, Task, window};

impl TradingTerminal {
    pub(crate) fn update_book_inspector(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InspectBookBlock { id, selection } => {
                let Some(instance) = self.order_books.get(&id) else {
                    return Task::none();
                };
                if !selection.valid()
                    || !crate::helpers::tick_sizes_match(
                        selection.tick,
                        instance.displayed_tick_size(),
                    )
                {
                    return Task::none();
                }
                let coin = match &instance.mode {
                    OrderBookSymbolMode::Active => self.active_symbol.clone(),
                    OrderBookSymbolMode::Fixed(coin) => coin.clone(),
                };
                if coin.is_empty() || self.symbol_key_is_hidden(&coin) {
                    return Task::none();
                }
                if let Some(state) = &mut self.book_inspector {
                    if state.coin != coin {
                        state.coin = coin;
                        state.restart();
                    }
                    state.select(selection);
                    return window::gain_focus(state.window_id);
                }
                let (id, task) = window::open(window::Settings {
                    size: Size::new(760.0, 460.0),
                    min_size: Some(Size::new(580.0, 280.0)),
                    ..crate::window_chrome::settings(
                        self.custom_window_chrome_active,
                        self.window_background_blur_enabled,
                    )
                });
                self.book_inspector = Some(BookInspectorState::new(id, coin, selection));
                return task.map(Message::WindowOpened);
            }
            Message::BookInspectorEvent(id, run_id, key_generation, event) => {
                if !self.hydromancer_key_generation_is_current(key_generation)
                    || self.hydromancer_api_key.trim().is_empty()
                {
                    return Task::none();
                }
                if self
                    .book_inspector
                    .as_ref()
                    .is_some_and(|state| self.symbol_key_is_hidden(&state.coin))
                {
                    return Task::none();
                }
                let Some(state) = self
                    .book_inspector
                    .as_mut()
                    .filter(|state| state.window_id == id && state.run_id == run_id)
                else {
                    return Task::none();
                };
                match event {
                    InspectorEvent::Page(page) if page.revision == state.selection.revision => {
                        state.status = None;
                        state.page = Some(page);
                    }
                    InspectorEvent::Status(status) => state.status = Some(status),
                    _ => {}
                }
            }
            Message::BookInspectorPage(id, page) => {
                if let Some(state) = self
                    .book_inspector
                    .as_mut()
                    .filter(|state| state.window_id == id)
                {
                    let mut selection = state.selection;
                    selection.page = page;
                    state.select(selection);
                }
            }
            Message::RetryBookInspector(id) => {
                if let Some(state) = self
                    .book_inspector
                    .as_mut()
                    .filter(|state| state.window_id == id)
                {
                    state.restart();
                }
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests;
