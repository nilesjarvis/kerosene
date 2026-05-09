use super::Timeframe;

// ---------------------------------------------------------------------------
// Timeframe Metadata Tables
// ---------------------------------------------------------------------------

const MINUTE_MS: u64 = 60 * 1000;
const HOUR_MS: u64 = 60 * MINUTE_MS;
const DAY_MS: u64 = 24 * HOUR_MS;
const TIMEFRAME_COUNT: usize = 14;

pub(super) const ALL_TIMEFRAMES: [Timeframe; TIMEFRAME_COUNT] = [
    Timeframe::M1,
    Timeframe::M3,
    Timeframe::M5,
    Timeframe::M15,
    Timeframe::M30,
    Timeframe::H1,
    Timeframe::H2,
    Timeframe::H4,
    Timeframe::H8,
    Timeframe::H12,
    Timeframe::D1,
    Timeframe::D3,
    Timeframe::W1,
    Timeframe::Mo1,
];

pub(super) const CONFIG_STRS: [&str; TIMEFRAME_COUNT] = [
    "M1", "M3", "M5", "M15", "M30", "H1", "H2", "H4", "H8", "H12", "D1", "D3", "W1", "Mo1",
];

pub(super) const API_STRS: [&str; TIMEFRAME_COUNT] = [
    "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "8h", "12h", "1d", "3d", "1w", "1M",
];

pub(super) const LABELS: [&str; TIMEFRAME_COUNT] = [
    "1m", "3m", "5m", "15m", "30m", "1H", "2H", "4H", "8H", "12H", "1D", "3D", "1W", "1M",
];

pub(super) const DURATIONS_MS: [u64; TIMEFRAME_COUNT] = [
    MINUTE_MS,
    3 * MINUTE_MS,
    5 * MINUTE_MS,
    15 * MINUTE_MS,
    30 * MINUTE_MS,
    HOUR_MS,
    2 * HOUR_MS,
    4 * HOUR_MS,
    8 * HOUR_MS,
    12 * HOUR_MS,
    DAY_MS,
    3 * DAY_MS,
    7 * DAY_MS,
    30 * DAY_MS,
];

pub(super) const LOOKBACKS_MS: [u64; TIMEFRAME_COUNT] = [
    12 * HOUR_MS,
    DAY_MS,
    2 * DAY_MS,
    5 * DAY_MS,
    10 * DAY_MS,
    20 * DAY_MS,
    30 * DAY_MS,
    60 * DAY_MS,
    120 * DAY_MS,
    180 * DAY_MS,
    365 * DAY_MS,
    2 * 365 * DAY_MS,
    3 * 365 * DAY_MS,
    5 * 365 * DAY_MS,
];
