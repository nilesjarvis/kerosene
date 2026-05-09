use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use chrono::{DateTime, Datelike, Duration, Local, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use iced::widget::{Row, row, text};

// ---------------------------------------------------------------------------
// Status Clock Row
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(super) fn status_clock_row(&self) -> Row<'static, Message> {
        let theme = self.theme();
        let now_utc = Utc::now();
        let local_now = Local::now();
        let local_text = format!(
            "Local {:02}:{:02}:{:02} {}",
            local_now.hour(),
            local_now.minute(),
            local_now.second(),
            local_now.format("%Z")
        );
        let europe_text = market_clock_text("Europe", now_utc, chrono_tz::Europe::London, 8, 0);
        let asia_text = market_clock_text("Asia", now_utc, chrono_tz::Asia::Tokyo, 9, 0);
        let ny_text = market_clock_text("New York", now_utc, chrono_tz::America::New_York, 9, 30);

        row![
            text(local_text).size(10).color(theme.palette().primary),
            helpers::vertical_spacer(),
            text(europe_text).size(10).color(theme.palette().primary),
            helpers::vertical_spacer(),
            text(asia_text).size(10).color(theme.palette().primary),
            helpers::vertical_spacer(),
            text(ny_text).size(10).color(theme.palette().primary),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
    }
}

fn format_market_open_in(now_utc: DateTime<Utc>, open_utc: DateTime<Utc>) -> String {
    let diff = open_utc - now_utc;
    let total_minutes = diff.num_minutes().max(0);
    let days = total_minutes / (24 * 60);
    let hours = (total_minutes % (24 * 60)) / 60;
    let minutes = total_minutes % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else {
        format!("{hours}h {minutes}m")
    }
}

fn next_market_open(
    now_utc: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Option<DateTime<Tz>> {
    let local_now = now_utc.with_timezone(&tz);
    for day_offset in 0..8 {
        let date = local_now.date_naive() + Duration::days(day_offset);
        if matches!(date.weekday(), Weekday::Sat | Weekday::Sun) {
            continue;
        }
        let naive = date.and_hms_opt(hour, minute, 0)?;
        let open_local = tz
            .from_local_datetime(&naive)
            .earliest()
            .or_else(|| tz.from_local_datetime(&naive).latest())?;
        if open_local > local_now {
            return Some(open_local);
        }
    }
    None
}

fn market_clock_text(
    label: &str,
    now_utc: DateTime<Utc>,
    tz: Tz,
    open_hour: u32,
    open_minute: u32,
) -> String {
    let now_local = now_utc.with_timezone(&tz);
    let next_open = next_market_open(now_utc, tz, open_hour, open_minute)
        .map(|dt| format_market_open_in(now_utc, dt.with_timezone(&Utc)))
        .unwrap_or_else(|| "n/a".to_string());
    format!(
        "{label} {:02}:{:02}:{:02} {} (opens in {next_open})",
        now_local.hour(),
        now_local.minute(),
        now_local.second(),
        now_local.format("%Z")
    )
}
