use super::*;

#[test]
fn default_theme_is_kerosene() {
    assert_eq!(default_theme(), "Custom: Kerosene");
}

fn default_theme_expectations() -> [ThemeExpectation<'static>; 11] {
    [
        ThemeExpectation {
            name: "Kerosene",
            background: "#080604",
            text: "#F4EEE3",
            primary: "#FF7A1A",
            success: "#FF7A1A",
            warning: "#FFB020",
            danger: "#F8EFE2",
            chart_bull: Some("#FF7A1A"),
            chart_bear: Some("#F8EFE2"),
        },
        ThemeExpectation {
            name: "Inverse E-Ink",
            background: "#000000",
            text: "#ffffff",
            primary: "#aaaaaa",
            success: "#666666",
            warning: "#888888",
            danger: "#dddddd",
            chart_bull: None,
            chart_bear: None,
        },
        ThemeExpectation {
            name: "Hyperliquid",
            background: "#0F1A1E",
            text: "#F6FEFD",
            primary: "#50D2C1",
            success: "#50D2C1",
            warning: "#FFB648",
            danger: "#ED7088",
            chart_bull: Some("#26A69A"),
            chart_bear: Some("#EF5350"),
        },
        ThemeExpectation {
            name: "XYZ",
            background: "#11151B",
            text: "#E8E8E8",
            primary: "#FFC028",
            success: "#08A088",
            warning: "#D8A828",
            danger: "#FF3848",
            chart_bull: Some("#08A088"),
            chart_bear: Some("#FF3848"),
        },
        ThemeExpectation {
            name: "Kraken",
            background: "#0B0711",
            text: "#E8E1F2",
            primary: "#7132F5",
            success: "#2BB67B",
            warning: "#ED9B35",
            danger: "#B2425F",
            chart_bull: Some("#2BB67B"),
            chart_bear: Some("#E34A6F"),
        },
        ThemeExpectation {
            name: "Bloomberg",
            background: "#000000",
            text: "#F2F2E8",
            primary: "#FF9F1A",
            success: "#00B050",
            warning: "#FFD84A",
            danger: "#B00024",
            chart_bull: Some("#00C853"),
            chart_bear: Some("#D50032"),
        },
        ThemeExpectation {
            name: "FTX",
            background: "#101824",
            text: "#D8E2EE",
            primary: "#00A8B8",
            success: "#08A67A",
            warning: "#F0A040",
            danger: "#F03060",
            chart_bull: Some("#08A67A"),
            chart_bear: Some("#F03060"),
        },
        ThemeExpectation {
            name: "IBKR Dark",
            background: "#101018",
            text: "#D8DCE6",
            primary: "#2878F0",
            success: "#2EBF7A",
            warning: "#D0A818",
            danger: "#F83048",
            chart_bull: Some("#2EBF7A"),
            chart_bear: Some("#F83048"),
        },
        ThemeExpectation {
            name: "bybit",
            background: "#101014",
            text: "#F5F5F5",
            primary: "#F4B444",
            success: "#55AF72",
            warning: "#E8A838",
            danger: "#DC5351",
            chart_bull: Some("#55AF72"),
            chart_bear: Some("#DC5351"),
        },
        ThemeExpectation {
            name: "coinbase-dark",
            background: "#090B0C",
            text: "#F5F7F9",
            primary: "#3474F4",
            success: "#44C48C",
            warning: "#F4941C",
            danger: "#EC6474",
            chart_bull: Some("#44C48C"),
            chart_bear: Some("#EC6474"),
        },
        ThemeExpectation {
            name: "coinbase-light",
            background: "#FFFFFF",
            text: "#0A0B0D",
            primary: "#0052FF",
            success: "#098551",
            warning: "#F7931A",
            danger: "#CF202F",
            chart_bull: Some("#098551"),
            chart_bear: Some("#CF202F"),
        },
    ]
}

#[test]
fn default_custom_themes_include_expected_palettes_and_chart_colors() {
    let themes = default_custom_themes();

    for expected in default_theme_expectations() {
        assert_theme_matches(&themes, expected);
    }
}
