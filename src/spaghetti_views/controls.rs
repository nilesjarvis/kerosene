use crate::app_state::TradingTerminal;
use crate::helpers::timeframe_button;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{AnchorGranularityOption, SpaghettiChartId, SpaghettiChartInstance};
use crate::timeframe::TIMEFRAME_OPTIONS;
use iced::widget::{button, container, pick_list, row, rule, text};
use iced::{Color, Element, Theme, color};

impl TradingTerminal {
    pub(super) fn view_spaghetti_controls(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'static, Message> {
        let active_tf = inst.interval;
        let session_locked = inst.canvas.active_session.is_some();
        let mut tf_row = if session_locked {
            let mut r = row![].spacing(8).align_y(iced::Alignment::Center);

            let mut options = vec![AnchorGranularityOption::Auto];
            let session_span = inst.canvas.active_session.map(|session| {
                let now_ms = Self::now_ms();
                now_ms.saturating_sub(session.last_open_ms(now_ms))
            });
            for &tf in TIMEFRAME_OPTIONS {
                if session_span
                    .is_none_or(|span| Self::spaghetti_session_granularity_fits(span, tf))
                {
                    options.push(AnchorGranularityOption::Manual(tf));
                }
            }

            let selected = match inst.session_granularity {
                Some(tf) if options.contains(&AnchorGranularityOption::Manual(tf)) => {
                    AnchorGranularityOption::Manual(tf)
                }
                _ => AnchorGranularityOption::Auto,
            };

            let picker = pick_list(options, Some(selected), move |opt| match opt {
                AnchorGranularityOption::Auto => Message::SpaghettiSetSessionGranularityAuto(id),
                AnchorGranularityOption::Manual(tf) => Message::SpaghettiSwitchTimeframe(id, tf),
            })
            .width(iced::Length::Shrink)
            .padding([2, 8])
            .text_size(11);

            let reload_btn = reload_button(id);
            let reset_view_btn = reset_view_button(id);
            r = r.push(picker).push(reload_btn).push(reset_view_btn);

            let mode_text = if let Some(tf) = inst.session_granularity {
                format!("Anchor mode: manual {}", tf.label())
            } else {
                "Anchor mode: auto".to_string()
            };
            r.push(text(mode_text).size(10).color(color!(0x8e9cc2)))
        } else {
            let picker = pick_list(TIMEFRAME_OPTIONS, Some(active_tf), move |tf| {
                Message::SpaghettiSwitchTimeframe(id, tf)
            })
            .width(iced::Length::Shrink)
            .padding([2, 8])
            .text_size(11);

            row![picker, reload_button(id), reset_view_button(id)]
                .spacing(4)
                .align_y(iced::Alignment::Center)
        };

        if !inst.pair_mode {
            let active_session = inst.canvas.active_session;
            tf_row = tf_row.push(container(rule::vertical(1)).height(16).width(8));

            for &session in spaghetti::SESSION_OPTIONS {
                let is_active = active_session == Some(session);
                tf_row = tf_row.push(timeframe_button(
                    session.label(),
                    is_active,
                    Message::SpaghettiSetSession(id, if is_active { None } else { Some(session) }),
                ));
            }
        } else {
            tf_row = tf_row
                .push(container(rule::vertical(1)).height(16).width(8))
                .push(timeframe_button(
                    "LINE",
                    !inst.pair_candle_mode,
                    Message::PairSetCandleMode(id, false),
                ))
                .push(timeframe_button(
                    "CANDLE",
                    inst.pair_candle_mode,
                    Message::PairSetCandleMode(id, true),
                ));

            let has_two = inst.canvas.series.len() >= 2;
            let pair_label = if has_two {
                format!(
                    "{} / {}",
                    inst.canvas.series[0].display, inst.canvas.series[1].display
                )
            } else {
                "Add two symbols".to_string()
            };
            tf_row = tf_row.push(text(pair_label).size(10).color(color!(0x8e9cc2)));
        }

        tf_row.into()
    }
}

fn reload_button(id: SpaghettiChartId) -> button::Button<'static, Message> {
    button(text("\u{27F3}").size(12))
        .on_press(Message::SpaghettiReload(id))
        .padding([2, 4])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                ..Default::default()
            }
        })
}

fn reset_view_button(id: SpaghettiChartId) -> Element<'static, Message> {
    timeframe_button("Reset View", false, Message::SpaghettiResetView(id))
}
