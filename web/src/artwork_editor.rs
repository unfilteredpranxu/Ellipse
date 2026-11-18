use crate::{
    media::{
        ART, Staged, copy_hex, error_text, frame_style, palette_from, source_url, tablet_tokens,
    },
    settings::Workspace,
};
use ellipse_core::{
    artwork::{Assignment, Fit, Frame, Slot, TabletTheme},
    home::{Appearance, Wallpaper},
};
use leptos::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::{JsFuture, spawn_local};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Block(Slot),
    Wallpaper,
}

pub fn default_frame(slot: Slot, zoom: u16) -> Frame {
    let (x, y, zoom) = match slot {
        Slot::NewTabHero => (50, 48, 100),
        Slot::NewTabDetail => (48, 45, zoom),
        Slot::TabletDetail | Slot::MagazineDetail => (48, 45, 100 + (zoom - 100) * 35 / 100),
        Slot::TabletHelmet | Slot::MagazineHelmet => (46, 39, 210),
        Slot::TabletOrb | Slot::MagazineOrb => (66, 76, 250),
        _ => (50, 52, 100),
    };
    Frame {
        x,
        y,
        zoom,
        fit: Fit::Cover,
    }
}

#[derive(Clone)]
struct Draft {
    source: Wallpaper,
    frame: Frame,
    opacity: u8,
}

