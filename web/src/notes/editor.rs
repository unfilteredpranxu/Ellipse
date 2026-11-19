use super::{
    NotesState, SaveIndicator, SaveState,
    inkpad::{Gesture, InkPad, Tool},
    io,
    study::StudyPanel,
};
use crate::settings::parse_color;
use ellipse_core::notes::{
    Id, Nib, Page, Pen, Point, Stroke, Template, Text, clip_disc, lasso_hits, transform,
};
use ellipse_core::notes::{NOTEBOOK_SCHEMA, NotebookFile};
use leptos::{html, prelude::*};
use std::{collections::BTreeSet, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy)]
enum PageAction {
    Add,
    Duplicate,
    Earlier,
    Later,
    Delete,
    Bookmark,
}

fn erase(ink: Vec<Stroke>, path: &[Point], whole: bool, next: &mut Id) -> Vec<Stroke> {
    let Some(&first) = path.first() else {
        return ink;
    };
    let mut samples = vec![first];
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let n = (a.distance(b) / 5.).ceil().clamp(1., 100.) as u32;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            samples.push(Point {
                x: a.x + (b.x - a.x) * t,
                y: a.y + (b.y - a.y) * t,
                ..a
            });
        }
        if samples.len() > 2048 {
            break;
        }
    }
    ink.into_iter()
        .flat_map(|stroke| {
            if whole {
                if samples
                    .iter()
                    .any(|point| clip_disc(&stroke, *point, 12.).is_some())
                {
                    return vec![];
                }
                return vec![stroke];
            }
            let mut current = vec![stroke];
            for point in &samples {
                current = current
                    .into_iter()
                    .flat_map(|stroke| match clip_disc(&stroke, *point, 10.) {
                        None => vec![stroke],
                        Some(pieces) => pieces
                            .into_iter()
                            .enumerate()
                            .map(|(index, points)| {
                                let id = if index == 0 {
                                    stroke.id
                                } else {
                                    let id = *next;
                                    *next += 1;
                                    id
                                };
                                Stroke {
                                    id,
                                    pen: stroke.pen,
                                    points,
                                }
                            })
                            .collect(),
                    })
                    .collect();
            }
            current
        })
        .collect()
}

