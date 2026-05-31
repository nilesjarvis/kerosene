use super::*;

// ---------------------------------------------------------------------------
// Farside BHYP Flow Parsing Tests
// ---------------------------------------------------------------------------

/// Minimal HTML fragment reproducing the Chart.js structure from the
/// Farside WordPress REST API response.
fn mock_farside_html(bhyp_cumulative: &str, labels: &str) -> String {
    format!(
        r#"<canvas id="hypStackedAreaChart"></canvas>
<script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
<script>
document.addEventListener("DOMContentLoaded", function() {{
    const ctx = document.getElementById('hypStackedAreaChart').getContext('2d');
    new Chart(ctx, {{
        type: 'line',
        data: {{
            labels: [{labels}],
            datasets: [{{"label":"Bitwise (BHYP)","data":[{bhyp_cumulative}],"backgroundColor":"rgba(26, 54, 93, 1)","borderColor":"rgba(26, 54, 93, 1)","fill":true,"pointRadius":0,"pointHoverRadius":4,"borderWidth":1}},{{"label":"21 Shares (THYP)","data":[1.2,2.6,7.5],"backgroundColor":"rgba(30, 136, 229, 1)","borderColor":"rgba(30, 136, 229, 1)","fill":true,"pointRadius":0,"pointHoverRadius":4,"borderWidth":1}}]
        }},
        options: {{}}
    }});
}});
</script>"#
    )
}

fn date_or_panic(raw: &str) -> String {
    match parse_farside_date(raw) {
        Ok(date) => date,
        Err(error) => panic!("valid Farside date {raw}: {error}"),
    }
}

#[test]
fn extract_bhyp_cumulative_parses_float_array() {
    let html = mock_farside_html(
        "0,0.7,2,4,9.7",
        r#""12 May 2026","13 May 2026","14 May 2026","15 May 2026","18 May 2026""#,
    );
    let result = extract_bhyp_cumulative(&html).unwrap();
    assert_eq!(result, vec![0.0, 0.7, 2.0, 4.0, 9.7]);
}

#[test]
fn extract_bhyp_cumulative_rejects_malformed_numeric_array() {
    let html = mock_farside_html(
        "0,not-a-number,2",
        r#""12 May 2026","13 May 2026","14 May 2026""#,
    );
    let result = extract_bhyp_cumulative(&html);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("data array parse failed"));
}

#[test]
fn extract_bhyp_cumulative_returns_error_when_marker_missing() {
    let html = "<script>new Chart(ctx, { data: { datasets: [] } });</script>";
    let result = extract_bhyp_cumulative(html);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("chart marker not found"));
}

#[test]
fn extract_bhyp_cumulative_returns_error_when_data_array_missing() {
    let html = r#"<script>"Bitwise (BHYP)"</script>"#;
    let result = extract_bhyp_cumulative(html);
    assert!(result.is_err());
}

