use super::*;

#[test]
fn split_portions_preserve_saved_ratio() {
    assert_eq!(split_portions(0.25), (250, 750));
    assert_eq!(split_portions(0.5), (500, 500));
    assert_eq!(split_portions(0.75), (750, 250));
}

#[test]
fn split_portions_clamp_extreme_or_invalid_ratios() {
    assert_eq!(split_portions(0.0), (80, 920));
    assert_eq!(split_portions(1.0), (920, 80));
    assert_eq!(split_portions(f32::NAN), (500, 500));
}

#[test]
fn preview_budget_stops_after_max_nodes() {
    let mut budget = PreviewBudget::new(MAX_PREVIEW_DEPTH, 2);

    assert!(budget.take_node());
    assert!(budget.take_node());
    assert!(!budget.take_node());
}
