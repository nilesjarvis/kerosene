use iced::Color;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Chart annotation types
// ---------------------------------------------------------------------------

pub type AnnotationId = u64;

/// Drawing tool the user can activate via toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawingTool {
    HorizontalLevel,
    TrendLine,
    Eraser,
}

/// A user-drawn annotation on a chart.
#[derive(Debug, Clone)]
pub enum AnnotationKind {
    /// A horizontal price level line spanning the full chart width.
    HorizontalLevel { price: f64 },
    /// A trend line between two (timestamp_ms, price) anchor points.
    TrendLine { start: (u64, f64), end: (u64, f64) },
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub id: AnnotationId,
    pub kind: AnnotationKind,
    pub color: Color,
}

/// Default color for horizontal levels (blue).
pub const DEFAULT_LEVEL_COLOR: Color = Color::from_rgb(0.478, 0.635, 0.969);
/// Default color for trend lines (yellow).
pub const DEFAULT_LINE_COLOR: Color = Color::from_rgb(0.945, 0.980, 0.549);

// ---------------------------------------------------------------------------
// Serializable config form for persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnnotationConfig {
    /// "level" or "trendline"
    #[serde(rename = "type")]
    pub kind: String,
    /// RGB color as [r, g, b] in 0.0..1.0
    pub color: [f32; 3],
    /// Price (for horizontal levels)
    #[serde(default)]
    pub price: Option<f64>,
    /// Start timestamp (for trend lines)
    #[serde(default)]
    pub start_time: Option<u64>,
    /// Start price (for trend lines)
    #[serde(default)]
    pub start_price: Option<f64>,
    /// End timestamp (for trend lines)
    #[serde(default)]
    pub end_time: Option<u64>,
    /// End price (for trend lines)
    #[serde(default)]
    pub end_price: Option<f64>,
}

fn valid_annotation_price(price: f64) -> bool {
    price.is_finite() && price > 0.0
}

fn valid_annotation_color_component(component: f32) -> bool {
    component.is_finite() && (0.0..=1.0).contains(&component)
}

fn color_from_config(color: [f32; 3]) -> Option<Color> {
    color
        .iter()
        .all(|component| valid_annotation_color_component(*component))
        .then(|| Color::from_rgb(color[0], color[1], color[2]))
}

impl AnnotationKind {
    fn is_valid(&self) -> bool {
        match self {
            Self::HorizontalLevel { price } => valid_annotation_price(*price),
            Self::TrendLine { start, end } => {
                valid_annotation_price(start.1) && valid_annotation_price(end.1)
            }
        }
    }
}

impl Annotation {
    pub fn is_valid(&self) -> bool {
        valid_annotation_color_component(self.color.r)
            && valid_annotation_color_component(self.color.g)
            && valid_annotation_color_component(self.color.b)
            && valid_annotation_color_component(self.color.a)
            && self.kind.is_valid()
    }

    pub fn to_config(&self) -> AnnotationConfig {
        match &self.kind {
            AnnotationKind::HorizontalLevel { price } => AnnotationConfig {
                kind: "level".to_string(),
                color: [self.color.r, self.color.g, self.color.b],
                price: Some(*price),
                start_time: None,
                start_price: None,
                end_time: None,
                end_price: None,
            },
            AnnotationKind::TrendLine { start, end } => AnnotationConfig {
                kind: "trendline".to_string(),
                color: [self.color.r, self.color.g, self.color.b],
                price: None,
                start_time: Some(start.0),
                start_price: Some(start.1),
                end_time: Some(end.0),
                end_price: Some(end.1),
            },
        }
    }

    pub fn from_config(id: AnnotationId, cfg: &AnnotationConfig) -> Option<Self> {
        let color = color_from_config(cfg.color)?;
        let kind = match cfg.kind.as_str() {
            "level" => AnnotationKind::HorizontalLevel { price: cfg.price? },
            "trendline" => AnnotationKind::TrendLine {
                start: (cfg.start_time?, cfg.start_price?),
                end: (cfg.end_time?, cfg.end_price?),
            },
            _ => return None,
        };
        let annotation = Annotation { id, kind, color };
        annotation.is_valid().then_some(annotation)
    }
}

/// Hit-test tolerance in pixels for selecting/erasing annotations.
pub const ANNOTATION_HIT_TOLERANCE: f32 = 6.0;
