use iced::Color;
use serde::{Deserialize, Deserializer, Serialize};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Chart annotation types
// ---------------------------------------------------------------------------

pub type AnnotationId = u64;

/// A single (timestamp_ms, price) anchor point.
pub type Anchor = (u64, f64);

/// Drawing tool the user can activate via toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawingTool {
    /// One-click horizontal price level spanning the full chart width.
    HorizontalLevel,
    /// Two-click line segment between two anchors.
    TrendLine,
    /// Two-click line that extends past the second anchor toward the right edge.
    Ray,
    /// Two-click line that extends past both anchors to the chart edges.
    ExtendedLine,
    /// One-click vertical line marking a point in time.
    VerticalLine,
    /// Two-click rectangle / zone (supply-demand, consolidation box).
    Rectangle,
    /// Two-click persistent price/time measurement.
    Measure,
    /// Two-click Fibonacci retracement grid.
    FibRetracement,
    /// Three-click Fibonacci extension (A-B-C projection).
    FibExtension,
    /// Selection / edit mode: pick, drag, and restyle existing annotations.
    Select,
    /// Click an annotation to delete it.
    Eraser,
}

impl DrawingTool {
    /// Number of anchor clicks the tool collects before it commits a shape.
    /// Zero for the non-drawing tools (Select / Eraser).
    pub fn anchor_count(self) -> usize {
        match self {
            Self::HorizontalLevel | Self::VerticalLine => 1,
            Self::TrendLine
            | Self::Ray
            | Self::ExtendedLine
            | Self::Rectangle
            | Self::Measure
            | Self::FibRetracement => 2,
            Self::FibExtension => 3,
            Self::Select | Self::Eraser => 0,
        }
    }

    /// True for tools that place a shape (i.e. collect anchors).
    pub fn is_shape(self) -> bool {
        self.anchor_count() > 0
    }
}

/// Stroke style for line-based annotations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

/// Which Fibonacci tool an annotation represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FibKind {
    Retracement,
    Extension,
}

/// Per-annotation presentation: color (RGBA, alpha preserved), stroke width,
/// line style, optional text label, and lock/visibility flags.
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationStyle {
    pub color: Color,
    pub width: f32,
    pub line_style: LineStyle,
    pub label: Option<String>,
    pub locked: bool,
    pub visible: bool,
}

/// Default stroke width for line-based annotations.
pub const DEFAULT_ANNOTATION_WIDTH: f32 = 1.5;

impl Default for AnnotationStyle {
    fn default() -> Self {
        Self {
            color: DEFAULT_LINE_COLOR,
            width: DEFAULT_ANNOTATION_WIDTH,
            line_style: LineStyle::Solid,
            label: None,
            locked: false,
            visible: true,
        }
    }
}

impl AnnotationStyle {
    /// Style seeded for a freshly created annotation of `tool`.
    pub fn for_tool(tool: DrawingTool) -> Self {
        let color = match tool {
            DrawingTool::HorizontalLevel => DEFAULT_LEVEL_COLOR,
            DrawingTool::Measure => DEFAULT_MEASURE_COLOR,
            _ => DEFAULT_LINE_COLOR,
        };
        Self {
            color,
            ..Self::default()
        }
    }
}

/// A user-drawn annotation on a chart.
#[derive(Debug, Clone, PartialEq)]
pub enum AnnotationKind {
    /// A horizontal price level line spanning the full chart width.
    HorizontalLevel { price: f64 },
    /// A trend line between two (timestamp_ms, price) anchor points.
    TrendLine { start: Anchor, end: Anchor },
    /// Like a trend line but extends past `end` toward the right edge.
    Ray { start: Anchor, end: Anchor },
    /// Like a trend line but extends past both anchors to the chart edges.
    ExtendedLine { start: Anchor, end: Anchor },
    /// A vertical line at a point in time spanning the full chart height.
    VerticalLine { time: u64 },
    /// A rectangle / zone between two opposite corners.
    Rectangle { a: Anchor, b: Anchor },
    /// A persistent price/time measurement between two anchors.
    Measure { start: Anchor, end: Anchor },
    /// A Fibonacci retracement (2 anchors) or extension (3 anchors) grid.
    Fib { kind: FibKind, points: Vec<Anchor> },
}

