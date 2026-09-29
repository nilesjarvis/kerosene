use crate::hype_unstaking_state::{HYPE_CORE_WEI_PER_TOKEN, HypeUnstakingEvent};

use iced::widget::container;
use iced::{Color, Theme};

// ---------------------------------------------------------------------------
// Amount Heat Styling
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub(super) struct HypeUnstakingAmountScale {
    min_ln: f64,
    max_ln: f64,
}

#[derive(Debug, Clone, Copy)]
struct HypeUnstakingAmountHeat {
    fill_pct: f32,
    alpha: f32,
}

pub(super) fn hype_unstaking_row_style(
    theme: &Theme,
    amount_wei: u64,
    scale: HypeUnstakingAmountScale,
    index: usize,
) -> container::Style {
    use iced::gradient;

    let heat = scale.heat(amount_wei);
    let mut start = theme.palette().primary;
    start.a = heat.alpha;
    let mut end = theme.palette().primary;
    end.a = heat.alpha * 0.55;
    let base = hype_unstaking_row_base_background(theme, index);

    let fill = heat.fill_pct.clamp(0.0, 1.0);
    let background = if fill >= 0.999 {
        gradient::Linear::new(iced::Degrees(90.0))
            .add_stop(0.0, start)
            .add_stop(1.0, end)
    } else {
        gradient::Linear::new(iced::Degrees(90.0))
            .add_stop(0.0, start)
            .add_stop(fill, end)
            .add_stop((fill + 0.0001).min(1.0), base)
            .add_stop(1.0, base)
    };

    container::Style {
        background: Some(background.into()),
        ..Default::default()
    }
}

fn hype_unstaking_row_base_background(theme: &Theme, index: usize) -> Color {
    if index.is_multiple_of(2) {
        Color {
            a: 0.12,
            ..theme.extended_palette().background.strong.color
        }
    } else {
        Color {
            a: 0.02,
            ..theme.extended_palette().background.weak.color
        }
    }
}

pub(super) fn hype_unstaking_amount_scale(
    events: &[&HypeUnstakingEvent],
) -> HypeUnstakingAmountScale {
    let (min_ln, max_ln) = events
        .iter()
        .map(|event| hype_amount_ln(event.amount_wei))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
            (min.min(value), max.max(value))
        });

    if !min_ln.is_finite() || !max_ln.is_finite() {
        return HypeUnstakingAmountScale {
            min_ln: 0.0,
            max_ln: 0.0,
        };
    }

    HypeUnstakingAmountScale { min_ln, max_ln }
}

impl HypeUnstakingAmountScale {
    fn heat(self, amount_wei: u64) -> HypeUnstakingAmountHeat {
        let amount_ln = hype_amount_ln(amount_wei);
        let span = self.max_ln - self.min_ln;
        let heat = if span.is_finite() && span > f64::EPSILON {
            ((amount_ln - self.min_ln) / span).clamp(0.0, 1.0)
        } else {
            hype_amount_absolute_heat(amount_wei)
        } as f32;

        HypeUnstakingAmountHeat {
            fill_pct: 0.10 + heat * 0.90,
            alpha: 0.035 + heat * 0.245,
        }
    }
}

fn hype_amount_ln(amount_wei: u64) -> f64 {
    let amount_hype = amount_wei as f64 / HYPE_CORE_WEI_PER_TOKEN as f64;
    amount_hype.max(0.0).ln_1p()
}

fn hype_amount_absolute_heat(amount_wei: u64) -> f64 {
    let amount_hype = amount_wei as f64 / HYPE_CORE_WEI_PER_TOKEN as f64;
    let max_reference = 100_000.0_f64.ln_1p();
    (amount_hype.max(0.0).ln_1p() / max_reference).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests;
