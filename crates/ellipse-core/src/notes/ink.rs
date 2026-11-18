use super::{Book, Id};
use crate::artwork::Color;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
    pub tilt: f32,
    pub azimuth: f32,
}

impl Point {
    pub fn valid(self) -> bool {
        [self.x, self.y, self.pressure, self.tilt, self.azimuth]
            .into_iter()
            .all(f32::is_finite)
            && self.x.abs() <= 100_000.
            && self.y.abs() <= 100_000.
            && (0. ..=1.).contains(&self.pressure)
    }

    fn lerp(self, end: Self, t: f32) -> Self {
        let mix = |a, b| a + (b - a) * t;
        Self {
            x: mix(self.x, end.x),
            y: mix(self.y, end.y),
            pressure: mix(self.pressure, end.pressure),
            tilt: mix(self.tilt, end.tilt),
            azimuth: mix(self.azimuth, end.azimuth),
        }
    }

    pub fn distance(self, other: Self) -> f32 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Nib {
    #[default]
    Ballpoint,
    Fountain,
    Fineliner,
    Pencil,
    Brush,
    Calligraphy,
    Highlighter,
}

impl Nib {
    pub const ALL: [Self; 7] = [
        Self::Ballpoint,
        Self::Fountain,
        Self::Fineliner,
        Self::Pencil,
        Self::Brush,
        Self::Calligraphy,
        Self::Highlighter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ballpoint => "Ballpoint",
            Self::Fountain => "Fountain",
            Self::Fineliner => "Fineliner",
            Self::Pencil => "Pencil",
            Self::Brush => "Brush",
            Self::Calligraphy => "Calligraphy",
            Self::Highlighter => "Highlighter",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pen {
    pub nib: Nib,
    pub width: f32,
    pub opacity: f32,
    pub color: Color,
    pub smoothing: f32,
}

impl Default for Pen {
    fn default() -> Self {
        Self {
            nib: Nib::Ballpoint,
            width: 3.,
            opacity: 1.,
            color: Color(33, 40, 57),
            smoothing: 0.23,
        }
    }
}

impl Pen {
    pub fn preset(nib: Nib) -> Self {
        let (width, opacity, color) = match nib {
            Nib::Ballpoint => (3., 1., Color(33, 40, 57)),
            Nib::Fountain => (4., 1., Color(48, 61, 97)),
            Nib::Fineliner => (1.7, 1., Color(35, 36, 38)),
            Nib::Pencil => (2.8, 0.67, Color(53, 51, 52)),
            Nib::Brush => (10., 0.91, Color(86, 40, 48)),
            Nib::Calligraphy => (8., 0.96, Color(32, 45, 38)),
            Nib::Highlighter => (19., 0.30, Color(233, 168, 40)),
        };
        Self {
            nib,
            width,
            opacity,
            color,
            ..Self::default()
        }
    }

    pub fn valid(self) -> bool {
        (0.2..=72.).contains(&self.width)
            && (0.05..=1.).contains(&self.opacity)
            && (0. ..=0.85).contains(&self.smoothing)
    }

    pub fn radius(self, point: Point, direction: f32) -> f32 {
        let p = point.pressure.clamp(0., 1.);
        let factor = match self.nib {
            Nib::Ballpoint => 0.62 + 0.65 * p,
            Nib::Fountain => 0.27 + 1.23 * p.sqrt(),
            Nib::Fineliner => 0.92 + 0.12 * p,
            Nib::Pencil => (0.5 + 0.7 * p) * (1. + 0.4 * point.tilt.clamp(0., 1.)),
            Nib::Brush => 0.18 + 1.65 * p.powf(1.4),
            Nib::Calligraphy => {
                (0.35 + 1.12 * (direction - point.azimuth - 0.65).sin().abs()) * (0.6 + 0.55 * p)
            }
            Nib::Highlighter => 1.,
        };
        self.width * factor * 0.5
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    pub id: Id,
    pub pen: Pen,
    pub points: Vec<Point>,
}

impl Stroke {
    pub fn valid(&self) -> bool {
        self.pen.valid()
            && !self.points.is_empty()
            && self.points.len() <= 32768
            && self.points.iter().all(|p| p.valid())
    }

    pub fn append(&mut self, point: Point) {
        if self.points.len() >= 32768 || !point.valid() {
            return;
        }
        if let Some(last) = self.points.last().copied() {
            if last.distance(point) < 0.17 && (last.pressure - point.pressure).abs() < 0.025 {
                return;
            }
            self.points.push(last.lerp(point, 1. - self.pen.smoothing));
        } else {
            self.points.push(point);
        }
    }
}

/// Render one joined shape per stroke so transparency never accumulates at its sample boundaries.
pub fn outline(stroke: &Stroke) -> Vec<Point> {
    let points = &stroke.points;
    if points.len() == 1 {
        let center = points[0];
        let r = stroke.pen.radius(center, 0.);
        return (0..16)
            .map(|i| {
                let theta = i as f32 * std::f32::consts::TAU / 16.;
                Point {
                    x: center.x + r * theta.cos(),
                    y: center.y + r * theta.sin(),
                    ..center
                }
            })
            .collect();
    }
    let mut sides = [
        Vec::with_capacity(points.len()),
        Vec::with_capacity(points.len()),
    ];
    for (index, &point) in points.iter().enumerate() {
        let left = points[index.saturating_sub(1)];
        let right = points[(index + 1).min(points.len() - 1)];
        let dir = (right.y - left.y).atan2(right.x - left.x);
        let r = stroke.pen.radius(point, dir);
        let (x, y) = (-dir.sin() * r, dir.cos() * r);
        sides[0].push(Point {
            x: point.x + x,
            y: point.y + y,
            ..point
        });
        sides[1].push(Point {
            x: point.x - x,
            y: point.y - y,
            ..point
        });
    }
    let [mut left, right] = sides;
    left.extend(right.into_iter().rev());
    left
}

/// Attach all surviving vector pieces after clipping each segment against an eraser disc.
pub fn clip_disc(stroke: &Stroke, center: Point, radius: f32) -> Option<Vec<Vec<Point>>> {
    let affected = radius + stroke.pen.width * 0.5;
    if stroke.points.len() == 1 {
        return (stroke.points[0].distance(center) < affected).then(Vec::new);
    }
    let mut pieces = Vec::new();
    let mut current = Vec::new();
    let mut clipped = false;
    for edge in stroke.points.windows(2) {
        let [a, b] = [edge[0], edge[1]];
        let (x, y) = (a.x - center.x, a.y - center.y);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx * dx + dy * dy;
        let dot = x * dx + y * dy;
        let disc = dot * dot - len * (x * x + y * y - affected * affected);
        let interval = if len <= f32::EPSILON {
            (a.distance(center) < affected).then_some((0., 1.))
        } else if disc > 0. {
            let spread = disc.sqrt();
            let enter = ((-dot - spread) / len).max(0.);
            let exit = ((-dot + spread) / len).min(1.);
            (enter < exit).then_some((enter, exit))
        } else {
            None
        };
        if let Some((enter, exit)) = interval {
            clipped = true;
            if enter > 0. {
                if current.is_empty() {
                    current.push(a);
                }
                current.push(a.lerp(b, enter));
            }
            if !current.is_empty() {
                pieces.push(std::mem::take(&mut current));
            }
            if exit < 1. {
                current.extend([a.lerp(b, exit), b]);
            }
        } else {
            if current.is_empty() {
                current.push(a);
            }
            current.push(b);
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }
    clipped.then_some(pieces)
}

pub fn lasso_contains(polygon: &[Point], point: Point) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut prev = *polygon.last().unwrap();
    for &next in polygon {
        if (prev.y > point.y) != (next.y > point.y)
            && point.x < (next.x - prev.x) * (point.y - prev.y) / (next.y - prev.y) + prev.x
        {
            inside = !inside;
        }
        prev = next;
    }
    inside
}

/// A nearly straight lasso drag is the diagonal of a selection box.
/// Adapted from OwlLuna Canvas/InkLasso.swift at 7ea2a8c7 (MIT, © 2024–2026 Owais Khan).
/// The notice ships as owlluna-license.txt.
pub fn lasso_outline(path: &[Point]) -> Vec<Point> {
    let [first, .., last] = path else {
        return path.to_vec();
    };
    let dx = last.x - first.x;
    let dy = last.y - first.y;
    let span = dx.hypot(dy);
    if span <= 12. || dx.abs() < 1. || dy.abs() < 1. {
        return path.to_vec();
    }
    let tolerance = (0.06 * span).max(4.);
    if path.iter().any(|point| {
        let x = point.x - first.x;
        let y = point.y - first.y;
        let along = ((x * dx + y * dy) / (span * span)).clamp(0., 1.);
        (x - dx * along).hypot(y - dy * along) > tolerance
    }) {
        return path.to_vec();
    }
    vec![
        *first,
        Point {
            x: last.x,
            ..*first
        },
        *last,
        Point {
            y: last.y,
            ..*first
        },
    ]
}

pub fn lasso_hits(polygon: &[Point], stroke: &Stroke) -> bool {
    let cross =
        |a: Point, b: Point, c: Point| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    stroke.points.iter().any(|p| lasso_contains(polygon, *p))
        || (polygon.len() >= 3
            && stroke.points.windows(2).any(|line| {
                polygon
                    .iter()
                    .zip(polygon.iter().cycle().skip(1))
                    .any(|(&a, &b)| {
                        let [p, q] = [line[0], line[1]];
                        cross(a, b, p) * cross(a, b, q) < 0. && cross(p, q, a) * cross(p, q, b) < 0.
                    })
            }))
}

pub fn transform(points: &mut [Point], center: Point, dx: f32, dy: f32, scale: f32, angle: f32) {
    let (sin, cos) = angle.sin_cos();
    for p in points {
        let (x, y) = ((p.x - center.x) * scale, (p.y - center.y) * scale);
        p.x = center.x + x * cos - y * sin + dx;
        p.y = center.y + x * sin + y * cos + dy;
    }
}

#[derive(Default)]
pub struct History {
    undo: Vec<Book>,
    redo: Vec<Book>,
}

impl History {
    pub fn record(&mut self, previous: Book) {
        self.redo.clear();
        self.undo.push(previous);
        while self.undo.len() > 48
            || (self.undo.len() > 1 && self.undo.iter().map(cost).sum::<usize>() > 12 * 1024 * 1024)
        {
            self.undo.remove(0);
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo(&mut self, book: &mut Book) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(book, previous));
        true
    }
    pub fn redo(&mut self, book: &mut Book) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(book, next));
        true
    }
}

fn cost(book: &Book) -> usize {
    book.pages
        .iter()
        .map(|page| {
            page.text.iter().map(|t| t.value.len()).sum::<usize>()
                + page
                    .ink
                    .iter()
                    .map(|s| s.points.len() * std::mem::size_of::<Point>() + 80)
                    .sum::<usize>()
                + 160
        })
        .sum()
}

#[cfg(test)]
mod tests {
    #[test]
    fn diagonal_lasso_encloses_a_box_and_curved_lasso_is_preserved() {
        let point = |x, y| Point {
            x,
            y,
            ..Point::default()
        };
        let path = [point(10., 20.), point(36., 45.), point(60., 70.)];
        let outline = lasso_outline(&path);
        assert_eq!(outline.len(), 4);
        assert!(lasso_contains(&outline, point(12., 65.)));
        let curved = [
            point(10., 20.),
            point(9., 70.),
            point(62., 71.),
            point(10., 20.),
        ];
        assert_eq!(lasso_outline(&curved), curved);
        assert_eq!(lasso_outline(&[]), []);
    }

