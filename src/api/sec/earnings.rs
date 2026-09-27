use super::{SecEarningsMetric, SecStructuredEarnings};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::HashMap;

const SEC_PERIODIC_FILING_MATCH_DAYS: i64 = 45;

#[derive(Default, Deserialize)]
pub(super) struct SecCompanyFacts {
    #[serde(default)]
    facts: HashMap<String, HashMap<String, SecCompanyConcept>>,
}

#[derive(Default, Deserialize)]
struct SecCompanyConcept {
    #[serde(default)]
    units: HashMap<String, Vec<SecCompanyFact>>,
}

#[derive(Clone, Deserialize)]
struct SecCompanyFact {
    #[serde(default)]
    start: String,
    #[serde(default)]
    end: String,
    #[serde(default)]
    val: Value,
    #[serde(default)]
    accn: String,
    #[serde(default)]
    form: String,
    #[serde(default)]
    filed: String,
}

#[derive(Clone, Copy)]
enum SecMetricFormat {
    Currency,
    PerShare,
}

#[derive(Clone, Copy)]
struct SecMetricSpec {
    label: &'static str,
    concepts: &'static [&'static str],
    unit: &'static str,
    format: SecMetricFormat,
}

const SEC_EARNINGS_METRICS: [SecMetricSpec; 3] = [
    SecMetricSpec {
        label: "Revenue",
        concepts: &[
            "RevenueFromContractWithCustomerExcludingAssessedTax",
            "Revenues",
            "SalesRevenueNet",
        ],
        unit: "USD",
        format: SecMetricFormat::Currency,
    },
    SecMetricSpec {
        label: "Diluted EPS",
        concepts: &["EarningsPerShareDiluted"],
        unit: "USD/shares",
        format: SecMetricFormat::PerShare,
    },
    SecMetricSpec {
        label: "Net income",
        concepts: &["NetIncomeLoss", "ProfitLoss"],
        unit: "USD",
        format: SecMetricFormat::Currency,
    },
];

#[derive(Clone)]
struct SecPeriodicFilingCandidate {
    accession_number: String,
    form: String,
    days_after_event: i64,
    metric_mask: u8,
}

#[derive(Clone, Copy)]
struct SecDatedFact {
    start: NaiveDate,
    end: NaiveDate,
    value: f64,
}

struct SecSelectedMetric {
    metric: SecEarningsMetric,
    period_end: NaiveDate,
}

pub(super) fn extract_structured_earnings(
    company_facts: &SecCompanyFacts,
    event_filing_date: &str,
) -> Option<SecStructuredEarnings> {
    let event_date = NaiveDate::parse_from_str(event_filing_date, "%Y-%m-%d").ok()?;
    let gaap_facts = company_facts.facts.get("us-gaap")?;
    let candidate = select_periodic_filing(gaap_facts, event_date)?;

    let mut metrics = Vec::new();
    let mut period_end = None;
    for spec in SEC_EARNINGS_METRICS {
        if let Some(selected) = select_structured_metric(gaap_facts, &candidate, spec) {
            period_end = Some(
                period_end.map_or(selected.period_end, |current: NaiveDate| {
                    current.max(selected.period_end)
                }),
            );
            metrics.push(selected.metric);
        }
    }

    let period_end = period_end?;
    Some(SecStructuredEarnings {
        source_form: candidate.form,
        source_accession_number: candidate.accession_number,
        period_end: period_end.format("%Y-%m-%d").to_string(),
        metrics,
    })
}

fn select_periodic_filing(
    gaap_facts: &HashMap<String, SecCompanyConcept>,
    event_date: NaiveDate,
) -> Option<SecPeriodicFilingCandidate> {
    let mut candidates = HashMap::<String, SecPeriodicFilingCandidate>::new();

    for (metric_index, spec) in SEC_EARNINGS_METRICS.iter().enumerate() {
        for concept_name in spec.concepts {
            let Some(facts) = gaap_facts
                .get(*concept_name)
                .and_then(|concept| concept.units.get(spec.unit))
            else {
                continue;
            };

            for fact in facts {
                if !matches!(fact.form.as_str(), "10-Q" | "10-K")
                    || sec_numeric_value(&fact.val).is_none()
                {
                    continue;
                }
                let Ok(filed) = NaiveDate::parse_from_str(&fact.filed, "%Y-%m-%d") else {
                    continue;
                };
                let days_after_event = (filed - event_date).num_days();
                if !(0..=SEC_PERIODIC_FILING_MATCH_DAYS).contains(&days_after_event)
                    || fact.accn.trim().is_empty()
                {
                    continue;
                }

                let candidate = candidates.entry(fact.accn.clone()).or_insert_with(|| {
                    SecPeriodicFilingCandidate {
                        accession_number: fact.accn.clone(),
                        form: fact.form.clone(),
                        days_after_event,
                        metric_mask: 0,
                    }
                });
                candidate.metric_mask |= 1 << metric_index;
            }
        }
    }

    candidates.into_values().min_by_key(|candidate| {
        (
            candidate.days_after_event,
            Reverse(candidate.metric_mask.count_ones()),
            candidate.accession_number.clone(),
        )
    })
}

