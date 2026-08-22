use crate::app_state::TradingTerminal;
use crate::canvas_state::{CanvasId, CanvasState, WorkspaceId};
use crate::chart_state::ChartInstance;
use crate::message::Message;
use crate::pane_state::PaneKind;
use iced::widget::pane_grid;
use iced::{Size, Task, window};

impl TradingTerminal {
    pub(crate) fn update_canvas(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CreateCanvas => self.create_canvas(),
            Message::OpenCanvas(id) => self.open_canvas_window(id),
            _ => Task::none(),
        }
    }

    fn create_canvas(&mut self) -> Task<Message> {
        let id = self.next_canvas_id;
        self.next_canvas_id = self.next_canvas_id.saturating_add(1);

        let chart_id = self.alloc_chart_id();
        let mut chart = ChartInstance::new_empty(chart_id);
        self.apply_chart_appearance_settings(&mut chart.chart);
        self.charts.insert(chart_id, chart);

        let (panes, pane) = pane_grid::State::new(PaneKind::Chart(chart_id));
        self.canvases.insert(
            id,
            CanvasState {
                id,
                label: format!("Canvas {}", id.saturating_add(1)),
                window_id: None,
                panes,
                focus: Some(pane),
                dragging_pane: None,
                width: crate::config::DEFAULT_CANVAS_WIDTH,
                height: crate::config::DEFAULT_CANVAS_HEIGHT,
                x: None,
                y: None,
                preserved_loaded_pane_layout: None,
            },
        );
        self.primary_chart_id = Some(chart_id);
        self.last_focused_workspace = WorkspaceId::Canvas(id);
        self.add_widget_workspace = WorkspaceId::Canvas(id);
        self.persist_config();
        self.open_canvas_window(id)
    }

    pub(crate) fn open_canvas_window(&mut self, id: CanvasId) -> Task<Message> {
        if let Some(window_id) = self.canvases.get(&id).and_then(|canvas| canvas.window_id) {
            self.last_focused_workspace = WorkspaceId::Canvas(id);
            self.add_widget_workspace = WorkspaceId::Canvas(id);
            return window::gain_focus(window_id);
        }

        let Some(canvas) = self.canvases.get(&id) else {
            return Task::none();
        };
        let settings = window::Settings {
            size: canvas.size(),
            min_size: Some(Size::new(360.0, 240.0)),
            position: canvas.position(),
            ..crate::window_chrome::settings(
                self.custom_window_chrome_active,
                self.window_background_blur_enabled,
            )
        };
        let (window_id, open_task) = window::open(settings);
        if let Some(canvas) = self.canvases.get_mut(&id) {
            canvas.window_id = Some(window_id);
        }
        self.last_focused_workspace = WorkspaceId::Canvas(id);
        self.add_widget_workspace = WorkspaceId::Canvas(id);
        self.persist_config();
        open_task.map(Message::WindowOpened)
    }

    pub(crate) fn focus_workspace_window(&mut self, workspace: WorkspaceId) -> Task<Message> {
        self.last_focused_workspace = workspace;
        self.add_widget_workspace = workspace;
        match workspace {
            WorkspaceId::Main => self
                .main_window_id
                .map(window::gain_focus)
                .unwrap_or_else(Task::none),
            WorkspaceId::Canvas(id) => self.open_canvas_window(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    #[test]
    fn create_close_and_reopen_preserves_canvas_workspace() {
        let (mut terminal, _) =
            TradingTerminal::boot_from_config(config::KeroseneConfig::default());

        let _task = terminal.update_canvas(Message::CreateCanvas);

        let canvas = terminal.canvases.get(&0).expect("created Canvas");
        let window_id = canvas.window_id.expect("Canvas window");
        let chart_id = canvas
            .panes
            .iter()
            .find_map(|(_, kind)| match kind {
                PaneKind::Chart(id) => Some(*id),
                _ => None,
            })
            .expect("initial chart");
        assert!(terminal.charts.contains_key(&chart_id));

        let _task = terminal.update_window(Message::WindowClosed(window_id));
        assert!(terminal.canvases.contains_key(&0));
        assert_eq!(terminal.canvases[&0].window_id, None);
        assert!(terminal.charts.contains_key(&chart_id));

        let _task = terminal.update_canvas(Message::OpenCanvas(0));
        assert!(terminal.canvases[&0].window_id.is_some());
        assert!(terminal.charts.contains_key(&chart_id));
    }

    #[test]
    fn add_chart_hotkey_targets_last_focused_canvas() {
        let (mut terminal, _) =
            TradingTerminal::boot_from_config(config::KeroseneConfig::default());
        let main_pane_count = terminal.panes.iter().count();
        let _task = terminal.update_canvas(Message::CreateCanvas);

        let _task = terminal.update(Message::ExecuteHotkey(
            config::HotkeyAction::AddCandlestickChart,
        ));

        assert_eq!(terminal.panes.iter().count(), main_pane_count);
        assert_eq!(terminal.canvases[&0].panes.iter().count(), 2);
    }

    #[test]
    fn saved_layout_snapshot_includes_canvas_tree_and_geometry() {
        let (mut terminal, _) =
            TradingTerminal::boot_from_config(config::KeroseneConfig::default());
        let _task = terminal.update_canvas(Message::CreateCanvas);
        let canvas = terminal.canvases.get_mut(&0).expect("created Canvas");
        canvas.width = 1440.0;
        canvas.height = 900.0;
        canvas.x = Some(1920.0);
        canvas.y = Some(40.0);

        let layout = terminal.saved_layout_snapshot("multi-monitor".to_string());

        assert_eq!(layout.canvases.len(), 1);
        assert_eq!(layout.canvases[0].label, "Canvas 1");
        assert!(layout.canvases[0].open);
        assert!(layout.canvases[0].pane_layout.is_some());
        assert_eq!(layout.canvases[0].width, 1440.0);
        assert_eq!(layout.canvases[0].x, Some(1920.0));
    }

    #[test]
    fn applying_named_layout_replaces_the_canvas_set() {
        let (mut terminal, _) =
            TradingTerminal::boot_from_config(config::KeroseneConfig::default());
        let _task = terminal.update_canvas(Message::CreateCanvas);
        let layout = terminal.saved_layout_snapshot("one-canvas".to_string());
        let _task = terminal.update_canvas(Message::CreateCanvas);
        assert_eq!(terminal.canvases.len(), 2);

        let _task = terminal.apply_layout(layout);

        assert_eq!(terminal.canvases.len(), 1);
        assert!(terminal.canvases.contains_key(&0));
        assert!(terminal.canvases[&0].window_id.is_some());
    }

    #[test]
    fn singleton_restrictions_span_main_and_canvas_workspaces() {
        let (mut terminal, _) =
            TradingTerminal::boot_from_config(config::KeroseneConfig::default());
        let _task = terminal.update_canvas(Message::CreateCanvas);
        terminal.add_widget_workspace = WorkspaceId::Canvas(0);
        let _task = terminal.update_panes(Message::AddPortfolioPane);
        assert!(
            terminal.canvases[&0]
                .panes
                .iter()
                .any(|(_, kind)| matches!(kind, PaneKind::Portfolio))
        );

        terminal.add_widget_workspace = WorkspaceId::Main;
        let _task = terminal.update_panes(Message::BeginWidgetPlacement(
            crate::pane_management::AddWidgetKind::Portfolio,
        ));

        assert_eq!(terminal.last_focused_workspace, WorkspaceId::Canvas(0));
        assert_eq!(terminal.placing_widget, None);
        assert!(
            !terminal
                .panes
                .iter()
                .any(|(_, kind)| matches!(kind, PaneKind::Portfolio))
        );
    }

    #[test]
    fn boot_restores_canvas_with_independent_chart_instance() {
        let config = config::KeroseneConfig {
            canvases: vec![config::CanvasConfig {
                id: 4,
                label: "Left monitor".to_string(),
                open: false,
                pane_layout: Some(config::PaneLayoutConfig::Leaf(
                    config::PaneKindConfig::Chart { chart_id: 42 },
                )),
                width: 1000.0,
                height: 700.0,
                x: None,
                y: None,
            }],
            charts: vec![config::ChartConfig::empty(42, "BTC", "H1")],
            ..config::KeroseneConfig::default()
        };

        let (terminal, _) = TradingTerminal::boot_from_config(config);

        let canvas = terminal.canvases.get(&4).expect("restored Canvas");
        assert_eq!(canvas.label, "Left monitor");
        assert!(canvas.window_id.is_none());
        assert!(
            canvas
                .panes
                .iter()
                .any(|(_, kind)| matches!(kind, PaneKind::Chart(42)))
        );
        let main_chart_id = terminal
            .panes
            .iter()
            .find_map(|(_, kind)| match kind {
                PaneKind::Chart(id) => Some(*id),
                _ => None,
            })
            .expect("default main chart");
        assert_ne!(main_chart_id, 42);
        assert!(terminal.charts.contains_key(&main_chart_id));
        assert!(terminal.charts.contains_key(&42));
    }

    #[test]
    fn boot_preserves_unavailable_future_canvas_for_the_next_save() {
        let future_pane = serde_json::json!({
            "FuturePane": {
                "id": 9,
                "label": "newer-version"
            }
        });
        let pane_layout =
            config::PaneLayoutConfig::Leaf(config::PaneKindConfig::Unknown(future_pane));
        let cfg = config::KeroseneConfig {
            canvases: vec![config::CanvasConfig {
                id: 8,
                label: "Future monitor".to_string(),
                open: true,
                pane_layout: Some(pane_layout.clone()),
                width: 1200.0,
                height: 800.0,
                x: Some(30.0),
                y: Some(40.0),
            }],
            ..config::KeroseneConfig::default()
        };

        let (mut terminal, _) = TradingTerminal::boot_from_config(cfg);

        assert!(terminal.canvases.is_empty());
        assert_eq!(terminal.preserved_unavailable_canvases.len(), 1);
        assert_eq!(terminal.next_canvas_id, 9);
        let snapshot = terminal.saved_layout_snapshot("future".to_string());
        assert_eq!(snapshot.canvases.len(), 1);
        assert_eq!(snapshot.canvases[0].id, 8);
        assert_eq!(snapshot.canvases[0].pane_layout, Some(pane_layout));

        let _task = terminal.update_canvas(Message::CreateCanvas);
        assert!(terminal.canvases.contains_key(&9));
        assert_eq!(
            terminal
                .saved_layout_snapshot("future-plus-local".to_string())
                .canvases
                .len(),
            2
        );

        let _task = terminal.apply_layout(snapshot);
        assert!(terminal.canvases.is_empty());
        assert_eq!(terminal.preserved_unavailable_canvases.len(), 1);
        assert_eq!(terminal.next_canvas_id, 9);
    }
}
