use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId, DetachedChartWindowState};
use crate::message::Message;
use iced::{Task, window};

mod surface_state;

impl TradingTerminal {
    pub(super) fn open_detached_chart_window(&mut self, chart_id: ChartId) -> Task<Message> {
        self.close_chart_header_menus();
        self.add_widget_menu_open = false;

        if !self.charts.contains_key(&chart_id) {
            self.push_toast(
                "Chart window unavailable: chart not found".to_string(),
                true,
            );
            return Task::none();
        }

        let detached_chart_id = self.alloc_chart_id();
        let (detached_symbol, detached_interval, detached_last_time, mut detached_instance) = {
            let Some(source) = self.charts.get(&chart_id) else {
                self.push_toast(
                    "Chart window unavailable: chart not found".to_string(),
                    true,
                );
                return Task::none();
            };
            (
                source.symbol.clone(),
                source.interval,
                source.chart.candles.last().map(|candle| candle.open_time),
                source.clone_for_detached_window(detached_chart_id),
            )
        };

        let state = DetachedChartWindowState::new(detached_chart_id);
        let settings = window::Settings {
            size: state.size(),
            position: state.position(),
            ..crate::window_chrome::settings()
        };
        let (window_id, task) = window::open(settings);
        detached_instance
            .chart
            .set_surface_id(ChartSurfaceId::Detached(window_id));
        self.charts.insert(detached_chart_id, detached_instance);
        self.detached_chart_windows.insert(window_id, state);
        self.persist_config();

        let mut tasks = vec![task.map(Message::WindowOpened)];
        if !detached_symbol.is_empty() {
            tasks.push(self.queue_candle_fetch_for(
                detached_chart_id,
                &detached_symbol,
                detached_interval,
                detached_last_time,
            ));
            tasks.extend(Self::fetch_macro_candles_tasks(
                detached_chart_id,
                &detached_symbol,
            ));
        }

        Task::batch(tasks)
    }
}

#[cfg(test)]
mod tests;
