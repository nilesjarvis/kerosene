use crate::app_state::TradingTerminal;
use crate::market_state::LiveWatchlistId;
use crate::message::Message;
use iced::widget::{Column, column, container, responsive, rule, scrollable, text};
use iced::{Element, Fill, Theme};

mod controls;
mod rows;

// ---------------------------------------------------------------------------
// Live Watchlist View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_live_watchlist(&self, id: LiveWatchlistId) -> Element<'_, Message> {
        responsive(move |size| self.view_live_watchlist_sized(id, size.width)).into()
    }

    fn view_live_watchlist_sized(
        &self,
        id: LiveWatchlistId,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let now_ms = Self::now_ms();

        let wl = if let Some(w) = self.live_watchlists.get(&id) {
            w
        } else {
            return text("Watchlist instance missing").into();
        };
        let display_columns =
            Self::live_watchlist_columns_for_width(&wl.visible_columns, available_width);

        let search_bar = self.view_live_watchlist_search_bar(id, &wl.search_query);
        let autocomplete = self.view_live_watchlist_autocomplete(id, &wl.search_query);
        let header = self.view_live_watchlist_header(id, wl, &display_columns);
        let column_controls = self.view_live_watchlist_column_controls(id, &wl.visible_columns);

        let mut list = Column::new().spacing(4);
        for data in &wl.row_cache {
            list =
                list.push(self.view_live_watchlist_row(id, data, &display_columns, now_ms, &theme));
        }

        let mut content = column![search_bar, column_controls, autocomplete].spacing(8);
        if let Some((status, is_error)) = &self.live_watchlist_status {
            content = content.push(text(status).size(10).color(if *is_error {
                theme.palette().danger
            } else {
                theme.extended_palette().background.weak.text
            }));
        } else if self.live_watchlist_contexts_loading || self.live_watchlist_history_loading {
            content = content.push(
                text("Refreshing market context")
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }
        let content = content
            .push(rule::horizontal(1))
            .push(header)
            .push(scrollable(list));

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .style(|t: &Theme| iced::widget::container::Style {
                background: Some(t.extended_palette().background.base.color.into()),
                ..Default::default()
            })
            .into()
    }
}