#[derive(Debug, Clone)]
pub struct Annotation {
    pub id: AnnotationId,
    pub kind: AnnotationKind,
    pub style: AnnotationStyle,
}

/// Default color for horizontal levels (blue).
pub const DEFAULT_LEVEL_COLOR: Color = Color::from_rgb(0.478, 0.635, 0.969);
/// Default color for trend lines and most shapes (yellow).
pub const DEFAULT_LINE_COLOR: Color = Color::from_rgb(0.945, 0.980, 0.549);
/// Default color for the measure tool (teal).
pub const DEFAULT_MEASURE_COLOR: Color = Color::from_rgb(0.4, 0.85, 0.78);

// ---------------------------------------------------------------------------
// Fibonacci levels
// ---------------------------------------------------------------------------

/// Retracement ratios drawn between the two anchors.
pub const FIB_RETRACEMENT_LEVELS: &[f64] = &[0.0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.0];
/// Extension ratios projected from the third anchor.
pub const FIB_EXTENSION_LEVELS: &[f64] = &[0.0, 0.618, 1.0, 1.618, 2.0, 2.618];

/// Price of a retracement level: `start + (end - start) * ratio`.
pub fn fib_retracement_price(start: Anchor, end: Anchor, ratio: f64) -> f64 {
    start.1 + (end.1 - start.1) * ratio
}

/// Price of an extension level: `c + (b - a) * ratio`.
pub fn fib_extension_price(a: Anchor, b: Anchor, c: Anchor, ratio: f64) -> f64 {
    c.1 + (b.1 - a.1) * ratio
}

// ---------------------------------------------------------------------------
// Serializable config form for persistence
// ---------------------------------------------------------------------------

/// A persisted (timestamp_ms, price) anchor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AnchorConfig {
    pub t: u64,
    pub p: f64,
}

impl From<Anchor> for AnchorConfig {
    fn from((t, p): Anchor) -> Self {
        Self { t, p }
    }
}

impl AnchorConfig {
    fn to_anchor(self) -> Anchor {
        (self.t, self.p)
    }
}

/// Persisted line-style discriminant (lowercase for stable JSON).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LineStyleConfig {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

impl<'de> Deserialize<'de> for LineStyleConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let Some(value) = Option::<String>::deserialize(deserializer)? else {
            return Ok(Self::Solid);
        };
        Ok(match value.as_str() {
            "solid" => Self::Solid,
            "dashed" => Self::Dashed,
            "dotted" => Self::Dotted,
            _ => Self::Solid,
        })
    }
}

impl LineStyleConfig {
    fn is_default(&self) -> bool {
        matches!(self, Self::Solid)
    }
}

impl From<LineStyle> for LineStyleConfig {
    fn from(style: LineStyle) -> Self {
        match style {
            LineStyle::Solid => Self::Solid,
            LineStyle::Dashed => Self::Dashed,
            LineStyle::Dotted => Self::Dotted,
        }
    }
}

