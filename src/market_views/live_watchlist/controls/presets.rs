use crate::app_state::TradingTerminal;
use crate::market_state::LiveWatchlistId;
use crate::message::Message;

use iced::widget::pick_list;
use iced::{Element, Length};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_preset_picker(
        &self,
        id: LiveWatchlistId,
        preset_id: Option<crate::config::WatchlistPresetId>,
    ) -> Element<'_, Message> {
        let selected = preset_id.and_then(|preset_id| self.watchlist_preset(preset_id));
        pick_list(self.watchlist_presets.as_slice(), selected, move |preset| {
            Message::LiveWatchlistPresetSelected(id, preset.id)
        })
        .placeholder("Select watchlist")
        .padding([5, 8])
        .text_size(11)
        .width(Length::Fixed(150.0))
        .into()
    }
}
