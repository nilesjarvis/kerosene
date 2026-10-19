use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::message::Message;
use iced::{Task, window};

mod surface_state;

impl TradingTerminal {
    pub(super) fn open_detached_chart_for_symbol(&mut self, symbol: String) -> Task<Message> {
        if symbol.is_empty() {
            return Task::none();
        }

        // Select on a fresh instance so chart validation, cached candles, and
        // account overlays follow the usual path without retargeting a workspace.
        let chart_id = self.alloc_chart_id();
        let mut instance = ChartInstance::new_empty(chart_id);
        self.apply_chart_appearance_settings(&mut instance.chart);
        self.charts.insert(chart_id, instance);
        let select_task = self.update_chart(Message::ChartSymbolSelected(chart_id, symbol));
        let Some(instance) = self.charts.remove(&chart_id) else {
            return select_task;
        };
        if instance.symbol.is_empty() {
            return select_task;
        }

        self.close_chart_header_menus();
        self.add_widget_menu_open = false;
        Task::batch([select_task, self.open_chart_instance_window(instance)])
    }

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
        let (detached_symbol, detached_interval, detached_last_time, detached_instance) = {
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

        let mut tasks = vec![self.open_chart_instance_window(detached_instance)];
        if !detached_symbol.is_empty()
            && self.charts.get(&detached_chart_id).is_some_and(|instance| {
                instance.chart.candles.is_empty()
                    || instance.candle_history_verified_at_ms.is_none()
                    || instance.candle_fetch_error.is_some()
                    || instance.candle_stream_error.is_some()
                    || instance
                        .candle_ws_updated_at_ms
                        .into_iter()
                        .chain(instance.candle_history_verified_at_ms)
                        .max()
                        .is_none_or(|at| Self::now_ms().saturating_sub(at) > 90_000)
            })
        {
            tasks.push(self.queue_candle_fetch_for(
                detached_chart_id,
                &detached_symbol,
                detached_interval,
                detached_last_time,
            ));
            tasks.extend(self.queue_macro_candles_tasks(detached_chart_id, &detached_symbol));
        }

        Task::batch(tasks)
    }

    fn open_chart_instance_window(&mut self, mut instance: ChartInstance) -> Task<Message> {
        let state = DetachedChartWindowState::new(instance.id);
        let settings = window::Settings {
            size: state.size(),
            position: state.position(),
            ..crate::window_chrome::settings(
                self.custom_window_chrome_active,
                self.window_background_blur_enabled,
            )
        };
        let (window_id, task) = window::open(settings);
        instance
            .chart
            .set_surface_id(ChartSurfaceId::Detached(window_id));
        self.charts.insert(instance.id, instance);
        self.detached_chart_windows.insert(window_id, state);
        self.persist_config();
        task.map(Message::WindowOpened)
    }
}

#[cfg(test)]
mod tests;
