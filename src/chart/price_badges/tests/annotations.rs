use super::*;
use crate::annotations::{Annotation, AnnotationKind, AnnotationStyle};
use crate::chart::fisheye::ChartFisheye;
use crate::chart::{CandlestickChart, ChartState};

fn horizontal_annotation(id: u64, price: f64, visible: bool) -> Annotation {
    Annotation {
        id,
        kind: AnnotationKind::HorizontalLevel { price },
        style: AnnotationStyle {
            visible,
            ..AnnotationStyle::default()
        },
    }
}

#[test]
fn hidden_horizontal_annotations_do_not_allocate_right_axis_badges() {
    let mut chart = CandlestickChart::new(1);
    chart
        .annotations
        .push(horizontal_annotation(7, 100.0, false));
    let state = ChartState::default();
    let price_to_y = |_| 50.0;

    let layout = chart.right_axis_badge_layout(
        &state,
        120.0,
        10.0,
        400.0,
        ChartFisheye::disabled(),
        &price_to_y,
    );

    assert!(
        layout
            .position(RightAxisBadgeKind::HorizontalAnnotation(0))
            .is_none(),
        "hidden annotations must not reserve or draw right-axis badges"
    );

    chart.annotations[0].style.visible = true;
    let layout = chart.right_axis_badge_layout(
        &state,
        120.0,
        10.0,
        400.0,
        ChartFisheye::disabled(),
        &price_to_y,
    );

    assert!(
        layout
            .position(RightAxisBadgeKind::HorizontalAnnotation(0))
            .is_some(),
        "visible horizontal annotations still get a right-axis badge"
    );
}
