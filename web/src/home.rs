use crate::{
    artwork_editor::{ArtworkLibrary, Editor, Target, ThemeEditor},
    auth::{self, Account},
    clock::{ClockFace, device_time},
    media::{ART, ART_ALT, frame_style, palette_from, source_url, tablet_tokens},
    settings::{
        ClockSettings, ClockSliders, FontChoices, LayoutSettings, LiveSliders, PresetSettings,
        Slider, Workspace,
    },
};
use ellipse_core::{
    artwork::Slot,
    home::{Appearance, Block, BlockSize, Mode, Wallpaper},
};
use leptos::{html, prelude::*};

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Clock,
    Layout,
    Presets,
    Artwork,
    EditArtwork(Slot),
    Wallpaper,
    Theme,
    Account,
}

impl Panel {
    fn label(self) -> &'static str {
        match self {
            Self::Clock => "Clock customisation",
            Self::Layout => "Appearance",
            Self::Presets => "Presets",
            Self::Artwork => "Workspace artwork",
            Self::EditArtwork(slot) => slot.label(),
            Self::Wallpaper => "Workspace wallpaper",
            Self::Theme => "Tablet palette",
            Self::Account => "Google account",
        }
    }
}

#[component]
pub fn Home(
    account: Account,
    message: RwSignal<String>,
    open_notes: Callback<()>,
) -> impl IntoView {
    let state = Workspace::new(account.uid);
    provide_context(state);
    let panel = RwSignal::new(None::<Panel>);
    let close = Callback::new(move |()| panel.set(None));
    let panel_signal = panel;
    let modal = NodeRef::<html::Dialog>::new();
    let now = device_time();
    let mode = Memo::new(move |_| state.preferences.with(|p| p.mode));
    let light = move || mode.get() == Mode::Tablet;
    let wallpaper = Memo::new(move |_| state.preferences.with(|p| p.appearance.wallpaper.clone()));
    let wallpaper_url = source_url(state, wallpaper, false, state.status);
    let need_palette = Memo::new(move |_| {
        state.preferences.with(|p| {
            p.mode == Mode::Tablet
                && p.appearance.tablet_theme.automatic
                && p.appearance.tablet_theme.palette.is_none()
        })
    });
    Effect::new(move |_| {
        if !need_palette.get() {
            return;
        }
        let Some(url) = wallpaper_url.get() else {
            return;
        };
        let source = wallpaper.get();
        spawn_local(async move {
            let result = palette_from(url.clone()).await;
            if need_palette.try_get() != Some(true)
                || wallpaper_url.try_get().flatten().as_deref() != Some(url.as_str())
                || wallpaper.try_get().as_ref() != Some(&source)
            {
                return;
            }
            match result {
                Ok(palette) => {
                    state.save_appearance(|look| look.tablet_theme.palette = Some(palette));
                }
                Err(error) => {
                    let _ = state.status.try_set(error);
                }
            }
        });
    });
    let tablet_palette = Memo::new(move |_| {
        light()
            .then(|| {
                state.theme_preview.get().or_else(|| {
                    state
                        .preferences
                        .with(|p| p.appearance.tablet_theme.palette)
                })
            })
            .flatten()
    });
    Effect::new(move |_| {
        if let Some(dialog) = modal.get() {
            if panel.get().is_some() && !dialog.open() {
                let _ = dialog.show_modal();
            } else if panel.get().is_none() && dialog.open() {
                dialog.close();
            }
        }
    });
    let name = StoredValue::new(account.name);
    let email = StoredValue::new(account.email);
    view! {
        <div class:workspace=true class:workspace-light=light class:workspace-themed=move || tablet_palette.with(Option::is_some)
            style=move || state.preferences.with(|p| {
                let a = &p.appearance;
                let opacity = if p.mode == Mode::Tablet { a.wallpaper_opacity.min(20) } else { a.wallpaper_opacity };
                let mut style = format!("--gap:{}px;--panel-opacity:{}%;--tone:{}%;--detail-zoom:{};--wall-opacity:{}%;",
                    a.gap, a.panel_opacity, a.tone, f64::from(a.zoom) / 100.0, opacity);
                if let Some(palette) = tablet_palette.get() {
                    style.push_str(&tablet_tokens(palette, a.wallpaper_opacity));
                }
                style
            })>
            <div class="workspace-wallpaper" aria-hidden="true">
                {move || wallpaper_url.get().map(|src| view! {
                    <img src=src alt="" style=move || state.preferences.with(|p| frame_style(p.appearance.wallpaper_frame))/>
                })}
            </div>
            <div class="workspace-inner">
                <header class="workspace-nav">
                    <a class="workspace-brand" href="#workspace-main"><span class="ellipse-mark" aria-hidden="true"></span>"Ellipse"</a>
                    <nav class="workspace-menu" aria-label="Workspace">
                        <ModeSelector state/>
                        <button class="nav-action" type="button" on:click=move |_|open_notes.run(())><Icon name="book"/>"Notes"</button>
                        <button class="nav-action" type="button" on:click=move |_| panel.set(Some(Panel::Artwork))><Icon name="image"/>"Artwork"</button>
                        <button class="nav-action" type="button" on:click=move |_| panel.set(Some(Panel::Presets))><Icon name="grid"/>"Presets"</button>
                        <button class="nav-action" type="button" on:click=move |_| panel.set(Some(Panel::Layout))><Icon name="settings"/>"Settings"</button>
                    </nav>
                    <button type="button" class="profile-button" aria-label="Google account" on:click=move |_| panel.set(Some(Panel::Account))>
                        <img src=ART width="44" height="44" alt=""/>
                        <Icon name="chevron"/>
                    </button>
                </header>
                <main id="workspace-main" tabindex="-1">
                    {move || match mode.get() {
                        Mode::NewTab => view! { <Bento state panel now/> }.into_any(),
                        Mode::Tablet => view! { <Tablet state panel now/> }.into_any(),
                        Mode::Magazine => view! { <Magazine state panel now/> }.into_any(),
                    }}
                </main>
                <p class="workspace-status" role="status" hidden=move || panel.get().is_some() || state.status.with(String::is_empty)>
                    {move || state.status.get()}
                </p>
                <dialog class="workspace-dialog" node_ref=modal aria-labelledby="panel-title"
                    on:cancel=move |event: web_sys::Event| { event.prevent_default(); panel.set(None); }>
                    <div class="panel-heading">
                        <h2 id="panel-title">{move || panel.get().map(Panel::label)}</h2>
                        <button type="button" class="home-button icon-button" aria-label="Close panel" autofocus=true
                            on:click=move |_| panel.set(None)><Icon name="close"/></button>
                    </div>
                    {move || panel.get().map(|panel| match panel {
                        Panel::Clock => view! {
                            <div class="clock-preview panel">
                                <span class="eyebrow">"Live preview"</span>
                                <ClockFace state now compact=true/>
                            </div>
                            <ClockSettings state/>
                        }.into_any(),
                        Panel::Layout => view! {
                            <LayoutSettings state edit_wallpaper=Callback::new(move |()| {
                                state.preview_ratio.set(1.65);
                                panel_signal.set(Some(Panel::Wallpaper));
                            })/>
                        }.into_any(),
                        Panel::Presets => view! { <PresetSettings state/> }.into_any(),
                        Panel::Artwork => view! { <ArtworkLibrary state/> }.into_any(),
                        Panel::EditArtwork(slot) => view! { <Editor state target=Target::Block(slot) close/> }.into_any(),
                        Panel::Wallpaper => view! { <Editor state target=Target::Wallpaper close/> }.into_any(),
                        Panel::Theme => view! { <ThemeEditor state close/> }.into_any(),
                        Panel::Account => view! {
                            <div class="account-information">
                                <strong>{name.get_value()}</strong><span>{email.get_value()}</span>
                            </div>
                            <p class="quiet">"Preferences on this device are saved to this Ellipse account."</p>
                            <button type="button" class="home-button" on:click=move |_| auth::ellipse_logout()>"Sign out"</button>
                            <p class="quiet" role="status">{move || message.get()}</p>
                        }.into_any(),
                    })}
                    <p class="dialog-status quiet" role="status">{move || state.status.get()}</p>
                </dialog>
            </div>
        </div>
    }
}

