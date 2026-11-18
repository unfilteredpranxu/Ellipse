use super::{Paper, Template};

impl Paper {
    /// Visits stationery rectangles in top-left document coordinates, in page points.
    /// Canvas display and vector PDF export use the same geometry.
    pub fn pattern_rectangles(&self, mut visit: impl FnMut([f64; 4])) {
        use Template::*;

        let (width, height) = (f64::from(self.width), f64::from(self.height));
        let spacing = f64::from(self.spacing).max((width * height / 12_000.).sqrt());
        let mut row = |x, y, thickness| visit([x, y - thickness / 2., width - x, thickness]);
        match self.template {
            Blank => {}
            Dotted => {
                for x in 1..(width / spacing) as u32 {
                    for y in 1..(height / spacing) as u32 {
                        visit([f64::from(x) * spacing, f64::from(y) * spacing, 1.2, 1.2]);
                    }
                }
            }
            Music => {
                for staff in 1..(height / (spacing * 4.)) as u32 {
                    for line in 0..5 {
                        row(
                            0.,
                            (f64::from(staff) * 4. + f64::from(line) * 0.31) * spacing,
                            0.5,
                        );
                    }
                }
            }
            Planner => {
                for line in 1..10 {
                    row(0., height * f64::from(line) / 10., 0.5);
                }
                for line in 1..7 {
                    visit([width * f64::from(line) / 7. - 0.25, 0., 0.5, height]);
                }
            }
            Cornell => {
                row(0., height * 0.81, 0.5);
                for line in 2..(height * 0.81 / spacing) as u32 {
                    row(width * 0.26, f64::from(line) * spacing, 0.5);
                }
                visit([width * 0.26 - 0.25, 0., 0.5, height]);
            }
            Ruled | Grid | Graph => {
                let minor = if self.template == Graph {
                    spacing / 2.
                } else {
                    spacing
                };
                for line in 1..(height / minor) as u32 {
                    row(0., f64::from(line) * minor, 0.5);
                }
                if self.template == Graph {
                    for line in 1..(height / (spacing * 2.5)) as u32 {
                        row(0., f64::from(line) * spacing * 2.5, 0.85);
                    }
                }
                if self.template != Ruled {
                    for line in 1..(width / minor) as u32 {
                        visit([f64::from(line) * minor - 0.25, 0., 0.5, height]);
                    }
                }
                if self.template == Graph {
                    for line in 1..(width / (spacing * 2.5)) as u32 {
                        visit([f64::from(line) * spacing * 2.5 - 0.425, 0., 0.85, height]);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stationery_marks_stay_on_the_page_and_within_budget() {
        for (width, height) in [(5000., 5000.), (5000., 12.), (12., 5000.), (612., 792.)] {
            for template in Template::ALL {
                let paper = Paper {
                    template,
                    width,
                    height,
                    spacing: 4.,
                    ..Paper::default()
                };
                let mut count = 0;
                paper.pattern_rectangles(|[x, y, w, h]| {
                    count += 1;
                    assert!(x >= 0. && y >= 0. && w > 0. && h > 0.);
                    assert!(x + w <= f64::from(width) && y + h <= f64::from(height));
                });
                assert!(count <= 12_000);
                if template == Template::Blank {
                    assert_eq!(count, 0);
                } else if width == height {
                    assert!(count > 0);
                }
            }
        }
    }
}