#[component]
pub fn Editor(state: Workspace, target: Target, close: Callback<()>) -> impl IntoView {
    let appearance = state.preferences.with_untracked(|p| p.appearance.clone());
    let original_frame = match target {
        Target::Block(slot) => default_frame(slot, appearance.zoom),
        Target::Wallpaper => Frame::default(),
    };
    let initial = match target {
        Target::Block(slot) => {
            let selection = state
                .artworks
                .with_untracked(|items| items.get(&slot).cloned());
            Draft {
                source: selection
                    .as_ref()
                    .and_then(|item| item.asset.clone())
                    .map_or(Wallpaper::Artwork, Wallpaper::Local),
                frame: selection.map_or(original_frame, |item| item.frame),
                opacity: 100,
            }
        }
        Target::Wallpaper => Draft {
            source: appearance.wallpaper,
            frame: appearance.wallpaper_frame,
            opacity: appearance.wallpaper_opacity,
        },
    };
    let draft = RwSignal::new(initial);
    let staged = RwSignal::new_local(None::<Rc<Staged>>);
    let busy = RwSignal::new(false);
    let note = RwSignal::new(String::new());
    let source = Memo::new(move |_| draft.with(|draft| draft.source.clone()));
    let saved_url = source_url(state, source, false, note);
    let load = Callback::new(move |file: web_sys::File| {
        if busy.get_untracked() {
            return;
        }
        busy.set(true);
        note.set("Reading image…".into());
        spawn_local(async move {
            let result = Staged::from_file(file).await;
            if busy.try_set(false).is_some() {
                return;
            }
            match result {
                Ok(image) => {
                    staged.set(Some(Rc::new(image)));
                    draft.update(|draft| draft.frame = Frame::default());
                    note.set(String::new());
                }
                Err(error) => note.set(error_text(error, "Could not read the selected image.")),
            }
        });
    });
    let reused = state.artworks.with_untracked(|items| {
        let mut choices = Vec::new();
        for (slot, assignment) in items {
            if let Some(id) = &assignment.asset
                && !choices.iter().any(|(key, _)| key == id)
            {
                choices.push((id.clone(), slot.label()));
            }
        }
        if let Wallpaper::Local(id) = state
            .preferences
            .with_untracked(|p| p.appearance.wallpaper.clone())
            && !choices.iter().any(|(key, _)| *key == id)
        {
            choices.push((id, "Workspace wallpaper"));
        }
        choices
    });
    let apply = move |_| {
        if busy.get_untracked() {
            return;
        }
        let item = staged.get_untracked();
        let mut selection = draft.get_untracked();
        let pending = item.as_ref().and_then(|item| state.library.save(item));
        busy.set(true);
        spawn_local(async move {
            if let Some(pending) = pending {
                match JsFuture::from(pending).await {
                    Ok(key) => {
                        let Some(key) = key.as_string() else {
                            return;
                        };
                        selection.source = Wallpaper::Local(key);
                    }
                    Err(error) => {
                        if busy.try_set(false).is_none() {
                            note.set(error_text(
                                error,
                                "Could not save the image on this device.",
                            ));
                        }
                        return;
                    }
                }
            }
            if busy.try_set(false).is_some() {
                return;
            }
            let saved = match target {
                Target::Block(slot) => state.save_artwork(|items| {
                    let asset = match selection.source {
                        Wallpaper::Local(id) => Some(id),
                        _ => None,
                    };
                    if asset.is_none() && selection.frame == original_frame {
                        items.remove(&slot);
                    } else {
                        items.insert(
                            slot,
                            Assignment {
                                asset,
                                frame: selection.frame,
                            },
                        );
                    }
                }),
                Target::Wallpaper => state.save_appearance(|look| {
                    if look.wallpaper != selection.source {
                        look.tablet_theme.palette = None;
                    }
                    look.wallpaper = selection.source;
                    look.wallpaper_frame = selection.frame;
                    look.wallpaper_opacity = selection.opacity;
                }),
            };
            if saved {
                let _ = state.status.try_set(String::new());
                close.run(());
            }
        });
    };
    view! {
        <div class="image-editor"
            on:dragover=move |event: web_sys::DragEvent| { event.prevent_default(); }
            on:drop=move |event: web_sys::DragEvent| {
                event.prevent_default();
                if let Some(file) = event.data_transfer().and_then(|data| data.files()).and_then(|files| files.get(0)) {
                    load.run(file);
                }
            }>
            <div class="image-preview" style=move || format!("--preview-ratio:{}", state.preview_ratio.get())>
                {move || {
                    let src = staged.with(|item| item.as_ref().and_then(|item| item.url())).or_else(|| saved_url.get());
                    src.map(|src| view! {
                        <img src=src alt="Artwork preview"
                            style=move || draft.with(|item| format!("{};opacity:{}%", frame_style(item.frame),
                                if target == Target::Wallpaper { item.opacity } else { 100 }))/>
                    })
                }}
            </div>
            <div class="image-source-controls">
                <label class="image-file"><span>"Choose local image"</span>
                    <input type="file" aria-label="Choose local image"
                        accept="image/jpeg,image/png,image/webp,image/heic,image/heif,.heic,.heif"
                        disabled=move || busy.get()
                        on:change=move |event| {
                            let input = event_target::<web_sys::HtmlInputElement>(&event);
                            let file = input.files().and_then(|files| files.get(0));
                            input.set_value("");
                            if let Some(file) = file { load.run(file); }
                        }/>
                </label>
                <button type="button" class="home-button" disabled=move || busy.get() on:click=move |_| {
                    staged.set(None);
                    draft.update(|item| { item.source = Wallpaper::Artwork; item.frame = original_frame; });
                    note.set(String::new());
                }>{if target == Target::Wallpaper { "Use Ellipse artwork" } else { "Restore original" }}</button>
                <Show when=move || target == Target::Wallpaper>
                    <button type="button" class="home-button" disabled=move || busy.get() on:click=move |_| {
                        staged.set(None);
                        draft.set(Draft { source: Wallpaper::Plain, frame: Frame::default(), opacity: Appearance::default().wallpaper_opacity });
                        note.set(String::new());
                    }>"Reset to plain"</button>
                </Show>
            </div>
            {(!reused.is_empty()).then(|| view! {
                <label class="media-field">"Reuse assigned image"
                    <select disabled=move || busy.get() on:change=move |event| {
                        let key = event_target_value(&event);
                        if !key.is_empty() {
                            staged.set(None);
                            draft.update(|item| { item.source = Wallpaper::Local(key); item.frame = Frame::default(); });
                        }
                    }>
                        <option value="">"Choose an image"</option>
                        {reused.into_iter().map(|(id, label)| view! { <option value=id>{label}</option> }).collect::<Vec<_>>()}
                    </select>
                </label>
            })}
            <p class="quiet media-policy">"Images stay on this device. Large photos may use a smaller display copy."</p>
            <div class="image-framing">
                <label class="media-field">"Fit"
                    <select prop:value=move || draft.with(|item| if item.frame.fit == Fit::Cover {"cover"} else {"contain"})
                        on:change=move |event| draft.update(|item| {
                            item.frame.fit = if event_target_value(&event) == "contain" { Fit::Contain } else { Fit::Cover };
                        })>
                        <option value="cover">"Fill and crop"</option>
                        <option value="contain">"Fit entire image"</option>
                    </select>
                </label>
                <DraftRange draft field=FrameField::Zoom/>
                <DraftRange draft field=FrameField::Horizontal/>
                <DraftRange draft field=FrameField::Vertical/>
                <Show when=move || target == Target::Wallpaper>
                    <DraftRange draft field=FrameField::Opacity/>
                </Show>
            </div>
            <p class="quiet media-note" role="status">{move || note.get()}</p>
            <div class="media-actions">
                <button class="home-button" type="button" on:click=move |_| close.run(())>"Cancel"</button>
                <button class="home-button primary-action" type="button" disabled=move || busy.get() on:click=apply>"Apply image"</button>
            </div>
        </div>
    }
}

