mod selected;
mod symbol_row;
mod top_bar;

use crate::api::ExchangeSymbol;
use crate::app_state::TradingTerminal;
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

        let current_label = text(format!("{} symbols selected", inst.canvas.series.len()))
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

        let content = column![
            top_bar,
            current_label,
            current_chips,
            rule::horizontal(1),
            scrollable(rows),
        ]
        .spacing(4);

        container(content).width(Fill).height(Fill).into()
    }
}
