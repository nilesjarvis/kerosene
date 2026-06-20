use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::journal::{JournalFilter, JournalSort};
use crate::journal_views::analytics::JournalKpis;
use crate::journal_views::style::{
    JOURNAL_CHIP_RADIUS, journal_accent_soft, journal_dim, journal_hairline, journal_muted,
    journal_rule_style, journal_segment_style,
};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Space, button, column, container, row, rule, text};
use iced::{Alignment, Border, Color, Element, Fill, Length, Theme};

const TITLE_BAR_HEIGHT: f32 = 42.0;
const TOOLBAR_HEIGHT: f32 = 46.0;
const KPI_STRIP_HEIGHT: f32 = 70.0;

// ---------------------------------------------------------------------------
// Fixed chrome: title bar, toolbar, KPI strip
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn view_journal_title_bar(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let mode = self.journal_account_mode();

        let logo = container(text("K").size(16).color(theme.palette().background))
            .center(Length::Fixed(26.0))
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.palette().primary.into()),
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

        let title = text("Trading Journal").size(19).color(theme.palette().text);

        let badge = container(
            text(mode.label())
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(mode.accent(&theme)),
        )
        .padding([2, 8])
        .style(move |theme: &Theme| {
            let accent = mode.accent(theme);
            container_style::Style {
                background: Some(Color { a: 0.12, ..accent }.into()),
                border: Border {
                    color: Color { a: 0.38, ..accent },
                    width: 1.0,
                    radius: JOURNAL_CHIP_RADIUS.into(),
                },
                ..Default::default()
            }
        });

        let account = text(self.journal_account_label())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(journal_dim(&theme));

        container(
            row![logo, title, badge, account, Space::new().width(Fill)]
                .spacing(10)
                .align_y(Alignment::Center),
        )
        .width(Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .padding([0, 16])
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    pub(super) fn view_journal_toolbar(&self, fills: usize, trades: usize) -> Element<'_, Message> {
        let theme = self.theme();
        let muted = journal_muted(&theme);

        let mut counts = row![
            toolbar_stat(format!("{trades} trades"), muted),
            toolbar_divider(&theme),
            toolbar_stat(format!("{fills} fills"), muted),
            toolbar_divider(&theme),
            self.journal_sync_status(&theme),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if self.connected_address.is_some() && !self.journal.loading {
            counts = counts.push(
                button(
                    text("Clear")
                        .size(10)
                        .font(crate::app_fonts::monospace_font()),
                )
                .on_press(Message::JournalClearCache)
                .padding([3, 8])
                .style(crate::journal_views::style::journal_ghost_button_style),
            );
        }

        let sort = row![
            toolbar_caption("SORT", &theme),
            segment_button(
                "Recent",
                self.journal.sort == JournalSort::TimeDesc,
                Message::JournalSortChanged(JournalSort::TimeDesc)
            ),
            segment_button(
                "PnL \u{2193}",
                self.journal.sort == JournalSort::PnlDesc,
                Message::JournalSortChanged(JournalSort::PnlDesc)
            ),
            segment_button(
                "PnL \u{2191}",
                self.journal.sort == JournalSort::PnlAsc,
                Message::JournalSortChanged(JournalSort::PnlAsc)
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center);

        let mut filter = row![
            toolbar_caption("FILTER", &theme),
            segment_button(
                "All",
                self.journal.filter == JournalFilter::All,
                Message::JournalFilterChanged(JournalFilter::All)
            ),
            segment_button(
                "Perp",
                self.journal.filter == JournalFilter::Perp,
                Message::JournalFilterChanged(JournalFilter::Perp)
            ),
            segment_button(
                "Spot",
                self.journal.filter == JournalFilter::Spot,
                Message::JournalFilterChanged(JournalFilter::Spot)
            ),
        ]
        .spacing(4)
        .align_y(Alignment::Center);
        if self
            .journal
            .trades
            .iter()
            .any(|trade| trade.coin.starts_with('#'))
        {
            filter = filter.push(segment_button(
                "Outcome",
                self.journal.filter == JournalFilter::Outcome,
                Message::JournalFilterChanged(JournalFilter::Outcome),
            ));
        }

        container(
            row![
                counts,
                Space::new().width(Fill),
                sort,
                Space::new().width(16.0),
                filter
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .height(Length::Fixed(TOOLBAR_HEIGHT))
        .padding([0, 16])
        .align_y(iced::alignment::Vertical::Center)
        .into()
    }

    fn journal_sync_status(&self, theme: &Theme) -> Element<'_, Message> {
        let label = if self.journal.loading {
            "Syncing…".to_string()
        } else if let Some(time) = self.journal.last_refresh_time {
            format!("Synced {}", helpers::format_timestamp_exact(time))
        } else {
            "Not synced".to_string()
        };
        let color = if self.journal.loading {
            theme.palette().primary
        } else if self.journal.last_refresh_time.is_some() {
            theme.palette().success
        } else {
            journal_muted(theme)
        };

        let content = text(label)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(color);

        if self.journal.loading {
            container(content).into()
        } else {
            button(content)
                .on_press(Message::JournalRefresh)
                .padding(0)
                .style(iced::widget::button::text)
                .into()
        }
    }

    pub(super) fn view_journal_kpi_strip(&self, kpis: &JournalKpis) -> Element<'_, Message> {
        let theme = self.theme();
        let denomination = self.display_denomination_context();
        let signed = |value: f64| helpers::signed_number_color(value, &theme);
        let text_color = theme.palette().text;

        let cells = [
            kpi_cell(
                "NET PNL",
                denomination.format_signed_value(kpis.net_pnl, 2),
                signed(kpis.net_pnl),
                &theme,
            ),
            kpi_cell(
                "WIN RATE",
                format!("{:.1}%", kpis.win_rate),
                text_color,
                &theme,
            ),
            kpi_cell(
                "PROFIT FACTOR",
                kpis.profit_factor
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "—".to_string()),
                text_color,
                &theme,
            ),
            kpi_cell(
                "EXPECTANCY",
                kpis.expectancy
                    .map(|value| denomination.format_signed_value(value, 2))
                    .unwrap_or_else(|| "—".to_string()),
                kpis.expectancy.map(signed).unwrap_or(text_color),
                &theme,
            ),
            kpi_cell(
                "AVG R",
                kpis.avg_r
                    .map(|value| format!("{value:+.2}R"))
                    .unwrap_or_else(|| "—".to_string()),
                kpis.avg_r.map(signed).unwrap_or(text_color),
                &theme,
            ),
            kpi_cell(
                "AVG WIN",
                kpis.avg_win
                    .map(|value| denomination.format_signed_value(value, 2))
                    .unwrap_or_else(|| "—".to_string()),
                theme.palette().success,
                &theme,
            ),
            kpi_cell(
                "AVG LOSS",
                kpis.avg_loss
                    .map(|value| denomination.format_signed_value(value, 2))
                    .unwrap_or_else(|| "—".to_string()),
                theme.palette().danger,
                &theme,
            ),
            kpi_cell(
                "FEES",
                denomination.format_value(kpis.total_fees, 2),
                theme.palette().warning,
                &theme,
            ),
        ];

        let mut strip = row![].height(Fill).align_y(Alignment::Center);
        for (index, cell) in cells.into_iter().enumerate() {
            if index > 0 {
                strip = strip.push(rule::vertical(1).style(journal_rule_style));
            }
            strip = strip.push(cell);
        }

        container(strip)
            .width(Fill)
            .height(Length::Fixed(KPI_STRIP_HEIGHT))
            .into()
    }

    fn journal_account_mode(&self) -> JournalAccountMode {
        if self.active_account_is_ghost() {
            JournalAccountMode::Ghost
        } else if self.active_account_can_trade() {
            JournalAccountMode::Trading
        } else {
            JournalAccountMode::WatchOnly
        }
    }

    fn journal_account_label(&self) -> String {
        let active_profile = self.accounts.get(self.active_account_index);
        let account_label = active_profile
            .map(|profile| profile.name.as_str())
            .unwrap_or("No account");
        let active_profile_address = active_profile
            .map(|profile| profile.wallet_address.trim())
            .filter(|address| !address.is_empty());
        let address_label = self
            .journal
            .loaded_address
            .as_deref()
            .or(active_profile_address)
            .or(self.connected_address.as_deref())
            .map(|address| self.wallet_display(address).primary)
            .unwrap_or_else(|| "Disconnected".to_string());
        format!("{account_label} · {address_label}")
    }
}

