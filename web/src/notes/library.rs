use super::appearance::{AppearancePanel, ImageChoice, LocalPicture};
use super::{NotesState, SaveIndicator, attach_pdf, io, now, study::StudyPanel};
use crate::settings::parse_color;
use ellipse_core::notes::{LocalImage, NotebookFile, SCHEMA};
use ellipse_core::{
    artwork::Color,
    notes::{Book, Cover, Disposition, Id, MAX_STATE, Paper, Template, Vault, label},
};
use leptos::{html, prelude::*};
use std::{collections::BTreeSet, rc::Rc};
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    All,
    Notebooks,
    Pdfs,
    Favorites,
    Recent,
    Archive,
    Trash,
}
impl Filter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Notebooks => "Notebooks",
            Self::Pdfs => "PDFs",
            Self::Favorites => "Favorites",
            Self::Recent => "Recent",
            Self::Archive => "Archive",
            Self::Trash => "Recently deleted",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layout {
    Shelf,
    Grid,
    List,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    Name,
    Newest,
    Changed,
}

#[derive(Clone, Copy)]
pub struct LibraryView {
    filter: RwSignal<Filter>,
    layout: RwSignal<Layout>,
    sort: RwSignal<Sort>,
    query: RwSignal<String>,
    folder: RwSignal<Option<Id>>,
    subject: RwSignal<String>,
    collection: RwSignal<String>,
    scroll: RwSignal<f64>,
}

impl LibraryView {
    pub fn new() -> Self {
        Self {
            filter: RwSignal::new(Filter::All),
            layout: RwSignal::new(Layout::Shelf),
            sort: RwSignal::new(Sort::Name),
            query: RwSignal::new(String::new()),
            folder: RwSignal::new(None),
            subject: RwSignal::new(String::new()),
            collection: RwSignal::new(String::new()),
            scroll: RwSignal::new(0.),
        }
    }
}

#[derive(Clone, PartialEq)]
struct Card {
    id: Id,
    title: String,
    color: Color,
    cover: Cover,
    cover_art: Option<LocalImage>,
    pages: usize,
    pdf: bool,
}

fn card(book: &Book) -> Card {
    Card {
        id: book.id,
        title: book.title.clone(),
        color: book.color,
        cover: book.cover,
        cover_art: book.cover_art.clone(),
        pages: book.pages.len(),
        pdf: book.pdf.is_some(),
    }
}