#[component]
fn ModeSelector(state: Workspace) -> impl IntoView {
    view! {
        <label class="mode-selector">
            <span class="sr-only">"Home mode"</span>
            <Icon name="home"/>
            <select
                prop:value=move || state.preferences.with(|p| p.mode.label())
                on:change=move |ev| {
                    let value = event_target_value(&ev);
                    if let Some(mode) = Mode::ALL.into_iter().find(|mode| mode.label() == value) {
                        state.preferences.update(|p| p.mode = mode);
                        state.persist();
                    }
                }>
                {Mode::ALL.into_iter().map(|mode| view! {
                    <option value=mode.label()>{mode.label()}</option>
                }).collect::<Vec<_>>()}
            </select>
            <Icon name="chevron"/>
        </label>
    }
}

#[component]
fn Icon(name: &'static str) -> impl IntoView {
    let path = match name {
        "book" => {
            "M4 4h7a2 2 0 0 1 2 2v14a4 4 0 0 0-3-1H4zM13 6a2 2 0 0 1 2-2h5v15h-4a4 4 0 0 0-3 1"
        }
        "home" => "M3 10l9-7 9 7M5 9v11h5v-6h4v6h5V9",
        "image" => "M3 4h18v16H3zM3 17l5-5 4 4 3-3 6 5M16 9h.01",
        "grid" => "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z",
        "settings" => "M4 7h16M4 17h16M8 4v6m8 4v6",
        "download" => "M12 3v12m-5-5 5 5 5-5M5 19h14",
        "arrow" => "M6 18 18 6M8 6h10v10",
        "chevron" => "m7 10 5 5 5-5",
        "close" => "m6 6 12 12M18 6 6 18",
        "clock" => "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM12 7v5l3 2",
        "contrast" => "M12 3v18M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z",
        _ => "",
    };
    view! {
        <svg class="home-icon" viewBox="0 0 24 24" width="20" height="20" aria-hidden="true"
            fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
            <path d=path/>
        </svg>
    }
}