impl From<LineStyleConfig> for LineStyle {
    fn from(style: LineStyleConfig) -> Self {
        match style {
            LineStyleConfig::Solid => Self::Solid,
            LineStyleConfig::Dashed => Self::Dashed,
            LineStyleConfig::Dotted => Self::Dotted,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnnotationConfig {
    /// "level", "trendline", "ray", "extended", "vline", "rect", "measure",
    /// "fib_retracement", "fib_extension".
    #[serde(rename = "type")]
    pub kind: String,
    /// RGB color as [r, g, b] in 0.0..1.0 (legacy field; alpha stored separately).
    pub color: [f32; 3],
    // --- Legacy scalar geometry fields (kept verbatim for byte-stable output) ---
    /// Price (for horizontal levels).
    #[serde(default)]
    pub price: Option<f64>,
    /// Start timestamp (for two-anchor lines).
    #[serde(default)]
    pub start_time: Option<u64>,
    /// Start price (for two-anchor lines).
    #[serde(default)]
    pub start_price: Option<f64>,
    /// End timestamp (for two-anchor lines).
    #[serde(default)]
    pub end_time: Option<u64>,
    /// End price (for two-anchor lines).
    #[serde(default)]
    pub end_price: Option<f64>,
    // --- Additive fields (absent for legacy annotations) ---
    /// Timestamp (for vertical lines).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<u64>,
    /// Generic multi-point anchors (rectangles, Fibonacci grids).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<AnchorConfig>,
    /// Color alpha (defaults to fully opaque).
    #[serde(default = "default_alpha", skip_serializing_if = "is_default_alpha")]
    pub alpha: f32,
    /// Stroke width.
    #[serde(default = "default_width", skip_serializing_if = "is_default_width")]
    pub width: f32,
    /// Stroke style.
    #[serde(default, skip_serializing_if = "LineStyleConfig::is_default")]
    pub line_style: LineStyleConfig,
    /// Optional text label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether the annotation is locked against edits.
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
    /// Whether the annotation is rendered.
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub visible: bool,
}

fn default_alpha() -> f32 {
    1.0
}
fn default_width() -> f32 {
    DEFAULT_ANNOTATION_WIDTH
}
fn default_true() -> bool {
    true
}
fn is_default_alpha(value: &f32) -> bool {
    *value == 1.0
}
fn is_default_width(value: &f32) -> bool {
    *value == DEFAULT_ANNOTATION_WIDTH
}
fn is_false(value: &bool) -> bool {
    !*value
}
fn is_true(value: &bool) -> bool {
    *value
}

impl Default for AnnotationConfig {
    fn default() -> Self {
        Self {
            kind: String::new(),
            color: [
                DEFAULT_LINE_COLOR.r,
                DEFAULT_LINE_COLOR.g,
                DEFAULT_LINE_COLOR.b,
            ],
            price: None,
            start_time: None,
            start_price: None,
            end_time: None,
            end_price: None,
            time: None,
            anchors: Vec::new(),
            alpha: default_alpha(),
            width: default_width(),
            line_style: LineStyleConfig::Solid,
            label: None,
            locked: false,
            visible: true,
        }
    }
}

fn valid_annotation_price(price: f64) -> bool {
    price.is_finite() && price > 0.0
}

fn valid_annotation_color_component(component: f32) -> bool {
    component.is_finite() && (0.0..=1.0).contains(&component)
}

/// Build a color from a persisted RGB triple + alpha. Rejects malformed RGB
/// (returns `None`); malformed alpha falls back to fully opaque.
fn color_from_config(rgb: [f32; 3], alpha: f32) -> Option<Color> {
    if !rgb
        .iter()
        .all(|component| valid_annotation_color_component(*component))
    {
        return None;
    }
    let a = if valid_annotation_color_component(alpha) {
        alpha
    } else {
        1.0
    };
    Some(Color::from_rgba(rgb[0], rgb[1], rgb[2], a))
}

fn anchors_finite(points: &[Anchor]) -> bool {
    points
        .iter()
        .all(|(_, price)| valid_annotation_price(*price))
}

impl AnnotationKind {
    fn is_valid(&self) -> bool {
        match self {
            Self::HorizontalLevel { price } => valid_annotation_price(*price),
            Self::TrendLine { start, end }
            | Self::Ray { start, end }
            | Self::ExtendedLine { start, end }
            | Self::Measure { start, end } => {
                valid_annotation_price(start.1) && valid_annotation_price(end.1)
            }
            Self::VerticalLine { .. } => true,
            Self::Rectangle { a, b } => valid_annotation_price(a.1) && valid_annotation_price(b.1),
            Self::Fib { kind, points } => {
                let expected = match kind {
                    FibKind::Retracement => 2,
                    FibKind::Extension => 3,
                };
                points.len() == expected && anchors_finite(points)
            }
        }
    }

    /// Anchor points exposed as draggable handles (in placement order).
    /// Single-axis shapes (horizontal level, vertical line) expose none;
    /// they move by body drag.
    pub fn anchor_points(&self) -> Vec<Anchor> {
        match self {
            Self::HorizontalLevel { .. } | Self::VerticalLine { .. } => Vec::new(),
            Self::TrendLine { start, end }
            | Self::Ray { start, end }
            | Self::ExtendedLine { start, end }
            | Self::Measure { start, end } => vec![*start, *end],
            Self::Rectangle { a, b } => vec![*a, *b],
            Self::Fib { points, .. } => points.clone(),
        }
    }

    /// Replace the anchor at `index` (no-op if out of range / unsupported kind).
    pub fn set_anchor(&mut self, index: usize, anchor: Anchor) {
        match self {
            Self::HorizontalLevel { price } => {
                if index == 0 {
                    *price = anchor.1;
                }
            }
            Self::VerticalLine { time } => {
                if index == 0 {
                    *time = anchor.0;
                }
            }
            Self::TrendLine { start, end }
            | Self::Ray { start, end }
            | Self::ExtendedLine { start, end }
            | Self::Measure { start, end } => match index {
                0 => *start = anchor,
                1 => *end = anchor,
                _ => {}
            },
            Self::Rectangle { a, b } => match index {
                0 => *a = anchor,
                1 => *b = anchor,
                _ => {}
            },
            Self::Fib { points, .. } => {
                if let Some(point) = points.get_mut(index) {
                    *point = anchor;
                }
            }
        }
    }

    /// Translate the whole shape by `(dt, dp)` in (milliseconds, price) space.
    /// Timestamps saturate at zero; prices shift directly.
    pub fn translate(&mut self, dt: i64, dp: f64) {
        let shift_time = |t: u64| -> u64 { (t as i64).saturating_add(dt).max(0) as u64 };
        let shift = |(t, p): &mut Anchor| {
            *t = shift_time(*t);
            *p += dp;
        };
        match self {
            Self::HorizontalLevel { price } => *price += dp,
            Self::VerticalLine { time } => *time = shift_time(*time),
            Self::TrendLine { start, end }
            | Self::Ray { start, end }
            | Self::ExtendedLine { start, end }
            | Self::Measure { start, end } => {
                shift(start);
                shift(end);
            }
            Self::Rectangle { a, b } => {
                shift(a);
                shift(b);
            }
            Self::Fib { points, .. } => points.iter_mut().for_each(shift),
        }
    }
}

impl Annotation {
    pub fn is_valid(&self) -> bool {
        valid_annotation_color_component(self.style.color.r)
            && valid_annotation_color_component(self.style.color.g)
            && valid_annotation_color_component(self.style.color.b)
            && valid_annotation_color_component(self.style.color.a)
            && self.style.width.is_finite()
            && self.style.width > 0.0
            && self.kind.is_valid()
    }

    fn config_base(&self) -> AnnotationConfig {
        AnnotationConfig {
            color: [self.style.color.r, self.style.color.g, self.style.color.b],
            alpha: self.style.color.a,
            width: self.style.width,
            line_style: self.style.line_style.into(),
            label: self.style.label.clone(),
            locked: self.style.locked,
            visible: self.style.visible,
            ..AnnotationConfig::default()
        }
    }

    pub fn to_config(&self) -> AnnotationConfig {
        let mut cfg = self.config_base();
        match &self.kind {
            AnnotationKind::HorizontalLevel { price } => {
                cfg.kind = "level".to_string();
                cfg.price = Some(*price);
            }
            AnnotationKind::TrendLine { start, end } => {
                cfg.kind = "trendline".to_string();
                set_two_anchor_fields(&mut cfg, *start, *end);
            }
            AnnotationKind::Ray { start, end } => {
                cfg.kind = "ray".to_string();
                set_two_anchor_fields(&mut cfg, *start, *end);
            }
            AnnotationKind::ExtendedLine { start, end } => {
                cfg.kind = "extended".to_string();
                set_two_anchor_fields(&mut cfg, *start, *end);
            }
            AnnotationKind::Measure { start, end } => {
                cfg.kind = "measure".to_string();
                set_two_anchor_fields(&mut cfg, *start, *end);
            }
            AnnotationKind::VerticalLine { time } => {
                cfg.kind = "vline".to_string();
                cfg.time = Some(*time);
            }
            AnnotationKind::Rectangle { a, b } => {
                cfg.kind = "rect".to_string();
                cfg.anchors = vec![(*a).into(), (*b).into()];
            }
            AnnotationKind::Fib { kind, points } => {
                cfg.kind = match kind {
                    FibKind::Retracement => "fib_retracement",
                    FibKind::Extension => "fib_extension",
                }
                .to_string();
                cfg.anchors = points.iter().map(|p| (*p).into()).collect();
            }
        }
        cfg
    }

    pub fn from_config(id: AnnotationId, cfg: &AnnotationConfig) -> Option<Self> {
        let color = color_from_config(cfg.color, cfg.alpha)?;
        let width = if cfg.width.is_finite() && cfg.width > 0.0 {
            cfg.width
        } else {
            DEFAULT_ANNOTATION_WIDTH
        };
        let style = AnnotationStyle {
            color,
            width,
            line_style: cfg.line_style.into(),
            label: cfg.label.clone().filter(|label| !label.is_empty()),
            locked: cfg.locked,
            visible: cfg.visible,
        };
        let kind = match cfg.kind.as_str() {
            "level" => AnnotationKind::HorizontalLevel { price: cfg.price? },
            "trendline" => {
                two_anchor_kind(cfg, |start, end| AnnotationKind::TrendLine { start, end })?
            }
            "ray" => two_anchor_kind(cfg, |start, end| AnnotationKind::Ray { start, end })?,
            "extended" => two_anchor_kind(cfg, |start, end| AnnotationKind::ExtendedLine {
                start,
                end,
            })?,
            "measure" => two_anchor_kind(cfg, |start, end| AnnotationKind::Measure { start, end })?,
            "vline" => AnnotationKind::VerticalLine { time: cfg.time? },
            "rect" => {
                let (a, b) = anchors_pair(cfg)?;
                AnnotationKind::Rectangle { a, b }
            }
            "fib_retracement" => AnnotationKind::Fib {
                kind: FibKind::Retracement,
                points: anchors_vec(cfg, 2)?,
            },
            "fib_extension" => AnnotationKind::Fib {
                kind: FibKind::Extension,
                points: anchors_vec(cfg, 3)?,
            },
            _ => return None,
        };
        let annotation = Annotation { id, kind, style };
        annotation.is_valid().then_some(annotation)
    }
}

fn set_two_anchor_fields(cfg: &mut AnnotationConfig, start: Anchor, end: Anchor) {
    cfg.start_time = Some(start.0);
    cfg.start_price = Some(start.1);
    cfg.end_time = Some(end.0);
    cfg.end_price = Some(end.1);
}

fn two_anchor_kind(
    cfg: &AnnotationConfig,
    build: impl Fn(Anchor, Anchor) -> AnnotationKind,
) -> Option<AnnotationKind> {
    let start = (cfg.start_time?, cfg.start_price?);
    let end = (cfg.end_time?, cfg.end_price?);
    Some(build(start, end))
}

fn anchors_pair(cfg: &AnnotationConfig) -> Option<(Anchor, Anchor)> {
    if cfg.anchors.len() < 2 {
        return None;
    }
    Some((cfg.anchors[0].to_anchor(), cfg.anchors[1].to_anchor()))
}

fn anchors_vec(cfg: &AnnotationConfig, expected: usize) -> Option<Vec<Anchor>> {
    if cfg.anchors.len() != expected {
        return None;
    }
    Some(cfg.anchors.iter().map(|a| a.to_anchor()).collect())
}

/// Hit-test tolerance in pixels for selecting/erasing annotations.
pub const ANNOTATION_HIT_TOLERANCE: f32 = 6.0;
