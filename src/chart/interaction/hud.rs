use super::super::state::{HudMarketSide, HudOrderKind};
use super::super::{CandlestickChart, ChartState};
use crate::message::Message;
use crate::sound::HudUiSound;
use iced::keyboard::{self, key};
use iced::widget::canvas;

// ---------------------------------------------------------------------------
// HUD Game-Mode Input
// ---------------------------------------------------------------------------

impl CandlestickChart {
    pub(super) fn handle_hud_key_pressed(
        &self,
        state: &mut ChartState,
        key: keyboard::Key<&str>,
        text: Option<&str>,
        modifiers: keyboard::Modifiers,
    ) -> Option<canvas::Action<Message>> {
        if !self.hud_game_mode_enabled()
            || state.cursor_position.is_none()
            || modifiers.control()
            || modifiers.alt()
            || modifiers.logo()
        {
            return None;
        }

        if state.hud_size_editing {
            return handle_hud_size_key(state, key, text).then(redraw_and_capture);
        }

        match key {
            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("a") => Some(
                canvas::Action::publish(Message::ChartHudArmToggled(self.id, self.surface_id))
                    .and_capture(),
            ),
            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("l") => {
                Some(self.set_hud_order_kind(state, HudOrderKind::Limit))
            }
            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("m") => {
                Some(self.set_hud_order_kind(state, HudOrderKind::Market))
            }
            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("y") => {
                Some(self.set_hud_market_side(state, HudMarketSide::Long))
            }
            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("x") => {
                Some(self.set_hud_market_side(state, HudMarketSide::Short))
            }
            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("s") => {
                state.hud_size_editing = true;
                state.hud_size_replace_on_type = true;
                Some(redraw_and_capture())
            }
            keyboard::Key::Character(value) if value.eq_ignore_ascii_case("c") => {
                state.hud_follow_price = !state.hud_follow_price;
                self.candle_cache.clear();
                Some(redraw_and_capture())
            }
            keyboard::Key::Named(key::Named::ArrowUp) => {
                Some(self.set_hud_market_side(state, HudMarketSide::Long))
            }
            keyboard::Key::Named(key::Named::ArrowDown) => {
                Some(self.set_hud_market_side(state, HudMarketSide::Short))
            }
            _ => None,
        }
    }

    pub(super) fn handle_hud_size_scroll(
        &self,
        state: &mut ChartState,
        dy: f32,
    ) -> Option<canvas::Action<Message>> {
        if !self.hud_game_mode_enabled() || dy == 0.0 || !dy.is_finite() {
            return None;
        }

        let current = hud_size_value(&state.hud_size_input).unwrap_or(1.0);
        let step = hud_size_scroll_step(current);
        let direction = dy.signum();
        let ticks = dy.abs().ceil().max(1.0);
        let next = (current + direction * step * ticks).max(0.0);
        let next_input = format_hud_size(next);
        let changed = state.hud_size_input != next_input;
        state.hud_size_input = next_input;
        state.hud_size_editing = false;
        state.hud_size_replace_on_type = false;
        state.hud_size_scroll_bias = direction;

        let sound = if direction > 0.0 {
            HudUiSound::SizeUp
        } else {
            HudUiSound::SizeDown
        };
        Some(self.hud_control_action(changed, sound))
    }

    pub(in crate::chart) fn hud_game_mode_enabled(&self) -> bool {
        self.crosshair_style.is_game_hud()
    }

    pub(super) fn hover_state_action(
        &self,
        order_cancel_hover_oid: Option<u64>,
        hovering: bool,
        earnings_marker_time_ms: Option<u64>,
    ) -> Option<canvas::Action<Message>> {
        let cancel_hover_changed = self.hover_order_cancel_oid != order_cancel_hover_oid;
        let earnings_hover_changed = self.hover_earnings_marker_time_ms != earnings_marker_time_ms;
        let hud_activity_needed = self.hud_game_mode_enabled()
            && (self.hud_hovering != hovering || (self.hud_armed && hovering));
        (cancel_hover_changed || earnings_hover_changed || hud_activity_needed).then(|| {
            canvas::Action::publish(Message::ChartHoverStateChanged(
                self.id,
                self.surface_id,
                order_cancel_hover_oid,
                hovering,
                earnings_marker_time_ms,
            ))
        })
    }
}

fn redraw_and_capture() -> canvas::Action<Message> {
    canvas::Action::request_redraw().and_capture()
}