#[component]
fn Art(
    class: &'static str,
    art_slot: Slot,
    #[prop(default = "")] crop: &'static str,
    #[prop(default = false)] lazy: bool,
    panel: RwSignal<Option<Panel>>,
) -> impl IntoView {
    let state = expect_context::<Workspace>();
    let assignment = Memo::new(move |_| state.artworks.with(|items| items.get(&art_slot).cloned()));
    let source = Memo::new(move |_| {
        assignment.with(|item| {
            item.as_ref()
                .and_then(|item| item.asset.clone())
                .map_or(Wallpaper::Artwork, Wallpaper::Local)
        })
    });
    let image_url = source_url(state, source, true, state.status);
    view! {
        <figure class=format!("art-tile {class} {crop}") data-artwork=art_slot.label()>
            <img src=move || image_url.get().unwrap_or_else(|| ART.into())
                alt=move || if source.with(|source| matches!(source, Wallpaper::Artwork)) { ART_ALT } else { "Custom workspace artwork" }
                width="3344" height="1883" decoding="async" loading=if lazy {"lazy"} else {"eager"}
                style=move || assignment.with(|item| item.as_ref().map(|item| frame_style(item.frame)).unwrap_or_default())/>
            <button type="button" class="art-open icon-button" aria-label=format!("Edit {} artwork", art_slot.label())
                title=format!("Edit {}", art_slot.label())
                on:click=move |event: web_sys::MouseEvent| {
                    let ratio = event.current_target().and_then(|node| node.dyn_into::<web_sys::Element>().ok())
                        .and_then(|button| button.parent_element()).map(|figure| {
                            let rect = figure.get_bounding_client_rect();
                            (rect.width() / rect.height().max(1.0)).clamp(0.52, 4.0)
                        }).unwrap_or(1.6);
                    state.preview_ratio.set(ratio);
                    panel.set(Some(Panel::EditArtwork(art_slot)));
                }><Icon name="image"/></button>
        </figure>
    }
}

