use super::CustomThemeConfig;

struct ThemeSpec {
    name: &'static str,
    colors: [&'static str; 6],
    chart: Option<(&'static str, &'static str)>,
}

pub(crate) fn default_custom_themes() -> Vec<CustomThemeConfig> {
    [
        ThemeSpec {
            name: "E-Ink",
            colors: [
                "#ffffff", "#000000", "#555555", "#999999", "#777777", "#222222",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "Charles Schwab",
            colors: [
                "#FFFFFF", "#1C1C1C", "#00A0DF", "#188B50", "#F2A900", "#D9272E",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "Interactive Brokers",
            colors: [
                "#FFFFFF", "#000000", "#D82724", "#008A00", "#FF8C00", "#D82724",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "Robinhood",
            colors: [
                "#000000", "#FFFFFF", "#00C805", "#00C805", "#FFB100", "#FF5000",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "Schwab Black",
            colors: [
                "#000000", "#F0F0F0", "#00A0DF", "#188B50", "#F2A900", "#D9272E",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "thinkorswim",
            colors: [
                "#131722", "#D9D9D9", "#F29333", "#00B159", "#F99127", "#E34538",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "OLED",
            colors: [
                "#000000", "#E0E0E0", "#3B82F6", "#10B981", "#F59E0B", "#EF4444",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "hsaka",
            colors: [
                "#000000", "#E0E0E0", "#3B82F6", "#3B82F6", "#F59E0B", "#FFFFFF",
            ],
            chart: None,
        },
        ThemeSpec {
            name: "Kerosene",
            colors: [
                "#080604", "#F4EEE3", "#FF7A1A", "#FF7A1A", "#FFB020", "#F8EFE2",
            ],
            chart: Some(("#FF7A1A", "#F8EFE2")),
        },
        ThemeSpec {
            name: "Hyperliquid",
            colors: [
                "#0F1A1E", "#F6FEFD", "#50D2C1", "#50D2C1", "#FFB648", "#ED7088",
            ],
            chart: Some(("#26A69A", "#EF5350")),
        },
    ]
    .into_iter()
    .map(theme_from_spec)
    .collect()
}

fn theme_from_spec(spec: ThemeSpec) -> CustomThemeConfig {
    let [background, text, primary, success, warning, danger] = spec.colors;
    let (chart_bull, chart_bear) = spec
        .chart
        .map(|(bull, bear)| (bull.to_owned(), bear.to_owned()))
        .unzip();

    CustomThemeConfig {
        name: spec.name.to_owned(),
        background: background.to_owned(),
        text: text.to_owned(),
        primary: primary.to_owned(),
        success: success.to_owned(),
        warning: warning.to_owned(),
        danger: danger.to_owned(),
        chart_bull,
        chart_bear,
    }
}
