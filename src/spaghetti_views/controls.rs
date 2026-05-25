mod components;

use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{AnchorGranularityOption, SpaghettiChartId, SpaghettiChartInstance};
use crate::timeframe::TIMEFRAME_OPTIONS;
use components::{
    reload_button, reset_view_button, spaghetti_controls_button,
    spaghetti_controls_pick_list_style, spaghetti_controls_separator,
    spaghetti_controls_status_label, spaghetti_controls_strip,
};
use iced::widget::{pick_list, row};
use iced::{Element, Length};

impl TradingTerminal {
    pub(super) fn view_spaghetti_controls(
        &self,
        id: SpaghettiChartId,
        inst: &SpaghettiChartInstance,
    ) -> Element<'static, Message> {
        let active_tf = inst.interval;
        let session_locked = inst.canvas.active_session.is_some();
        let mut tf_row = if session_locked {
            let mut r = row![].spacing(0).align_y(iced::Alignment::Center);

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
            .width(Length::Shrink)
            .padding([3, 8])
            .text_size(11)
            .style(spaghetti_controls_pick_list_style);

            let reload_btn = reload_button(id);
            let reset_view_btn = reset_view_button(id);
            r = r
                .push(picker)
                .push(spaghetti_controls_separator())
                .push(reload_btn)
                .push(spaghetti_controls_separator())
                .push(reset_view_btn);

            if let Some(tf) = inst.session_granularity {
                r = r
                    .push(spaghetti_controls_separator())
                    .push(spaghetti_controls_status_label(format!(
                        "Anchor mode: manual {}",
                        tf.label()
                    )));
            }

            r
        } else {
            let picker = pick_list(TIMEFRAME_OPTIONS, Some(active_tf), move |tf| {
                Message::SpaghettiSwitchTimeframe(id, tf)
            })
            .width(Length::Shrink)
            .padding([3, 8])
            .text_size(11)
            .style(spaghetti_controls_pick_list_style);

            row![
                picker,
                spaghetti_controls_separator(),
                reload_button(id),
                spaghetti_controls_separator(),
                reset_view_button(id),
            ]
            .spacing(0)
            .align_y(iced::Alignment::Center)
        };

        if !inst.pair_mode {
            let active_session = inst.canvas.active_session;

            for &session in spaghetti::SESSION_OPTIONS {
                let is_active = active_session == Some(session);
                tf_row =
                    tf_row
                        .push(spaghetti_controls_separator())
                        .push(spaghetti_controls_button(
                            session.label(),
                            is_active,
                            Message::SpaghettiSetSession(
                                id,
                                if is_active { None } else { Some(session) },
                            ),
                        ));
            }
        } else {
            tf_row = tf_row
                .push(spaghetti_controls_separator())
                .push(spaghetti_controls_button(
                    "LINE",
                    !inst.pair_candle_mode,
                    Message::PairSetCandleMode(id, false),
                ))
                .push(spaghetti_controls_separator())
                .push(spaghetti_controls_button(
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
            tf_row = tf_row
                .push(spaghetti_controls_separator())
                .push(spaghetti_controls_status_label(pair_label));
        }

        spaghetti_controls_strip(tf_row)
    }
}
