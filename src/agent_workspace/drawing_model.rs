use super::MAX_DRAWING_LABEL_CHARS;
use crate::annotations::{
    Annotation, AnnotationId, AnnotationKind, AnnotationStyle, DEFAULT_LEVEL_COLOR,
    DEFAULT_LINE_COLOR, DEFAULT_MEASURE_COLOR, DrawingTool, FibKind, LineStyle,
};
use crate::chart_state::ChartId;
use iced::Color;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Assistant Drawing Request and Style Contract
// ---------------------------------------------------------------------------

pub(crate) const ASSISTANT_DRAWING_CATALOG: [(&str, &str, usize); 9] = [
    ("horizontal_level", "Horizontal level", 1),
    ("vertical_line", "Vertical line", 1),
    ("trend_line", "Trend line", 2),
    ("ray", "Ray", 2),
    ("extended_line", "Extended line", 2),
    ("rectangle", "Rectangle / zone", 2),
    ("measure", "Price / time measurement", 2),
    ("fib_retracement", "Fibonacci retracement", 2),
    ("fib_extension", "Fibonacci extension", 3),
];

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ChartDrawingOperation {
    Add {
        chart_id: ChartId,
        drawing: ChartDrawingSpec,
    },
    Remove {
        chart_id: ChartId,
        drawing_id: AnnotationId,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum ChartDrawingSpec {
    HorizontalLevel {
        price: f64,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    VerticalLine {
        time_ms: u64,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    TrendLine {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    Ray {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    ExtendedLine {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    Rectangle {
        a: AgentDrawingAnchor,
        b: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    Measure {
        start: AgentDrawingAnchor,
        end: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    FibRetracement {
        a: AgentDrawingAnchor,
        b: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
    FibExtension {
        a: AgentDrawingAnchor,
        b: AgentDrawingAnchor,
        c: AgentDrawingAnchor,
        #[serde(default)]
        style: AgentDrawingStyle,
    },
}

impl ChartDrawingSpec {
    pub(super) fn into_annotation(self) -> Result<Annotation, &'static str> {
        let (kind, tool, style) = match self {
            Self::HorizontalLevel { price, style } => (
                AnnotationKind::HorizontalLevel { price },
                DrawingTool::HorizontalLevel,
                style,
            ),
            Self::VerticalLine { time_ms, style } => (
                AnnotationKind::VerticalLine { time: time_ms },
                DrawingTool::VerticalLine,
                style,
            ),
            Self::TrendLine { start, end, style } => (
                AnnotationKind::TrendLine {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::TrendLine,
                style,
            ),
            Self::Ray { start, end, style } => (
                AnnotationKind::Ray {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::Ray,
                style,
            ),
            Self::ExtendedLine { start, end, style } => (
                AnnotationKind::ExtendedLine {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::ExtendedLine,
                style,
            ),
            Self::Rectangle { a, b, style } => (
                AnnotationKind::Rectangle {
                    a: a.into_anchor(),
                    b: b.into_anchor(),
                },
                DrawingTool::Rectangle,
                style,
            ),
            Self::Measure { start, end, style } => (
                AnnotationKind::Measure {
                    start: start.into_anchor(),
                    end: end.into_anchor(),
                },
                DrawingTool::Measure,
                style,
            ),
            Self::FibRetracement { a, b, style } => (
                AnnotationKind::Fib {
                    kind: FibKind::Retracement,
                    points: vec![a.into_anchor(), b.into_anchor()],
                },
                DrawingTool::FibRetracement,
                style,
            ),
            Self::FibExtension { a, b, c, style } => (
                AnnotationKind::Fib {
                    kind: FibKind::Extension,
                    points: vec![a.into_anchor(), b.into_anchor(), c.into_anchor()],
                },
                DrawingTool::FibExtension,
                style,
            ),
        };
        let mut annotation = Annotation {
            id: 0,
            kind,
            style: AnnotationStyle::for_tool(tool),
        };
        style.apply_to(&mut annotation.style)?;
        annotation
            .is_valid()
            .then_some(annotation)
            .ok_or("The drawing contains invalid time or price coordinates")
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentDrawingAnchor {
    time_ms: u64,
    price: f64,
}

impl AgentDrawingAnchor {
    fn into_anchor(self) -> (u64, f64) {
        (self.time_ms, self.price)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AgentDrawingStyle {
    color: Option<AgentDrawingColor>,
    width: Option<f32>,
    line_style: Option<AgentDrawingLineStyle>,
    label: Option<String>,
}

impl AgentDrawingStyle {
    fn apply_to(self, target: &mut AnnotationStyle) -> Result<(), &'static str> {
        if let Some(color) = self.color {
            target.color = color.color();
        }
        if let Some(width) = self.width {
            if ![1.0, 1.5, 2.5, 4.0].contains(&width) {
                return Err("Drawing width must be one of 1, 1.5, 2.5, or 4");
            }
            target.width = width;
        }
        if let Some(line_style) = self.line_style {
            target.line_style = line_style.into();
        }
        if let Some(label) = self.label {
            let label = label.trim();
            if label.is_empty()
                || label.chars().count() > MAX_DRAWING_LABEL_CHARS
                || label.chars().any(char::is_control)
            {
                return Err("Drawing labels must contain 1 to 80 printable characters");
            }
            target.label = Some(label.to_string());
        }
        target.locked = false;
        target.visible = true;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentDrawingColor {
    Blue,
    Yellow,
    Teal,
    Red,
    Purple,
    White,
}

impl AgentDrawingColor {
    fn color(self) -> Color {
        match self {
            Self::Blue => DEFAULT_LEVEL_COLOR,
            Self::Yellow => DEFAULT_LINE_COLOR,
            Self::Teal => DEFAULT_MEASURE_COLOR,
            Self::Red => Color::from_rgb(0.95, 0.45, 0.45),
            Self::Purple => Color::from_rgb(0.62, 0.55, 0.95),
            Self::White => Color::from_rgb(0.92, 0.92, 0.92),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentDrawingLineStyle {
    Solid,
    Dashed,
    Dotted,
}

impl From<AgentDrawingLineStyle> for LineStyle {
    fn from(style: AgentDrawingLineStyle) -> Self {
        match style {
            AgentDrawingLineStyle::Solid => Self::Solid,
            AgentDrawingLineStyle::Dashed => Self::Dashed,
            AgentDrawingLineStyle::Dotted => Self::Dotted,
        }
    }
}

impl ChartDrawingOperation {
    pub(super) fn chart_id(&self) -> ChartId {
        match self {
            Self::Add { chart_id, .. } | Self::Remove { chart_id, .. } => *chart_id,
        }
    }
}

pub(crate) fn annotation_kind_key(kind: &AnnotationKind) -> &'static str {
    match kind {
        AnnotationKind::HorizontalLevel { .. } => "horizontal_level",
        AnnotationKind::VerticalLine { .. } => "vertical_line",
        AnnotationKind::TrendLine { .. } => "trend_line",
        AnnotationKind::Ray { .. } => "ray",
        AnnotationKind::ExtendedLine { .. } => "extended_line",
        AnnotationKind::Rectangle { .. } => "rectangle",
        AnnotationKind::Measure { .. } => "measure",
        AnnotationKind::Fib {
            kind: FibKind::Retracement,
            ..
        } => "fib_retracement",
        AnnotationKind::Fib {
            kind: FibKind::Extension,
            ..
        } => "fib_extension",
    }
}
