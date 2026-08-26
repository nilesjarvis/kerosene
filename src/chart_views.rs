mod editor;
mod header;
mod indicator_badges;
mod indicator_menu;
mod quick_trade;
mod skeleton;
mod toolbar;

use self::skeleton::chart_skeleton_overlay;
use crate::app_state::TradingTerminal;
use crate::chart::ChartStatus;
use crate::chart_state::{CandleFetchMode, ChartId, ChartInstance, ChartSurfaceId};
use crate::message::Message;
use iced::widget::{Space, button, canvas, column, container, rule, stack, text, text::Wrapping};
use iced::{Color, Element, Fill, Theme};

impl TradingTerminal {
    pub(crate) fn view_chart(&self, chart_id: ChartId, chart_count: usize) -> Element<'_, Message> {
        self.view_chart_surface(chart_id, chart_count, ChartSurfaceId::Docked(chart_id))
    }

    pub(crate) fn view_detached_chart_window(
        &self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
    ) -> Element<'_, Message> {
        container(self.view_chart_surface(chart_id, self.charts.len(), surface_id))
            .width(Fill)
            .height(Fill)
            .padding(4)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            })
            .into()
    }

    pub(crate) fn view_chart_surface(
        &self,
        chart_id: ChartId,
        _chart_count: usize,
        surface_id: ChartSurfaceId,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(instance) = self.charts.get(&chart_id) else {
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

        // ----- Empty chart (no symbol selected yet) -----
        if instance.symbol.is_empty() {
            let open_editor_btn = button(
                text("Select Symbol")
                    .size(14)
                    .center()
                    .width(Fill)
                    .color(theme.palette().text),
            )
            .on_press(Message::ChartOpenEditor(chart_id))
            .padding([8, 16])
            .style(|theme: &Theme, status| {
                let bg = match status {
                    button::Status::Hovered => theme.extended_palette().background.strong.color,
                    _ => theme.extended_palette().background.weak.color,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: theme.palette().text,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

            let empty_chart: Element<'_, Message> = container(open_editor_btn)
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(10)
                .into();

            let mut empty_layers = vec![empty_chart];
            if instance.editor_open {
                empty_layers.push(self.view_chart_editor(chart_id, instance));
            }

            return stack(empty_layers).width(Fill).height(Fill).into();
        }

        if self.symbol_key_is_hidden(&instance.symbol) {
            let hidden_label = if self.is_ticker_muted(&instance.symbol) {
                "Muted ticker"
            } else {
                "Outside selected market"
            };
            let content = column![
                text(hidden_label)
                    .size(13)
                    .color(theme.extended_palette().background.weak.text),
                button(text("Select Symbol").size(12))
                    .on_press(Message::ChartOpenEditor(chart_id))
                    .padding([6, 12]),
            ]
            .spacing(10)
            .align_x(iced::Alignment::Center);
            let hidden_chart: Element<'_, Message> = container(content)
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(10)
                .into();

            let mut hidden_layers = vec![hidden_chart];
            if instance.editor_open {
                hidden_layers.push(self.view_chart_editor(chart_id, instance));
            }

            return stack(hidden_layers).width(Fill).height(Fill).into();
        }

        // The toolbar elements are now integrated directly into the header row.

        // Determine status message overlay (if any)
        let status_overlay: Option<Element<'_, Message>> = match &instance.chart.status {
            ChartStatus::Loading if instance.chart.candles.is_empty() => {
                Some(chart_skeleton_overlay(&instance.chart, self.spinner_phase))
            }
            ChartStatus::Error(err) if instance.chart.candles.is_empty() => Some(
                container(
                    text(format!("Error: {err}"))
                        .size(14)
                        .width(Fill)
                        .wrapping(Wrapping::Word)
                        .color(theme.palette().danger),
                )
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(12)
                .into(),
            ),
            _ => None,
        };

        // Build header + timeframe row + canvas area.
        // Always use the same 4-element column structure so the canvas
        // stays at the same widget tree position (preserving ChartState).
        {
            let header = self.view_chart_header(chart_id, instance, surface_id);
            let toolbar = self.view_chart_toolbar(chart_id, instance, surface_id);
            let quick_order_on_surface = self.chart_surface_has_quick_order(chart_id, surface_id);
            let chart_canvas: Element<'_, Message> =
                canvas(&instance.chart).width(Fill).height(Fill).into();
            let mut canvas_layers = vec![chart_canvas];
            if let Some(indicator_badges) = self.view_chart_indicator_badges(chart_id, instance) {
                canvas_layers.push(indicator_badges);
            }
            if let Some(surface_status) =
                view_chart_surface_status_badge(instance, &theme, self.status_bar_now_ms)
            {
                canvas_layers.push(surface_status);
            }
            let chart_surface: Element<'_, Message> =
                stack(canvas_layers).width(Fill).height(Fill).into();

            // Always wrap the canvas in a stack to keep the widget tree
            // structure stable.
            let chart_base: Element<'_, Message> =
                if quick_order_on_surface && let Some(form) = &instance.quick_order {
                    let cid = chart_id;
                    self.view_quick_order_card(cid, form, surface_id, chart_surface)
                } else if let Some(overlay) = status_overlay {
                    stack![chart_surface, overlay]
                        .width(Fill)
                        .height(Fill)
                        .into()
                } else {
                    stack![chart_surface].width(Fill).height(Fill).into()
                };

            let mut chart_area_layers = vec![chart_base];
            if instance.editor_open {
                chart_area_layers.push(self.view_chart_editor(chart_id, instance));
            }
            if instance.secondary_editor_open {
                chart_area_layers.push(self.view_chart_secondary_editor(chart_id, instance));
            }

            let chart_area: Element<'_, Message> =
                stack(chart_area_layers).width(Fill).height(Fill).into();
            let chart_area = container(chart_area)
                .id(Self::chart_screenshot_canvas_id(surface_id))
                .width(Fill)
                .height(Fill);

            let padded_header = container(header).width(Fill).padding([0, 4]);
            let padded_chart_area = container(chart_area)
                .width(Fill)
                .height(Fill)
                .padding([0, 4]);

            let quick_trade_panel: Element<'_, Message> =
                if instance.macro_indicators.show_quick_trade {
                    self.view_quick_trade_panel(chart_id, instance, surface_id)
                } else {
                    Space::new().height(0.0).into()
                };
            let chart_body = column![padded_chart_area, quick_trade_panel]
                .spacing(0)
                .width(Fill)
                .height(Fill);

            let content = column![
                padded_header,
                chart_header_separator(),
                toolbar,
                chart_header_separator(),
                chart_body
            ]
            .spacing(0)
            .width(Fill)
            .height(Fill);

            let mut chart_layers: Vec<Element<'_, Message>> = vec![content.into()];

            if instance.macro_menu_open {
                chart_layers.push(self.view_macro_indicator_menu(chart_id, instance));
            }
            if self.chart_screenshot_menu_open == Some(surface_id) {
                chart_layers.push(self.view_chart_screenshot_menu(surface_id));
            }

            container(stack(chart_layers))
                .width(Fill)
                .height(Fill)
                .padding([4, 0])
                .into()
        }
    }
}

fn chart_header_separator() -> Element<'static, Message> {
    rule::horizontal(1)
        .style(|theme: &Theme| rule::Style {
            color: Color {
                a: 0.10,
                ..theme.extended_palette().background.weak.text
            },
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        })
        .into()
}

fn view_chart_surface_status_badge(
    instance: &ChartInstance,
    theme: &Theme,
    now_ms: u64,
) -> Option<Element<'static, Message>> {
    let (label, is_error) = chart_surface_status_label(instance, now_ms)?;
    let text_color = if is_error {
        theme.palette().danger
    } else {
        theme.palette().warning
    };
    let badge = container(
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(text_color),
    )
    .padding([4, 8])
    .style(move |theme: &Theme| {
        let accent = if is_error {
            theme.palette().danger
        } else {
            theme.palette().warning
        };
        container::Style {
            background: Some(
                Color {
                    a: 0.90,
                    ..theme.extended_palette().background.base.color
                }
                .into(),
            ),
            text_color: Some(accent),
            border: iced::Border {
                radius: 4.0.into(),
                width: 1.0,
                color: Color { a: 0.55, ..accent },
            },
            ..Default::default()
        }
    });

    Some(
        container(badge)
            .width(Fill)
            .height(Fill)
            .padding(8)
            .align_x(iced::alignment::Horizontal::Right)
            .align_y(iced::alignment::Vertical::Bottom)
            .into(),
    )
}

