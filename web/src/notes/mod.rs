mod appearance;
mod editor;
mod inkpad;
mod io;
mod library;
mod study;

use ellipse_core::notes::{Book, History, Id, Vault};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use std::{collections::BTreeMap, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SaveState {
    Clean,
    Saving,
    Error,
}

#[derive(Clone, Copy)]
struct NotesState {
    vault: RwSignal<Vault>,
    local: bool,
    media: crate::media::Library,
    library_preview: RwSignal<Option<ellipse_core::notes::LibraryAppearance>>,
    head: RwSignal<u32>,
    epoch: RwSignal<u32>,
    saving: RwSignal<SaveState>,
    maintenance: RwSignal<bool>,
    recovery_required: RwSignal<bool>,
    error: RwSignal<String>,
    current: RwSignal<Option<Id>>,
    selected: RwSignal<Option<Id>>,
    history: StoredValue<History>,
    pdfs: StoredValue<SendWrapper<BTreeMap<Id, Rc<io::PdfHandle>>>>,
}

fn now() -> i64 {
    js_sys::Date::now() as i64
}

impl NotesState {
    fn new(local: bool) -> Self {
        Self {
            vault: RwSignal::new(Vault::default()),
            local,
            media: crate::media::Library::new("ellipse-notes-device-v1"),
            library_preview: RwSignal::new(None),
            head: RwSignal::new(0),
            epoch: RwSignal::new(0),
            saving: RwSignal::new(SaveState::Clean),
            maintenance: RwSignal::new(false),
            recovery_required: RwSignal::new(false),
            error: RwSignal::new(String::new()),
            current: RwSignal::new(None),
            selected: RwSignal::new(None),
            history: StoredValue::new(History::default()),
            pdfs: StoredValue::new(SendWrapper::new(BTreeMap::new())),
        }
    }

    fn edit(self, change: impl FnOnce(&mut Vault)) {
        self.vault.update(change);
        if self.local {
            self.epoch.update(|i| *i = i.wrapping_add(1));
            self.flush();
        }
    }

    fn record(self, id: Id, edit: impl FnOnce(&mut Vault)) {
        if let Some(book) = self.vault.with_untracked(|v| v.book(id).cloned()) {
            self.history.update_value(|history| history.record(book));
            self.edit(|vault| {
                edit(vault);
                if let Some(book) = vault.book_mut(id) {
                    book.modified = now();
                }
            });
        }
    }

    fn flush(self) {
        if self.saving.get_untracked() == SaveState::Saving {
            return;
        }
        self.saving.set(SaveState::Saving);
        spawn_local(async move {
            loop {
                let Some(vault) = self.vault.try_get() else {
                    return;
                };
                let epoch = self.epoch.get_untracked();
                let result = io::save(self.head.get_untracked(), &vault).await;
                if self.saving.try_get().is_none() {
                    return;
                }
                match result {
                    Ok(head) => {
                        self.head.set(head);
                        if epoch == self.epoch.get_untracked() {
                            batch(|| {
                                self.saving.set(SaveState::Clean);
                                self.error.set(String::new());
                                self.recovery_required.set(false);
                            });
                            return;
                        }
                    }
                    Err(error) => {
                        batch(|| {
                            self.error.set(error);
                            self.saving.set(SaveState::Error);
                        });
                        return;
                    }
                }
            }
        });
    }

    fn open(self, id: Id) {
        self.selected.set(Some(id));
        self.history.set_value(History::default());
        if self.local {
            self.edit(|v| {
                if let Some(book) = v.book_mut(id) {
                    book.opened = now();
                }
            });
        }
        self.current.set(Some(id));
    }

    fn ready(self) -> bool {
        self.saving.get() == SaveState::Clean
    }
}

#[component]
pub fn Notes(local: bool, back: Callback<()>) -> impl IntoView {
    let state = NotesState::new(local);
    let loaded = RwSignal::new(!local);
    let bootstrap_error = RwSignal::new(String::new());
    let load = move || {
        spawn_local(async move {
            match io::load().await {
                Ok(snapshot) => {
                    if loaded.try_get_untracked().is_none() {
                        return;
                    }
                    batch(|| {
                        state.head.set(snapshot.head);
                        state.vault.set(snapshot.vault);
                        state.recovery_required.set(snapshot.recovered);
                        loaded.set(true);
                    });
                }
                Err(error) => {
                    let _ = bootstrap_error.try_set(error);
                }
            }
        });
    };
    if local {
        load();
    }
    let settings = library::LibraryView::new();
    view! {
        <div class:notes=true class:notes-dark=move || state.vault.with(|v| v.dark)>
            <Show when=move || loaded.get() fallback=move || view! {
                <main class="notes-empty">
                    <h1>"Ellipse Notes"</h1>
                    <p role="status">{move || if bootstrap_error.with(String::is_empty) {"Opening local Notes…".into()} else {bootstrap_error.get()}}</p>
                    <button type="button" on:click=move |_| back.run(())>"Back to Ellipse"</button>
                </main>
            }>
                <Show when=move || state.recovery_required.get() fallback=move || view! {
                    {move || match state.current.get() {
                        Some(id) => view! { <editor::Editor state id/> }.into_any(),
                        None => view! { <library::Library state settings back/> }.into_any(),
                    }}
                }>
                    <main class="notes-empty">
                        <h1>"Recover Notes"</h1>
                        <p>"Notes found a recoverable checkpoint. Save it before editing documents."</p>
                        <Show when=move || state.saving.get()==SaveState::Error>
                            <p role="alert">{move || state.error.get()}</p>
                        </Show>
                        <button type="button" disabled=move || state.saving.get()==SaveState::Saving
                            on:click=move |_| state.flush()>
                            {move || if state.saving.get()==SaveState::Saving {"Saving recovered checkpoint…"} else {"Use recovered checkpoint"}}
                        </button>
                        <button type="button" disabled=move || state.saving.get()==SaveState::Saving
                            on:click=move |_| back.run(())>"Back to Ellipse"</button>
                    </main>
                </Show>
            </Show>
        </div>
    }
}

#[component]
fn SaveIndicator(state: NotesState) -> impl IntoView {
    view! {
        <span class="notes-save" role="status">{move || {
            if !state.local {return "Read only".into();}
            match state.saving.get() {
                SaveState::Clean=>"Saved on this device".into(),
                SaveState::Saving=>"Saving…".into(),
                SaveState::Error=>state.error.get(),
            }
        }}</span>
        <Show when=move || state.saving.get()==SaveState::Error>
            <button type="button" on:click=move |_| state.flush()>"Retry save"</button>
        </Show>
    }
}

fn attach_pdf(vault: &mut Vault, mut book: Book, pages: Vec<io::PdfPage>, asset: String) -> Book {
    use ellipse_core::notes::{Page, Paper, Pdf, Template};
    let transforms = pages.iter().map(|page| page.matrix).collect();
    book.pdf = Some(Pdf {
        asset,
        title: book.title.clone(),
        transforms,
    });
    book.pages = pages
        .into_iter()
        .map(|page| Page {
            id: vault.allocate(),
            paper: Paper {
                width: page.width,
                height: page.height,
                template: Template::Blank,
                ..Paper::default()
            },
            ..Page::default()
        })
        .collect();
    book
}
