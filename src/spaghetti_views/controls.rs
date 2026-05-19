use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::spaghetti;
use crate::spaghetti_state::{AnchorGranularityOption, SpaghettiChartId, SpaghettiChartInstance};
use crate::timeframe::TIMEFRAME_OPTIONS;
use iced::widget::{Row, button, container, pick_list, row, rule, text};
use iced::{Color, Element, Fill, Length, Theme, color};

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

            let mode_text = if let Some(tf) = inst.session_granularity {
                format!("Anchor mode: manual {}", tf.label())
            } else {
                "Anchor mode: auto".to_string()
            };
            r.push(spaghetti_controls_separator())
                .push(spaghetti_controls_status_label(mode_text))
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

fn spaghetti_controls_strip<'a>(content: Row<'a, Message>) -> Element<'a, Message> {
    container(content.width(Fill).wrap().vertical_spacing(0))
        .width(Fill)
        .style(|theme: &Theme| {
            let background = Color {
                a: 0.04,
                ..theme.extended_palette().background.weak.color
            };
            container::Style {
                background: Some(background.into()),
                ..Default::default()
            }
        })
        .into()
}

fn spaghetti_controls_button(
    label: &'static str,
    active: bool,
    msg: Message,
) -> Element<'static, Message> {
    button(text(label).size(11).center())
        .on_press(msg)
        .padding([3, 8])
        .style(move |theme: &Theme, status| spaghetti_controls_button_style(theme, status, active))
        .into()
}

fn reload_button(id: SpaghettiChartId) -> Element<'static, Message> {
    button(text("\u{27F3}").size(12))
        .on_press(Message::SpaghettiReload(id))
        .padding([3, 8])
        .style(|theme: &Theme, status| spaghetti_controls_button_style(theme, status, false))
        .into()
}

fn reset_view_button(id: SpaghettiChartId) -> Element<'static, Message> {
    spaghetti_controls_button("Reset View", false, Message::SpaghettiResetView(id))
}

fn spaghetti_controls_separator() -> Element<'static, Message> {
    container(rule::vertical(1).style(|theme: &Theme| rule::Style {
        color: Color {
            a: 0.12,
            ..theme.extended_palette().background.weak.text
        },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }))
    .height(14)
    .width(1)
    .into()
}

fn spaghetti_controls_status_label(label: String) -> Element<'static, Message> {
    container(text(label).size(10).color(color!(0x8e9cc2)))
        .padding([3, 8])
        .into()
}

fn spaghetti_controls_button_style(
    theme: &Theme,
    status: button::Status,
    active: bool,
) -> button::Style {
    let background = if active {
        Color {
            a: 0.10,
            ..theme.palette().primary
        }
    } else {
        match status {
            button::Status::Hovered => Color {
                a: 0.55,
                ..theme.extended_palette().background.strong.color
            },
            _ => Color::TRANSPARENT,
        }
    };

    button::Style {
        background: Some(background.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.extended_palette().background.weak.text
        },
        border: iced::Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn spaghetti_controls_pick_list_style(
    theme: &Theme,
    status: pick_list::Status,
) -> pick_list::Style {
    let background = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => Color {
            a: 0.55,
            ..theme.extended_palette().background.strong.color
        },
        pick_list::Status::Active => Color::TRANSPARENT,
    };

    pick_list::Style {
        text_color: theme.extended_palette().background.weak.text,
        placeholder_color: theme.extended_palette().background.weak.text,
        handle_color: theme.extended_palette().background.weak.text,
        background: background.into(),
        border: iced::Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}