fn select_structured_metric(
    gaap_facts: &HashMap<String, SecCompanyConcept>,
    candidate: &SecPeriodicFilingCandidate,
    spec: SecMetricSpec,
) -> Option<SecSelectedMetric> {
    for concept_name in spec.concepts {
        let Some(facts) = gaap_facts
            .get(*concept_name)
            .and_then(|concept| concept.units.get(spec.unit))
        else {
            continue;
        };
        let dated_facts = facts
            .iter()
            .filter(|fact| fact.accn == candidate.accession_number && fact.form == candidate.form)
            .filter_map(sec_dated_fact)
            .filter(|fact| fact_duration_matches_form(fact, &candidate.form))
            .collect::<Vec<_>>();

        let target_days = if candidate.form == "10-Q" { 91 } else { 365 };
        let current = dated_facts.iter().copied().min_by_key(|fact| {
            (
                Reverse(fact.end),
                ((fact.end - fact.start).num_days() - target_days).abs(),
            )
        });
        let Some(current) = current else {
            continue;
        };

        let current_duration = (current.end - current.start).num_days();
        let previous = dated_facts
            .iter()
            .copied()
            .filter(|fact| {
                let year_gap = (current.end - fact.end).num_days();
                fact.end < current.end && (300..=430).contains(&year_gap)
            })
            .min_by_key(|fact| {
                let year_gap = (current.end - fact.end).num_days();
                let duration = (fact.end - fact.start).num_days();
                (
                    (year_gap - 365).abs(),
                    (duration - current_duration).abs(),
                    Reverse(fact.end),
                )
            });

        return Some(SecSelectedMetric {
            metric: SecEarningsMetric {
                label: spec.label.to_string(),
                value: format_sec_metric_value(current.value, spec.format),
                yoy_change: previous
                    .and_then(|fact| format_sec_yoy_change(current.value, fact.value)),
            },
            period_end: current.end,
        });
    }

    None
}

fn sec_dated_fact(fact: &SecCompanyFact) -> Option<SecDatedFact> {
    let start = NaiveDate::parse_from_str(&fact.start, "%Y-%m-%d").ok()?;
    let end = NaiveDate::parse_from_str(&fact.end, "%Y-%m-%d").ok()?;
    let value = sec_numeric_value(&fact.val)?;
    (end >= start).then_some(SecDatedFact { start, end, value })
}

fn fact_duration_matches_form(fact: &SecDatedFact, form: &str) -> bool {
    let duration = (fact.end - fact.start).num_days();
    match form {
        "10-Q" => (50..=125).contains(&duration),
        "10-K" => (300..=430).contains(&duration),
        _ => false,
    }
}

fn sec_numeric_value(value: &Value) -> Option<f64> {
    value.as_f64().filter(|value| value.is_finite())
}

fn format_sec_metric_value(value: f64, format: SecMetricFormat) -> String {
    let sign = if value < 0.0 { "-" } else { "" };
    let absolute = value.abs();
    match format {
        SecMetricFormat::PerShare => format!("{sign}${absolute:.2}"),
        SecMetricFormat::Currency if absolute >= 1_000_000_000_000.0 => {
            format!("{sign}${:.2}T", absolute / 1_000_000_000_000.0)
        }
        SecMetricFormat::Currency if absolute >= 1_000_000_000.0 => {
            format!("{sign}${:.1}B", absolute / 1_000_000_000.0)
        }
        SecMetricFormat::Currency if absolute >= 1_000_000.0 => {
            format!("{sign}${:.1}M", absolute / 1_000_000.0)
        }
        SecMetricFormat::Currency if absolute >= 1_000.0 => {
            format!("{sign}${:.1}K", absolute / 1_000.0)
        }
        SecMetricFormat::Currency => format!("{sign}${absolute:.0}"),
    }
}

fn format_sec_yoy_change(current: f64, previous: f64) -> Option<String> {
    if !current.is_finite() || !previous.is_finite() || previous.abs() < f64::EPSILON {
        return None;
    }
    let change = (current - previous) / previous.abs() * 100.0;
    change.is_finite().then(|| format!("{change:+.1}% YoY"))
}

#[cfg(test)]
mod tests;