fn kpi_cell(
    label: &'static str,
    value: String,
    value_color: Color,
    theme: &Theme,
) -> Element<'static, Message> {
    container(
        column![
            text(label)
                .size(9)
                .font(crate::app_fonts::monospace_font())
                .color(journal_muted(theme)),
            text(value)
                .size(18)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
        ]
        .spacing(5),
    )
    .width(Fill)
    .height(Fill)
    .padding([0, 14])
    .center_y(Fill)
    .into()
}

fn segment_button(
    label: &'static str,
    active: bool,
    message: Message,
) -> Element<'static, Message> {
    button(
        text(label)
            .size(11)
            .font(crate::app_fonts::monospace_font()),
    )
    .on_press(message)
    .padding([4, 10])
    .style(journal_segment_style(active))
    .into()
}

fn toolbar_caption(label: &'static str, theme: &Theme) -> Element<'static, Message> {
    text(label)
        .size(9)
        .font(crate::app_fonts::monospace_font())
        .color(journal_muted(theme))
        .into()
}

fn toolbar_stat(label: String, color: Color) -> Element<'static, Message> {
    text(label)
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(color)
        .into()
}

fn toolbar_divider(theme: &Theme) -> Element<'static, Message> {
    container(Space::new())
        .width(Length::Fixed(1.0))
        .height(Length::Fixed(14.0))
        .style({
            let color = journal_hairline(theme);
            move |_theme: &Theme| container_style::Style {
                background: Some(color.into()),
                ..Default::default()
            }
        })
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JournalAccountMode {
    Trading,
    WatchOnly,
    Ghost,
}

impl JournalAccountMode {
    fn label(self) -> &'static str {
        match self {
            JournalAccountMode::Trading => "TRADING",
            JournalAccountMode::WatchOnly => "WATCH ONLY",
            JournalAccountMode::Ghost => "GHOST",
        }
    }

    fn accent(self, theme: &Theme) -> Color {
        match self {
            JournalAccountMode::Trading => theme.palette().success,
            JournalAccountMode::WatchOnly => journal_accent_soft(theme),
            JournalAccountMode::Ghost => theme.palette().primary,
        }
    }
}
