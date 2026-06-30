use crate::app_state::TradingTerminal;
use crate::config;
use crate::pane_state::PaneKind;
use crate::x_feed::{XFeedInstance, XFeedSource};

impl TradingTerminal {
    pub(super) fn restore_layout_x_feeds(&mut self, layout: &config::SavedLayout) {
        self.x_feed.instances = layout
            .x_feeds
            .iter()
            .map(|config| {
                (
                    config.id,
                    XFeedInstance::new(config.id, config.source.clone()),
                )
            })
            .collect();

        let missing_ids = self
            .panes
            .iter()
            .filter_map(|(_, kind)| match kind {
                PaneKind::XFeed(id) if !self.x_feed.instances.contains_key(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        for id in missing_ids {
            self.x_feed
                .instances
                .insert(id, XFeedInstance::new(id, XFeedSource::Following));
        }
    }
}