#[component]
fn OpenPanel(
    panel: RwSignal<Option<Panel>>,
    target: Panel,
    icon: &'static str,
    label: &'static str,
) -> impl IntoView {
    view! {
        <button class="home-button icon-button" type="button" aria-label=label title=label
            on:click=move |_| panel.set(Some(target))><Icon name=icon/></button>
    }
}

#[component]
fn Actions(panel: RwSignal<Option<Panel>>, #[prop(default = false)] small: bool) -> impl IntoView {
    view! {
        <div class:quick-actions=true class:quick-actions-small=small>
            <OpenPanel panel target=Panel::Artwork icon="image" label="View artwork"/>
            <a class="home-button icon-button primary-action" href=ART download="ellipse-knight.jpg"
                aria-label="Download artwork" title="Download artwork"><Icon name="download"/></a>
            <OpenPanel panel target=Panel::Clock icon="clock" label="Clock settings"/>
            <OpenPanel panel target=Panel::Presets icon="grid" label="Open presets"/>
            <Show when=move || !small>
                <OpenPanel panel target=Panel::Layout icon="settings" label="Layout settings"/>
                <OpenPanel panel target=Panel::Account icon="contrast" label="Google account settings"/>
            </Show>
        </div>
    }
}

#[component]
fn Bento(state: Workspace, panel: RwSignal<Option<Panel>>, now: RwSignal<i64>) -> impl IntoView {
    let tiles = Memo::new(move |_| state.preferences.with(|p| p.appearance.tiles.clone()));
    let standard = Appearance::default().tiles;
    let custom = Memo::new(move |_| tiles.with(|tiles| *tiles != standard));
    view! {
        <h1 class="sr-only">"New Tab"</h1>
        <div class:bento=true class:bento-custom=move || custom.get()>
            <For each=move || tiles.get().into_iter().filter(|tile| tile.visible)
                key=|tile| (tile.block.css(), tile.size as u8)
                children=move |tile| {
                    let block = tile.block;
                    view! {
                        <div data-block=block.css() class:wide=tile.size == BlockSize::Wide class:tall=tile.size == BlockSize::Tall>
                            {match block {
                                Block::Clock => view! {
                                    <section class="panel clock-tile">
                                        <div class="tile-heading">
                                            <h2 class="eyebrow">"Your clock"</h2>
                                            <OpenPanel panel target=Panel::Clock icon="settings" label="Customise clock"/>
                                        </div>
                                        <ClockFace state now/>
                                    </section>
                                }.into_any(),
                                Block::Hero => view! { <Art class="bento-hero" art_slot=Slot::NewTabHero panel/> }.into_any(),
                                Block::ClockControls => view! {
                                    <section class="panel clock-controls-tile">
                                        <div class="tile-heading">
                                            <h2 class="eyebrow">"Clock customisation"</h2>
                                            <button class="preview-link" type="button" on:click=move |_| panel.set(Some(Panel::Clock))>"Preview" <Icon name="arrow"/></button>
                                        </div>
                                        <div class="inline-clock-settings">
                                            <div><ClockSliders state/><FontChoices state/></div>
                                            <button class="home-button customise-button" type="button" on:click=move |_| panel.set(Some(Panel::Clock))>
                                                <Icon name="settings"/><span>"Customise"</span>
                                            </button>
                                        </div>
                                    </section>
                                }.into_any(),
                                Block::Intro => view! {
                                    <section class="panel intro-tile">
                                        <div class="tile-heading"><h2 class="eyebrow">"Ellipse · New Tab"</h2><OpenPanel panel target=Panel::Layout icon="arrow" label="Customise workspace"/></div>
                                        <p class="intro-title">"Your space,"<br/><span>"refined."</span></p>
                                        <p class="quiet">"Artwork, local time and saved layouts."</p>
                                    </section>
                                }.into_any(),
                                Block::Presets => view! {
                                    <section class="panel presets-tile">
                                        <div class="tile-heading"><h2 class="eyebrow">"Saved presets"</h2><OpenPanel panel target=Panel::Presets icon="arrow" label="Manage presets"/></div>
                                        <p class="preset-count">{move || state.presets.with(Vec::len)}</p>
                                        <p class="quiet">"Layouts saved on this device"</p>
                                    </section>
                                }.into_any(),
                                Block::Palette => view! { <Palette state/> }.into_any(),
                                Block::Actions => view! {
                                    <section class="panel actions-tile"><h2 class="eyebrow">"Quick actions"</h2><Actions panel/></section>
                                }.into_any(),
                                Block::LiveControls => view! {
                                    <section class="panel live-tile"><h2 class="eyebrow">"Live controls"</h2>
                                        <LiveSliders state/>
                                        <Slider label="Panel tone" low=0 high=40 unit="%"
                                            value=Signal::derive(move || state.preferences.with(|p| p.appearance.tone.into()))
                                            change=Callback::new(move |v| state.preferences.update(|p| p.appearance.tone = v as u8)) state/>
                                    </section>
                                }.into_any(),
                                Block::Detail => view! { <Art class="bento-detail" art_slot=Slot::NewTabDetail crop="crop-detail" lazy=true panel/> }.into_any(),
                            }}
                        </div>
                    }
                }/>
        </div>
    }
}

