use super::analytics::{
    JournalKpis, journal_asset_pnls, journal_direction_split, journal_kpis, journal_time_of_day,
};
use crate::app_state::TradingTerminal;
use crate::journal::AggregatedTrade;
use crate::journal_views::style::{
    journal_card_style, journal_dim, journal_muted, journal_rule_style, journal_segment_style,
};
use crate::message::Message;
use crate::portfolio_state::PortfolioWindow;
use iced::widget::{Space, button, column, container, row, rule, text};
use iced::{Alignment, Element, Fill, Length, Theme};

mod bars;
mod heatmap;
mod tiles;
mod win_loss;

use bars::view_journal_direction_bars;
use heatmap::view_journal_heatmap;
use tiles::view_journal_kpi_tiles;
use win_loss::view_journal_winloss_body;

const COCKPIT_WINDOWS: [PortfolioWindow; 7] = [
    PortfolioWindow::Day,
    PortfolioWindow::Week,
    PortfolioWindow::Mtd,
    PortfolioWindow::Month,
    PortfolioWindow::Quarter,
    PortfolioWindow::Ytd,
    PortfolioWindow::AllTime,
];

const PANEL_TITLE_HEIGHT: f32 = 30.0;

impl TradingTerminal {
    pub(super) fn view_journal_cockpit<'a>(
        &'a self,
        filtered_trades: &[&'a AggregatedTrade],
    ) -> Element<'a, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let include_fees = self.journal.include_fees_in_pnl;

        // The cockpit timeframe windows the analytics; the global KPI strip
        // above stays all-time.
        let cutoff = self
            .journal
            .portfolio_window
            .cutoff_ms(self.status_bar_now_ms);
        let windowed: Vec<&AggregatedTrade> = filtered_trades
            .iter()
            .copied()
            .filter(|trade| cutoff.is_none_or(|cutoff| trade.start_time >= cutoff))
            .collect();
        let kpis = journal_kpis(&windowed, include_fees);
        let split = journal_direction_split(&windowed, include_fees);
        let assets = journal_asset_pnls(&windowed, include_fees);
        let heatmap = journal_time_of_day(&windowed, include_fees);

        let header = row![
            text("Performance Overview")
                .size(20)
                .color(theme.palette().text),
            Space::new().width(16.0),
            cockpit_timeframe_row(self.journal.portfolio_window),
            Space::new().width(Fill),
            text("Select a trade for detail →")
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(&theme)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let equity = cockpit_panel(
            "EQUITY CURVE",
            self.view_journal_equity_panel_body(filtered_trades, &kpis, &theme),
            &theme,
        );

        let donut = cockpit_panel(
            "WIN / LOSS",
            view_journal_winloss_body(&kpis, &denomination, &theme),
            &theme,
        );

        let tiles = cockpit_panel(
            "KEY METRICS",
            view_journal_kpi_tiles(&kpis, &denomination, &theme),
            &theme,
        );

        let direction = cockpit_panel(
            "LONG vs SHORT vs SPOT",
            view_journal_direction_bars(&split, &denomination, &theme),
            &theme,
        );

        let heat = cockpit_panel(
            "EDGE BY TIME OF DAY (UTC)",
            view_journal_heatmap(&heatmap, &theme),
            &theme,
        );

        let asset_bars = cockpit_panel(
            "PNL BY ASSET",
            self.view_journal_asset_bars(&assets, &denomination, &theme),
            &theme,
        );

        let content = column![
            header,
            equity,
            row![donut, tiles].spacing(14),
            direction,
            heat,
            asset_bars,
        ]
        .spacing(14)
        .padding(16)
        .width(Fill);

        iced::widget::scrollable(content)
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

    fn view_journal_equity_panel_body<'a>(
        &'a self,
        filtered_trades: &[&'a AggregatedTrade],
        kpis: &JournalKpis,
        theme: &Theme,
    ) -> Element<'a, Message> {
        column![
            text("Cumulative realized PnL")
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(theme)),
            self.view_journal_summary_chart(
                filtered_trades,
                kpis.net_pnl,
                kpis.total_fees,
                kpis.win_rate,
                kpis.scored,
            ),
        ]
        .spacing(8)
        .into()
    }
}

// ---- Panel chrome ----

fn cockpit_panel<'a>(
    title: &'static str,
    body: Element<'a, Message>,
    theme: &Theme,
) -> Element<'a, Message> {
    container(
        column![
            container(
                text(title)
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(theme)),
            )
            .height(Length::Fixed(PANEL_TITLE_HEIGHT))
            .padding([0, 12])
            .align_y(iced::alignment::Vertical::Center)
            .width(Fill),
            rule::horizontal(1).style(journal_rule_style),
            container(body).padding(12).width(Fill),
        ]
        .width(Fill),
    )
    .width(Fill)
    .style(journal_card_style)
    .into()
}

fn cockpit_timeframe_row(selected: PortfolioWindow) -> Element<'static, Message> {
    let mut row = row![].spacing(4).align_y(Alignment::Center);
    for window in COCKPIT_WINDOWS {
        row = row.push(
            button(
                text(window.label())
                    .size(10)
                    .font(crate::app_fonts::monospace_font()),
            )
            .on_press(Message::JournalPortfolioWindowChanged(window))
            .padding([3, 9])
            .style(journal_segment_style(selected == window)),
        );
    }
    row.into()
}
