use super::super::display_text::pnl_card_render_text;
use super::super::metrics::PnlCardMetrics;
use super::super::model::PnlCardWindowState;
use super::super::rendering::PnlCardCanvas;
use super::super::style::pnl_card_frame_style;

use crate::app_state::TradingTerminal;
use crate::message::Message;

use iced::widget::grid::aspect_ratio;
use iced::widget::{Grid, canvas, container};
use iced::{Element, Fill, Theme};

// ---------------------------------------------------------------------------
// Preview
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_pnl_card_preview<'a>(
        &'a self,
        state: &'a PnlCardWindowState,
        metrics: PnlCardMetrics,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let pnl_color = self.direction_color(theme, metrics.upnl);
        let denomination = self.display_denomination_context();
        let render_text = pnl_card_render_text(state, &metrics, &denomination);
        let card = PnlCardCanvas::new(render_text, metrics.upnl, pnl_color);
        let surface = canvas(card).width(Fill).height(Fill);
        let aspect_locked = Grid::new()
            .columns(1)
            .height(aspect_ratio(16.0, 9.0))
            .push(surface);

        container(aspect_locked)
            .width(Fill)
            .style(move |theme: &Theme| pnl_card_frame_style(theme, pnl_color))
            .into()
    }
}
