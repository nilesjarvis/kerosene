use super::*;
use crate::annotations::{Annotation, AnnotationStyle};
use crate::api::Candle;

fn chart_with_annotations() -> CandlestickChart {
    let mut chart = CandlestickChart::new(1);
    chart.set_candles(
        (1..=20)
            .map(|index| {
                Candle::test_price(index * 1_000, if index % 2 == 0 { 100.0 } else { 110.0 })
            })
            .collect(),
    );
    chart
}

#[test]
fn annotation_hit_priority_keeps_topmost_bodies_locked_targets_and_anchor_indices() {
    for inverted in [false, true] {
        let mut chart = chart_with_annotations();
        chart.inverted = inverted;
        let state = ChartState::default();
        let (hi, range, height) = chart
            .visible_price_params(&state, 400.0, 240.0)
            .expect("price range");
        let start = (6_000, 102.0);
        let end = (15_000, 108.0);
        let start_pos = Point::new(
            chart
                .timestamp_to_x(start.0, &state, 400.0)
                .expect("start x"),
            chart.price_to_y_with(start.1, hi, range, height),
        );
        let end_pos = Point::new(
            chart.timestamp_to_x(end.0, &state, 400.0).expect("end x"),
            chart.price_to_y_with(end.1, hi, range, height),
        );
        chart.annotations = vec![
            Annotation {
                id: 1,
                kind: AnnotationKind::TrendLine { start, end },
                style: AnnotationStyle {
                    locked: true,
                    ..Default::default()
                },
            },
            Annotation {
                id: 2,
                kind: AnnotationKind::HorizontalLevel { price: start.1 },
                style: AnnotationStyle::default(),
            },
        ];
        let hit = chart
            .hit_test_annotation(&state, start_pos, 400.0, 240.0)
            .expect("topmost body");
        assert_eq!((hit.id, hit.part), (2, AnnotationHitPart::Body));
        chart.annotations[1].style.visible = false;
        for (index, pos) in [start_pos, end_pos].into_iter().enumerate() {
            let hit = chart
                .hit_test_annotation(&state, pos, 400.0, 240.0)
                .expect("locked anchor remains selectable");
            assert_eq!((hit.id, hit.part), (1, AnnotationHitPart::Anchor(index)));
        }
        chart.annotations[0].style.visible = false;
        assert!(
            chart
                .hit_test_annotation(&state, start_pos, 400.0, 240.0)
                .is_none()
        );
    }
}

#[test]
fn line_rectangle_and_fib_bodies_remain_hittable_away_from_handles() {
    let mut chart = chart_with_annotations();
    let state = ChartState::default();
    let (hi, range, height) = chart
        .visible_price_params(&state, 400.0, 240.0)
        .expect("price range");
    let start = (6_000, 102.0);
    let end = (15_000, 108.0);
    let middle = Point::new(
        chart
            .timestamp_to_x(10_500, &state, 400.0)
            .expect("middle x"),
        chart.price_to_y_with(105.0, hi, range, height),
    );
    for kind in [
        AnnotationKind::TrendLine { start, end },
        AnnotationKind::Ray { start, end },
        AnnotationKind::ExtendedLine { start, end },
        AnnotationKind::Measure { start, end },
        AnnotationKind::Rectangle { a: start, b: end },
    ] {
        chart.annotations = vec![Annotation {
            id: 3,
            kind,
            style: AnnotationStyle::default(),
        }];
        let hit = chart
            .hit_test_annotation(&state, middle, 400.0, 240.0)
            .expect("shape body");
        assert_eq!((hit.id, hit.part), (3, AnnotationHitPart::Body));
    }
    for (kind, points, price) in [
        (FibKind::Retracement, vec![start, end], 102.0),
        (FibKind::Extension, vec![start, end, (18_000, 104.0)], 104.0),
    ] {
        chart.annotations = vec![Annotation {
            id: 4,
            kind: AnnotationKind::Fib { kind, points },
            style: AnnotationStyle::default(),
        }];
        let pos = Point::new(399.0, chart.price_to_y_with(price, hi, range, height));
        let hit = chart
            .hit_test_annotation(&state, pos, 400.0, 240.0)
            .expect("fib level");
        assert_eq!((hit.id, hit.part), (4, AnnotationHitPart::Body));
    }
}