#[derive(Clone, Copy)]
enum FrameField {
    Zoom,
    Horizontal,
    Vertical,
    Opacity,
}

#[component]
fn DraftRange(draft: RwSignal<Draft>, field: FrameField) -> impl IntoView {
    let (label, min, max) = match field {
        FrameField::Zoom => ("Zoom", 100, 300),
        FrameField::Horizontal => ("Horizontal position", 0, 100),
        FrameField::Vertical => ("Vertical position", 0, 100),
        FrameField::Opacity => ("Wallpaper opacity", 0, 45),
    };
    let value = move || {
        draft.with(|item| match field {
            FrameField::Zoom => item.frame.zoom,
            FrameField::Horizontal => u16::from(item.frame.x),
            FrameField::Vertical => u16::from(item.frame.y),
            FrameField::Opacity => u16::from(item.opacity),
        })
    };
    view! {
        <label class="media-field">
            <span>{label} <span class="media-value" aria-hidden="true">{move || format!("{}%", value())}</span></span>
            <input type="range" min=min max=max prop:value=value on:input=move |event| {
                if let Ok(next) = event_target_value(&event).parse::<u16>() {
                    let next = next.clamp(min, max);
                    draft.update(|item| match field {
                        FrameField::Zoom => item.frame.zoom = next,
                        FrameField::Horizontal => item.frame.x = next as u8,
                        FrameField::Vertical => item.frame.y = next as u8,
                        FrameField::Opacity => item.opacity = next as u8,
                    });
                }
            }/>
        </label>
    }
}

#[component]
pub fn ArtworkLibrary(state: Workspace) -> impl IntoView {
    let confirming = RwSignal::new(false);
    view! {
        <img class="artwork-original" src=ART width="3344" height="1883" alt=crate::media::ART_ALT/>
        <div class="home-button-group">
            <a class="home-button" href=ART download="ellipse-knight.jpg">"Download original"</a>
            <button class="home-button" type="button" disabled=move || state.artworks.with(|items| items.is_empty())
                on:click=move |_| confirming.set(true)>"Restore all default artwork"</button>
        </div>
        <Show when=move || confirming.get()>
            <div class="artwork-confirm">
                <p>"Restore the original images and crops in every mode?"</p>
                <div class="home-button-group">
                    <button class="home-button" type="button" on:click=move |_| confirming.set(false)>"Cancel reset"</button>
                    <button class="home-button primary-action" type="button" on:click=move |_| {
                        if state.save_artwork(|items| items.clear()) {
                            confirming.set(false);
                            state.status.set("Default block artwork restored.".into());
                        }
                    }>"Restore all"</button>
                </div>
            </div>
        </Show>
    }
}

