use super::analytics::{
    JournalKpis, journal_effective_pnl, journal_is_non_perp, journal_trade_r_multiple,
};
use super::trade_card::journal_chip;
use super::trade_list::journal_asset_badge;
use super::trades::trade_duration_ms;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::journal::{AggregatedTrade, JournalSnapshotCoverage, JournalTradeSnapshotStatus};
use crate::journal_views::style::{
    journal_accent_soft, journal_card_style, journal_dim, journal_muted, journal_rule_style,
    journal_segment_style,
};
use crate::message::Message;
use crate::timeframe::Timeframe;
use iced::widget::{Space, button, column, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill, Length, Theme};

const DETAIL_TIMEFRAMES: [Timeframe; 3] = [Timeframe::M1, Timeframe::M5, Timeframe::H1];

impl TradingTerminal {
    pub(super) fn view_journal_detail<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
        kpis: &JournalKpis,
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let include_fees = self.journal.include_fees_in_pnl;
        let net_pnl = journal_effective_pnl(trade, include_fees);
        let pnl_color = helpers::signed_number_color(net_pnl, &theme);
        let denomination = self.display_denomination_context();

        // ---- Header ----
        let monogram = journal_asset_badge(
            &self.display_coin_for_journal(&trade.coin),
            34.0,
            20,
            &theme,
        );

        let side = if journal_is_non_perp(&trade.coin) {
            ("SPOT", journal_muted(&theme))
        } else if trade.is_long {
            ("LONG", theme.palette().success)
        } else {
            ("SHORT", theme.palette().danger)
        };

        // Still-open positions wear the accent so they read as live.
        let status_tint = if trade.status == "OPEN" {
            theme.palette().primary
        } else {
            journal_muted(&theme)
        };

        let back = button(
            text("← Overview")
                .size(11)
                .font(crate::app_fonts::monospace_font()),
        )
        .on_press(Message::JournalDeselectTrade)
        .padding([5, 10])
        .style(crate::journal_views::style::journal_ghost_button_style);

        let header = row![
            back,
            monogram,
            text(self.display_coin_for_journal(&trade.coin))
                .size(20)
                .color(theme.palette().text),
            journal_chip(side.0, side.1),
            journal_chip(trade.status.clone(), status_tint),
            Space::new().width(Fill),
            text(denomination.format_signed_value(net_pnl, 2))
                .size(24)
                .font(crate::app_fonts::monospace_font())
                .color(pnl_color),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let held = helpers::format_duration(trade_duration_ms(trade, self.status_bar_now_ms));
        let mut head = column![
            header,
            text(format!(
                "Opened {} · Held {} · {} fills",
                helpers::format_timestamp_exact(trade.start_time),
                held,
                trade.fill_count
            ))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(journal_dim(&theme)),
        ]
        .spacing(6);
        if !trade.basis_complete {
            head = head.push(
                text("Partial history — opening fills are outside the loaded data; metrics may be incomplete.")
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.palette().warning),
            );
        }

        // ---- Body ----
        let content = column![
            head,
            self.view_journal_detail_snapshot(trade, &theme),
            self.view_journal_detail_stats(trade, kpis, &theme),
            container(self.view_journal_reflection(trade))
                .padding(14)
                .width(Fill)
                .style(journal_card_style),
        ]
        .spacing(14)
        .padding(16)
        .width(Fill);

