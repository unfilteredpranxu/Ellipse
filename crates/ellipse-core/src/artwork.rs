use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_IMAGE_BYTES: usize = 24 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Slot {
    NewTabHero,
    NewTabDetail,
    TabletFeature,
    TabletDetail,
    TabletHelmet,
    TabletOrb,
    MagazineFeature,
    MagazineDetail,
    MagazineHelmet,
    MagazineOrb,
    MagazineWorkspace,
    MagazinePresets,
}

impl Slot {
    pub fn label(self) -> &'static str {
        match self {
            Self::NewTabHero => "New Tab hero",
            Self::NewTabDetail => "New Tab detail",
            Self::TabletFeature => "Tablet feature",
            Self::TabletDetail => "Tablet detail",
            Self::TabletHelmet => "Tablet lower left",
            Self::TabletOrb => "Tablet lower right",
            Self::MagazineFeature => "Magazine feature",
            Self::MagazineDetail => "Magazine portrait",
            Self::MagazineHelmet => "Magazine lower left",
            Self::MagazineOrb => "Magazine lower right",
            Self::MagazineWorkspace => "Magazine workspace thumbnail",
            Self::MagazinePresets => "Magazine presets thumbnail",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fit {
    #[default]
    Cover,
    Contain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Frame {
    pub x: u8,
    pub y: u8,
    pub zoom: u16,
    pub fit: Fit,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            x: 50,
            y: 50,
            zoom: 100,
            fit: Fit::Cover,
        }
    }
}

impl Frame {
    pub fn normalize(&mut self) {
        self.x = self.x.min(100);
        self.y = self.y.min(100);
        self.zoom = self.zoom.clamp(100, 300);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Assignment {
    pub asset: Option<String>,
    pub frame: Frame,
}

pub type Artworks = BTreeMap<Slot, Assignment>;

pub fn valid_asset_id(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
}

pub fn normalize_artworks(items: &mut Artworks) {
    items.retain(|_, item| {
        item.frame.normalize();
        item.asset.as_deref().is_none_or(valid_asset_id)
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(pub u8, pub u8, pub u8);

impl Color {
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }

    fn channels(self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }

    fn mix(self, other: Self, amount: u32) -> Self {
        let blend =
            |a: u8, b: u8| ((u32::from(a) * (100 - amount) + u32::from(b) * amount) / 100) as u8;
        Self(
            blend(self.0, other.0),
            blend(self.1, other.1),
            blend(self.2, other.2),
        )
    }

    fn spread(self) -> u8 {
        let [r, g, b] = self.channels();
        r.max(g).max(b) - r.min(g).min(b)
    }

    fn distance(self, other: Self) -> u32 {
        self.channels()
            .into_iter()
            .zip(other.channels())
            .map(|(a, b)| u32::from(a.abs_diff(b)).pow(2))
            .sum()
    }

    fn luminance(self) -> f64 {
        let [r, g, b] = self.channels().map(|channel| {
            let channel = f64::from(channel) / 255.0;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        });
        r * 0.2126 + g * 0.7152 + b * 0.0722
    }

    pub fn contrast(self, other: Self) -> f64 {
        let a = self.luminance();
        let b = other.luminance();
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    pub dominant: Color,
    pub secondary: Color,
    pub accent: Color,
}

pub struct ThemeTokens {
    pub canvas: Color,
    pub surface: Color,
    pub ink: Color,
    pub muted: Color,
    pub border: Color,
    pub selected: Color,
    pub selected_ink: Color,
}

impl Palette {
    pub fn tokens(self) -> ThemeTokens {
        // Keep the editorial page light and the interactive contrasts independent of source saturation.
        let canvas = Color(251, 250, 249).mix(self.dominant, 8);
        let surface = canvas.mix(self.secondary, 9);
        let ink = Color(17, 16, 16).mix(self.dominant, 8);
        let muted = Color(42, 41, 41).mix(self.secondary, 16);
        let selected = Color(21, 21, 21).mix(self.accent, 23);
        ThemeTokens {
            canvas,
            surface,
            ink,
            muted,
            border: surface.mix(muted, 28),
            selected,
            selected_ink: Color(255, 255, 255),
        }
    }

    pub fn swatches(self) -> [(&'static str, Color); 5] {
        let tokens = self.tokens();
        [
            ("Dominant", self.dominant),
            ("Secondary", self.secondary),
            ("Accent", self.accent),
            ("Surface", tokens.surface),
            ("Text", tokens.ink),
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TabletTheme {
    pub automatic: bool,
    pub palette: Option<Palette>,
}

impl Default for TabletTheme {
    fn default() -> Self {
        Self {
            automatic: true,
            palette: None,
        }
    }
}

pub fn extract_palette(rgba: &[u8]) -> Option<Palette> {
    // Fixed 4-bit histogram, with actual channel means and stable tie-breaking.
    let mut bins = vec![[0u32; 4]; 4096];
    let mut chromatic = 0u32;
    let mut total = 0u32;
    for p in rgba
        .as_chunks::<4>()
        .0
        .iter()
        .take(48 * 48)
        .filter(|pixel| pixel[3] >= 128)
    {
        let [r, g, b] = [p[0], p[1], p[2]];
        let index = (usize::from(r >> 4) << 8) | (usize::from(g >> 4) << 4) | usize::from(b >> 4);
        let bin = &mut bins[index];
        bin[0] += 1;
        for (sum, channel) in bin[1..].iter_mut().zip([r, g, b]) {
            *sum += u32::from(channel);
        }
        chromatic += u32::from(Color(r, g, b).spread() >= 21);
        total += 1;
    }
    let mut colors: Vec<_> = bins
        .into_iter()
        .filter(|bin| bin[0] > 0)
        .map(|[n, r, g, b]| (n, Color((r / n) as u8, (g / n) as u8, (b / n) as u8)))
        .collect();
    colors.sort_by_key(|(count, color)| (std::cmp::Reverse(*count), color.channels()));
    let dominant = colors.first()?.1;
    if chromatic * 12 < total {
        let gray = (u16::from(dominant.0) + u16::from(dominant.1) + u16::from(dominant.2)) / 3;
        let dominant = Color(gray as u8, gray as u8, gray as u8);
        let secondary = if gray < 125 {
            Color(207, 207, 207)
        } else {
            Color(74, 74, 74)
        };
        let accent = if gray.abs_diff(130) < 32 {
            Color(28, 28, 28)
        } else {
            Color(130, 130, 130)
        };
        return Some(Palette {
            dominant,
            secondary,
            accent,
        });
    }
    let secondary = colors
        .iter()
        .find(|(_, color)| color.distance(dominant) >= 4800)
        .map_or(dominant.mix(Color(231, 225, 218), 57), |(_, color)| *color);
    let neutral = if dominant.luminance() < 0.22 {
        Color(230, 220, 205)
    } else {
        Color(36, 38, 42)
    };
    let accent = colors
        .iter()
        .filter(|(_, color)| {
            color.distance(dominant) >= 2900
                && color.distance(secondary) >= 2900
                && color.spread() > 24
        })
        .max_by_key(|(n, color)| *n * (u32::from(color.spread()) + 25))
        .map_or(dominant.mix(neutral, 65), |(_, color)| {
            color.mix(Color(137, 130, 125), 20)
        });
    Some(Palette {
        dominant,
        secondary,
        accent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantization_is_deterministic_and_chromatic() {
        let bytes = [
            [100, 27, 20, 255],
            [100, 27, 20, 255],
            [17, 77, 165, 255],
            [91, 147, 43, 255],
        ]
        .concat();
        let palette = extract_palette(&bytes).unwrap();
        assert_eq!(extract_palette(&bytes), Some(palette));
        assert_eq!(palette.dominant, Color(100, 27, 20));
        assert_ne!(palette.secondary, palette.accent);
        assert_eq!(extract_palette(&[17, 89, 30, 0]), None);
        let mono = extract_palette(&[17, 16, 18, 255]).unwrap();
        assert!(mono.swatches()[..3]
            .iter()
            .all(|(_, color)| color.spread() == 0));
    }

    #[test]
    fn theme_text_meets_aa_for_extreme_input_colors() {
        let colors = [
            Color(0, 0, 0),
            Color(255, 255, 255),
            Color(255, 0, 0),
            Color(0, 255, 0),
            Color(0, 0, 255),
        ];
        for dominant in colors {
            for secondary in colors {
                for accent in colors {
                    let tokens = Palette {
                        dominant,
                        secondary,
                        accent,
                    }
                    .tokens();
                    for bg in [tokens.canvas, tokens.surface] {
                        assert!(tokens.ink.contrast(bg) >= 4.5);
                        assert!(tokens.muted.contrast(bg) >= 4.5);
                    }
                    assert!(tokens.selected.contrast(tokens.selected_ink) >= 4.5);
                }
            }
        }
    }
}
