mod chart_area;
mod controls;
mod editor;
mod pair;
mod toolbar;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti_state::SpaghettiChartId;
use iced::widget::{column, container, pane_grid, text};
use iced::{Element, Fill};

impl TradingTerminal {
    pub(crate) fn view_spaghetti_chart(
        &self,
        id: SpaghettiChartId,
        _pane: pane_grid::Pane,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(inst) = self.spaghetti_charts.get(&id) else {
            return container(
                text("Chart not found")
                    .size(14)
                    .color(theme.palette().danger),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .into();
        };

        // Editor overlay for adding/removing symbols
        if inst.editor_open {
            return self.view_spaghetti_editor(id, inst);
        }

        let toolbar = self.view_spaghetti_toolbar(id, inst);
        let tf_row = self.view_spaghetti_controls(id, inst);
        let chart_area = self.view_spaghetti_chart_area(inst, &theme);

        let pair_controls = self.view_spaghetti_pair_controls(id, inst, &theme);

        let content = if let Some(panel) = pair_controls {
            column![toolbar, tf_row, chart_area, panel].spacing(4)
        } else {
            column![toolbar, tf_row, chart_area].spacing(4)
        };

        container(content)
            .width(Fill)
            .height(Fill)
            .padding([4, 4])
            .into()
    }
}