        scrollable(content)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .margin(0)
                    .scroller_width(4),
            ))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_journal_detail_snapshot<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let is_perp = !journal_is_non_perp(&trade.coin);
        let active_timeframe = self
            .journal
            .snapshots
            .get(&trade.id)
            .map(|snapshot| snapshot.timeframe)
            .or_else(|| {
                self.journal
                    .snapshot_requests
                    .get(&trade.id)
                    .map(|request| request.timeframe)
            });

        // Match the caption to how the chart actually renders: a live-position
        // chart (entry guide, no fill markers) only when the loaded snapshot is
        // flagged live. Before the snapshot loads, fill-less open positions
        // (fill_count 0) are the live case.
        let is_live_chart = self
            .journal
            .snapshots
            .get(&trade.id)
            .map(|snapshot| snapshot.live_position)
            .unwrap_or_else(|| trade.end_time.is_none() && trade.fill_count == 0);
        let caption = if is_live_chart {
            "CHART SNAPSHOT · LIVE POSITION"
        } else {
            "CHART SNAPSHOT · ENTRY → EXIT"
        };
        let mut header = row![
            text(caption)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(journal_accent_soft(theme)),
            Space::new().width(Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if is_perp {
            let mut coverage_selector = row![
                text("COVERAGE")
                    .size(9)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_dim(theme)),
            ]
            .spacing(4)
            .align_y(Alignment::Center);
            for coverage in JournalSnapshotCoverage::OPTIONS {
                let active = self.journal.snapshot_coverage == coverage;
                coverage_selector = coverage_selector.push(
                    button(
                        text(coverage.label())
                            .size(10)
                            .font(crate::app_fonts::monospace_font()),
                    )
                    .on_press(Message::JournalSnapshotCoverageChanged(coverage))
                    .padding([3, 9])
                    .style(journal_segment_style(active)),
                );
            }

            let mut selector = row![].spacing(4).align_y(Alignment::Center);
            for timeframe in DETAIL_TIMEFRAMES {
                let active = active_timeframe == Some(timeframe);
                selector = selector.push(
                    button(
                        text(timeframe.label())
                            .size(10)
                            .font(crate::app_fonts::monospace_font()),
                    )
                    .on_press(Message::JournalSnapshotTimeframe(
                        trade.id.clone(),
                        timeframe,
                    ))
                    .padding([3, 9])
                    .style(journal_segment_style(active)),
                );
            }
            header = header.push(coverage_selector);
            header = header.push(selector);
        }

        container(
            column![header, self.view_journal_trade_snapshot(trade)]
                .spacing(8)
                .width(Fill),
        )
        .padding(14)
        .width(Fill)
        .style(journal_card_style)
        .into()
    }

    fn view_journal_detail_stats(
        &self,
        trade: &AggregatedTrade,
        kpis: &JournalKpis,
        theme: &Theme,
    ) -> Element<'_, Message> {
        let denomination = self.display_denomination_context();
        let include_fees = self.journal.include_fees_in_pnl;
        let net_pnl = journal_effective_pnl(trade, include_fees);

        let (entry_display, exit_display) = if journal_is_non_perp(&trade.coin) {
            non_perp_entry_exit_display(trade)
        } else {
            let snapshot = self.journal.snapshots.get(&trade.id);
            let loaded = snapshot.is_some_and(|snapshot| {
                matches!(snapshot.status, JournalTradeSnapshotStatus::Loaded)
            });
            let entry_price = snapshot
                .filter(|_| loaded)
                .map(|snapshot| snapshot.metrics.entry_price)
                .filter(|price| price.is_finite() && *price > 0.0)
                .unwrap_or(trade.avg_entry_price);
            let entry_display = if entry_price.is_finite() && entry_price > 0.0 {
                helpers::format_price(entry_price)
            } else {
                "—".to_string()
            };
            let exit_display = snapshot
                .filter(|_| loaded && trade.end_time.is_some())
                .map(|snapshot| helpers::format_price(snapshot.metrics.exit_price))
                .unwrap_or_else(|| "—".to_string());
            (entry_display, exit_display)
        };

        let r_display = journal_trade_r_multiple(trade, kpis.r_unit, include_fees)
            .map(|r| format!("{r:+.2}R"))
            .unwrap_or_else(|| "—".to_string());

        let text_color = theme.palette().text;
        let top = row![
            stat_cell("ENTRY", entry_display, text_color, theme),
            stat_divider(),
            stat_cell("EXIT", exit_display, text_color, theme),
            stat_divider(),
            stat_cell(
                "SIZE",
                self.journal_max_position_label(trade),
                text_color,
                theme
            ),
            stat_divider(),
            stat_cell(
                "DURATION",
                helpers::format_duration(trade_duration_ms(trade, self.status_bar_now_ms)),
                text_color,
                theme,
            ),
        ]
        .align_y(Alignment::Center);

        let bottom = row![
            stat_cell("FILLS", trade.fill_count.to_string(), text_color, theme),
            stat_divider(),
            stat_cell(
                "FEES",
                denomination.format_value(trade.fee, 2),
                theme.palette().warning,
                theme,
            ),
            stat_divider(),
            stat_cell(
                "NET PNL",
                denomination.format_signed_value(net_pnl, 2),
                helpers::signed_number_color(net_pnl, theme),
                theme,
            ),
            stat_divider(),
            stat_cell(
                "R MULTIPLE",
                r_display.clone(),
                journal_trade_r_multiple(trade, kpis.r_unit, include_fees)
                    .map(|r| helpers::signed_number_color(r, theme))
                    .unwrap_or(text_color),
                theme,
            ),
        ]
        .align_y(Alignment::Center);

        container(
            column![top, rule::horizontal(1).style(journal_rule_style), bottom,]
                .spacing(0)
                .width(Fill),
        )
        .width(Fill)
        .style(journal_card_style)
        .into()
    }
}

