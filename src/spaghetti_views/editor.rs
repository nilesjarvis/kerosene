mod presets;
mod selected;
mod symbol_row;
mod top_bar;

use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
use crate::market_state::market_browser::{MarketPicker, MarketVenue};
use crate::message::Message;
use crate::spaghetti_state::{SpaghettiChartId, SpaghettiChartInstance};
use iced::widget::{Column, column, container, rule, scrollable, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_spaghetti_editor(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let top_bar = self.view_spaghetti_editor_top_bar(id, inst);
        let venue_switch = self.view_market_venue_switch(MarketPicker::Spaghetti(id));
        if self.market_browser.venue(MarketPicker::Spaghetti(id)) == MarketVenue::Papertrade {
            return column![
                top_bar,
                venue_switch,
                self.view_papertrade_markets(&inst.editor_search_query)
            ]
            .spacing(4)
            .width(Fill)
            .height(Fill)
            .into();
        }

        let link_label = inst
            .watchlist_preset_id
            .and_then(|preset_id| self.watchlist_preset_name(preset_id))
            .map(|name| format!(" · linked to {name}"))
            .unwrap_or_default();
        let current_label = text(format!(
            "{} symbols selected{link_label}",
            inst.canvas.series.len()
        ))
        .size(11)
        .color(theme.extended_palette().background.weak.text);
        let current_chips = self.view_spaghetti_editor_selected_chips(id, inst);

        let query = inst.editor_search_query.to_lowercase();
        let mut filtered: Vec<&ExchangeSymbol> = if query.is_empty() {
            self.exchange_symbols
                .iter()
                .filter(|sym| sym.is_user_selectable_market())
                .filter(|sym| !self.exchange_symbol_is_hidden(sym))
                .collect()
        } else {
            self.exchange_symbols
                .iter()
                .filter(|sym| sym.is_user_selectable_market())
                .filter(|sym| !self.exchange_symbol_is_hidden(sym))
                .filter(|sym| {
                    sym.ticker.to_lowercase().contains(&query)
                        || sym.category.to_lowercase().contains(&query)
                        || sym
                            .display_name
                            .as_ref()
                            .is_some_and(|dn| dn.to_lowercase().contains(&query))
                        || sym.key.to_lowercase().contains(&query)
                })
                .collect()
        };

        let favs = &self.favourite_symbols;
        filtered.sort_by(|a, b| {
            let a_fav = favs.iter().position(|k| k == &a.key);
            let b_fav = favs.iter().position(|k| k == &b.key);
            match (a_fav, b_fav) {
                (Some(ai), Some(bi)) => ai.cmp(&bi),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        let existing: Vec<&str> = inst
            .canvas
            .series
            .iter()
            .map(|s| s.symbol.as_str())
            .collect();
        let sid = id;
        let rows = filtered.iter().fold(Column::new().spacing(2), |col, sym| {
            let is_added = existing.contains(&sym.key.as_str());
            col.push(self.view_spaghetti_editor_symbol_row(sid, sym, is_added, &theme))
        });

        let mut content = column![top_bar, venue_switch].spacing(4);
        if !inst.pair_mode {
            content = content.push(self.view_spaghetti_watchlist_preset_controls(id, inst));
        }
        content = content
            .push(current_label)
            .push(current_chips)
            .push(rule::horizontal(1))
            .push(scrollable(rows));

        container(content).width(Fill).height(Fill).into()
    }
}
