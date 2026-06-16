// ---------------------------------------------------------------------------
// Chart Geometry
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

/// Pixel tolerance for hitting an order line with the cursor.
pub(super) const ORDER_HIT_TOLERANCE: f32 = 5.0;

/// Distance from point (px, py) to the line segment (x1,y1)-(x2,y2).
pub(super) fn point_to_segment_dist(px: f32, py: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-6 {
        return ((px - x1).powi(2) + (py - y1).powi(2)).sqrt();
    }
    let t = ((px - x1) * dx + (py - y1) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj_x = x1 + t * dx;
    let proj_y = y1 + t * dy;
    ((px - proj_x).powi(2) + (py - proj_y).powi(2)).sqrt()
}

/// How far past the anchors a line should be extrapolated before clipping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineExtension {
    /// Draw exactly the segment between the two anchors.
    Segment,
    /// Extend past the second anchor toward the chart edge (a ray).
    Forward,
    /// Extend past both anchors to the chart edges (an infinite line).
    Both,
}

/// Extrapolate the line through `(x1,y1)-(x2,y2)` per `extension` and clip it to
/// the rectangle `[0,w] x [0,h]`. Returns the visible segment endpoints, or
/// `None` when the line is degenerate or lies entirely outside the rectangle.
///
/// Endpoints are produced in source (pre-fisheye) coordinates so the caller can
/// project the already-clipped, in-bounds points without the projection
/// clamping a far-off endpoint and bending the line.
pub(super) fn extend_and_clip_line(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    w: f32,
    h: f32,
    extension: LineExtension,
) -> Option<(f32, f32, f32, f32)> {
    if !(x1.is_finite() && y1.is_finite() && x2.is_finite() && y2.is_finite())
        || w <= 0.0
        || h <= 0.0
    {
        return None;
    }

    let (ax, ay, bx, by) = match extension {
        LineExtension::Segment => (x1, y1, x2, y2),
        LineExtension::Forward | LineExtension::Both => {
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 1e-6 {
                return None;
            }
            let reach = (w + h) * 2.0 + len;
            let ux = dx / len;
            let uy = dy / len;
            let far_b = (x2 + ux * reach, y2 + uy * reach);
            let near_a = match extension {
                LineExtension::Forward => (x1, y1),
                _ => (x1 - ux * reach, y1 - uy * reach),
            };
            (near_a.0, near_a.1, far_b.0, far_b.1)
        }
    };

    clip_segment_to_rect(ax, ay, bx, by, w, h)
}

/// Liang-Barsky clip of segment `(x1,y1)-(x2,y2)` to `[0,w] x [0,h]`.
pub(super) fn clip_segment_to_rect(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    w: f32,
    h: f32,
) -> Option<(f32, f32, f32, f32)> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;

    // (p, q) pairs for the four edges: left, right, top, bottom.
    let edges = [(-dx, x1), (dx, w - x1), (-dy, y1), (dy, h - y1)];
    for (p, q) in edges {
        if p.abs() < 1e-9 {
            // Line parallel to this edge; reject if it starts outside.
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                if r > t1 {
                    return None;
                }
                if r > t0 {
                    t0 = r;
                }
            } else {
                if r < t0 {
                    return None;
                }
                if r < t1 {
                    t1 = r;
                }
            }
        }
    }

    if t1 < t0 {
        return None;
    }
    Some((x1 + t0 * dx, y1 + t0 * dy, x1 + t1 * dx, y1 + t1 * dy))
}