    use super::*;
    use crate::notes::{Paper, Vault};

    #[test]
    fn sparse_stroke_splits_and_preserves_pressure() {
        let left = Point {
            x: 0.,
            pressure: 0.2,
            ..Point::default()
        };
        let right = Point {
            x: 100.,
            pressure: 0.8,
            ..left
        };
        let pen = Pen {
            width: 2.,
            ..Pen::default()
        };
        let stroke = Stroke {
            id: 4,
            pen,
            points: vec![left, right],
        };
        let pieces = clip_disc(&stroke, Point { x: 50., ..left }, 9.).unwrap();
        assert_eq!(pieces.len(), 2);
        assert!((pieces[0][1].x - 40.).abs() < 0.0001);
        assert!((pieces[0][1].pressure - 0.44).abs() < 0.0001);
        assert!(clip_disc(
            &stroke,
            Point {
                x: 50.,
                y: 50.,
                ..left
            },
            9.
        )
        .is_none());
        assert!(pen.radius(right, 0.) > pen.radius(left, 0.));
        let polygon =
            [(30., -5.), (60., -5.), (60., 5.), (30., 5.)].map(|(x, y)| Point { x, y, ..left });
        assert!(lasso_hits(&polygon, &stroke));
    }

    #[test]
    fn undo_redo_groups_gesture_and_preserves_original() {
        let mut book = Vault::default().new_book("Test", 1, Paper::default());
        let original = book.clone();
        let mut history = History::default();
        history.record(original.clone());
        book.pages[0].ink.push(Stroke {
            id: 3,
            pen: Pen::default(),
            points: vec![Point::default()],
        });
        let changed = book.clone();
        assert!(history.undo(&mut book));
        assert_eq!(book, original);
        assert!(history.redo(&mut book));
        assert_eq!(book, changed);
    }
}
