use crate::app_state::TradingTerminal;
use crate::helpers::{format_price, format_signed_percent_value};
use crate::journal::{
    AggregatedTrade, JournalTradeSnapshot, JournalTradeSnapshotMetrics, JournalTradeSnapshotStatus,
};
use crate::message::Message;
use iced::widget::{Column, Row, Space, column, container, row, text};
use iced::{Color, Element, Fill, Theme, alignment};

mod canvas;
mod drawing;
mod interaction;
mod markers;
mod plot;

use canvas::JournalSnapshotCanvas;

const SNAPSHOT_HEIGHT: f32 = 240.0;

impl TradingTerminal {
    pub(in crate::journal_views) fn view_journal_trade_snapshot<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let muted = theme.extended_palette().background.weak.text;

        if let Some(request) = self.journal.snapshot_requests.get(&trade.id) {
            return container(
                row![
                    text("Loading chart snapshot")
                        .size(11)
                        .color(theme.palette().success),
                    Space::new().width(8.0),
                    text(request.timeframe.label()).size(11).color(muted),
                ]
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .height(SNAPSHOT_HEIGHT)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .into();
        }

        let Some(snapshot) = self.journal.snapshots.get(&trade.id) else {
            return unavailable_snapshot_view("Snapshot unavailable.", &theme);
        };

        match &snapshot.status {
            JournalTradeSnapshotStatus::Loaded => loaded_snapshot_view(snapshot, &theme),
            JournalTradeSnapshotStatus::Unavailable(reason) => {
                unavailable_snapshot_view(reason.as_str(), &theme)
            }
        }
    }
}

fn loaded_snapshot_view<'a>(
    snapshot: &'a JournalTradeSnapshot,
    theme: &Theme,
) -> Element<'a, Message> {
    let chart: Element<'a, Message> = iced::widget::canvas(JournalSnapshotCanvas { snapshot })
        .width(Fill)
        .height(SNAPSHOT_HEIGHT)
        .into();

    column![chart, metrics_rows(&snapshot.metrics, theme)]
        .spacing(6)
        .into()
}

fn unavailable_snapshot_view(reason: &str, theme: &Theme) -> Element<'static, Message> {
    container(
        text(reason.to_string())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .align_x(alignment::Horizontal::Center)
            .color(theme.extended_palette().background.weak.text),
    )
    .width(Fill)
    .height(SNAPSHOT_HEIGHT)
    .align_x(alignment::Horizontal::Center)
    .align_y(alignment::Vertical::Center)
    .padding(16)
    .style(|theme: &Theme| iced::widget::container::Style {
        background: Some(
            Color {
                a: 0.04,
                ..theme.palette().primary
            }
            .into(),
        ),
        border: iced::Border {
            color: Color {
                a: 0.5,
                ..theme.palette().primary
            },
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn metrics_rows(metrics: &JournalTradeSnapshotMetrics, theme: &Theme) -> Element<'static, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let text_color = theme.palette().text;

    let top = Row::new()
        .spacing(12)
        .push(metric_pair(
            "TF",
            metrics.timeframe.label().to_string(),
            text_color,
            muted,
        ))
        .push(metric_pair(
            "Candles",
            metrics.candle_count.to_string(),
            text_color,
            muted,
        ))
        .push(metric_pair(
            "Raw",
            format_pct(metrics.raw_asset_move),
            signed_color(metrics.raw_asset_move, theme),
            muted,
        ))
        .push(metric_pair(
            "Dir",
            format_pct(metrics.directional_move),
            signed_color(metrics.directional_move, theme),
            muted,
        ));

    let bottom = Row::new()
        .spacing(12)
        .push(metric_pair(
            "MAE",
            format_pct(metrics.max_adverse_excursion),
            signed_color(metrics.max_adverse_excursion, theme),
            muted,
        ))
        .push(metric_pair(
            "MFE",
            format_pct(metrics.max_favorable_excursion),
            signed_color(metrics.max_favorable_excursion, theme),
            muted,
        ))
        .push(metric_pair(
            "DD",
            format_pct(metrics.asset_drawdown),
            signed_color(metrics.asset_drawdown, theme),
            muted,
        ))
        .push(metric_pair(
            "Entry",
            format_price(metrics.entry_price),
            text_color,
            muted,
        ))
        .push(metric_pair(
            "Exit",
            format_price(metrics.exit_price),
            text_color,
            muted,
        ));

    Column::new().spacing(3).push(top).push(bottom).into()
}

fn metric_pair(
    label: &'static str,
    value: String,
    value_color: Color,
    label_color: Color,
) -> Element<'static, Message> {
    row![
        text(label).size(10).color(label_color),
        text(value)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(value_color),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
    .into()
}

fn format_pct(value: f64) -> String {
    format_signed_percent_value(value * 100.0)
}

fn signed_color(value: f64, theme: &Theme) -> Color {
    if value > 0.0 {
        theme.palette().success
    } else if value < 0.0 {
        theme.palette().danger
    } else {
        theme.palette().text
    }
}