fn folders(vault: &Vault) -> Vec<(Id, String)> {
    vault
        .folders
        .iter()
        .map(|folder| {
            let mut parts = vec![folder.title.as_str()];
            let mut next = folder.parent;
            for _ in 0..vault.folders.len() {
                let Some(parent) =
                    next.and_then(|id| vault.folders.iter().find(|entry| entry.id == id))
                else {
                    break;
                };
                parts.push(&parent.title);
                next = parent.parent;
            }
            parts.reverse();
            (folder.id, parts.join(" / "))
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    New,
    Details,
    Folder,
    Study,
    Appearance,
    None,
}

fn picked_file(event: &web_sys::Event) -> Option<web_sys::File> {
    let input = event_target::<web_sys::HtmlInputElement>(event);
    let file = input.files().and_then(|files| files.get(0));
    input.set_value("");
    file
}

#[component]
pub fn Library(state: NotesState, settings: LibraryView, back: Callback<()>) -> impl IntoView {
    let panel = RwSignal::new(Panel::None);
    let notice = RwSignal::new(String::new());
    let appearance = Memo::new(move |_| {
        state
            .library_preview
            .get()
            .unwrap_or_else(|| state.vault.with(|v| v.appearance.clone()))
    });
    let loading = RwSignal::new(false);
    let shelf = NodeRef::<html::Div>::new();
    let track = StoredValue::new(None::<(i32, f64, f64)>);
    let dragged = StoredValue::new(false);
    let cards = Memo::new(move |_| {
        state.vault.with(|vault| {
            let filter = settings.filter.get();
            let search = settings.query.get().trim().to_lowercase();
            let folder = settings.folder.get();
            let subject = settings.subject.get();
            let collection = settings.collection.get();
            let mut books: Vec<_> = vault
                .books
                .iter()
                .filter(|book| {
                    let disposition = match filter {
                        Filter::Trash => Disposition::Trash,
                        Filter::Archive => Disposition::Archived,
                        _ => Disposition::Active,
                    };
                    book.disposition == disposition
                        && folder.is_none_or(|id| book.folder == Some(id))
                        && (subject.is_empty() || book.subject == subject)
                        && (collection.is_empty() || book.collection == collection)
                        && match filter {
                            Filter::Notebooks => book.pdf.is_none(),
                            Filter::Pdfs => book.pdf.is_some(),
                            Filter::Favorites => book.favorite,
                            Filter::Recent => book.opened > 0,
                            _ => true,
                        }
                        && (search.is_empty()
                            || [book.title.as_str(), &book.subject, &book.collection]
                                .iter()
                                .any(|s| s.to_lowercase().contains(&search))
                            || book
                                .tags
                                .iter()
                                .any(|tag| tag.to_lowercase().contains(&search))
                            || book.pages.iter().any(|page| {
                                page.text
                                    .iter()
                                    .any(|t| t.value.to_lowercase().contains(&search))
                            }))
                })
                .collect();
            if filter == Filter::Recent {
                books.sort_by_key(|b| std::cmp::Reverse(b.opened));
            } else {
                match settings.sort.get() {
                    Sort::Name => books.sort_by_cached_key(|b| (b.title.to_lowercase(), b.id)),
                    Sort::Newest => books.sort_by_key(|b| (std::cmp::Reverse(b.created), b.id)),
                    Sort::Changed => books.sort_by_key(|b| (std::cmp::Reverse(b.modified), b.id)),
                }
            }
            books.into_iter().map(card).collect::<Vec<_>>()
        })
    });
    let selected = Memo::new(move |_| {
        let selected = state.selected.get();
        cards.with(|cards| {
            cards
                .iter()
                .find(|card| Some(card.id) == selected)
                .or(cards.first())
                .cloned()
        })
    });
    Effect::new(move |previous: Option<bool>| {
        if let Some(shelf) = shelf.get() {
            if previous != Some(true) {
                shelf.set_scroll_left(settings.scroll.get_untracked() as i32);
            }
            true
        } else {
            false
        }
    });
    let choose = move |id: Id| {
        state.selected.set(Some(id));
        leptos::leptos_dom::helpers::request_animation_frame(move || {
            if state.selected.try_get() != Some(Some(id)) {
                return;
            }
            if let Some(track) = shelf.try_get().flatten() {
                let selector = format!("[data-note-id='{id}']");
                if let Ok(Some(node)) = track.query_selector(&selector) {
                    let options = web_sys::ScrollIntoViewOptions::new();
                    options.set_block(web_sys::ScrollLogicalPosition::Nearest);
                    options.set_inline(web_sys::ScrollLogicalPosition::Center);
                    if !window()
                        .match_media("(prefers-reduced-motion: reduce)")
                        .ok()
                        .flatten()
                        .is_some_and(|query| query.matches())
                    {
                        options.set_behavior(web_sys::ScrollBehavior::Smooth);
                    }
                    node.scroll_into_view_with_scroll_into_view_options(&options);
                }
            }
        });
    };
    let step = move |offset: isize| {
        let id = selected.get_untracked().map(|card| card.id);
        cards.with_untracked(|cards| {
            if !cards.is_empty() {
                let index = cards
                    .iter()
                    .position(|card| Some(card.id) == id)
                    .unwrap_or(0)
                    .saturating_add_signed(offset)
                    .min(cards.len() - 1);
                choose(cards[index].id);
            }
        });
    };
    let open = move || {
        if settings.filter.get_untracked() == Filter::Trash
            || settings.filter.get_untracked() == Filter::Archive
        {
            return;
        }
        if let Some(card) = selected.get_untracked() {
            state.open(card.id);
        }
    };
    let open_pdf = move |event: web_sys::Event| {
        let Some(file) = picked_file(&event) else {
            return;
        };
        if !state.local && state.pdfs.with_value(|assets| assets.len() >= 4) {
            notice.set(
                "This browser session can preview up to four PDFs. Reopen Notes to clear them."
                    .into(),
            );
            return;
        }
        loading.set(true);
        notice.set(String::new());
        spawn_local(async move {
            let result = async {
                let handle = Rc::new(io::PdfHandle(io::result(io::read_local_pdf(&file)).await?));
                let pages = io::typed(io::pdf_info(&handle.0)).await?;
                let key = if state.local {
                    io::result(io::store_pdf(&handle.0))
                        .await?
                        .as_string()
                        .ok_or("Invalid PDF asset identity.")?
                } else {
                    "0".repeat(64)
                };
                Ok::<_, String>((handle, pages, key))
            }
            .await;
            if loading.try_set(false).is_some() {
                return;
            }
            match result {
                Ok((handle, pages, key)) => {
                    let title = file.name();
                    let name = title
                        .strip_suffix(".pdf")
                        .or_else(|| title.strip_suffix(".PDF"))
                        .unwrap_or(&title);
                    let mut id = 0;
                    state.edit(|vault| {
                        let mut book = vault.new_book(
                            if label(name).is_empty() { "PDF" } else { name },
                            now(),
                            Paper::default(),
                        );
                        id = book.id;
                        book.folder = settings.folder.get_untracked();
                        book.color = Color(114, 69, 59);
                        let book = attach_pdf(vault, book, pages, key);
                        vault.books.push(book);
                    });
                    state.pdfs.update_value(|items| {
                        items.insert(id, handle);
                    });
                    settings.filter.set(Filter::All);
                    settings.query.set(String::new());
                    settings.subject.set(String::new());
                    settings.collection.set(String::new());
                    state.open(id);
                }
                Err(error) => notice.set(error),
            }
        });
    };
    let open_backup = move |event: web_sys::Event| {
        let Some(file) = picked_file(&event) else {
            return;
        };
        loading.set(true);
        notice.set(String::new());
        spawn_local(async move {
            let result = async {
                if file.size() > MAX_STATE as f64 {
                    return Err("That notebook exceeds the supported size.");
                }
                let text = JsFuture::from(file.text())
                    .await
                    .ok()
                    .and_then(|v| v.as_string())
                    .ok_or("The notebook could not be read.")?;
                let file: NotebookFile = serde_json::from_str(&text)
                    .map_err(|_| "This is not a supported Ellipse notebook backup.")?;
                if file.version != SCHEMA {
                    return Err("This notebook needs a newer version of Ellipse.");
                }
                let mut book = file.notebook;
                if book.pdf.is_some() {
                    return Err("Open the original PDF using Choose PDF.");
                }
                book.folder = None;
                book.cover_art = None;
                book.disposition = Disposition::Active;
                book.favorite = false;
                book.opened = 0;
                let mut check = Vault::default();
                let max = book
                    .pages
                    .iter()
                    .flat_map(|p| {
                        std::iter::once(p.id)
                            .chain(p.ink.iter().map(|s| s.id))
                            .chain(p.text.iter().map(|t| t.id))
                    })
                    .chain([book.id])
                    .max()
                    .unwrap_or(0);
                check.next = max.checked_add(1).ok_or("Invalid notebook identifiers.")?;
                check.books.push(book);
                check.validate()?;
                Ok(check.books.remove(0))
            }
            .await;
            if loading.try_set(false).is_some() {
                return;
            }
            match result {
                Ok(mut book) => {
                    let mut id = 0;
                    state.edit(|vault| {
                        book.id = vault.allocate();
                        id = book.id;
                        for page in &mut book.pages {
                            vault.reidentify(page);
                        }
                        vault.books.push(book);
                    });
                    settings.filter.set(Filter::All);
                    settings.folder.set(None);
                    settings.query.set(String::new());
                    settings.subject.set(String::new());
                    settings.collection.set(String::new());
                    state.selected.set(Some(id));
                }
                Err(error) => notice.set(error.into()),
            }
        });
    };
    view! {
        <Show when=move ||state.maintenance.get()>
            <p class="notes-library-status" role="status">"Working with the Notes vault…"</p>
        </Show>
        <main class="notes-library" aria-label="Notes library"
            inert=move ||state.maintenance.get()>
            <div class="notes-wallpaper" aria-hidden="true" style=move ||appearance.with(|look|format!(
                "--wall-opacity:{}%;--wall-brightness:{}%;--wall-blur:{}px;--wall-frost:{}%;",
                look.opacity,look.brightness,look.blur,look.frost
            ))>
                <LocalPicture library=state.media image=Signal::derive(move ||appearance.with(|look|look.wallpaper.clone()))
                    class="notes-wallpaper-image" report=notice/>
            </div>
            <header class="notes-nav">
                <button class="notes-brand" type="button" disabled=move || state.local && !state.ready()
                    on:click=move |_| {if state.ready(){back.run(());}}>
                    <span class="ellipse-mark" aria-hidden="true"></span><span>"Ellipse"</span><em>"Notes"</em>
                </button>
                <nav aria-label="Notes navigation">
                    <button type="button" class="notes-selected" on:click=move |_| {
                        panel.set(Panel::None); settings.filter.set(Filter::All); settings.folder.set(None);
                        settings.subject.set(String::new()); settings.collection.set(String::new());
                    }>"Library"</button>
                    <Show when=move || state.local>
                        <button type="button" on:click=move |_| panel.set(Panel::Folder)>"Folders"</button>
                        <button type="button" on:click=move |_| panel.set(Panel::Study)>"Study"</button>
                        <button type="button" on:click=move |_| panel.set(Panel::Appearance)>"Appearance"</button>
                    </Show>
                </nav>
                <button type="button" class="notes-theme" on:click=move |_| state.edit(|v|v.dark = !v.dark)>
                    {move || if state.vault.with(|v|v.dark) {"Cream appearance"} else {"Dark appearance"}}
                </button>
            </header>
            <section class="notes-library-heading">
                <h1>"My Notebooks"</h1>
                <p class="notes-meta">{move || cards.with(|items|format!("{} {}",items.len(),if items.len()==1{"item"}else{"items"}))}</p>
                <div class="notes-filters" role="group" aria-label="Library filter">
                    { [Filter::All,Filter::Notebooks,Filter::Pdfs,Filter::Favorites,Filter::Recent].into_iter()
                        .filter(|filter| state.local || !matches!(filter,Filter::Favorites|Filter::Recent)).map(|filter|view!{
                            <button type="button" class:notes-selected=move || settings.filter.get()==filter
                                aria-pressed=move || (settings.filter.get()==filter).to_string()
                                on:click=move |_|{settings.filter.set(filter);settings.scroll.set(0.);}>{filter.label()}</button>
                        }).collect::<Vec<_>>() }
                </div>
            </section>
            <div class="notes-tools" aria-label="Library controls">
                <input type="search" aria-label="Search notebooks" placeholder="Find a notebook…" prop:value=move ||settings.query.get()
                    on:input=move |e|settings.query.set(event_target_value(&e))/>
                <select aria-label="Sort library" prop:value=move || match settings.sort.get(){Sort::Name=>"name",Sort::Newest=>"new",Sort::Changed=>"changed"}
                    on:change=move |e|settings.sort.set(match event_target_value(&e).as_str(){"new"=>Sort::Newest,"changed"=>Sort::Changed,_=>Sort::Name})>
                    <option value="name">"Name"</option><option value="new">"Newest"</option><option value="changed">"Last edited"</option>
                </select>
                <select aria-label="Library layout" prop:value=move || match settings.layout.get(){Layout::Shelf=>"shelf",Layout::Grid=>"grid",Layout::List=>"list"}
                    on:change=move |e|settings.layout.set(match event_target_value(&e).as_str(){"grid"=>Layout::Grid,"list"=>Layout::List,_=>Layout::Shelf})>
                    <option value="shelf">"Shelf"</option><option value="grid">"Grid"</option><option value="list">"List"</option>
                </select>
                <Show when=move ||state.local>
                    <button type="button" class="notes-primary" on:click=move |_|panel.set(Panel::New)>"New notebook"</button>
                </Show>
                <label class="notes-file">{if state.local {"Import PDF"} else {"Choose PDF"}}
                    <input type="file" accept="application/pdf,.pdf" disabled=move ||loading.get() on:change=open_pdf/>
                </label>
                <label class="notes-file">"Open notebook backup"
                    <input type="file" accept="application/json,.json" disabled=move ||loading.get() on:change=open_backup/>
                </label>
            </div>
            <p class="notes-library-status" role="status">{move || if loading.get() {"Reading file…".into()} else {notice.get()}}</p>
            <Show when=move ||panel.get()!=Panel::None>
                <section class="notes-side" aria-label="Library panel">
                    <button type="button" class="notes-panel-close" aria-label="Close library panel" on:click=move |_|panel.set(Panel::None)>"Close"</button>
                    {move || match panel.get(){
                        Panel::New=>view!{<BookForm state settings id=None done=Callback::new(move |()|panel.set(Panel::None))/>}.into_any(),
                        Panel::Details=>view!{<BookForm state settings id=selected.get_untracked().map(|card|card.id)
                            done=Callback::new(move |()|panel.set(Panel::None))/>}.into_any(),
                        Panel::Folder=>view!{<FolderPanel state settings/>}.into_any(),
                        Panel::Study=>view!{<StudyPanel state book=None/>}.into_any(),
                        Panel::Appearance=>view!{<AppearancePanel state done=Callback::new(move |()|panel.set(Panel::None))/>}.into_any(),
                        Panel::None=>().into_any(),
                    }}
                </section>
            </Show>
            <Show when=move || cards.with(Vec::is_empty) fallback=move ||view!{
                <div class:notes-shelf=true class:notes-grid=move ||settings.layout.get()==Layout::Grid
                    class:notes-list=move ||settings.layout.get()==Layout::List
                    node_ref=shelf tabindex="0" aria-label="Notebooks"
                    on:keydown=move |event:web_sys::KeyboardEvent| if event.key()=="ArrowLeft" || event.key()=="ArrowRight" {
                        event.prevent_default();step(if event.key()=="ArrowLeft"{-1}else{1});
                    }
                    on:scroll=move |_|if let Some(track)=shelf.get_untracked() {settings.scroll.set(f64::from(track.scroll_left()));}
                    on:wheel=move |event:web_sys::WheelEvent| {
                        if event.delta_y().abs()>event.delta_x().abs() && settings.layout.get_untracked()==Layout::Shelf && !event.ctrl_key()
                            && let Some(track)=shelf.get_untracked() && track.scroll_width()>track.client_width() {
                            event.prevent_default();track.set_scroll_left((f64::from(track.scroll_left())+event.delta_y()) as i32);
                        }
                    }
                    on:pointerdown=move |event:web_sys::PointerEvent| {
                        if event.pointer_type()=="mouse" && event.button()==0 && let Some(node)=shelf.get_untracked(){
                            track.set_value(Some((event.pointer_id(),f64::from(event.client_x()),f64::from(node.scroll_left()))));dragged.set_value(false);
                        }
                    }
                    on:pointermove=move |event:web_sys::PointerEvent| {
                        if let Some((id,start,scroll))=track.get_value()
                            && let Some(node)=shelf.get_untracked() && id==event.pointer_id() && event.buttons()==1 {
                            let delta=f64::from(event.client_x())-start;
                            if delta.abs()>6. || dragged.get_value(){
                                if !dragged.get_value(){let _=node.set_pointer_capture(id);dragged.set_value(true);}
                                node.set_scroll_left((scroll-delta) as i32);
                            }
                        }
                    }
                    on:pointerup=move |_|track.set_value(None) on:pointercancel=move |_|track.set_value(None)>
                    <For each=move ||cards.get() key=|card|(card.id,card.title.clone(),card.color.hex(),card.cover as u8, card.pages,
                            card.cover_art.as_ref().map(|image|(image.asset.clone(),image.frame.x,image.frame.y,image.frame.zoom,image.frame.fit as u8)))
                        children=move |card|{
                            let id=card.id;
                            let active=move ||selected.with(|card|card.as_ref().is_some_and(|item|item.id==id));
                            let label=card.title.clone();
                            let cover_art=StoredValue::new(card.cover_art);
                            let pattern=match card.cover{Cover::Cloth=>"cloth",Cover::Bands=>"bands",Cover::Orbit=>"orbit"};
                            view!{
                                <button type="button" class:notes-book=true class:notes-book-active=active data-note-id=id
                                    data-cover=pattern aria-label=label aria-pressed=move ||active().to_string()
                                    style=format!("--book-color:{};--spine:{}px",card.color.hex(),64+(card.pages.min(220)/8))
                                    on:click=move |_|{if dragged.get_value(){dragged.set_value(false);return;}
                                        if active(){open();}else{choose(id);}
                                    }>
                                    <span class="notes-spine" aria-hidden="true"><span class="binding-mark"></span><span>{card.title.clone()}</span></span>
                                    <span class="notes-cover" aria-hidden="true">
                                        <Show when=move || !cover_art.with_value(Option::is_none) && (active() || settings.layout.get()!=Layout::Shelf)>
                                            <LocalPicture library=state.media image=Signal::derive(move ||cover_art.get_value()) class="notes-cover-photo" report=notice/>
                                        </Show>
                                        <span class="cover-imprint">"Ellipse Notes"</span>
                                        <span class="cover-title">{card.title}</span>
                                        <span class="cover-ornament"></span>
                                    </span>
                                </button>
                            }
                        }/>
                </div>
                <div class="notes-book-meta">
                    <button type="button" class="notes-edge" aria-label="Previous notebook" on:click=move |_|step(-1)>"←"</button>
                    <div>
                        <h2>{move || selected.with(|card|card.as_ref().map(|c|c.title.clone()))}</h2>
                        <p class="notes-meta">{move ||selected.with(|card|card.as_ref().map(|c|format!("{} · {} {}",if c.pdf{"PDF"}else{"Notebook"},c.pages,if c.pages==1{"page"}else{"pages"})))}</p>
                        <div class="notes-selection-tools">
                            <Show when=move || !matches!(settings.filter.get(),Filter::Archive|Filter::Trash)>
                                <button type="button" class="notes-primary" on:click=move |_|open()>"Open"</button>
                            </Show>
                            <Show when=move ||state.local>
                                <button type="button" on:click=move |_|panel.set(Panel::Details)>"Details"</button>
                            </Show>
                        </div>
                    </div>
                    <button type="button" class="notes-edge" aria-label="Next notebook" on:click=move |_|step(1)>"→"</button>
                </div>
            }>
                <div class="notes-blank-shelf">
                    <span class="notes-binding-icon" aria-hidden="true"></span>
                    <h2>{move ||if state.local && settings.filter.get()==Filter::All && settings.query.with(String::is_empty) {"Your shelf is empty"} else {"No notebooks here"}}</h2>
                    <Show when=move ||!state.local>
                        <p>"Open a PDF or notebook backup from this device. Your installed library stays on that device."</p>
                    </Show>
                </div>
            </Show>
            <footer class="notes-footer"><span>"Ellipse Notes"</span><SaveIndicator state/></footer>
        </main>
    }
}

#[component]
fn FolderPanel(state: NotesState, settings: LibraryView) -> impl IntoView {
    let title = RwSignal::new(String::new());
    view! {
        <h2>"Folders and shelves"</h2>
        <label>"Shelf"<select prop:value=move ||settings.folder.get().map(|i|i.to_string()).unwrap_or_default()
            on:change=move |e|settings.folder.set(event_target_value(&e).parse().ok())>
            <option value="">"All shelves"</option>
            {move ||state.vault.with(folders).into_iter().map(|(id,title)|view!{<option value=id>{title}</option>}).collect::<Vec<_>>()}
        </select></label>
        <form on:submit=move |e|{
            e.prevent_default();
            let title_value=title.get_untracked();
            if label(&title_value).is_empty(){return;}
            state.edit(|v|if let Ok(id)=v.new_folder(&title_value,settings.folder.get_untracked()){settings.folder.set(Some(id));});
            title.set(String::new());
        }>
            <label>"Folder name"<input required maxlength=120 prop:value=move ||title.get() on:input=move |e|title.set(event_target_value(&e))/></label>
            <button type="submit">"Add folder to selected shelf"</button>
        </form>
        <label>"Subject"<select on:change=move |e|settings.subject.set(event_target_value(&e))>
            <option value="">"All subjects"</option>
            {move ||state.vault.with(|v|v.books.iter().filter(|b|!b.subject.is_empty()).map(|b|b.subject.clone()).collect::<BTreeSet<_>>())
                .into_iter().map(|name|{let value=name.clone();view!{<option value=value>{name}</option>}}).collect::<Vec<_>>()}
        </select></label>
        <label>"Collection"<select on:change=move |e|settings.collection.set(event_target_value(&e))>
            <option value="">"All collections"</option>
            {move ||state.vault.with(|v|v.books.iter().filter(|b|!b.collection.is_empty()).map(|b|b.collection.clone()).collect::<BTreeSet<_>>())
                .into_iter().map(|name|{let value=name.clone();view!{<option value=value>{name}</option>}}).collect::<Vec<_>>()}
        </select></label>
        <div class="notes-row">
            <button type="button" on:click=move |_|settings.filter.set(Filter::Archive)>"Archive"</button>
            <button type="button" on:click=move |_|settings.filter.set(Filter::Trash)>"Recently deleted"</button>
        </div>
        <Show when=move ||state.local>
            <VaultBackup state settings/>
        </Show>
    }
}

#[component]
fn VaultBackup(state: NotesState, settings: LibraryView) -> impl IntoView {
    let status = RwSignal::new(String::new());
    view! {
        <details>
            <summary>"Vault backup"</summary>
            <p>"Save your Notes library and its PDF originals, covers and wallpaper in one file."</p>
            <button type="button" disabled=move ||!state.ready() || state.maintenance.get()
                on:click=move |_|{
                    if !state.ready() || state.maintenance.get_untracked(){return;}
                    state.maintenance.set(true);
                    let head=state.head.get_untracked();
                    spawn_local(async move {
                        let result=io::backup(head).await;
                        let _=state.maintenance.try_set(false);
                        let _=status.try_set(result.map(|path|format!("Vault backup saved to {path}")).unwrap_or_else(|error|error));
                    });
                }>"Save vault backup"</button>
            <label class="notes-file">"Restore vault backup"
                <input type="file" accept=".ellipsevault" disabled=move ||!state.ready() || state.maintenance.get()
                    on:change=move |event|{
                        let Some(file)=picked_file(&event) else{return};
                        if !state.ready() || state.maintenance.get_untracked(){return;}
                        if !window().confirm_with_message("Replace this device’s Notes vault with the selected backup? Save a backup of your current vault first if you want to keep it.").unwrap_or(false){return;}
                        state.maintenance.set(true);
                        let head=state.head.get_untracked();
                        spawn_local(async move {
                            let result=io::typed::<io::Snapshot>(io::restore_notes(head,&file)).await;
                            if state.maintenance.try_set(false).is_some(){return;}
                            match result {
                                Ok(snapshot)=>batch(||{
                                    state.head.set(snapshot.head);
                                    state.selected.set(None);
                                    state.history.update_value(|history|*history=Default::default());
                                    state.pdfs.update_value(|cache|cache.clear());
                                    state.vault.set(snapshot.vault);
                                    settings.filter.set(Filter::All);
                                    settings.folder.set(None);
                                    settings.subject.set(String::new());
                                    settings.collection.set(String::new());
                                    settings.query.set(String::new());
                                    settings.scroll.set(0.);
                                    let _=status.try_set("Notes vault restored.".into());
                                }),
                                Err(error)=>{let _=status.try_set(error);}
                            }
                        });
                    }/>
            </label>
            <p role="status">{move ||status.get()}</p>
        </details>
    }
}

#[component]
fn BookForm(
    state: NotesState,
    settings: LibraryView,
    id: Option<Id>,
    done: Callback<()>,
) -> impl IntoView {
    let original = id.and_then(|id| state.vault.with_untracked(|v| v.book(id).cloned()));
    let text = |f: fn(&Book) -> String| original.as_ref().map(f).unwrap_or_default();
    let title = RwSignal::new(text(|b| b.title.clone()));
    let subject = RwSignal::new(text(|b| b.subject.clone()));
    let collection = RwSignal::new(text(|b| b.collection.clone()));
    let tags = RwSignal::new(text(|b| b.tags.join(", ")));
    let color = RwSignal::new(
        original
            .as_ref()
            .map_or("#3d4e69".into(), |b| b.color.hex()),
    );
    let folder = RwSignal::new(
        original
            .as_ref()
            .map_or(settings.folder.get_untracked(), |book| book.folder),
    );
    let cover = RwSignal::new(original.as_ref().map_or(Cover::Cloth, |b| b.cover));
    let template = RwSignal::new(Template::default());
    let cover_art = RwSignal::new(original.as_ref().and_then(|book| book.cover_art.clone()));
    let disposition = original.as_ref().map(|b| b.disposition);
    let favorite = RwSignal::new(original.as_ref().is_some_and(|b| b.favorite));
    let error = RwSignal::new(String::new());
    view! {
        <h2>{if id.is_some() {"Notebook details"} else {"New notebook"}}</h2>
        <form class="notes-form" on:submit=move |event|{
            event.prevent_default();
            let title=label(&title.get_untracked());
            if title.is_empty(){error.set("Enter a notebook name.".into());return;}
            let make=|book:&mut Book|{
                book.title=title;
                book.subject=label(&subject.get_untracked());
                book.collection=label(&collection.get_untracked());
                book.tags=tags.get_untracked().split(',').map(label).filter(|s|!s.is_empty()).take(20).collect();
                book.color=parse_color(&color.get_untracked()).unwrap_or(Color(61,78,105));
                book.folder=folder.get_untracked();
                book.cover=cover.get_untracked();
                book.cover_art=cover_art.get_untracked();
                book.favorite=favorite.get_untracked();
                book.modified=now();
            };
            state.edit(|vault|{
                if let Some(id)=id{
                    if let Some(book)=vault.book_mut(id){make(book);}
                }else{
                    let mut book=vault.new_book("Notebook",now(),Paper{template:template.get_untracked(),..Paper::default()});
                    make(&mut book);
                    state.selected.set(Some(book.id));
                    vault.books.push(book);
                    settings.filter.set(Filter::All);
                    settings.folder.set(folder.get_untracked());
                    settings.subject.set(String::new());
                    settings.collection.set(String::new());
                    settings.query.set(String::new());
                }
            });
            done.run(());
        }>
            <label>"Name"<input required maxlength=120 prop:value=move ||title.get() on:input=move |e|title.set(event_target_value(&e))/></label>
            <label>"Subject"<input maxlength=120 prop:value=move ||subject.get() on:input=move |e|subject.set(event_target_value(&e))/></label>
            <label>"Collection"<input maxlength=120 prop:value=move ||collection.get() on:input=move |e|collection.set(event_target_value(&e))/></label>
            <label>"Tags"<input maxlength=240 placeholder="Comma separated" prop:value=move ||tags.get() on:input=move |e|tags.set(event_target_value(&e))/></label>
            <label>"Folder"<select prop:value=move ||folder.get().map(|i|i.to_string()).unwrap_or_default()
                on:change=move |e|folder.set(event_target_value(&e).parse().ok())>
                <option value="">"Library"</option>
                {move ||state.vault.with(folders).into_iter().map(|(id,title)|view!{<option value=id>{title}</option>}).collect::<Vec<_>>()}
            </select></label>
            <label>"Cover color"<input type="color" prop:value=move ||color.get() on:input=move |e|color.set(event_target_value(&e))/></label>
            <label>"Binding"<select prop:value=move ||match cover.get(){Cover::Cloth=>"cloth",Cover::Bands=>"bands",Cover::Orbit=>"orbit"}
                on:change=move |e|cover.set(match event_target_value(&e).as_str(){"bands"=>Cover::Bands,"orbit"=>Cover::Orbit,_=>Cover::Cloth})>
                <option value="cloth">"Cloth"</option><option value="bands">"Bands"</option><option value="orbit">"Orbit"</option>
            </select></label>
            <details>
                <summary>"Cover artwork"</summary>
                <ImageChoice library=state.media image=cover_art label="Choose cover image"/>
            </details>
            <Show when=move ||id.is_none()>
                <label>"Initial paper"<select prop:value=move ||template.get().label()
                    on:change=move |e|if let Some(value)=Template::ALL.into_iter().find(|t|t.label()==event_target_value(&e)){template.set(value);}>
                    {Template::ALL.into_iter().map(|item|view!{<option value=item.label()>{item.label()}</option>}).collect::<Vec<_>>()}
                </select></label>
            </Show>
            <label class="notes-checkbox"><input type="checkbox" prop:checked=move ||favorite.get() on:change=move |e|favorite.set(event_target_checked(&e))/>"Favorite"</label>
            <p role="status">{move ||error.get()}</p>
            <button type="submit" class="notes-primary">{if id.is_some(){"Apply details"}else{"Create notebook"}}</button>
        </form>
        {id.map(|id| view!{
            <div class="notes-row">
                <button type="button" on:click=move |_|{
                    state.edit(|v|{v.duplicate(id,now());});done.run(());
                }>"Duplicate"</button>
                <button type="button" on:click=move |_|{
                    state.edit(|v|if let Some(book)=v.book_mut(id){
                        book.disposition=if disposition==Some(Disposition::Active){Disposition::Archived}else{Disposition::Active};
                    });done.run(());
                }>{if disposition==Some(Disposition::Active){"Archive"}else{"Restore to library"}}</button>
                <button type="button" on:click=move |_|{
                    if disposition==Some(Disposition::Trash){
                        if !window().confirm_with_message("Permanently remove this notebook from your library?").unwrap_or(false){return;}
                        state.edit(|v|v.books.retain(|book|book.id!=id));
                        state.pdfs.update_value(|items|{items.remove(&id);});
                    }else{state.edit(|v|if let Some(book)=v.book_mut(id){book.disposition=Disposition::Trash;});}
                    done.run(());
                }>{if disposition==Some(Disposition::Trash){"Delete permanently"}else{"Move to Recently deleted"}}</button>
            </div>
        })}
    }
}
