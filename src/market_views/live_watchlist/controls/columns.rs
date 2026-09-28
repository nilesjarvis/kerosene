use crate::app_state::TradingTerminal;
use crate::config;
use crate::helpers;
use crate::market_state::{LiveWatchlistId, LiveWatchlistInstance};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{
    button, checkbox, column, container, pick_list, row, rule, scrollable, text, text_input,
    tooltip,
};
use iced::{Color, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_settings_button(
        &self,
        id: LiveWatchlistId,
        menu_open: bool,
    ) -> Element<'static, Message> {
        tooltip(
            button(text("\u{2699}").size(13).center())
                .on_press(Message::ToggleLiveWatchlistSettings(id))
                .padding([2, 7])
                .style(move |theme: &Theme, status| {
                    let bg = match (menu_open, status) {
                        (_, button::Status::Hovered) => {
                            theme.extended_palette().background.strong.color
                        }
                        (true, _) => theme.extended_palette().background.strong.color,
                        (false, _) => theme.extended_palette().background.weak.color,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: if menu_open {
                            theme.palette().primary
                        } else {
                            theme.palette().text
                        },
                        border: iced::Border {
                            radius: 3.0.into(),
                            width: if menu_open { 1.0 } else { 0.0 },
                            color: Color {
                                a: 0.45,
                                ..theme.palette().primary
                            },
                        },
                        ..Default::default()
                    }
                }),
            text("Watchlist settings").size(10),
            tooltip::Position::Top,
        )
        .into()
    }

    pub(in crate::market_views::live_watchlist) fn view_live_watchlist_settings_dropdown<'a>(
        &'a self,
        id: LiveWatchlistId,
        watchlist: &'a LiveWatchlistInstance,
    ) -> Element<'a, Message> {
        let preset_id = watchlist.preset_id;
        let visible_columns = &watchlist.visible_columns;
        let theme = self.theme();
        let mut column_controls = column![].spacing(6).padding(6).width(Fill);
        if let Some(preset) = preset_id.and_then(|preset_id| self.watchlist_preset(preset_id)) {
            let preset_id = preset.id;
            let mut actions = row![
                button(text("New").size(10))
                    .on_press(Message::LiveWatchlistCreatePreset(id))
                    .padding([3, 7]),
            ]
            .spacing(4);
            if self.watchlist_presets.len() > 1 {
                actions = actions.push(
                    button(text("Delete").size(10).color(theme.palette().danger))
                        .on_press(Message::WatchlistPresetDelete(preset_id))
                        .padding([3, 7]),
                );
            }
            column_controls = column_controls
                .push(
                    text("Preset name")
                        .size(10)
                        .font(crate::app_fonts::monospace_font())
                        .color(theme.extended_palette().background.weak.text),
                )
                .push(
                    text_input("Watchlist name", &preset.name)
                        .style(helpers::text_input_style)
                        .on_input(move |name| Message::WatchlistPresetNameChanged(preset_id, name))
                        .size(11)
                        .padding([4, 7]),
                )
                .push(actions)
                .push(rule::horizontal(1));
        }
        column_controls = column_controls.push(
            text("Columns")
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text),
        );
        for column in config::LiveWatchlistColumn::ALL {
            let enabled = visible_columns.contains(&column);
            column_controls = column_controls.push(
                checkbox(enabled)
                    .label(column.label())
                    .on_toggle(move |checked| {
                        Message::LiveWatchlistColumnToggled(id, column, checked)
                    })
                    .size(12)
                    .spacing(5)
                    .width(Fill)
                    .text_size(10)
                    .font(crate::app_fonts::monospace_font()),
            );
        }

        if visible_columns.contains(&config::LiveWatchlistColumn::EmaDistance) {
            let valid_period = watchlist
                .ema_period_input
                .parse::<usize>()
                .ok()
                .filter(|period| (1..=config::LiveWatchlistEmaConfig::MAX_PERIOD).contains(period));
            column_controls = column_controls
                .push(rule::horizontal(1))
                .push(text("EMA period (1–1000)").size(10))
                .push(
                    row![
                        text_input("20", &watchlist.ema_period_input)
                            .on_input(move |input| Message::LiveWatchlistEmaPeriodInputChanged(
                                id, input
                            ))
                            .on_submit(Message::LiveWatchlistEmaPeriodApplied(id))
                            .style(helpers::text_input_style)
                            .size(11)
                            .padding([4, 7]),
                        button(text("Apply").size(10))
                            .on_press_maybe(
                                valid_period.map(|_| Message::LiveWatchlistEmaPeriodApplied(id))
                            )
                            .padding([4, 7]),
                    ]
                    .spacing(6),
                )
                .push(text("EMA timeframe").size(10))
                .push(
                    pick_list(
                        config::LiveWatchlistEmaConfig::TIMEFRAMES,
                        Some(watchlist.ema.timeframe.as_str()),
                        move |timeframe| {
                            Message::LiveWatchlistEmaTimeframeChanged(id, timeframe.to_string())
                        },
                    )
                    .text_size(11)
                    .padding([4, 7])
                    .width(Fill),
                );
        }

        container(scrollable(column_controls).height(Length::Shrink))
            .max_height(460)
            .width(Length::Fixed(210.0))
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.weak.color,
                },
                ..Default::default()
            })
            .into()
    }
}
