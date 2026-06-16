use super::*;
use crate::annotations::{Annotation, AnnotationKind, AnnotationStyle, DrawingTool};
use crate::chart_state::ChartSurfaceId;

fn sample_annotation(id: u64) -> Annotation {
    Annotation {
        id,
        kind: AnnotationKind::HorizontalLevel { price: 100.0 },
        style: AnnotationStyle::default(),
    }
}

#[test]
fn drawing_and_annotation_messages_route_to_annotations() {
    let surface = ChartSurfaceId::Docked(7);
    assert_route(
        Message::SetDrawingTool(7, surface, Some(DrawingTool::Rectangle)),
        UpdateRoute::Annotations,
    );
    assert_route(
        Message::AddAnnotation(7, sample_annotation(0)),
        UpdateRoute::Annotations,
    );
    assert_route(Message::RemoveAnnotation(7, 3), UpdateRoute::Annotations);
    assert_route(
        Message::UpdateAnnotation(7, sample_annotation(3)),
        UpdateRoute::Annotations,
    );
    assert_route(
        Message::SelectAnnotation(7, Some(3)),
        UpdateRoute::Annotations,
    );
    assert_route(
        Message::RestyleAnnotation(7, 3, AnnotationStyle::default()),
        UpdateRoute::Annotations,
    );
    assert_route(
        Message::ClearDrawingTool(7, surface),
        UpdateRoute::Annotations,
    );
}
