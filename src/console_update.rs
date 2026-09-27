use crate::app_state::TradingTerminal;
use crate::console_state::{PAGE_SIZE, scroll_id};
use crate::message::Message;
use iced::{Size, Task, widget::scrollable, window};

impl TradingTerminal {
    pub(crate) fn update_console(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenConsoleWindow => {
                self.add_widget_menu_open = false;
                self.layout_menu_open = false;
                self.account_picker_open = false;
                self.account_picker_rename_index = None;
                if let Some(id) = self.console.window_id {
                    return window::gain_focus(id);
                }
                self.console.paused = false;
                self.console.page = 0;
                self.console.refresh();
                let (id, task) = window::open(window::Settings {
                    size: Size::new(1120.0, 680.0),
                    min_size: Some(Size::new(820.0, 380.0)),
                    ..crate::window_chrome::settings(
                        self.custom_window_chrome_active,
                        self.window_background_blur_enabled,
                    )
                });
                self.console.window_id = Some(id);
                return task.map(Message::WindowOpened);
            }
            Message::ConsoleTick => {
                if self.console.window_id.is_some() {
                    self.console.refresh();
                }
            }
            Message::ConsoleTogglePause => {
                self.console.paused = !self.console.paused;
                if !self.console.paused {
                    self.console.page = 0;
                    self.console.refresh();
                    return iced::widget::operation::snap_to(
                        scroll_id(),
                        scrollable::RelativeOffset::START,
                    );
                }
            }
            Message::ConsoleProviderChanged(provider) => {
                self.console.provider = provider;
                self.console.page = 0;
                return iced::widget::operation::snap_to(
                    scroll_id(),
                    scrollable::RelativeOffset::START,
                );
            }
            Message::ConsoleFilterChanged(filter) => {
                self.console.filter = filter;
                self.console.page = 0;
                return iced::widget::operation::snap_to(
                    scroll_id(),
                    scrollable::RelativeOffset::START,
                );
            }
            Message::ConsolePageChanged(page) => {
                let last_page = self.console.entries().count().saturating_sub(1) / PAGE_SIZE;
                self.console.page = page.min(last_page);
                self.console.paused = true;
                return iced::widget::operation::snap_to(
                    scroll_id(),
                    scrollable::RelativeOffset::START,
                );
            }
            Message::ConsoleScrolled(viewport) => {
                if viewport.absolute_offset().y > 0.0 {
                    self.console.paused = true;
                }
            }
            Message::ConsoleClear => {
                // Clear displayed history, without changing traffic counters or other consumers.
                self.console.snapshot = crate::network_activity::snapshot();
                self.console.cleared_through = self.console.snapshot.sequence;
                self.console.snapshot.entries.clear();
                self.console.page = 0;
                return iced::widget::operation::snap_to(
                    scroll_id(),
                    scrollable::RelativeOffset::START,
                );
            }
            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_opens_once_closes_and_reopens_without_affecting_the_main_window() {
        let (mut terminal, _) = TradingTerminal::boot();
        let main = terminal.main_window_id;
        let _ = terminal.update_console(Message::OpenConsoleWindow);
        let id = terminal
            .console
            .window_id
            .expect("console window allocated");
        assert_eq!(terminal.window_title(id), "Kerosene Console");
        let _ = terminal.update_console(Message::OpenConsoleWindow);
        assert_eq!(terminal.console.window_id, Some(id));
        let _ = terminal.update_window(Message::WindowClosed(id));
        assert_eq!(terminal.console.window_id, None);
        assert_eq!(terminal.main_window_id, main);
        let _ = terminal.update_console(Message::OpenConsoleWindow);
        assert!(
            terminal
                .console
                .window_id
                .is_some_and(|new_id| new_id != id)
        );
    }
}