impl CandlestickChart {
    /// Publishing routes through the app update, which plays the interface
    /// click (on change) and pops the weapon-selector list on this chart.
    fn hud_control_action(&self, changed: bool, sound: HudUiSound) -> canvas::Action<Message> {
        canvas::Action::publish(Message::ChartHudControlChanged(
            self.id,
            self.surface_id,
            sound,
            changed,
        ))
        .and_capture()
    }

    fn set_hud_order_kind(
        &self,
        state: &mut ChartState,
        kind: HudOrderKind,
    ) -> canvas::Action<Message> {
        let changed = state.hud_order_kind != kind;
        state.hud_order_kind = kind;
        let sound = match kind {
            HudOrderKind::Limit => HudUiSound::ModeLimit,
            HudOrderKind::Market => HudUiSound::ModeMarket,
        };
        self.hud_control_action(changed, sound)
    }

    fn set_hud_market_side(
        &self,
        state: &mut ChartState,
        side: HudMarketSide,
    ) -> canvas::Action<Message> {
        let changed = state.hud_market_side != side;
        state.hud_market_side = side;
        let sound = match side {
            HudMarketSide::Long => HudUiSound::SideLong,
            HudMarketSide::Short => HudUiSound::SideShort,
        };
        self.hud_control_action(changed, sound)
    }
}

fn handle_hud_size_key(
    state: &mut ChartState,
    key: keyboard::Key<&str>,
    text: Option<&str>,
) -> bool {
    match key {
        keyboard::Key::Named(key::Named::Enter) => {
            finish_hud_size_edit(state);
            true
        }
        keyboard::Key::Named(key::Named::Escape) => {
            finish_hud_size_edit(state);
            true
        }
        keyboard::Key::Named(key::Named::Backspace) => {
            if state.hud_size_replace_on_type {
                state.hud_size_input.clear();
                state.hud_size_replace_on_type = false;
            } else {
                state.hud_size_input.pop();
            }
            true
        }
        keyboard::Key::Named(key::Named::Delete) => {
            state.hud_size_input.clear();
            state.hud_size_replace_on_type = false;
            true
        }
        _ => text.is_some_and(|text| append_hud_size_text(state, text)),
    }
}

fn append_hud_size_text(state: &mut ChartState, text: &str) -> bool {
    let mut changed = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            if state.hud_size_replace_on_type {
                state.hud_size_input.clear();
                state.hud_size_replace_on_type = false;
            }
            state.hud_size_input.push(ch);
            changed = true;
        } else if ch == '.' && !state.hud_size_input.contains('.') {
            if state.hud_size_replace_on_type {
                state.hud_size_input.clear();
                state.hud_size_replace_on_type = false;
            }
            if state.hud_size_input.is_empty() {
                state.hud_size_input.push('0');
            }
            state.hud_size_input.push('.');
            changed = true;
        }
    }

    if changed {
        normalize_hud_size_input(state);
    }
    changed
}

fn normalize_hud_size_input(state: &mut ChartState) {
    const MAX_LEN: usize = 12;
    if state.hud_size_input.len() > MAX_LEN {
        state.hud_size_input.truncate(MAX_LEN);
    }
}

fn finish_hud_size_edit(state: &mut ChartState) {
    if state.hud_size_input.trim().is_empty() {
        state.hud_size_input = "0".to_string();
    }
    state.hud_size_editing = false;
    state.hud_size_replace_on_type = false;
}

fn hud_size_value(input: &str) -> Option<f32> {
    let value = input.trim().parse::<f32>().ok()?;
    value.is_finite().then_some(value.max(0.0))
}

fn hud_size_scroll_step(current: f32) -> f32 {
    if current >= 100.0 {
        10.0
    } else if current >= 10.0 {
        1.0
    } else if current >= 1.0 {
        0.1
    } else {
        0.01
    }
}

fn format_hud_size(value: f32) -> String {
    let mut label = if value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else if value >= 1.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.4}")
    };

    while label.contains('.') && label.ends_with('0') {
        label.pop();
    }
    if label.ends_with('.') {
        label.pop();
    }
    label
}

#[cfg(test)]
mod tests {
    use super::{format_hud_size, hud_size_scroll_step};
    use crate::chart::{CandlestickChart, ChartState};
    use crate::config::ChartCrosshairStyle;
    use crate::message::Message;
    use crate::sound::HudUiSound;
    use iced::{Point, keyboard};