#[component]
fn Palette(state: Workspace) -> impl IntoView {
    let swatches = [
        ("#0a0a0a", 0),
        ("#353232", 15),
        ("#201e1e", 10),
        ("#706666", 32),
        ("#818080", 40),
    ];
    view! {
        <section class="panel palette-tile">
            <div class="tile-heading"><h2 class="eyebrow">"Palette"</h2>
                <span class="eyebrow">{move || {
                    let active = state.preferences.with(|p| p.appearance.tone);
                    swatches.iter().find(|(_, tone)| *tone == active).map(|(hex, _)| *hex).unwrap_or("Custom")
                }}</span>
            </div>
            <div class="swatches" role="group" aria-label="Panel tones">
                {swatches.into_iter().map(|(hex, tone)| view! {
                    <button type="button" aria-label=format!("Panel tone {hex}") title=hex
                        aria-pressed=move || state.preferences.with(|p| p.appearance.tone == tone).to_string()
                        style:background=hex on:click=move |_| state.appearance(|p| p.tone = tone)></button>
                }).collect::<Vec<_>>()}
            </div>
            <p class="quiet">"Choose a panel tone."</p>
        </section>
    }
}

#[component]
fn EditorialRail(
    state: Workspace,
    panel: RwSignal<Option<Panel>>,
    magazine: bool,
) -> impl IntoView {
    view! {
        <aside class="editorial-rail">
            {if magazine {
                view! {
                    <h2>"Your space,"<br/><span>"refined."</span></h2>
                    <p class="quiet">"Personal artwork and layouts, saved on this device."</p>
                }.into_any()
            } else {
                view! { <h2>"A personal space for focus, creation and collection."</h2> }.into_any()
            }}
            <nav class="rail-links" aria-label="Workspace controls">
                <h3 class="eyebrow">"Workspace"</h3>
                {[(Panel::Artwork, "image", "Artwork"), (Panel::Presets, "grid", "Saved presets"),
                  (Panel::Clock, "clock", "Clock"), (Panel::Layout, "settings", "Appearance")]
                    .into_iter().map(|(target, icon, label)| view! {
                        <button type="button" on:click=move |_| panel.set(Some(target))>
                            <Icon name=icon/><span>{label}</span><Icon name="arrow"/>
                        </button>
                    }).collect::<Vec<_>>()}
            </nav>
            <Show when=move || !magazine>
                <section class="rail-actions">
                    <h3 class="eyebrow">"Quick actions"</h3>
                    <Actions panel small=true/>
                </section>
                <p class="rail-local eyebrow"><span>"Local presets"</span><strong>{move || state.presets.with(Vec::len)}</strong></p>
            </Show>
        </aside>
    }
}

