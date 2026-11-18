use crate::artwork::{valid_asset_id, Frame, TabletTheme};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    NewTab,
    Tablet,
    Magazine,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::NewTab, Self::Tablet, Self::Magazine];

    pub fn label(self) -> &'static str {
        match self {
            Self::NewTab => "New Tab",
            Self::Tablet => "Tablet",
            Self::Magazine => "Magazine",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClockFont {
    #[default]
    Serif,
    Sans,
    Mono,
    Italic,
}

impl ClockFont {
    pub const ALL: [Self; 4] = [Self::Serif, Self::Sans, Self::Mono, Self::Italic];

    pub fn label(self) -> &'static str {
        match self {
            Self::Serif => "Serif",
            Self::Sans => "Sans",
            Self::Mono => "Monospace",
            Self::Italic => "Italic serif",
        }
    }

    pub fn css(self) -> &'static str {
        match self {
            Self::Serif => "serif",
            Self::Sans => "sans",
            Self::Mono => "mono",
            Self::Italic => "italic",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum DateStyle {
    #[default]
    Long,
    Short,
    Numeric,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum Alignment {
    Start,
    #[default]
    Center,
    End,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Clock {
    pub opacity: u8,
    pub blur: u8,
    pub font: ClockFont,
    pub size: u8,
    pub weight: u16,
    pub color: String,
    pub twenty_four: bool,
    pub seconds: bool,
    pub show_date: bool,
    pub date: DateStyle,
    pub horizontal: Alignment,
    pub vertical: Alignment,
    pub card: bool,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            opacity: 96,
            blur: 24,
            font: ClockFont::Serif,
            size: 100,
            weight: 400,
            color: "#ffffff".into(),
            twenty_four: false,
            seconds: false,
            show_date: true,
            date: DateStyle::Long,
            horizontal: Alignment::Center,
            vertical: Alignment::Center,
            card: true,
        }
    }
}

impl Clock {
    pub fn normalize(&mut self) {
        self.opacity = self.opacity.clamp(75, 100);
        self.blur = self.blur.min(40);
        self.size = self.size.clamp(55, 115);
        self.weight = self.weight.clamp(400, 800);
        let hex = self.color.strip_prefix('#').unwrap_or_default();
        let bright = hex.len() == 6
            && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
            && u32::from_str_radix(hex, 16).is_ok_and(|rgb| {
                let linear = |shift: u32| {
                    let channel = ((rgb >> shift) & 255) as f64 / 255.0;
                    ((channel + 0.055) / 1.055).powf(2.4)
                };
                0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0) >= 0.40
            });
        if !bright {
            self.color = "#ffffff".into();
        }
    }

    pub fn time(&self, hour: u32, minute: u32, second: u32) -> (String, Option<&'static str>) {
        let (mut time, period) = if self.twenty_four {
            (format!("{hour:02}:{minute:02}"), None)
        } else {
            let display = (hour + 11) % 12 + 1;
            let period = if hour < 12 { "AM" } else { "PM" };
            (format!("{display}:{minute:02}"), Some(period))
        };
        if self.seconds {
            time.push_str(&format!(":{second:02}"));
        }
        (time, period)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Block {
    Clock,
    Hero,
    ClockControls,
    Intro,
    Presets,
    Palette,
    Actions,
    LiveControls,
    Detail,
}

impl Block {
    pub const ALL: [Self; 9] = [
        Self::Clock,
        Self::Hero,
        Self::ClockControls,
        Self::Intro,
        Self::Presets,
        Self::Palette,
        Self::Actions,
        Self::LiveControls,
        Self::Detail,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Clock => "Clock",
            Self::Hero => "Hero artwork",
            Self::ClockControls => "Clock controls",
            Self::Intro => "Introduction",
            Self::Presets => "Presets",
            Self::Palette => "Palette",
            Self::Actions => "Quick actions",
            Self::LiveControls => "Live controls",
            Self::Detail => "Artwork detail",
        }
    }

    pub fn css(self) -> &'static str {
        match self {
            Self::Clock => "clock",
            Self::Hero => "hero",
            Self::ClockControls => "clock-controls",
            Self::Intro => "intro",
            Self::Presets => "presets",
            Self::Palette => "palette",
            Self::Actions => "actions",
            Self::LiveControls => "live",
            Self::Detail => "detail",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlockSize {
    #[default]
    Standard,
    Wide,
    Tall,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Tile {
    pub block: Block,
    pub size: BlockSize,
    pub visible: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Wallpaper {
    #[default]
    Plain,
    Artwork,
    Photo(String),
    Local(String),
}

pub const MAX_WALLPAPER_BYTES: usize = 512 * 1024;
pub const MAX_PRESETS: usize = 4;
pub const MAX_PRESET_BYTES: usize = 3 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub clock: Clock,
    pub wallpaper: Wallpaper,
    pub wallpaper_opacity: u8,
    pub wallpaper_frame: Frame,
    pub tablet_theme: TabletTheme,
    pub panel_opacity: u8,
    pub zoom: u16,
    pub gap: u8,
    pub tone: u8,
    pub tiles: Vec<Tile>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            clock: Clock::default(),
            wallpaper: Wallpaper::Plain,
            wallpaper_opacity: 18,
            wallpaper_frame: Frame::default(),
            tablet_theme: TabletTheme::default(),
            panel_opacity: 94,
            zoom: 260,
            gap: 13,
            tone: 24,
            tiles: Block::ALL
                .into_iter()
                .map(|block| Tile {
                    block,
                    size: BlockSize::Standard,
                    visible: true,
                })
                .collect(),
        }
    }
}

impl Appearance {
    pub fn normalize(&mut self) {
        self.clock.normalize();
        self.wallpaper_opacity = self.wallpaper_opacity.min(45);
        self.panel_opacity = self.panel_opacity.clamp(80, 100);
        self.zoom = self.zoom.clamp(100, 320);
        self.gap = self.gap.clamp(8, 24);
        self.tone = self.tone.min(40);
        self.wallpaper_frame.normalize();
        let valid = match &self.wallpaper {
            Wallpaper::Photo(uri) => valid_wallpaper(uri),
            Wallpaper::Local(id) => valid_asset_id(id),
            _ => true,
        };
        if !valid {
            self.wallpaper = Wallpaper::Plain;
            self.tablet_theme.palette = None;
        }
        let mut unique = Vec::<Tile>::with_capacity(Block::ALL.len());
        for tile in self.tiles.drain(..).chain(Self::default().tiles) {
            if !unique.iter().any(|item| item.block == tile.block) {
                unique.push(tile);
            }
        }
        unique
            .iter_mut()
            .filter(|tile| tile.block == Block::Clock)
            .for_each(|tile| tile.visible = true);
        self.tiles = unique;
    }

    pub fn move_tile(&mut self, block: Block, offset: isize) {
        if let Some(from) = self.tiles.iter().position(|tile| tile.block == block) {
            let to = from.saturating_add_signed(offset).min(self.tiles.len() - 1);
            self.tiles.swap(from, to);
        }
    }
}

fn valid_wallpaper(uri: &str) -> bool {
    uri.len() <= MAX_WALLPAPER_BYTES.div_ceil(3) * 4 + 32
        && uri.split_once(',').is_some_and(|(kind, content)| {
            matches!(
                kind,
                "data:image/jpeg;base64" | "data:image/png;base64" | "data:image/webp;base64"
            ) && !content.is_empty()
                && content
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
        })
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub mode: Mode,
    pub appearance: Appearance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub settings: Preferences,
}

pub fn normalize_presets(presets: &mut Vec<Preset>) {
    presets.retain_mut(|preset| {
        preset.name = preset.name.trim().chars().take(48).collect();
        preset.settings.appearance.normalize();
        !preset.name.is_empty()
    });
    presets.truncate(MAX_PRESETS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_formats_midnight_noon_and_seconds() {
        let mut clock = Clock::default();
        assert_eq!(clock.time(0, 9, 4), ("12:09".into(), Some("AM")));
        assert_eq!(clock.time(12, 9, 4), ("12:09".into(), Some("PM")));
        clock.twenty_four = true;
        clock.seconds = true;
        assert_eq!(clock.time(0, 9, 4), ("00:09:04".into(), None));
        assert_eq!(clock.time(23, 59, 3), ("23:59:03".into(), None));
    }

    #[test]
    fn imported_appearance_keeps_clock_visible_and_bounds_untrusted_values() {
        let mut look = Appearance {
            clock: Clock {
                color: "#011000".into(),
                opacity: 0,
                blur: 255,
                ..Default::default()
            },
            wallpaper: Wallpaper::Photo("data:image/svg+xml;base64,AAAA".into()),
            tiles: vec![
                Tile {
                    block: Block::Clock,
                    size: BlockSize::Wide,
                    visible: false
                };
                2
            ],
            ..Default::default()
        };
        look.normalize();
        assert_eq!(look.clock.color, "#ffffff");
        assert_eq!((look.clock.opacity, look.clock.blur), (75, 40));
        assert!(matches!(look.wallpaper, Wallpaper::Plain));
        assert_eq!(look.tiles.len(), Block::ALL.len());
        assert!(look.tiles[0].visible);
        look.move_tile(Block::Clock, 1);
        assert_eq!(look.tiles[1].block, Block::Clock);
    }
}