    fn pressed_control(
        chart: &CandlestickChart,
        state: &mut ChartState,
        key: &str,
    ) -> Option<(HudUiSound, bool)> {
        let action = chart
            .handle_hud_key_pressed(
                state,
                keyboard::Key::Character(key),
                Some(key),
                keyboard::Modifiers::NONE,
            )
            .expect("selector key should be handled");
        let (message, _, _) = action.into_inner();
        match message {
            Some(Message::ChartHudControlChanged(id, surface_id, control, changed)) => {
                assert_eq!(id, chart.id);
                assert_eq!(surface_id, chart.surface_id);
                Some((control, changed))
            }
            Some(other) => panic!("expected ChartHudControlChanged, got {other:?}"),
            None => None,
        }
    }

    #[test]
    fn mode_key_always_pops_the_selector_but_clicks_only_on_change() {
        let mut chart = CandlestickChart::new(1);
        chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        let mut state = ChartState {
            cursor_position: Some(Point::ORIGIN),
            ..ChartState::default()
        };

        // Default mode is Limit, so M is a change and clicks.
        assert_eq!(
            pressed_control(&chart, &mut state, "m"),
            Some((HudUiSound::ModeMarket, true))
        );

        // Repeating M still publishes (the weapon list re-opens, Battlefield
        // style) but is flagged unchanged so no click replays.
        assert_eq!(
            pressed_control(&chart, &mut state, "m"),
            Some((HudUiSound::ModeMarket, false))
        );
    }

    #[test]
    fn side_keys_publish_direction_coded_controls() {
        let mut chart = CandlestickChart::new(1);
        chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        let mut state = ChartState {
            cursor_position: Some(Point::ORIGIN),
            ..ChartState::default()
        };

        // Default side is Long, so X is a change.
        assert_eq!(
            pressed_control(&chart, &mut state, "x"),
            Some((HudUiSound::SideShort, true))
        );
        assert_eq!(
            pressed_control(&chart, &mut state, "y"),
            Some((HudUiSound::SideLong, true))
        );
    }

    #[test]
    fn size_scroll_publishes_direction_coded_ticks() {
        let chart = {
            let mut chart = CandlestickChart::new(1);
            chart.set_crosshair_style(ChartCrosshairStyle::Hud);
            chart
        };
        let mut state = ChartState::default();

        let action = chart
            .handle_hud_size_scroll(&mut state, 1.0)
            .expect("scroll up should be handled");
        let (message, _, _) = action.into_inner();
        assert!(matches!(
            message,
            Some(Message::ChartHudControlChanged(
                _,
                _,
                HudUiSound::SizeUp,
                true
            ))
        ));

        let action = chart
            .handle_hud_size_scroll(&mut state, -1.0)
            .expect("scroll down should be handled");
        let (message, _, _) = action.into_inner();
        assert!(matches!(
            message,
            Some(Message::ChartHudControlChanged(
                _,
                _,
                HudUiSound::SizeDown,
                true
            ))
        ));
    }

    #[test]
    fn hud_size_format_trims_fractional_padding() {
        assert_eq!(format_hud_size(1.0), "1");
        assert_eq!(format_hud_size(1.2), "1.2");
        assert_eq!(format_hud_size(0.125), "0.125");
        assert_eq!(format_hud_size(123.0), "123");
    }

    #[test]
    fn hud_size_scroll_step_scales_with_size() {
        assert_eq!(hud_size_scroll_step(0.5), 0.01);
        assert_eq!(hud_size_scroll_step(1.0), 0.1);
        assert_eq!(hud_size_scroll_step(10.0), 1.0);
        assert_eq!(hud_size_scroll_step(100.0), 10.0);
    }

    #[test]
    fn c_key_toggles_hud_follow_price() {
        let mut chart = CandlestickChart::new(1);
        chart.set_crosshair_style(ChartCrosshairStyle::Hud);
        let mut state = ChartState {
            cursor_position: Some(Point::ORIGIN),
            ..ChartState::default()
        };

        let action = chart.handle_hud_key_pressed(
            &mut state,
            keyboard::Key::Character("c"),
            Some("c"),
            keyboard::Modifiers::NONE,
        );

        assert!(action.is_some());
        assert!(state.hud_follow_price);

        let action = chart.handle_hud_key_pressed(
            &mut state,
            keyboard::Key::Character("c"),
            Some("c"),
            keyboard::Modifiers::NONE,
        );

        assert!(action.is_some());
        assert!(!state.hud_follow_price);
    }
}