fn chart_surface_status_label(instance: &ChartInstance, now_ms: u64) -> Option<(String, bool)> {
    let mut parts = Vec::new();
    let mut is_error = false;

    if let Some((label, candle_error)) = candle_surface_status(instance, now_ms) {
        parts.push(label);
        is_error |= candle_error;
    }

    collect_chart_surface_status(
        &mut parts,
        &mut is_error,
        instance.show_liquidations,
        instance.liquidation_fetching,
        "LIQ loading",
        &instance.liquidation_status,
    );
    collect_chart_surface_status(
        &mut parts,
        &mut is_error,
        instance.show_heatmap,
        instance.heatmap_fetching,
        "HEAT loading",
        &instance.heatmap_status,
    );
    collect_chart_surface_status(
        &mut parts,
        &mut is_error,
        instance.show_earnings_markers,
        instance.earnings_fetching,
        "EARN loading",
        &instance.earnings_status,
    );
    collect_chart_surface_status(
        &mut parts,
        &mut is_error,
        instance.macro_indicators.show_funding_rate,
        instance.funding_fetch_request.is_some(),
        "Funding loading",
        &instance.chart.funding_status,
    );

    (!parts.is_empty()).then(|| (parts.join(" / "), is_error))
}

fn candle_surface_status(instance: &ChartInstance, now_ms: u64) -> Option<(String, bool)> {
    if instance.chart.candles.is_empty() {
        return None;
    }

    if let Some(request) = instance.candle_fetch_request.as_ref() {
        let label = match request.mode {
            CandleFetchMode::BackfillOlder => "Loading older candles".to_string(),
            CandleFetchMode::Refresh
                if instance.candle_history_verified_at_ms.is_none()
                    && instance.candle_ws_updated_at_ms.is_some() =>
            {
                "Live candle · verifying history".to_string()
            }
            CandleFetchMode::Refresh if instance.candle_history_verified_at_ms.is_none() => {
                "Cached candles · verifying provider".to_string()
            }
            CandleFetchMode::Refresh => "Refreshing candle history".to_string(),
        };
        return Some((label, false));
    }

    if instance.candle_fetch_error.is_some() {
        return Some((
            format!(
                "CANDLES STALE · latest bucket {}",
                candle_tail_age_label(instance, now_ms)
            ),
            true,
        ));
    }

    if instance.candle_history_verified_at_ms.is_none()
        && instance.candle_ws_updated_at_ms.is_none()
    {
        return Some(("CACHED CANDLES · NOT VERIFIED".to_string(), true));
    }

    if candle_tail_is_stale(instance, now_ms) {
        return Some((
            format!(
                "CANDLES MAY BE STALE · latest bucket {}",
                candle_tail_age_label(instance, now_ms)
            ),
            true,
        ));
    }

    if instance.candle_interval_gap {
        return Some(("Candle interval gap · provider response".to_string(), false));
    }

    None
}

