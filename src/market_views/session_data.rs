use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::session_data_state::{
    SessionDataId, SessionDataInstance, SessionGroup, SessionVerdict, session_verdict,
    weekday_dispersions,
};

use iced::widget::{canvas as canvas_widget, column, container, responsive, scrollable, text};
use iced::{Element, Fill, Length, Theme};
use lane::{LaneRow, SessionLane, lane_height};

mod controls;
mod lane;
mod summary;
mod tooltips;

const BODY_ERROR_CHARS: usize = 140;
const COMPACT_LANE_WIDTH: f32 = 360.0;

// ---------------------------------------------------------------------------
// Session Data View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_session_data(&self, id: SessionDataId) -> Element<'_, Message> {
        responsive(move |size| self.view_session_data_sized(id, size.width)).into()
    }

    fn view_session_data_sized(
        &self,
        id: SessionDataId,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(instance) = self.session_data.get(&id) else {
            return container(
                text("Session Data instance missing")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        };

        let header = self.view_session_data_header(instance, &theme, available_width);
        let mut content = column![header].spacing(8).padding(8);
        if instance.symbol_picker_open {
            content = content.push(self.view_session_data_symbol_dropdown(instance, &theme));
        }
        content = content.push(self.view_session_data_body(instance, &theme, available_width));

        container(scrollable(content).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_session_data_body<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        if instance.loading && instance.bars.is_empty() {
            return container(self.view_spinner(20))
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .into();
        }

        if instance.bars.is_empty() {
            let message = instance
                .error
                .as_deref()
                .unwrap_or("No session history available");
            return container(
                text(message)
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        }

        let verdict = session_verdict(
            &instance.weekday_summaries,
            &instance.session_summaries,
            instance.bars.len(),
        );
        let best_key: Option<(SessionGroup, &str)> = match &verdict {
            SessionVerdict::Edge { strongest, .. } => Some((strongest.group, strongest.label)),
            SessionVerdict::Insufficient { .. } => None,
        };

        let dispersions = weekday_dispersions(&instance.bars);
        let weekday_rows: Vec<LaneRow> = instance
            .weekday_summaries
            .iter()
            .map(|summary| {
                let label = summary.weekday.label();
                LaneRow {
                    label,
                    sample_count: summary.sample_count,
                    average_return_pct: summary.average_return_pct,
                    win_rate_pct: summary.win_rate_pct,
                    dispersion_pct: dispersions[summary.weekday.index()],
                    is_best: best_key == Some((SessionGroup::Weekday, label)),
                }
            })
            .collect();
        let session_rows: Vec<LaneRow> = instance
            .session_summaries
            .iter()
            .map(|summary| {
                let label = summary.session.short_label();
                LaneRow {
                    label,
                    sample_count: summary.sample_count,
                    average_return_pct: summary.average_return_pct,
                    win_rate_pct: summary.win_rate_pct,
                    dispersion_pct: None,
                    is_best: best_key == Some((SessionGroup::Session, label)),
                }
            })
            .collect();

        let scale_max = weekday_rows
            .iter()
            .chain(session_rows.iter())
            .map(|row| row.average_return_pct.abs())
            .fold(0.0_f64, f64::max)
            .max(0.1) as f32;
        let lane_height = lane_height(weekday_rows.len(), session_rows.len());
        let lane = canvas_widget(SessionLane {
            weekday_rows,
            session_rows,
            scale_max,
            compact: available_width < COMPACT_LANE_WIDTH,
        })
        .width(Fill)
        .height(Length::Fixed(lane_height));

        let summary = self.view_session_data_summary(instance, theme, &verdict, available_width);
        let mut content = column![summary, lane].spacing(10);

        if let Some(error) = &instance.error {
            content = content.push(
                text(helpers::ellipsized_text(error, BODY_ERROR_CHARS))
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(content).width(Fill).height(Fill).into()
    }
}
