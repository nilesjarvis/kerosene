use super::formatting::{format_amount, format_hype, format_pct, format_usd_value};
use super::metrics::{metric, metric_grid};
use crate::denomination::DisplayDenominationContext;
use crate::helpers::signed_number_color;
use crate::hype_etf_state::{HypeEtfFund, HypeEtfTotals, HypeEtfView};
use crate::message::Message;

use iced::widget::{column, container, row, text};
use iced::{Color, Element, Fill, Theme};

// ---------------------------------------------------------------------------
// HYPE ETF Sections
// ---------------------------------------------------------------------------

pub(super) fn summary_section(
    theme: &Theme,
    view: HypeEtfView,
    totals: HypeEtfTotals,
    fund_count: usize,
    available_width: f32,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let title = if view == HypeEtfView::All {
        format!("All HYPE ETFs ({fund_count})")
    } else {
        view.label().to_string()
    };

    column![
        text(title).size(12).color(theme.palette().text),
        metric_grid(
            vec![
                metric(
                    "Total Assets",
                    format_usd_value(totals.net_assets_usd, 2, denomination),
                    None
                ),
                metric("HYPE Exposure", format_hype(totals.hype_exposure), None),
                metric("Share Volume", format_amount(totals.daily_volume), None),
                metric(
                    "Premium/Discount",
                    format_pct(totals.weighted_premium_discount_pct),
                    totals
                        .weighted_premium_discount_pct
                        .map(|value| signed_number_color(value, theme)),
                ),
                metric(
                    "30D Spread",
                    format_pct(totals.weighted_median_spread_pct),
                    None,
                ),
                metric(
                    "Fund Shares",
                    format_amount(totals.shares_outstanding),
                    None,
                ),
            ],
            available_width,
        )
    ]
    .spacing(6)
    .into()
}

pub(super) fn fund_section(
    theme: &Theme,
    fund: &HypeEtfFund,
    available_width: f32,
    denomination: &DisplayDenominationContext,
) -> Element<'static, Message> {
    let title = row![
        text(fund.ticker.label())
            .size(12)
            .color(theme.palette().primary),
        text(fund.ticker.name())
            .size(11)
            .color(theme.extended_palette().background.weak.text)
            .width(Fill),
        text(
            fund.as_of_date
                .clone()
                .unwrap_or_else(|| "date n/a".to_string()),
        )
        .size(10)
        .color(theme.extended_palette().background.weak.text),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center);

    let mut items = vec![
        metric(
            "Assets",
            format_usd_value(fund.net_assets_usd, 2, denomination),
            None,
        ),
        metric("HYPE", format_hype(fund.hype_exposure), None),
        metric(
            "NAV",
            format_usd_value(fund.nav_per_share, 2, denomination),
            None,
        ),
        metric(
            "Market",
            format_usd_value(fund.market_price, 2, denomination),
            None,
        ),
        metric(
            "NAV 1D",
            format_pct(fund.nav_change_pct),
            fund.nav_change_pct
                .map(|value| signed_number_color(value, theme)),
        ),
        metric(
            "Market 1D",
            format_pct(fund.market_price_change_pct),
            fund.market_price_change_pct
                .map(|value| signed_number_color(value, theme)),
        ),
        metric(
            "Premium/Discount",
            format_pct(fund.premium_discount_pct),
            fund.premium_discount_pct
                .map(|value| signed_number_color(value, theme)),
        ),
        metric("Volume", format_amount(fund.daily_volume), None),
        metric("30D Spread", format_pct(fund.median_spread_pct), None),
        metric(
            "HYPE Px",
            format_usd_value(fund.hype_reference_price, 2, denomination),
            None,
        ),
    ];

    if fund.thirty_day_volume.is_some() {
        items.push(metric(
            "30D Volume",
            format_amount(fund.thirty_day_volume),
            None,
        ));
    }
    if fund.staking_net_rate_pct.is_some() {
        items.push(metric(
            "Net Staking",
            format_pct(fund.staking_net_rate_pct),
            None,
        ));
    }
    if fund.staking_current_pct.is_some() {
        items.push(metric(
            "Assets Staked",
            format_pct(fund.staking_current_pct),
            None,
        ));
    }

    let mut section = column![title, metric_grid(items, available_width)].spacing(6);
    if let Some(updated_at) = &fund.updated_at {
        section = section.push(
            text(format!("Updated {updated_at}"))
                .size(10)
                .color(theme.extended_palette().background.weak.text),
        );
    }

    container(section)
        .width(Fill)
        .padding([8, 10])
        .style(move |theme: &Theme| container::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color {
                    a: 0.22,
                    ..theme.extended_palette().background.strong.color
                },
            },
            ..Default::default()
        })
        .into()
}