fn candle_tail_is_stale(instance: &ChartInstance, now_ms: u64) -> bool {
    if instance.interval.uses_orderbook_tick_candles()
        || instance.interval == crate::timeframe::Timeframe::Mo1
        || instance.symbol.starts_with('@')
        || instance.symbol.starts_with('#')
        || instance.symbol.contains('/')
        || crate::schwab::is_schwab_symbol_key(&instance.symbol)
    {
        return false;
    }
    let Some(last) = instance.chart.candles.last() else {
        return false;
    };
    let grace_ms = (instance.interval.duration_ms() / 10).clamp(30_000, 5 * 60_000);
    let last_observed_ms = instance
        .candle_ws_updated_at_ms
        .into_iter()
        .chain(instance.candle_history_verified_at_ms)
        .max()
        .unwrap_or(0);
    now_ms > last.close_time.saturating_add(grace_ms)
        && now_ms > last_observed_ms.saturating_add(grace_ms)
}

fn candle_tail_age_label(instance: &ChartInstance, now_ms: u64) -> String {
    let close_time = instance
        .chart
        .candles
        .last()
        .map(|candle| candle.close_time)
        .unwrap_or(now_ms);
    compact_age(now_ms.saturating_sub(close_time))
}

fn compact_age(age_ms: u64) -> String {
    if age_ms < 60_000 {
        format!("{}s ago", age_ms / 1_000)
    } else if age_ms < 60 * 60_000 {
        format!("{}m ago", age_ms / 60_000)
    } else if age_ms < 24 * 60 * 60_000 {
        format!("{}h ago", age_ms / (60 * 60_000))
    } else {
        format!("{}d ago", age_ms / (24 * 60 * 60_000))
    }
}

