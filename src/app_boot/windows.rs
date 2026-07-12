use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartSurfaceId, DetachedChartWindowState};
use crate::config::KeroseneConfig;
use crate::message::Message;
use crate::wallet_cluster_state::wallet_cluster_window_settings;
use iced::{Point, Size, Task, window};

impl TradingTerminal {
    pub(super) fn boot_window_tasks(&mut self, cfg: &KeroseneConfig) -> Vec<Task<Message>> {
        let mut boot_tasks = Vec::new();
        let main_min_size = self.main_window_min_size();
        let requested_main_size = self.main_window_size.unwrap_or(Size::new(1600.0, 960.0));

        let main_window_settings = window::Settings {
            size: Size::new(
                requested_main_size.width.max(main_min_size.width),
                requested_main_size.height.max(main_min_size.height),
            ),
            min_size: Some(main_min_size),
            position: self
                .main_window_pos
                .map(crate::window_chrome::restored_position)
                .unwrap_or_else(|| window::Position::Centered),
            ..crate::window_chrome::settings(self.custom_window_chrome_active)
        };
        let (main_id, main_open_task) = window::open(main_window_settings);
        self.main_window_id = Some(main_id);
        boot_tasks.push(main_open_task.map(Message::WindowOpened));

        if self.wallet_tracker.open {
            let tracker_settings = window::Settings {
                size: Size::new(self.wallet_tracker.width, self.wallet_tracker.height),
                position: self
                    .wallet_tracker
                    .x
                    .zip(self.wallet_tracker.y)
                    .map(|(x, y)| crate::window_chrome::restored_position(Point::new(x, y)))
                    .unwrap_or_else(|| window::Position::Centered),
                ..crate::window_chrome::settings(self.custom_window_chrome_active)
            };
            let (wallet_id, wallet_open_task) = window::open(tracker_settings);
            self.wallet_tracker.window_id = Some(wallet_id);
            boot_tasks.push(wallet_open_task.map(Message::WindowOpened));
            self.queue_wallet_tracker_core_refresh_all();
            boot_tasks.push(self.refresh_next_wallet_tracker_core());
        }

        if self.wallet_clusters.open {
            let settings = wallet_cluster_window_settings(
                &self.wallet_clusters,
                self.custom_window_chrome_active,
            );
            let (wallet_clusters_id, wallet_clusters_open_task) = window::open(settings);
            self.wallet_clusters.window_id = Some(wallet_clusters_id);
            boot_tasks.push(wallet_clusters_open_task.map(Message::WindowOpened));
            boot_tasks.push(self.refresh_selected_wallet_cluster());
        }

        for detached_cfg in &cfg.detached_chart_windows {
            if !self.charts.contains_key(&detached_cfg.chart_id)
                || self
                    .detached_chart_windows
                    .values()
                    .any(|state| state.chart_id == detached_cfg.chart_id)
            {
                continue;
            }

            let state = DetachedChartWindowState::from_config(detached_cfg);
            let settings = window::Settings {
                size: state.size(),
                position: state.position(),
                ..crate::window_chrome::settings(self.custom_window_chrome_active)
            };
            let (window_id, open_task) = window::open(settings);
            if let Some(instance) = self.charts.get_mut(&state.chart_id) {
                instance
                    .chart
                    .set_surface_id(ChartSurfaceId::Detached(window_id));
            }
            self.detached_chart_windows.insert(window_id, state);
            boot_tasks.push(open_task.map(Message::WindowOpened));
        }

        boot_tasks
    }
}
