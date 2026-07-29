use crate::app_state::TradingTerminal;
use crate::chart_state::ChartBackfillFetchContext;
use crate::message::Message;
use crate::spaghetti_state::{DetachedSpaghettiWindowState, SpaghettiChartId};

use iced::Task;

impl TradingTerminal {
    pub(super) fn open_detached_spaghetti_window(
        &mut self,
        chart_id: SpaghettiChartId,
    ) -> Task<Message> {
        self.close_chart_header_menus();
        self.add_widget_menu_open = false;

        if !self.spaghetti_charts.contains_key(&chart_id) {
            self.push_toast(
                "Comparison chart window unavailable: chart not found".to_string(),
                true,
            );
            return Task::none();
        }

        let detached_id = self.next_spaghetti_id;
        self.next_spaghetti_id += 1;

        let (symbols_to_fetch, interval, session, session_granularity, detached_instance) = {
            let Some(source) = self.spaghetti_charts.get(&chart_id) else {
                self.push_toast(
                    "Comparison chart window unavailable: chart not found".to_string(),
                    true,
                );
                return Task::none();
            };
            let symbols: Vec<String> = source
                .canvas
                .series
                .iter()
                .filter(|s| !s.loaded && !s.symbol.is_empty())
                .map(|s| s.symbol.clone())
                .collect();
            (
                symbols,
                source.interval,
                source.canvas.active_session,
                source.session_granularity,
                source.clone_for_detached_window(detached_id),
            )
        };

        let state = DetachedSpaghettiWindowState::new(detached_id);
        let settings = iced::window::Settings {
            size: state.size(),
            position: state.position(),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (window_id, task) = iced::window::open(settings);
        let chart_backfill_source = self.chart_backfill_source;
        let read_data_provider_generation = self.read_data_provider_generation;
        let hydromancer_generation = self.hydromancer_key_generation;
        let hydromancer_api_key = self.hydromancer_api_key_for_task();
        let instance_epoch = self.spaghetti_instance_epoch;

        self.spaghetti_charts.insert(detached_id, detached_instance);
        self.detached_spaghetti_windows.insert(window_id, state);
        self.persist_config();

        let mut tasks = vec![task.map(Message::WindowOpened)];
        for symbol in &symbols_to_fetch {
            tasks.push(Self::fetch_spaghetti_candles(
                detached_id,
                instance_epoch,
                symbol,
                interval,
                session,
                session_granularity,
                ChartBackfillFetchContext::new(
                    chart_backfill_source,
                    read_data_provider_generation,
                    hydromancer_generation,
                    hydromancer_api_key.clone(),
                ),
            ));
        }

        Task::batch(tasks)
    }

    pub(crate) fn spaghetti_is_docked(&self, chart_id: SpaghettiChartId) -> bool {
        self.panes
            .iter()
            .any(|(_, kind)| matches!(kind, crate::pane_state::PaneKind::SpaghettiChart(id) if *id == chart_id))
    }

    pub(crate) fn detached_spaghetti_window_for(
        &self,
        chart_id: SpaghettiChartId,
    ) -> Option<iced::window::Id> {
        self.detached_spaghetti_windows
            .iter()
            .find_map(|(window_id, state)| (state.chart_id == chart_id).then_some(*window_id))
    }

    pub(crate) fn remove_detached_spaghetti_window_state(
        &mut self,
        window_id: iced::window::Id,
    ) -> bool {
        let Some(state) = self.detached_spaghetti_windows.remove(&window_id) else {
            return false;
        };
        if !self.spaghetti_is_docked(state.chart_id) {
            self.spaghetti_charts.remove(&state.chart_id);
        }
        true
    }
}
