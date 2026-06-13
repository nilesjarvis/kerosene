use crate::app_state::TradingTerminal;
use crate::market_state::LiveWatchlistId;
use crate::message::Message;
use iced::widget::{
    Column, Space, column, container, float, responsive, row, rule, scrollable, stack, text,
};
use iced::{Element, Fill, Theme, Vector};

const LIVE_WATCHLIST_SETTINGS_DROPDOWN_OFFSET: f32 = 32.0;

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
        let now_ms = self.status_bar_now_ms;

        let wl = if let Some(w) = self.live_watchlists.get(&id) {
            w
        } else {
            return text("Watchlist instance missing").into();
        };
        let display_columns =
            Self::live_watchlist_columns_for_width(&wl.visible_columns, available_width);

        let settings_open = self.live_watchlist_settings_menu_open == Some(id);
        let search_bar = self.view_live_watchlist_search_bar(id, &wl.search_query);
        let autocomplete = self.view_live_watchlist_autocomplete(id, &wl.search_query);
        let header = self.view_live_watchlist_header(id, wl, &display_columns);
        let top_controls = row![
            search_bar,
            self.view_live_watchlist_settings_button(id, settings_open)
        ]
        .spacing(8)
        .width(Fill)
        .align_y(iced::Alignment::Center);

        let mut list = Column::new().spacing(4);
        for data in &wl.row_cache {
            list =
                list.push(self.view_live_watchlist_row(id, data, &display_columns, now_ms, &theme));
        }

        let mut content = column![top_controls].spacing(8);
        if !settings_open {
            content = content.push(autocomplete);
        }
        if let Some((status, is_error)) = &self.live_watchlist_status {
            content = content.push(text(status).size(10).color(if *is_error {
                theme.palette().danger
            } else {
                theme.extended_palette().background.weak.text
            }));
        }
        let content = content
            .push(rule::horizontal(1))
            .push(header)
            .push(scrollable(list));

        let mut layers: Vec<Element<'_, Message>> = vec![content.into()];
        if settings_open {
            let dropdown_layer = float(
                row![
                    Space::new().width(Fill),
                    self.view_live_watchlist_settings_dropdown(id, &wl.visible_columns),
                ]
                .width(Fill)
                .align_y(iced::Alignment::Center),
            )
            .translate(|_bounds, _viewport| {
                Vector::new(0.0, LIVE_WATCHLIST_SETTINGS_DROPDOWN_OFFSET)
            });

            layers.push(dropdown_layer.into());
        }

        container(stack(layers).width(Fill).height(Fill))
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