#[component]
fn Tablet(state: Workspace, panel: RwSignal<Option<Panel>>, now: RwSignal<i64>) -> impl IntoView {
    view! {
        <section class="tablet-layout">
            <div class="tablet-edition">
                <p class="edition-line eyebrow"><span>"Your Ellipse"</span><span>"Tablet mode"</span></p>
                <button type="button" class="tablet-palette-button" on:click=move |_| panel.set(Some(Panel::Theme))>
                    <Icon name="contrast"/>"Palette"
                </button>
            </div>
            <h1 class="tablet-title">"Your Space, Refined."</h1>
            <div class="tablet-grid">
                <EditorialRail state panel magazine=false/>
                <div class="tablet-art-grid">
                    <Art class="tablet-feature" art_slot=Slot::TabletFeature panel/>
                    <Art class="tablet-detail" art_slot=Slot::TabletDetail crop="crop-detail" panel/>
                    <section class="panel editorial-clock">
                        <div class="tile-heading"><h2 class="eyebrow">"Local time"</h2><OpenPanel panel target=Panel::Clock icon="arrow" label="Customise local time"/></div>
                        <ClockFace state now compact=true/>
                    </section>
                    <Art class="tablet-small-a" art_slot=Slot::TabletHelmet crop="crop-helmet" lazy=true panel/>
                    <Art class="tablet-small-b" art_slot=Slot::TabletOrb crop="crop-orb" lazy=true panel/>
                </div>
            </div>
        </section>
    }
}

#[component]
fn Magazine(state: Workspace, panel: RwSignal<Option<Panel>>, now: RwSignal<i64>) -> impl IntoView {
    view! {
        <section class="magazine-layout">
            <div class="magazine-masthead"><h1>"Ellipse"</h1><p class="eyebrow">"A personal space "<br/>"for focus, ideas "<br/>"and the web."</p></div>
            <div class="magazine-grid">
                <EditorialRail state panel magazine=true/>
                <div class="magazine-art-grid">
                    <Art class="magazine-feature" art_slot=Slot::MagazineFeature panel/>
                    <Art class="magazine-detail" art_slot=Slot::MagazineDetail crop="crop-detail" panel/>
                    <Art class="magazine-small-a" art_slot=Slot::MagazineHelmet crop="crop-helmet" lazy=true panel/>
                    <Art class="magazine-small-b" art_slot=Slot::MagazineOrb crop="crop-orb" lazy=true panel/>
                </div>
            </div>
            <div class="magazine-footer">
                <section class="panel editorial-clock">
                    <div class="tile-heading"><h2 class="eyebrow">"Local time"</h2><OpenPanel panel target=Panel::Clock icon="arrow" label="Customise local time"/></div>
                    <ClockFace state now compact=true/>
                </section>
                <section class="panel magazine-module">
                    <div class="tile-heading"><h2 class="eyebrow">"Your workspace"</h2><OpenPanel panel target=Panel::Layout icon="arrow" label="Open appearance settings"/></div>
                    <div><p>"Appearance"<br/><span>"Wallpaper & layout"</span></p>
                        <Art class="module-art" art_slot=Slot::MagazineWorkspace lazy=true panel/></div>
                </section>
                <section class="panel magazine-module">
                    <div class="tile-heading"><h2 class="eyebrow">"Saved presets"</h2><OpenPanel panel target=Panel::Presets icon="arrow" label="Manage presets"/></div>
                    <div><Art class="module-art" art_slot=Slot::MagazinePresets lazy=true panel/><p>{move || state.presets.with(Vec::len)}<br/><span>"On this device"</span></p></div>
                </section>
            </div>
        </section>
    }
}