#[component]
pub fn ThemeEditor(state: Workspace, close: Callback<()>) -> impl IntoView {
    let original = state
        .preferences
        .with_untracked(|p| p.appearance.tablet_theme);
    let palette = RwSignal::new(original.palette);
    let automatic = RwSignal::new(original.automatic);
    let note = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let source = Memo::new(move |_| state.preferences.with(|p| p.appearance.wallpaper.clone()));
    let url = source_url(state, source, false, note);
    on_cleanup(move || {
        let _ = state.theme_preview.try_set(None);
    });
    view! {
        <div class="theme-editor">
            <label class="theme-switch">
                <input type="checkbox" prop:checked=move || automatic.get() on:change=move |ev| automatic.set(event_target_checked(&ev))/>
                "Auto theme from wallpaper"
            </label>
            <div class="home-button-group">
                <button type="button" class="home-button" disabled=move || busy.get() || url.with(Option::is_none)
                    on:click=move |_| {
                        let Some(url) = url.get_untracked() else { return; };
                        busy.set(true);
                        note.set("Reading wallpaper colors…".into());
                        spawn_local(async move {
                            let result = palette_from(url).await;
                            if busy.try_set(false).is_some() { return; }
                            match result {
                                Ok(colors) => { palette.set(Some(colors)); note.set(String::new()); }
                                Err(error) => note.set(error),
                            }
                        });
                    }>"Regenerate palette"</button>
                <button type="button" class="home-button" disabled=move || palette.with(Option::is_none)
                    on:click=move |_| state.theme_preview.set(palette.get_untracked())>"Preview palette"</button>
            </div>
            {move || palette.get().map(|colors| view! {
                <div class="theme-sample" style=tablet_tokens(colors, 0)>
                    <strong>"Your Space, Refined."</strong>
                    <span>"Tablet palette"</span>
                </div>
                <div class="theme-swatches" aria-label="Wallpaper palette">
                    {colors.swatches().into_iter().map(|(label, color)| {
                        let hex = color.hex();
                        let value = hex.clone();
                        let copy_label = format!("Copy {label} {hex}");
                        view! {
                            <button type="button" class="theme-swatch" aria-label=copy_label
                                on:click=move |_| {
                                    let pending = copy_hex(&value);
                                    spawn_local(async move {
                                        let text = if JsFuture::from(pending).await.is_ok() { "Color copied." } else { "Clipboard is unavailable. Select the HEX value to copy it." };
                                        let _ = note.try_set(text.into());
                                    });
                                }>
                                <span class="theme-chip" style:background=hex.clone()></span>
                                <span>{label}</span><code>{hex}</code>
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            })}
            <p class="quiet media-note" role="status">{move || note.get()}</p>
            <div class="media-actions theme-actions">
                <button class="home-button" type="button" on:click=move |_| {
                    if state.save_appearance(|look| look.tablet_theme = TabletTheme { automatic: false, palette: None }) { close.run(()); }
                }>"Reset Tablet theme"</button>
                <button class="home-button" type="button" on:click=move |_| close.run(())>"Cancel"</button>
                <button class="home-button primary-action" type="button" disabled=move || busy.get() on:click=move |_| {
                    if state.save_appearance(|look| look.tablet_theme = TabletTheme {
                        automatic: automatic.get_untracked(),
                        palette: palette.get_untracked(),
                    }) { close.run(()); }
                }>"Apply palette"</button>
            </div>
        </div>
    }
}