#[component]
pub fn Editor(state: NotesState, id: Id) -> impl IntoView {
    let original = state
        .vault
        .with_untracked(|v| v.book(id).cloned())
        .expect("opened book");
    let is_pdf = original.pdf.is_some();
    let title = Memo::new(move |_| {
        state
            .vault
            .with(|v| v.book(id).map_or(String::new(), |b| b.title.clone()))
    });
    let page = Memo::new(move |_| {
        state.vault.with(|v| {
            v.book(id)
                .map(|b| b.pages[b.page].clone())
                .unwrap_or_default()
        })
    });
    let page_index = Memo::new(move |_| state.vault.with(|v| v.book(id).map_or(0, |b| b.page)));
    let page_total = Memo::new(move |_| {
        state
            .vault
            .with(|v| v.book(id).map_or(1, |b| b.pages.len()))
    });
    let zoom = Memo::new(move |_| state.vault.with(|v| v.book(id).map_or(100, |b| b.zoom)));
    let pen = Memo::new(move |_| {
        state
            .vault
            .with(|v| v.book(id).map_or(Pen::default(), |b| b.pen))
    });
    let tool = RwSignal::new(if state.local { Tool::Ink } else { Tool::Pan });
    let selected = RwSignal::new(BTreeSet::<Id>::new());
    let clipboard = StoredValue::new(Vec::<Stroke>::new());
    let touch = RwSignal::new(false);
    let focus = RwSignal::new(false);
    let pages = RwSignal::new(false);
    let timer = RwSignal::new(false);
    let notice = RwSignal::new(String::new());
    let text = RwSignal::new(String::new());
    let leaving = RwSignal::new(false);
    let scroll = NodeRef::<html::Div>::new();
    let original_scroll = original.scroll;
    Effect::new(move |_| {
        if let Some(scroll) = scroll.get() {
            scroll.set_scroll_top(original_scroll as i32);
        }
    });
    let pdf = RwSignal::new_local(if state.local {
        state
            .pdfs
            .try_update_value(|items| items.remove(&id))
            .flatten()
    } else {
        state.pdfs.with_value(|items| items.get(&id).cloned())
    });
    if pdf.with_untracked(Option::is_none)
        && let Some(original) = original.pdf
    {
        spawn_local(async move {
            match io::result(io::read_stored_pdf(&original.asset)).await {
                Ok(value) => {
                    let _ = pdf.try_set(Some(Rc::new(io::PdfHandle(value))));
                }
                Err(error) => {
                    let _ = notice.try_set(error);
                }
            }
        });
    }
    let pdf_canvas = NodeRef::<html::Canvas>::new();
    let resize = RwSignal::new(0u32);
    let listener =
        leptos::leptos_dom::helpers::window_event_listener(leptos::ev::resize, move |_| {
            resize.update(|n| *n = n.wrapping_add(1))
        });
    on_cleanup(move || listener.remove());
    Effect::new(move |_| {
        zoom.track();
        resize.track();
        let index = page_index.get();
        if let (Some(pdf), Some(canvas)) = (pdf.get(), pdf_canvas.get()) {
            spawn_local(async move {
                if let Err(error) =
                    io::result(io::render_pdf(&pdf.0, index as u32 + 1, &canvas)).await
                {
                    let _ = notice.try_set(error);
                }
            });
        }
    });
    let navigate = move |index: usize| {
        selected.set(BTreeSet::new());
        state.edit(|v| {
            if let Some(book) = v.book_mut(id) {
                book.page = index.min(book.pages.len() - 1);
                book.scroll = 0.;
            }
        });
        if let Some(scroll) = scroll.get_untracked() {
            scroll.set_scroll_top(0);
        }
    };
    let leave = move || {
        if !state.local {
            state.current.set(None);
            return;
        }
        if leaving.get_untracked() && state.saving.get_untracked() != SaveState::Error {
            return;
        }
        let offset = scroll.get_untracked().map_or(0, |node| node.scroll_top());
        leaving.set(true);
        state.edit(|v| {
            if let Some(book) = v.book_mut(id) {
                book.scroll = f64::from(offset.max(0));
            }
        });
    };
    Effect::new(move |_| {
        if leaving.get() && state.saving.get() == SaveState::Clean {
            state.current.set(None);
        }
    });
    let page_action = move |action: PageAction| {
        if !state.local || is_pdf && !matches!(action, PageAction::Bookmark) {
            return;
        }
        let index = page_index.get_untracked();
        if matches!(action, PageAction::Delete)
            && (page_total.get_untracked() <= 1
                || !window()
                    .confirm_with_message(
                        "Delete this page? Undo can restore it until the notebook is closed.",
                    )
                    .unwrap_or(false))
        {
            return;
        }
        state.record(id, |v| {
            let mut new_page = match action {
                PageAction::Add => v.book(id).map(|book| Page {
                    paper: book.pages[index].paper.clone(),
                    ..Page::default()
                }),
                PageAction::Duplicate => v.book(id).map(|book| book.pages[index].clone()),
                _ => None,
            };
            if let Some(page) = &mut new_page {
                v.reidentify(page);
            }
            let Some(book) = v.book_mut(id) else { return };
            match action {
                PageAction::Add | PageAction::Duplicate => {
                    if book.pages.len() < 1000
                        && let Some(page) = new_page
                    {
                        book.pages.insert(index + 1, page);
                        book.page = index + 1;
                    }
                }
                PageAction::Earlier if index > 0 => {
                    book.pages.swap(index, index - 1);
                    book.page -= 1;
                }
                PageAction::Later if index + 1 < book.pages.len() => {
                    book.pages.swap(index, index + 1);
                    book.page += 1;
                }
                PageAction::Delete => {
                    book.pages.remove(index);
                    book.page = index.min(book.pages.len() - 1);
                }
                PageAction::Bookmark => {
                    book.pages[index].bookmarked = !book.pages[index].bookmarked;
                }
                _ => {}
            }
        });
        selected.set(BTreeSet::new());
    };
    let history = move |undo: bool| {
        if !state.local {
            return;
        }
        let changed = state
            .history
            .with_value(|h| if undo { h.can_undo() } else { h.can_redo() });
        if !changed {
            return;
        }
        state.edit(|v| {
            if let Some(book) = v.book_mut(id) {
                state.history.update_value(|h| {
                    if undo {
                        h.undo(book);
                    } else {
                        h.redo(book);
                    }
                });
            }
        });
        selected.set(BTreeSet::new());
    };
    let commit = Callback::new(move |mut gesture: Gesture| {
        let index = page_index.get_untracked();
        if matches!(gesture.tool, Tool::Lasso | Tool::SelectBox) {
            selected.set(page.with_untracked(|page| {
                page.ink
                    .iter()
                    .filter(|stroke| lasso_hits(&gesture.stroke.points, stroke))
                    .map(|s| s.id)
                    .collect()
            }));
            return;
        }
        if gesture.tool == Tool::Text && text.with_untracked(|text| text.trim().is_empty()) {
            notice.set("Enter text in the tool tray, then choose its position on the page.".into());
            return;
        }
        state.record(id, |v| match gesture.tool {
            Tool::Ink | Tool::Line | Tool::Ellipse | Tool::Rectangle | Tool::Arrow => {
                gesture.stroke.id = v.allocate();
                if let Some(book) = v.book_mut(id) {
                    book.pages[index].ink.push(gesture.stroke);
                }
            }
            Tool::Text => {
                let object = Text {
                    id: v.allocate(),
                    at: gesture.stroke.points[0],
                    value: text.get_untracked(),
                    color: pen.get_untracked().color,
                    size: 17.,
                    bold: false,
                    italic: false,
                };
                if let Some(book) = v.book_mut(id) {
                    book.pages[index].text.push(object);
                }
                text.set(String::new());
            }
            Tool::Whole | Tool::Partial => {
                let mut next = v.next;
                if let Some(book) = v.book_mut(id) {
                    let ink = std::mem::take(&mut book.pages[index].ink);
                    book.pages[index].ink = erase(
                        ink,
                        &gesture.stroke.points,
                        gesture.tool == Tool::Whole,
                        &mut next,
                    );
                }
                v.next = next;
            }
            _ => {}
        });
    });
    let adjust = move |dx: f32, dy: f32, scale: f32, angle: f32| {
        let ids = selected.get_untracked();
        if ids.is_empty() {
            return;
        }
        let mut n = 0;
        let mut center = Point::default();
        page.with_untracked(|p| {
            for point in p
                .ink
                .iter()
                .filter(|s| ids.contains(&s.id))
                .flat_map(|s| &s.points)
            {
                center.x += point.x;
                center.y += point.y;
                n += 1;
            }
        });
        if n == 0 {
            return;
        }
        center.x /= n as f32;
        center.y /= n as f32;
        state.record(id, |v| {
            if let Some(book) = v.book_mut(id) {
                for stroke in book.pages[book.page]
                    .ink
                    .iter_mut()
                    .filter(|s| ids.contains(&s.id))
                {
                    transform(&mut stroke.points, center, dx, dy, scale, angle);
                    stroke.pen.width = (stroke.pen.width * scale).clamp(0.2, 72.);
                }
            }
        });
    };
    let copy = move || {
        let ids = selected.get_untracked();
        clipboard.set_value(page.with_untracked(|p| {
            p.ink
                .iter()
                .filter(|s| ids.contains(&s.id))
                .cloned()
                .collect()
        }));
    };
    let paste = move || {
        let mut ink = clipboard.get_value();
        if ink.is_empty() {
            return;
        }
        state.record(id, |v| {
            for stroke in &mut ink {
                stroke.id = v.allocate();
                transform(&mut stroke.points, Point::default(), 14., 14., 1., 0.);
            }
            selected.set(ink.iter().map(|s| s.id).collect());
            if let Some(book) = v.book_mut(id) {
                book.pages[book.page].ink.extend(ink);
            }
        });
    };
    let remove_selection = move || {
        let ids = selected.get_untracked();
        if ids.is_empty() {
            return;
        }
        state.record(id, |v| {
            if let Some(book) = v.book_mut(id) {
                book.pages[book.page].ink.retain(|s| !ids.contains(&s.id));
            }
        });
        selected.set(BTreeSet::new());
    };
    let find = RwSignal::new(String::new());
    let results = RwSignal::new(Vec::<io::Hit>::new());
    let searching = RwSignal::new(false);
    let search = move |event: web_sys::SubmitEvent| {
        event.prevent_default();
        let query = find.get_untracked();
        results.set(vec![]);
        notice.set(String::new());
        if query.trim().is_empty() {
            return;
        }
        if let Some(pdf) = pdf.get_untracked() {
            searching.set(true);
            spawn_local(async move {
                let outcome = io::typed::<io::SearchResult>(io::find_pdf(&pdf.0, &query)).await;
                if searching.try_set(false).is_some() {
                    return;
                }
                match outcome {
                    Ok(found) if !found.has_text => {
                        notice.set("This PDF has no searchable text layer.".into())
                    }
                    Ok(found) if found.hits.is_empty() => {
                        notice.set("No matching PDF text.".into())
                    }
                    Ok(found) => results.set(found.hits),
                    Err(error) => notice.set(error),
                }
            });
        } else if !is_pdf {
            let query = query.to_lowercase();
            results.set(state.vault.with_untracked(|v| {
                v.book(id)
                    .map(|book| {
                        book.pages
                            .iter()
                            .enumerate()
                            .filter_map(|(page, data)| {
                                data.text
                                    .iter()
                                    .find(|t| t.value.to_lowercase().contains(&query))
                                    .map(|t| io::Hit {
                                        page: page + 1,
                                        text: t.value.chars().take(120).collect(),
                                        runs: vec![],
                                    })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            }));
            if results.with_untracked(Vec::is_empty) {
                notice.set("No matching notebook text.".into());
            }
        }
    };
    let mark = move |index: usize, annotation: u8| {
        let Some(hit) = results.with_untracked(|hits| hits.get(index).cloned()) else {
            return;
        };
        if hit.runs.is_empty() || !state.local {
            return;
        }
        state.record(id, |vault| {
            let mut strokes = Vec::new();
            for run in hit.runs {
                let (points, mut pen) = match annotation {
                    0 => (run.highlight, Pen::preset(Nib::Highlighter)),
                    1 => (run.underline, Pen::default()),
                    _ => (run.strike, Pen::default()),
                };
                pen.width = if annotation == 0 { run.width } else { 1.4 };
                strokes.push(Stroke {
                    id: vault.allocate(),
                    pen,
                    points: points
                        .into_iter()
                        .map(|[x, y]| Point {
                            x,
                            y,
                            pressure: 0.5,
                            ..Point::default()
                        })
                        .collect(),
                });
            }
            if let Some(book) = vault.book_mut(id)
                && let Some(page) = book.pages.get_mut(hit.page - 1)
            {
                page.ink.extend(strokes);
                book.page = hit.page - 1;
            }
        });
        if let Some(scroll) = scroll.get_untracked() {
            scroll.set_scroll_top(0);
        }
    };
    let export = move |_| {
        let Some(book) = state.vault.with_untracked(|v| v.book(id).cloned()) else {
            return;
        };
        let pdf = pdf.get_untracked();
        if is_pdf && pdf.is_none() {
            return;
        }
        notice.set("Preparing PDF…".into());
        spawn_local(async move {
            let status = io::export(&book, pdf.as_deref()).await;
            let _ = notice.try_set(
                status
                    .err()
                    .unwrap_or_else(|| "PDF generated. Check your downloads.".into()),
            );
        });
    };
    view! {
        <main class="notes-editor" aria-label="Document editor"
            on:keydown=move |e:web_sys::KeyboardEvent| {
                if state.local && (e.ctrl_key()||e.meta_key()) && e.key().eq_ignore_ascii_case("z")
                    && !e.target().and_then(|t|wasm_bindgen::JsCast::dyn_into::<web_sys::Element>(t).ok()).is_some_and(|t|matches!(t.tag_name().as_str(),"INPUT"|"TEXTAREA")){
                    e.prevent_default();history(!e.shift_key());
                }
            }>
            <header class="notes-editor-bar">
                <button type="button" on:click=move |_|leave()>"← Library"</button>
                <h1>{move ||title.get()}</h1>
                <SaveIndicator state/>
                <button type="button" aria-pressed=move ||pages.get().to_string() on:click=move |_|pages.update(|v|*v = !*v)>"Pages"</button>
                <button type="button" aria-pressed=move ||focus.get().to_string() on:click=move |_|focus.update(|v|*v = !*v)>
                    {move ||if focus.get(){"Show tools"}else{"Focus"}}
                </button>
                <Show when=move ||state.local>
                    <button type="button" aria-pressed=move ||timer.get().to_string() on:click=move |_|timer.update(|v|*v = !*v)>"Timer"</button>
                </Show>
            </header>
            <Show when=move || !focus.get()>
                <div class="notes-page-bar">
                    <button type="button" aria-label="Previous page" disabled=move ||page_index.get()==0
                        on:click=move |_|navigate(page_index.get_untracked().saturating_sub(1))>"←"</button>
                    <span aria-label="Current page">{move ||format!("{} / {}",page_index.get()+1,page_total.get())}</span>
                    <button type="button" aria-label="Next page" disabled=move || { page_index.get() + 1 >= page_total.get() }
                        on:click=move |_|navigate(page_index.get_untracked()+1)>"→"</button>
                    <label>"Zoom "<select aria-label="Zoom" prop:value=move ||zoom.get().to_string() on:change=move |e|{
                        if let Ok(zoom)=event_target_value(&e).parse::<u16>(){state.edit(|v|if let Some(book)=v.book_mut(id){book.zoom=zoom.clamp(50,250);});}
                    }>
                        { [50,75,100,125,150,200,250].into_iter().map(|value|view!{<option value=value>{format!("{value}%")}</option>}).collect::<Vec<_>>() }
                    </select></label>
                    <Show when=move ||state.local>
                        <button type="button" on:click=move |_|history(true)>"Undo"</button>
                        <button type="button" on:click=move |_|history(false)>"Redo"</button>
                        <button type="button" on:click=move |_|page_action(PageAction::Bookmark)>
                            {move ||if page.with(|p|p.bookmarked){"Remove bookmark"}else{"Bookmark"}}
                        </button>
                        <button type="button" on:click=export>"Export PDF"</button>
                        <Show when=move || !is_pdf>
                            <button type="button" on:click=move |_|{
                                if let Some(mut book)=state.vault.with_untracked(|v|v.book(id).cloned()){
                                    book.cover_art=None;
                                    let name=format!("{}.ellipse.json",book.title);
                                    if let Ok(json)=serde_json::to_string_pretty(&NotebookFile{version:NOTEBOOK_SCHEMA,notebook:book}){
                                        io::download_notebook(&json,&name);
                                    }
                                }
                            }>"Download notebook backup"</button>
                        </Show>
                    </Show>
                    <Show when=move ||is_pdf>
                        <button type="button" disabled=move ||pdf.with(Option::is_none)
                            on:click=move |_|if let Some(pdf)=pdf.get_untracked(){io::download_original(&pdf.0,&format!("{}.pdf",title.get_untracked()));}>
                            "Download original"
                        </button>
                    </Show>
                </div>
            </Show>
            <p class="notes-editor-notice" role="status">{move ||notice.get()}</p>
            <Show when=move ||timer.get()>
                <aside class="notes-float" aria-label="Study timer"><StudyPanel state book=Some(id)/></aside>
            </Show>
            <Show when=move ||pages.get()>
                <aside class="notes-sidebar" aria-label="Page navigator">
                    <h2>"Pages"</h2>
                    <form on:submit=search>
                        <input type="search" aria-label="Find in document" prop:value=move ||find.get() on:input=move |e|find.set(event_target_value(&e))/>
                        <button type="submit" disabled=move ||searching.get()>"Find"</button>
                    </form>
                    {move ||results.get().into_iter().enumerate().map(|(index,hit)|{
                        let page=hit.page;
                        let markable=state.local && !hit.runs.is_empty();
                        view!{
                            <button type="button" class="notes-found" on:click=move |_|navigate(page-1)>{format!("{} · {}",page,hit.text)}</button>
                            {markable.then(||view!{
                                <div class="notes-row">
                                    <button type="button" on:click=move |_|mark(index,0)>"Highlight matching run"</button>
                                    <button type="button" on:click=move |_|mark(index,1)>"Underline"</button>
                                    <button type="button" on:click=move |_|mark(index,2)>"Strike through"</button>
                                </div>
                            })}
                        }
                    }).collect::<Vec<_>>()}
                    <Show when=move ||state.local && !is_pdf>
                        <div class="notes-row">
                            <button type="button" on:click=move |_|page_action(PageAction::Add)>"Insert after"</button>
                            <button type="button" on:click=move |_|page_action(PageAction::Duplicate)>"Duplicate page"</button>
                            <button type="button" on:click=move |_|page_action(PageAction::Earlier)>"Move earlier"</button>
                            <button type="button" on:click=move |_|page_action(PageAction::Later)>"Move later"</button>
                            <button type="button" on:click=move |_|page_action(PageAction::Delete)>"Delete page"</button>
                        </div>
                        <PaperControls state id index=page_index page/>
                    </Show>
                    {if is_pdf {
                        view!{<super::pdf_navigator::PdfNavigator pdf page_index page_total navigate=Callback::new(navigate)/>}.into_any()
                    } else {
                        view!{
                    <div class="notes-page-list">
                        {move ||state.vault.with(|v|v.book(id).map(|b|b.pages.iter().enumerate().map(|(i,page)|{
                            view!{<button type="button" aria-current=move ||if page_index.get()==i{"page"}else{"false"}
                                on:click=move |_|navigate(i)>{format!("{}{}",i+1,if page.bookmarked{" · Bookmark"}else{""})}</button>}
                        }).collect::<Vec<_>>()))}
                    </div>
                        }.into_any()
                    }}
                </aside>
            </Show>
            <div class="notes-workarea" node_ref=scroll>
                <div class="notes-paper" style=move ||page.with(|page|format!(
                    "--paper-width:{}px;--paper-ratio:{};",f64::from(page.paper.width)*f64::from(zoom.get())/100.,
                    f64::from(page.paper.width)/f64::from(page.paper.height)
                ))>
                    {is_pdf.then(||view!{<canvas class="notes-pdf" node_ref=pdf_canvas aria-label="PDF page"></canvas>})}
                    <InkPad page pen tool touch selected is_pdf editable=state.local commit zoom/>
                </div>
            </div>
            <Show when=move ||state.local && !focus.get()>
                <div class="notes-tray" aria-label="Writing tools">
                    <select aria-label="Tool" prop:value=move ||tool.get().label() on:change=move |event|{
                        if let Some(value)=[Tool::Ink,Tool::Whole,Tool::Partial,Tool::Lasso,Tool::SelectBox,Tool::Text,Tool::Line,Tool::Rectangle,Tool::Ellipse,Tool::Arrow,Tool::Pan]
                            .into_iter().find(|tool|tool.label()==event_target_value(&event)){tool.set(value);}
                    }>
                        {[Tool::Ink,Tool::Whole,Tool::Partial,Tool::Lasso,Tool::SelectBox,Tool::Text,Tool::Line,Tool::Rectangle,Tool::Ellipse,Tool::Arrow,Tool::Pan]
                            .into_iter().map(|tool|view!{<option value=tool.label()>{tool.label()}</option>}).collect::<Vec<_>>()}
                    </select>
                    <PenControls state id pen tool touch/>
                    <Show when=move ||tool.get()==Tool::Text>
                        <textarea aria-label="Text to place" placeholder="Type, then tap the page" rows=2 maxlength=4000
                            prop:value=move ||text.get() on:input=move |e|text.set(event_target_value(&e))></textarea>
                        <TextControls state id page/>
                    </Show>
                    <Show when=move ||matches!(tool.get(),Tool::Lasso|Tool::SelectBox)>
                        <details>
                            <summary>{move ||format!("{} selected",selected.with(BTreeSet::len))}</summary>
                            <div class="notes-row">
                                <button type="button" on:click=move |_|copy()>"Copy ink"</button>
                                <button type="button" on:click=move |_|{copy();remove_selection();}>"Cut ink"</button>
                                <button type="button" on:click=move |_|paste()>"Paste ink"</button>
                                <button type="button" on:click=move |_|{copy();paste();}>"Duplicate ink"</button>
                                <button type="button" on:click=move |_|remove_selection()>"Delete ink"</button>
                                <button type="button" on:click=move |_|adjust(-12.,0.,1.,0.)>"Move left"</button>
                                <button type="button" on:click=move |_|adjust(12.,0.,1.,0.)>"Move right"</button>
                                <button type="button" on:click=move |_|adjust(0.,-12.,1.,0.)>"Move up"</button>
                                <button type="button" on:click=move |_|adjust(0.,12.,1.,0.)>"Move down"</button>
                                <button type="button" on:click=move |_|adjust(0.,0.,1.15,0.)>"Scale up"</button>
                                <button type="button" on:click=move |_|adjust(0.,0.,0.87,0.)>"Scale down"</button>
                                <button type="button" on:click=move |_|adjust(0.,0.,1.,std::f32::consts::FRAC_PI_6)>"Rotate"</button>
                            </div>
                        </details>
                    </Show>
                </div>
            </Show>
        </main>
    }
}

#[component]
fn PenControls(
    state: NotesState,
    id: Id,
    pen: Memo<Pen>,
    tool: RwSignal<Tool>,
    touch: RwSignal<bool>,
) -> impl IntoView {
    let change = move |update: fn(&mut Pen, f32), value: f32| {
        state.edit(|v| {
            if let Some(book) = v.book_mut(id) {
                update(&mut book.pen, value);
            }
        })
    };
    view! {
        <select aria-label="Pen nib" prop:value=move ||pen.get().nib.label() on:change=move |event|{
            if let Some(nib)=Nib::ALL.into_iter().find(|nib|nib.label()==event_target_value(&event)){
                state.edit(|v|if let Some(book)=v.book_mut(id){book.pen=Pen::preset(nib);});tool.set(Tool::Ink);
            }
        }>
            {Nib::ALL.into_iter().map(|nib|view!{<option value=nib.label()>{nib.label()}</option>}).collect::<Vec<_>>()}
        </select>
        <input aria-label="Ink color" type="color" prop:value=move ||pen.get().color.hex() on:change=move |e|
            if let Some(color)=parse_color(&event_target_value(&e)){state.edit(|v|if let Some(book)=v.book_mut(id){book.pen.color=color;});}/>
        <details class="notes-pen-options">
            <summary>"Pen settings"</summary>
            <div class="notes-pen-fields">
                <label>"Width"<input min="0.2" max="72" step="0.2" type="range" prop:value=move ||pen.get().width.to_string()
                    on:change=move |e|{if let Ok(value)=event_target_value(&e).parse::<f32>(){change(|p,v|p.width=v.clamp(0.2,72.),value);}}/></label>
                <label>"Opacity"<input min="0.05" max="1" step="0.05" type="range" prop:value=move ||pen.get().opacity.to_string()
                    on:change=move |e|{if let Ok(value)=event_target_value(&e).parse::<f32>(){change(|p,v|p.opacity=v.clamp(0.05,1.),value);}}/></label>
                <label>"Smoothing"<input min="0" max="0.85" step="0.05" type="range" prop:value=move ||pen.get().smoothing.to_string()
                    on:change=move |e|{if let Ok(value)=event_target_value(&e).parse::<f32>(){change(|p,v|p.smoothing=v.clamp(0.,0.85),value);}}/></label>
                <label class="notes-checkbox"><input type="checkbox" prop:checked=move ||touch.get() on:change=move |e|touch.set(event_target_checked(&e))/>"Finger writing"</label>
                <button type="button" on:click=move |_|state.edit(|v|if let Some(book)=v.book_mut(id) && book.presets.len()<8{book.presets.push(book.pen);})
                    disabled=move ||state.vault.with(|v|v.book(id).is_some_and(|book|book.presets.len()>=8))>"Save pen preset"</button>
                {move ||state.vault.with(|v|v.book(id).map(|book|book.presets.iter().enumerate().map(|(i,&preset)|view!{
                    <div class="notes-row">
                        <button type="button" on:click=move |_|{state.edit(|v|if let Some(book)=v.book_mut(id){book.pen=preset;});tool.set(Tool::Ink);}>
                            {format!("{} · {} · {}",i+1,preset.nib.label(),preset.color.hex())}
                        </button>
                        <button type="button" aria-label=format!("Delete pen preset {}",i+1) on:click=move |_|state.edit(|v|if let Some(book)=v.book_mut(id) && i<book.presets.len(){book.presets.remove(i);})>"Delete"</button>
                    </div>
                }).collect::<Vec<_>>()))}
            </div>
        </details>
    }
}

#[component]
fn PaperControls(state: NotesState, id: Id, index: Memo<usize>, page: Memo<Page>) -> impl IntoView {
    let change = move |f: Box<dyn FnOnce(&mut ellipse_core::notes::Paper)>| {
        state.record(id, |v| {
            if let Some(book) = v.book_mut(id) {
                f(&mut book.pages[index.get_untracked()].paper);
            }
        });
    };
    view! {
        <details class="notes-paper-options">
            <summary>"Paper"</summary>
            <label>"Template"<select prop:value=move ||page.with(|p|p.paper.template.label()) on:change=move |e|
                if let Some(template)=Template::ALL.into_iter().find(|t|t.label()==event_target_value(&e)){change(Box::new(move |p|p.template=template));}>
                {Template::ALL.into_iter().map(|t|view!{<option value=t.label()>{t.label()}</option>}).collect::<Vec<_>>()}
            </select></label>
            <label>"Paper color"<input type="color" prop:value=move ||page.with(|p|p.paper.background.hex()) on:change=move |e|
                if let Some(color)=parse_color(&event_target_value(&e)){change(Box::new(move |p|p.background=color));}/></label>
            <label>"Pattern color"<input type="color" prop:value=move ||page.with(|p|p.paper.pattern.hex()) on:change=move |e|
                if let Some(color)=parse_color(&event_target_value(&e)){change(Box::new(move |p|p.pattern=color));}/></label>
            <label>"Spacing"<input type="range" min=4 max=100 prop:value=move ||page.with(|p|p.paper.spacing.to_string()) on:change=move |e|{if let Ok(value)=event_target_value(&e).parse::<f32>(){change(Box::new(move |p|p.spacing=value.clamp(4.,100.)));}}/></label>
            <button type="button" on:click=move |_|change(Box::new(|p|std::mem::swap(&mut p.width,&mut p.height)))>"Rotate paper"</button>
            <button type="button" on:click=move |_|{
                let paper=page.with_untracked(|p|p.paper.clone());
                state.record(id,|v|if let Some(book)=v.book_mut(id){for page in &mut book.pages{page.paper=paper.clone();}});
            }>"Apply paper to all pages"</button>
        </details>
    }
}

#[component]
fn TextControls(state: NotesState, id: Id, page: Memo<Page>) -> impl IntoView {
    let update = move |text_id: Id, change: Box<dyn FnOnce(&mut Text)>| {
        state.record(id, |vault| {
            if let Some(book) = vault.book_mut(id)
                && let Some(text) = book.pages[book.page]
                    .text
                    .iter_mut()
                    .find(|text| text.id == text_id)
            {
                change(text);
            }
        });
    };
    view! {
        <details>
            <summary>"Edit text boxes"</summary>
            <div class="notes-text-fields">
                {move ||page.with(|page|page.text.iter().cloned().enumerate().map(|(index,text)|{
                    let text_id=text.id;
                    let value_label=format!("Text box {}",index+1);
                    view!{
                        <section class="notes-text-fields">
                            <label>{value_label}<textarea maxlength=4000 prop:value=text.value on:change=move |e|{
                                let value=event_target_value(&e);
                                update(text_id,Box::new(move |text|text.value=value));
                            }></textarea></label>
                            <label>"Size"<input type="number" min=6 max=96 value=text.size.to_string() on:change=move |e|{
                                if let Ok(size)=event_target_value(&e).parse::<f32>() && size.is_finite(){update(text_id,Box::new(move |text|text.size=size.clamp(6.,96.)));}
                            }/></label>
                            <label class="notes-checkbox"><input type="checkbox" prop:checked=text.bold on:change=move |e|{
                                let bold=event_target_checked(&e);update(text_id,Box::new(move |text|text.bold=bold));
                            }/>"Bold"</label>
                            <label class="notes-checkbox"><input type="checkbox" prop:checked=text.italic on:change=move |e|{
                                let italic=event_target_checked(&e);update(text_id,Box::new(move |text|text.italic=italic));
                            }/>"Italic"</label>
                            <div class="notes-row">
                                { [("Move text left",-12.,0.),("Move text right",12.,0.),("Move text up",0.,-12.),("Move text down",0.,12.)].into_iter()
                                    .map(|(name,dx,dy)|view!{
                                        <button type="button" on:click=move |_|update(text_id,Box::new(move |text|{text.at.x+=dx;text.at.y+=dy;}))>{name}</button>
                                    }).collect::<Vec<_>>() }
                                <button type="button" on:click=move |_|state.record(id,|v|if let Some(book)=v.book_mut(id){
                                    book.pages[book.page].text.retain(|text|text.id!=text_id);
                                })>"Remove text"</button>
                            </div>
                        </section>
                    }
                }).collect::<Vec<_>>())}
            </div>
        </details>
    }
}