/// `(ENTRY, EXIT)` stat values for a spot/outcome trade. One non-perp trade is
/// a single order whose fills all share one side, so its execution VWAP
/// (accumulated in `avg_entry_price`) is an entry price for buys but the sale
/// price for sells — a sell's VWAP belongs under EXIT, never ENTRY.
fn non_perp_entry_exit_display(trade: &AggregatedTrade) -> (String, String) {
    let vwap = trade.avg_entry_price;
    let vwap_display = if vwap.is_finite() && vwap > 0.0 {
        helpers::format_price(vwap)
    } else {
        "—".to_string()
    };
    if trade.is_long {
        (vwap_display, "—".to_string())
    } else {
        ("—".to_string(), vwap_display)
    }
}

fn stat_cell(
    label: &'static str,
    value: String,
    value_color: iced::Color,
    theme: &Theme,
) -> Element<'static, Message> {
    container(
        column![
            text(label)
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(theme)),
            text(value)
                .size(14)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
        ]
        .spacing(5),
    )
    .width(Fill)
    .padding([12, 12])
    .into()
}

fn stat_divider() -> Element<'static, Message> {
    container(rule::vertical(1).style(journal_rule_style))
        .height(Length::Fixed(48.0))
        .width(Length::Fixed(1.0))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn non_perp_trade(is_long: bool, vwap: f64) -> AggregatedTrade {
        AggregatedTrade {
            id: "spot:@107:9001".to_string(),
            legacy_note_ids: Vec::new(),
            coin: "@107".to_string(),
            start_time: 1_000,
            end_time: Some(1_000),
            max_position: if is_long { 1.0 } else { -1.0 },
            volume: vwap,
            fee: 0.0,
            pnl: 0.0,
            status: "FILLED".to_string(),
            fill_count: 1,
            avg_entry_price: vwap,
            total_entry_notional: vwap,
            total_entry_size: 1.0,
            is_long,
            basis_complete: true,
        }
    }

    #[test]
    fn non_perp_buy_vwap_shows_under_entry() {
        let (entry, exit) = non_perp_entry_exit_display(&non_perp_trade(true, 40.0));

        assert_eq!(entry, helpers::format_price(40.0));
        assert_eq!(exit, "—");
    }

    #[test]
    fn non_perp_sell_vwap_shows_under_exit_not_entry() {
        // A spot sell's VWAP is its sale price; rendering it as ENTRY implied a
        // nonexistent cost basis.
        let (entry, exit) = non_perp_entry_exit_display(&non_perp_trade(false, 50.0));

        assert_eq!(entry, "—");
        assert_eq!(exit, helpers::format_price(50.0));
    }

    #[test]
    fn non_perp_missing_vwap_shows_dashes() {
        let (entry, exit) = non_perp_entry_exit_display(&non_perp_trade(false, 0.0));

        assert_eq!(entry, "—");
        assert_eq!(exit, "—");
    }
}