fn collect_chart_surface_status(
    parts: &mut Vec<String>,
    is_error: &mut bool,
    enabled: bool,
    fetching: bool,
    loading_label: &str,
    status: &Option<(String, bool)>,
) {
    if !enabled {
        return;
    }
    if fetching {
        parts.push(loading_label.to_string());
    } else if let Some((label, true)) = status {
        parts.push(label.clone());
        *is_error = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Candle;
    use crate::chart_state::ChartInstance;
    use crate::config::ChartBackfillSource;
    use crate::timeframe::Timeframe;

    #[test]
    fn chart_surface_status_label_includes_enabled_overlay_errors() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.show_heatmap = true;
        instance.heatmap_status = Some(("HEAT no recent data".to_string(), true));
        instance.show_liquidations = false;
        instance.liquidation_status = Some(("LIQ stale".to_string(), true));

        let (label, is_error) = chart_surface_status_label(&instance, 1_000).expect("status label");

        assert!(is_error);
        assert_eq!(label, "HEAT no recent data");
    }

    #[test]
    fn chart_surface_status_label_includes_funding_failures_with_visible_points() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.macro_indicators.show_funding_rate = true;
        instance
            .chart
            .set_funding_status("Funding fetch failed".to_string(), true);

        let (label, is_error) = chart_surface_status_label(&instance, 1_000).expect("status label");

        assert!(is_error);
        assert_eq!(label, "Funding fetch failed");
    }

    #[test]
    fn chart_surface_status_label_uses_loading_without_marking_error() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance.show_earnings_markers = true;
        instance.earnings_fetching = true;

        let (label, is_error) = chart_surface_status_label(&instance, 1_000).expect("status label");

        assert!(!is_error);
        assert_eq!(label, "EARN loading");
    }

    #[test]
    fn cached_history_is_explicitly_unverified_while_provider_refreshes() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::M1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(60_000, 100.0)]);
        instance.candle_fetch_request = Some(crate::chart_state::CandleFetchRequest {
            chart_id: 1,
            symbol: "BTC".to_string(),
            timeframe: Timeframe::M1,
            mode: CandleFetchMode::Refresh,
            source: ChartBackfillSource::Hyperliquid,
            read_data_provider_generation: 0,
            hydromancer_key_generation: 0,
            start_ms: 0,
            end_ms: 120_000,
            attempt: 0,
        });

        let (label, is_error) =
            chart_surface_status_label(&instance, 120_000).expect("status label");

        assert!(!is_error);
        assert_eq!(label, "Cached candles · verifying provider");
    }

    #[test]
    fn unverified_cache_without_refresh_is_an_error_status() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::M1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(60_000, 100.0)]);

        let (label, is_error) =
            chart_surface_status_label(&instance, 120_000).expect("status label");

        assert!(is_error);
        assert_eq!(label, "CACHED CANDLES · NOT VERIFIED");
    }

    #[test]
    fn continuous_market_tail_becomes_visibly_stale_without_updates() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::M1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(60_000, 100.0)]);
        instance.candle_history_verified_at_ms = Some(120_000);

        let (label, is_error) =
            chart_surface_status_label(&instance, 300_000).expect("status label");

        assert!(is_error);
        assert!(label.contains("CANDLES MAY BE STALE"), "{label}");
    }

    #[test]
    fn provider_interval_gap_is_visible_without_reclassifying_prices_as_stale() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(3_600_000, 100.0)]);
        instance.candle_history_verified_at_ms = Some(3_700_000);
        instance.candle_interval_gap = true;

        let (label, is_error) =
            chart_surface_status_label(&instance, 3_700_000).expect("status label");

        assert!(!is_error);
        assert_eq!(label, "Candle interval gap · provider response");
    }

    #[test]
    fn interval_gap_does_not_mask_a_stale_continuous_market_tail() {
        let mut instance = ChartInstance::new(1, "BTC".to_string(), Timeframe::H1);
        instance
            .chart
            .set_candles(vec![Candle::test_flat(3_600_000, 100.0)]);
        instance.candle_history_verified_at_ms = Some(3_700_000);
        instance.candle_interval_gap = true;

        let (label, is_error) =
            chart_surface_status_label(&instance, 4_100_000).expect("status label");

        assert!(is_error);
        assert!(label.contains("CANDLES MAY BE STALE"), "{label}");
    }
}
