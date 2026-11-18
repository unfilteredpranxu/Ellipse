mod ink;
mod study;

use crate::artwork::{valid_asset_id, Color, Frame};
pub use ink::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub use study::*;

pub const SCHEMA: u32 = 1;
pub const MAX_PDF: usize = 24 * 1024 * 1024;
pub const MAX_STATE: usize = 32 * 1024 * 1024;
pub type Id = u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Template {
    Blank,
    #[default]
    Ruled,
    Dotted,
    Grid,
    Graph,
    Cornell,
    Planner,
    Music,
}

impl Template {
    pub const ALL: [Self; 8] = [
        Self::Blank,
        Self::Ruled,
        Self::Dotted,
        Self::Grid,
        Self::Graph,
        Self::Cornell,
        Self::Planner,
        Self::Music,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Blank => "Blank",
            Self::Ruled => "Ruled",
            Self::Dotted => "Dotted",
            Self::Grid => "Grid",
            Self::Graph => "Graph",
            Self::Cornell => "Cornell",
            Self::Planner => "Planner",
            Self::Music => "Music staff",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Paper {
    pub template: Template,
    pub width: f32,
    pub height: f32,
    pub background: Color,
    pub pattern: Color,
    pub spacing: f32,
}

impl Default for Paper {
    fn default() -> Self {
        Self {
            template: Template::Ruled,
            width: 612.,
            height: 792.,
            background: Color(255, 252, 245),
            pattern: Color(189, 184, 174),
            spacing: 24.,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub id: Id,
    pub paper: Paper,
    pub ink: Vec<Stroke>,
    pub text: Vec<Text>,
    pub bookmarked: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub id: Id,
    pub at: Point,
    pub value: String,
    pub size: f32,
    pub color: Color,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
}

#[derive(Serialize, Deserialize)]
pub struct NotebookFile {
    pub version: u32,
    pub notebook: Book,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Folder {
    pub id: Id,
    pub title: String,
    pub parent: Option<Id>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Disposition {
    #[default]
    Active,
    Archived,
    Trash,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pdf {
    pub asset: String,
    pub title: String,
    // PDF.js viewport matrices at scale 1. Each maps PDF points to top-left page space.
    pub transforms: Vec<[f32; 6]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Book {
    pub id: Id,
    pub title: String,
    pub subject: String,
    pub collection: String,
    pub tags: Vec<String>,
    pub color: Color,
    pub cover: Cover,
    #[serde(default)]
    pub cover_art: Option<LocalImage>,
    pub folder: Option<Id>,
    pub favorite: bool,
    pub disposition: Disposition,
    pub created: i64,
    pub modified: i64,
    pub opened: i64,
    pub page: usize,
    pub zoom: u16,
    pub scroll: f64,
    pub pen: Pen,
    #[serde(default)]
    pub presets: Vec<Pen>,
    pub pdf: Option<Pdf>,
    pub pages: Vec<Page>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cover {
    #[default]
    Cloth,
    Bands,
    Orbit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vault {
    pub schema: u32,
    pub next: Id,
    pub books: Vec<Book>,
    pub folders: Vec<Folder>,
    pub study: Study,
    pub dark: bool,
    #[serde(default)]
    pub appearance: LibraryAppearance,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalImage {
    pub asset: String,
    pub frame: Frame,
}

impl LocalImage {
    fn valid(&self) -> bool {
        valid_asset_id(&self.asset)
            && self.frame.x <= 100
            && self.frame.y <= 100
            && (100..=300).contains(&self.frame.zoom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LibraryAppearance {
    pub wallpaper: Option<LocalImage>,
    pub opacity: u8,
    pub brightness: u8,
    pub blur: u8,
    pub frost: u8,
}

impl Default for LibraryAppearance {
    fn default() -> Self {
        Self {
            wallpaper: None,
            opacity: 15,
            brightness: 90,
            blur: 0,
            frost: 0,
        }
    }
}

impl Default for Vault {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            next: 1,
            books: vec![],
            folders: vec![],
            study: Study::default(),
            dark: false,
            appearance: LibraryAppearance::default(),
        }
    }
}

pub fn valid_pdf(bytes: &[u8]) -> bool {
    bytes.len() <= MAX_PDF
        && bytes.starts_with(b"%PDF-")
        && bytes[bytes.len().saturating_sub(4096)..]
            .windows(5)
            .any(|chunk| chunk == b"%%EOF")
}

pub fn label(input: &str) -> String {
    input
        .trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect()
}

impl Vault {
    pub fn allocate(&mut self) -> Id {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    pub fn book(&self, id: Id) -> Option<&Book> {
        self.books.iter().find(|book| book.id == id)
    }

    pub fn book_mut(&mut self, id: Id) -> Option<&mut Book> {
        self.books.iter_mut().find(|book| book.id == id)
    }

    pub fn new_book(&mut self, title: &str, now: i64, paper: Paper) -> Book {
        let id = self.allocate();
        let page = Page {
            id: self.allocate(),
            paper,
            ..Page::default()
        };
        Book {
            id,
            title: label(title),
            subject: String::new(),
            collection: String::new(),
            tags: vec![],
            color: Color(61, 78, 105),
            cover: Cover::Cloth,
            cover_art: None,
            folder: None,
            favorite: false,
            disposition: Disposition::Active,
            created: now,
            modified: now,
            opened: 0,
            page: 0,
            zoom: 100,
            scroll: 0.,
            pen: Pen::default(),
            presets: vec![],
            pdf: None,
            pages: vec![page],
        }
    }

    pub fn duplicate(&mut self, id: Id, now: i64) -> Option<Id> {
        let mut book = self.book(id)?.clone();
        book.id = self.allocate();
        book.title = label(&format!("{} copy", book.title));
        book.created = now;
        book.modified = now;
        book.opened = 0;
        for page in &mut book.pages {
            self.reidentify(page);
        }
        let id = book.id;
        self.books.push(book);
        Some(id)
    }

    pub fn reidentify(&mut self, page: &mut Page) {
        page.id = self.allocate();
        for stroke in &mut page.ink {
            stroke.id = self.allocate();
        }
        for text in &mut page.text {
            text.id = self.allocate();
        }
    }

    pub fn new_folder(&mut self, title: &str, parent: Option<Id>) -> Result<Id, &'static str> {
        let title = label(title);
        if title.is_empty()
            || parent.is_some_and(|parent| !self.folders.iter().any(|f| f.id == parent))
        {
            return Err("Choose a valid shelf and folder name.");
        }
        let id = self.allocate();
        self.folders.push(Folder { id, title, parent });
        Ok(id)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema != SCHEMA
            || self.next == u32::MAX
            || self.folders.len() > 4096
            || self.books.len() > 4096
        {
            return Err("Unsupported Notes format or library size.");
        }
        let look = &self.appearance;
        if look.wallpaper.as_ref().is_some_and(|image| !image.valid())
            || look.opacity > 35
            || !(45..=120).contains(&look.brightness)
            || look.blur > 14
            || look.frost > 70
        {
            return Err("Invalid library appearance.");
        }
        let mut ids = BTreeSet::new();
        let mut insert = |id| id != 0 && id < self.next && ids.insert(id);
        let folders: std::collections::BTreeMap<_, _> =
            self.folders.iter().map(|f| (f.id, f.parent)).collect();
        for folder in &self.folders {
            if !insert(folder.id) || label(&folder.title).is_empty() {
                return Err("Invalid folder record.");
            }
            let mut ancestor = folder.parent;
            for _ in 0..=folders.len() {
                let Some(id) = ancestor else {
                    break;
                };
                if id == folder.id {
                    return Err("A folder cannot contain itself.");
                }
                ancestor = *folders.get(&id).ok_or("Missing parent folder.")?;
            }
            if ancestor.is_some() {
                return Err("Folder cycle.");
            }
        }
        for book in &self.books {
            if !insert(book.id)
                || label(&book.title).is_empty()
                || book.title.len() > 480
                || book.cover_art.as_ref().is_some_and(|image| !image.valid())
                || book.folder.is_some_and(|id| !folders.contains_key(&id))
                || book.pages.is_empty()
                || book.pages.len() > 1000
                || book.page >= book.pages.len()
                || !(50..=250).contains(&book.zoom)
                || !book.scroll.is_finite()
                || book.scroll < 0.
                || !book.pen.valid()
                || book.presets.len() > 8
                || book.presets.iter().any(|pen| !pen.valid())
            {
                return Err("Invalid notebook metadata.");
            }
            if let Some(pdf) = &book.pdf {
                if !valid_asset_id(&pdf.asset)
                    || pdf.transforms.len() != book.pages.len()
                    || pdf
                        .transforms
                        .iter()
                        .any(|matrix| inverse_pdf(*matrix, Point::default()).is_none())
                {
                    return Err("Invalid PDF metadata.");
                }
            }
            for page in &book.pages {
                let paper = &page.paper;
                if !insert(page.id)
                    || !(12. ..=5000.).contains(&paper.width)
                    || !(12. ..=5000.).contains(&paper.height)
                    || !(4. ..=100.).contains(&paper.spacing)
                    || page.ink.iter().any(|s| !insert(s.id) || !s.valid())
                    || page.text.iter().any(|t| {
                        !insert(t.id)
                            || !t.at.valid()
                            || t.value.len() > 65536
                            || !(6. ..=96.).contains(&t.size)
                    })
                {
                    return Err("Invalid Notes page or ink record.");
                }
            }
        }
        self.study.validate()
    }
}

pub fn inverse_pdf(m: [f32; 6], point: Point) -> Option<Point> {
    let [a, b, c, d, e, f] = m;
    let det = a * d - b * c;
    if !det.is_finite() || det.abs() < 0.00001 || !m.iter().all(|v| v.is_finite()) {
        return None;
    }
    Some(Point {
        x: (d * (point.x - e) - c * (point.y - f)) / det,
        y: (a * (point.y - f) - b * (point.x - e)) / det,
        ..point
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplication_and_invalid_relationships() {
        let mut vault = Vault::default();
        let root = vault.new_folder("Root", None).unwrap();
        let child = vault.new_folder("Child", Some(root)).unwrap();
        let mut book = vault.new_book("Study", 17, Paper::default());
        book.folder = Some(child);
        let source = book.id;
        vault.books.push(book);
        let duplicated = vault.duplicate(source, 23).unwrap();
        assert_ne!(
            vault.book(source).unwrap().pages[0].id,
            vault.book(duplicated).unwrap().pages[0].id
        );
        assert!(vault.validate().is_ok());
        vault.folders[0].parent = Some(child);
        assert!(vault.validate().is_err());
    }

    #[test]
    fn rotated_pdf_coordinates_round_trip() {
        let source = Point {
            x: 30.,
            y: 52.,
            ..Point::default()
        };
        for m in [
            [1., 0., 0., -1., -20., 620.],
            [0., 1., 1., 0., -20., -30.],
            [-1., 0., 0., 1., 600., -20.],
            [0., -1., -1., 0., 600., 700.],
        ] {
            let [a, b, c, d, e, f] = m;
            let view = Point {
                x: a * source.x + c * source.y + e,
                y: b * source.x + d * source.y + f,
                ..source
            };
            assert_eq!(inverse_pdf(m, view), Some(source));
        }
        assert!(!valid_pdf(b"%PDF-1.4\ntruncated"));
    }
}