#[test]
fn extract_labels_parses_string_array() {
    let html = mock_farside_html("0,0.7", r#""12 May 2026","13 May 2026""#);
    let result = extract_labels(&html).unwrap();
    assert_eq!(result, vec!["12 May 2026", "13 May 2026"]);
}

#[test]
fn extract_chart_data_uses_labels_from_the_bhyp_chart() {
    let html = format!(
        r#"<script>
new Chart(ctx, {{
    data: {{
        labels: ["1 January 2026"],
        datasets: [{{"label":"Other","data":[99]}}]
    }}
}});
</script>
{}"#,
        mock_farside_html("0,0.7", r#""12 May 2026","13 May 2026""#)
    );

    let (cumulative, labels) = extract_chart_data(&html).unwrap();

    assert_eq!(cumulative, vec![0.0, 0.7]);
    assert_eq!(labels, vec!["12 May 2026", "13 May 2026"]);
}

#[test]
fn extract_labels_returns_error_when_missing() {
    let html = "<script>empty</script>";
    let result = extract_labels(html);
    assert!(result.is_err());
}

#[test]
fn label_data_mismatch_errors() {
    let html = mock_farside_html("0,0.7,2", r#""12 May 2026","13 May 2026""#);
    let result = extract_chart_data(&html);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("labels count (2) != data count (3)")
    );
}

#[test]
fn daily_flows_from_cumulative_handles_normal_sequence() {
    let cumulative = vec![0.0, 0.7, 2.0, 4.0, 9.7];
    let labels: Vec<String> = vec![
        "12 May 2026".into(),
        "13 May 2026".into(),
        "14 May 2026".into(),
        "15 May 2026".into(),
        "18 May 2026".into(),
    ];

    let flows = daily_flows_from_cumulative(&cumulative, &labels).unwrap();

    assert_eq!(flows.len(), 5);
    // Day 1: 0.0 - 0.0 = 0.0; zero-flow days are included
    assert_eq!(flows[0].date, "2026-05-12");
    assert_eq!(flows[0].amount_usd, 0.0);
    // Day 2: 0.7 - 0.0 = 0.7 * 1M = 700,000
    assert_eq!(flows[1].date, "2026-05-13");
    assert_eq!(flows[1].amount_usd, 700_000.0);
    // Day 3: 2.0 - 0.7 = 1.3 * 1M
    assert_eq!(flows[2].amount_usd, 1_300_000.0);
    // Day 4: 4.0 - 2.0 = 2.0 * 1M
    assert_eq!(flows[3].amount_usd, 2_000_000.0);
    // Day 5: 9.7 - 4.0 = 5.7 * 1M
    assert!((flows[4].amount_usd - 5_700_000.0).abs() < 0.01);
}

#[test]
fn daily_flows_from_cumulative_skips_first_nonzero_baseline() {
    let cumulative = vec![4.0, 5.5, 7.0];
    let labels: Vec<String> = vec![
        "12 May 2026".into(),
        "13 May 2026".into(),
        "14 May 2026".into(),
    ];

    let flows = daily_flows_from_cumulative(&cumulative, &labels).unwrap();

    assert_eq!(flows.len(), 2);
    assert_eq!(flows[0].date, "2026-05-13");
    assert_eq!(flows[0].amount_usd, 1_500_000.0);
    assert_eq!(flows[1].date, "2026-05-14");
    assert_eq!(flows[1].amount_usd, 1_500_000.0);
}

#[test]
fn daily_flows_from_cumulative_rejects_invalid_dates() {
    let cumulative = vec![0.0];
    let labels: Vec<String> = vec!["32 May 2026".into()];

    let result = daily_flows_from_cumulative(&cumulative, &labels);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("invalid date label"));
}

#[test]
fn parse_farside_date_canonical_formats() {
    assert_eq!(date_or_panic("12 May 2026"), "2026-05-12");
    assert_eq!(date_or_panic("1 Jan 2025"), "2025-01-01");
    assert_eq!(date_or_panic("31 Dec 2026"), "2026-12-31");
}

#[test]
fn parse_farside_date_full_month_names() {
    assert_eq!(date_or_panic("12 January 2026"), "2026-01-12");
    assert_eq!(date_or_panic("1 February 2026"), "2026-02-01");
    assert_eq!(date_or_panic("15 March 2026"), "2026-03-15");
    assert_eq!(date_or_panic("20 June 2026"), "2026-06-20");
    assert_eq!(date_or_panic("4 July 2026"), "2026-07-04");
    assert_eq!(date_or_panic("10 August 2026"), "2026-08-10");
    assert_eq!(date_or_panic("5 September 2026"), "2026-09-05");
    assert_eq!(date_or_panic("31 October 2026"), "2026-10-31");
    assert_eq!(date_or_panic("30 November 2026"), "2026-11-30");
    assert_eq!(date_or_panic("25 December 2026"), "2026-12-25");
}

#[test]
fn parse_farside_date_unknown_month_errors() {
    assert!(parse_farside_date("1 Plugh 2026").is_err());
}

#[test]
fn parse_farside_date_short_string_errors() {
    assert!(parse_farside_date("nope").is_err());
}

#[test]
fn parse_farside_date_invalid_day_errors() {
    assert!(parse_farside_date("0 May 2026").is_err());
    assert!(parse_farside_date("32 May 2026").is_err());
}

#[test]
fn parse_bhyp_flows_full_pipeline() {
    let html = mock_farside_html(
        "0,0.7,2,4",
        r#""12 May 2026","13 May 2026","14 May 2026","15 May 2026""#,
    );

    let flows = parse_bhyp_flows_from_html(&html).unwrap();

    assert_eq!(flows.len(), 4);
    assert_eq!(flows[0].date, "2026-05-12");
    assert_eq!(flows[0].amount_usd, 0.0);
    assert_eq!(flows[3].date, "2026-05-15");
    assert_eq!(flows[3].amount_usd, 2_000_000.0);
}

#[test]
fn empty_cumulative_errors() {
    let html = mock_farside_html("", r#""12 May 2026""#);
    let result = parse_bhyp_flows_from_html(&html);
    assert!(result.is_err());
}
