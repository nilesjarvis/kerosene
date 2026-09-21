use crate::app_state::TradingTerminal;
use crate::console_state::{ConsoleFilter, PAGE_SIZE, scroll_id};
use crate::message::Message;
use crate::network_activity::{ActivityEntry, ActivityKind, HISTORY_LIMIT, Provider};
use iced::widget::{
    Column, button, column, container, pick_list, row, rule, scrollable, space, text,
};
use iced::{Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_console(&self) -> Element<'_, Message> {
        let state = &self.console;
        let theme = self.theme();
        let palette = theme.extended_palette();
        let recent = state.snapshot.recent[state.provider as usize];
        let total = state.snapshot.totals[state.provider as usize];
        let entries: Vec<_> = state.entries().collect();
        let start = state.page * PAGE_SIZE;
        let end = (start + PAGE_SIZE).min(entries.len());

        let controls = row![
            text("Console").size(16),
            pick_list(
                Provider::ALL,
                Some(state.provider),
                Message::ConsoleProviderChanged
            )
            .text_size(12),
            pick_list(
                ConsoleFilter::ALL,
                Some(state.filter),
                Message::ConsoleFilterChanged
            )
            .text_size(12),
            space::horizontal(),
            text(if state.paused { "Paused" } else { "Live" })
                .size(12)
                .color(if state.paused {
                    palette.warning.base.color
                } else {
                    palette.success.base.color
                }),
            button(text(if state.paused { "Resume" } else { "Pause" }).size(12))
                .on_press(Message::ConsoleTogglePause),
            button(text("Clear").size(12)).on_press(Message::ConsoleClear),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let metrics = row![
            text(format!("{} HTTP requests / 60s", recent.requests)).size(12),
            text(format!(
                "{} in flight",
                total.requests.saturating_sub(total.finished)
            ))
            .size(12),
            text(format!(
                "{:.1} WS receives/s",
                recent.ws_received as f64 / 60.0
            ))
            .size(12),
            text(format!(
                "{:.1} KiB/s WS ↓",
                recent.ws_bytes as f64 / 60.0 / 1024.0
            ))
            .size(12),
            text(format!(
                "{} errors · {} HTTP 429 / 60s",
                recent.errors, recent.rate_limited
            ))
            .size(12)
            .color(if recent.errors > 0 {
                palette.danger.base.color
            } else {
                palette.background.weak.text
            }),
        ]
        .spacing(16)
        .wrap();

        let header = row![
            text("Time (UTC)").size(11).width(108),
            text("Provider").size(11).width(100),
            text("Activity").size(11).width(108),
            text("Operation").size(11).width(Fill),
            text("Result / size / time to headers").size(11).width(240),
        ]
        .spacing(10);

        let mut rows = Column::new().spacing(0).width(Fill);
        for entry in entries.iter().skip(start).take(PAGE_SIZE) {
            rows = rows.push(view_entry(entry));
        }
        if entries.is_empty() {
            rows = rows.push(container(text("No matching network activity").size(12)).padding(20));
        }

        let footer = row![
            text(format!(
                "{}–{} of {} · newest first · retains {} events",
                if entries.is_empty() { 0 } else { start + 1 },
                end,
                entries.len(),
                HISTORY_LIMIT
            ))
            .size(11),
            space::horizontal(),
            button(text("Newer").size(11)).on_press_maybe(
                (state.page > 0).then(|| Message::ConsolePageChanged(state.page - 1))
            ),
            button(text("Older").size(11)).on_press_maybe(
                (end < entries.len()).then_some(Message::ConsolePageChanged(state.page + 1))
            ),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        container(
            column![
                controls,
                metrics,
                rule::horizontal(1),
                header,
                scrollable(rows)
                    .id(scroll_id())
                    .on_scroll(Message::ConsoleScrolled)
                    .height(Fill),
                rule::horizontal(1),
                footer,
            ]
            .spacing(10),
        )
        .padding(14)
        .width(Fill)
        .height(Fill)
        .into()
    }
}

fn view_entry(entry: &ActivityEntry) -> Element<'_, Message> {
    let timestamp = chrono::DateTime::from_timestamp_millis(entry.timestamp_ms as i64)
        .map(|time| time.format("%H:%M:%S%.3f").to_string())
        .unwrap_or_default();
    let operation = if entry.method.is_empty() {
        entry.operation.to_string()
    } else {
        format!(
            "{} {}{}",
            entry.method,
            entry.operation,
            if entry.proxied { " [proxy]" } else { "" }
        )
    };
    let mut detail = match entry.kind {
        ActivityKind::HttpResponse(status) => format!("{status}"),
        ActivityKind::HttpFailed => "Transport error".to_string(),
        ActivityKind::HttpCancelled => "Cancelled".to_string(),
        ActivityKind::HttpSend => "Sent".to_string(),
        _ => entry
            .bytes
            .map(|bytes| format!("{bytes} B"))
            .unwrap_or_default(),
    };
    if let Some(ms) = entry.elapsed_ms {
        detail.push_str(&format!(" · {ms} ms"));
    }
    if let Some(id) = entry.request_id {
        detail.push_str(&format!(" · #{id}"));
    }
    let kind = entry.kind;
    let font = crate::app_fonts::monospace_font();
    container(
        row![
            text(timestamp).font(font).size(11).width(108),
            text(entry.provider.to_string())
                .font(font)
                .size(11)
                .width(100),
            text(kind.label())
                .font(font)
                .size(11)
                .width(108)
                .style(move |theme: &Theme| {
                    let palette = theme.extended_palette();
                    iced::widget::text::Style {
                        color: Some(if kind.is_error() {
                            palette.danger.base.color
                        } else if kind == ActivityKind::HttpSend || kind == ActivityKind::WsSend {
                            palette.primary.base.color
                        } else {
                            palette.success.base.color
                        }),
                    }
                }),
            text(operation).font(font).size(11).width(Fill),
            text(detail).font(font).size(11).width(240),
        ]
        .spacing(10),
    )
    .padding([5, 0])
    .width(Fill)
    .into()
}
