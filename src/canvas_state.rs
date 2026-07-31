use crate::app_state::TradingTerminal;
use crate::config;
use crate::pane_state::PaneKind;
use iced::widget::pane_grid;
use iced::{Point, Size, window};

pub(crate) type CanvasId = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum WorkspaceId {
    Main,
    Canvas(CanvasId),
}

pub(crate) struct CanvasState {
    pub(crate) id: CanvasId,
    pub(crate) label: String,
    pub(crate) window_id: Option<window::Id>,
    pub(crate) panes: pane_grid::State<PaneKind>,
    pub(crate) focus: Option<pane_grid::Pane>,
    pub(crate) dragging_pane: Option<pane_grid::Pane>,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
    pub(crate) preserved_loaded_pane_layout: Option<config::PaneLayoutConfig>,
}

impl CanvasState {
    pub(crate) fn from_config(
        config: &config::CanvasConfig,
        panes: pane_grid::State<PaneKind>,
    ) -> Self {
        Self {
            id: config.id,
            label: if config.label.trim().is_empty() {
                format!("Canvas {}", config.id.saturating_add(1))
            } else {
                config.label.trim().to_string()
            },
            window_id: None,
            panes,
            focus: None,
            dragging_pane: None,
            width: normalized_extent(config.width, config::DEFAULT_CANVAS_WIDTH),
            height: normalized_extent(config.height, config::DEFAULT_CANVAS_HEIGHT),
            x: config.x.filter(|value| value.is_finite()),
            y: config.y.filter(|value| value.is_finite()),
            preserved_loaded_pane_layout: config.pane_layout.clone(),
        }
    }

    pub(crate) fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub(crate) fn position(&self) -> window::Position {
        self.x
            .zip(self.y)
            .map(|(x, y)| crate::window_chrome::restored_position(Point::new(x, y)))
            .unwrap_or(window::Position::Centered)
    }
}

fn normalized_extent(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value.max(320.0)
    } else {
        fallback
    }
}

impl TradingTerminal {
    pub(crate) fn workspace_panes(
        &self,
        workspace: WorkspaceId,
    ) -> Option<&pane_grid::State<PaneKind>> {
        match workspace {
            WorkspaceId::Main => Some(&self.panes),
            WorkspaceId::Canvas(id) => self.canvases.get(&id).map(|canvas| &canvas.panes),
        }
    }

    pub(crate) fn workspace_panes_mut(
        &mut self,
        workspace: WorkspaceId,
    ) -> Option<&mut pane_grid::State<PaneKind>> {
        match workspace {
            WorkspaceId::Main => Some(&mut self.panes),
            WorkspaceId::Canvas(id) => self.canvases.get_mut(&id).map(|canvas| &mut canvas.panes),
        }
    }

    pub(crate) fn workspace_focus(&self, workspace: WorkspaceId) -> Option<pane_grid::Pane> {
        match workspace {
            WorkspaceId::Main => self.focus,
            WorkspaceId::Canvas(id) => self.canvases.get(&id).and_then(|canvas| canvas.focus),
        }
    }

    pub(crate) fn set_workspace_focus(
        &mut self,
        workspace: WorkspaceId,
        focus: Option<pane_grid::Pane>,
    ) {
        match workspace {
            WorkspaceId::Main => self.focus = focus,
            WorkspaceId::Canvas(id) => {
                if let Some(canvas) = self.canvases.get_mut(&id) {
                    canvas.focus = focus;
                }
            }
        }
    }

    pub(crate) fn workspace_dragging_pane(
        &self,
        workspace: WorkspaceId,
    ) -> Option<pane_grid::Pane> {
        match workspace {
            WorkspaceId::Main => self.dragging_pane,
            WorkspaceId::Canvas(id) => self
                .canvases
                .get(&id)
                .and_then(|canvas| canvas.dragging_pane),
        }
    }

    pub(crate) fn set_workspace_dragging_pane(
        &mut self,
        workspace: WorkspaceId,
        pane: Option<pane_grid::Pane>,
    ) {
        match workspace {
            WorkspaceId::Main => self.dragging_pane = pane,
            WorkspaceId::Canvas(id) => {
                if let Some(canvas) = self.canvases.get_mut(&id) {
                    canvas.dragging_pane = pane;
                }
            }
        }
    }

    pub(crate) fn workspace_for_window(&self, window_id: window::Id) -> Option<WorkspaceId> {
        if self.main_window_id == Some(window_id) {
            return Some(WorkspaceId::Main);
        }
        self.canvases.iter().find_map(|(id, canvas)| {
            (canvas.window_id == Some(window_id)).then_some(WorkspaceId::Canvas(*id))
        })
    }

    pub(crate) fn workspace_pane_kinds(
        &self,
    ) -> impl Iterator<Item = (WorkspaceId, pane_grid::Pane, &PaneKind)> {
        std::iter::once((WorkspaceId::Main, &self.panes))
            .chain(
                self.canvases
                    .iter()
                    .map(|(id, canvas)| (WorkspaceId::Canvas(*id), &canvas.panes)),
            )
            .flat_map(|(workspace, panes)| {
                panes
                    .iter()
                    .map(move |(pane, kind)| (workspace, *pane, kind))
            })
    }

    #[cfg(test)]
    pub(crate) fn insert_test_canvas_pane(
        &mut self,
        id: CanvasId,
        kind: PaneKind,
    ) -> pane_grid::Pane {
        let (panes, pane) = pane_grid::State::new(kind);
        self.canvases.insert(
            id,
            CanvasState {
                id,
                label: format!("Canvas {}", id.saturating_add(1)),
                window_id: None,
                panes,
                focus: Some(pane),
                dragging_pane: None,
                width: config::DEFAULT_CANVAS_WIDTH,
                height: config::DEFAULT_CANVAS_HEIGHT,
                x: None,
                y: None,
                preserved_loaded_pane_layout: None,
            },
        );
        pane
    }
}
