use crate::auth;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ellipse_core::home::{
    Alignment, Appearance, Block, BlockSize, Clock, DateStyle, MAX_PRESET_BYTES, MAX_PRESETS,
    MAX_WALLPAPER_BYTES, Preferences, Preset, Wallpaper, normalize_presets,
};
use leptos::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::Storage;

#[derive(Clone, Copy)]
pub struct Workspace {
    pub preferences: RwSignal<Preferences>,
    pub presets: RwSignal<Vec<Preset>>,
    pub status: RwSignal<String>,
    pub drive_busy: RwSignal<bool>,
    uid: StoredValue<String>,
}

fn storage() -> Option<Storage> {
    window().local_storage().ok().flatten()
}

fn storage_key(uid: &str, kind: &str) -> String {
    format!("ellipse:{uid}:{kind}:v1")
}

fn load<T: serde::de::DeserializeOwned>(uid: &str, kind: &str) -> Option<T> {
    let raw = storage()?.get_item(&storage_key(uid, kind)).ok()??;
    (raw.len() <= MAX_PRESET_BYTES)
        .then(|| serde_json::from_str(&raw).ok())
        .flatten()
}

impl Workspace {
    pub fn new(uid: String) -> Self {
        let mut preferences = load::<Preferences>(&uid, "appearance").unwrap_or_default();
        preferences.appearance.normalize();
        let mut presets = load::<Vec<Preset>>(&uid, "presets").unwrap_or_default();
        normalize_presets(&mut presets);
        Self {
            preferences: RwSignal::new(preferences),
            presets: RwSignal::new(presets),
            status: RwSignal::new(String::new()),
            drive_busy: RwSignal::new(false),
            uid: StoredValue::new(uid),
        }
    }

    fn write(self, kind: &str, json: &str) -> bool {
        let ok = self
            .uid
            .try_with_value(|uid| {
                json.len() <= MAX_PRESET_BYTES
                    && storage().is_some_and(|storage| {
                        storage.set_item(&storage_key(uid, kind), json).is_ok()
                    })
            })
            .unwrap_or(false);
        if !ok {
            let _ = self.status.try_set(
                "Local storage is unavailable or full. Changes will last until this session ends."
                    .into(),
            );
        }
        ok
    }

    pub fn persist(self) {
        if let Some(Ok(json)) = self.preferences.try_with(serde_json::to_string) {
            self.write("appearance", &json);
        }
    }

    pub fn appearance(self, change: impl FnOnce(&mut Appearance)) {
        self.preferences.update(|p| change(&mut p.appearance));
        self.persist();
    }

    pub fn clock(self, change: impl FnOnce(&mut Clock)) {
        self.appearance(|look| {
            change(&mut look.clock);
            look.clock.normalize();
        });
    }

    fn save_presets(self, presets: Vec<Preset>) -> bool {
        if let Ok(json) = serde_json::to_string(&presets)
            && self.write("presets", &json)
        {
            return self.presets.try_set(presets).is_none();
        }
        false
    }

    fn backup(self, save: bool) {
        if self.drive_busy.get_untracked() {
            return;
        }
        let payload = if save {
            match self.presets.with_untracked(serde_json::to_string) {
                Ok(json) => Some(json),
                Err(_) => {
                    self.status.set("Could not read your local presets.".into());
                    return;
                }
            }
        } else {
            None
        };
        self.drive_busy.set(true);
        self.status.set("Waiting for Google Drive…".into());
        let operation = self.uid.with_value(|uid| auth::drive_presets(uid, payload));
        spawn_local(async move {
            let result = JsFuture::from(operation).await;
            let _ = self.drive_busy.try_set(false);
            let message = match result {
                Ok(_) if save => "Presets backed up to Google Drive.".to_owned(),
                Ok(value) => match value
                    .as_string()
                    .filter(|json| json.len() <= MAX_PRESET_BYTES)
                    .and_then(|json| serde_json::from_str::<Vec<Preset>>(&json).ok())
                {
                    Some(mut presets) => {
                        normalize_presets(&mut presets);
                        if !self.save_presets(presets) {
                            return;
                        }
                        "Presets restored from Google Drive.".into()
                    }
                    None => "The Google Drive backup could not be read.".into(),
                },
                Err(error) => js_sys::Reflect::get(&error, &"message".into())
                    .ok()
                    .and_then(|message| message.as_string())
                    .unwrap_or_else(|| "Google Drive could not complete the request.".into()),
            };
            let _ = self.status.try_set(message);
        });
    }
}

#[component]
pub fn Slider(
    label: &'static str,
    low: u16,
    high: u16,
    #[prop(default = 1)] step: u16,
    #[prop(default = "")] unit: &'static str,
    value: Signal<u16>,
    change: Callback<u16>,
    state: Workspace,
) -> impl IntoView {
    view! {
        <label class="home-slider">
            <span>{label}</span>
            <input type="range" min=low max=high step=step
                prop:value=move || value.get()
                on:input=move |ev| {
                    if let Ok(value) = event_target_value(&ev).parse::<u16>() {
                        change.run(value.clamp(low, high));
                    }
                }
                on:change=move |_| state.persist()/>
            <output>{move || format!("{}{unit}", value.get())}</output>
        </label>
    }
}

#[component]
pub fn ClockSliders(state: Workspace) -> impl IntoView {
    view! {
        <Slider label="Opacity" low=75 high=100 unit="%"
            value=Signal::derive(move || state.preferences.with(|p| p.appearance.clock.opacity.into()))
            change=Callback::new(move |v| state.preferences.update(|p| p.appearance.clock.opacity = v as u8)) state/>
        <Slider label="Blur" low=0 high=40 unit=" px"
            value=Signal::derive(move || state.preferences.with(|p| p.appearance.clock.blur.into()))
            change=Callback::new(move |v| state.preferences.update(|p| p.appearance.clock.blur = v as u8)) state/>
    }
}

#[component]
pub fn FontChoices(state: Workspace) -> impl IntoView {
    view! {
        <div class="font-choices" role="group" aria-label="Clock font">
            {ClockFontButtons(state)}
        </div>
    }
}

#[allow(non_snake_case)]
fn ClockFontButtons(state: Workspace) -> Vec<impl IntoView> {
    ellipse_core::home::ClockFont::ALL.into_iter().map(|font| view! {
        <button type="button" class="home-button" data-font=font.css() aria-label=font.label()
            title=font.label()
            aria-pressed=move || state.preferences.with(|p| p.appearance.clock.font == font).to_string()
            on:click=move |_| state.clock(|clock| clock.font = font)>"Aa"</button>
    }).collect()
}

#[component]
pub fn ClockSettings(state: Workspace) -> impl IntoView {
    view! {
        <div class="settings-stack">
            <ClockSliders state/>
            <FontChoices state/>
            <Slider label="Size" low=55 high=115 unit="%"
                value=Signal::derive(move || state.preferences.with(|p| p.appearance.clock.size.into()))
                change=Callback::new(move |v| state.preferences.update(|p| p.appearance.clock.size = v as u8)) state/>
            <Slider label="Weight" low=400 high=800 step=100
                value=Signal::derive(move || state.preferences.with(|p| p.appearance.clock.weight))
                change=Callback::new(move |v| state.preferences.update(|p| p.appearance.clock.weight = v)) state/>
            <div class="settings-fields">
                <label>"Clock color"
                    <input type="color" aria-describedby="clock-color-hint"
                        prop:value=move || state.preferences.with(|p| p.appearance.clock.color.clone())
                        on:change=move |ev| {
                            let color = event_target_value(&ev);
                            state.clock(|clock| clock.color = color.clone());
                            if state.preferences.with_untracked(|p| p.appearance.clock.color != color) {
                                state.status.set("Choose a light clock color for readable contrast.".into());
                                let input = event_target::<web_sys::HtmlInputElement>(&ev);
                                input.set_value("#ffffff");
                            }
                        }/>
                </label>
                <label>"Time format"
                    <select prop:value=move || state.preferences.with(|p| if p.appearance.clock.twenty_four {"24"} else {"12"})
                        on:change=move |ev| state.clock(|clock| clock.twenty_four = event_target_value(&ev) == "24")>
                        <option value="12">"12 hour"</option><option value="24">"24 hour"</option>
                    </select>
                </label>
                <label>"Date format"
                    <select prop:value=move || state.preferences.with(|p| match p.appearance.clock.date {
                        DateStyle::Long => "long", DateStyle::Short => "short", DateStyle::Numeric => "numeric"
                    }) on:change=move |ev| state.clock(|clock| clock.date = match event_target_value(&ev).as_str() {
                        "short" => DateStyle::Short, "numeric" => DateStyle::Numeric, _ => DateStyle::Long
                    })>
                        <option value="long">"Long"</option><option value="short">"Short"</option><option value="numeric">"Numeric"</option>
                    </select>
                </label>
                <AlignmentField label="Horizontal" state/>
                <AlignmentField label="Vertical" vertical=true state/>
            </div>
            <p class="quiet" id="clock-color-hint">"Light text keeps the clock readable at every opacity."</p>
            <div class="settings-checks">
                <label><input type="checkbox" prop:checked=move || state.preferences.with(|p| p.appearance.clock.seconds)
                    on:change=move |ev| state.clock(|c| c.seconds = event_target_checked(&ev))/>"Seconds"</label>
                <label><input type="checkbox" prop:checked=move || state.preferences.with(|p| p.appearance.clock.show_date)
                    on:change=move |ev| state.clock(|c| c.show_date = event_target_checked(&ev))/>"Show date"</label>
                <label><input type="checkbox" prop:checked=move || state.preferences.with(|p| p.appearance.clock.card)
                    on:change=move |ev| state.clock(|c| c.card = event_target_checked(&ev))/>"Clock card"</label>
            </div>
            <button type="button" class="home-button" on:click=move |_| state.clock(|clock| *clock = Clock::default())>"Reset clock"</button>
        </div>
    }
}

#[component]
fn AlignmentField(
    label: &'static str,
    state: Workspace,
    #[prop(default = false)] vertical: bool,
) -> impl IntoView {
    view! {
        <label>{label}
            <select prop:value=move || state.preferences.with(|p| {
                let clock = &p.appearance.clock;
                match if vertical { clock.vertical } else { clock.horizontal } {
                    Alignment::Start => "start", Alignment::Center => "center", Alignment::End => "end",
                }
            }) on:change=move |ev| {
                let align = match event_target_value(&ev).as_str() {
                    "start" => Alignment::Start, "end" => Alignment::End, _ => Alignment::Center,
                };
                state.clock(|clock| if vertical { clock.vertical = align } else { clock.horizontal = align });
            }>
                <option value="start">{if vertical { "Top" } else { "Left" }}</option>
                <option value="center">"Center"</option>
                <option value="end">{if vertical { "Bottom" } else { "Right" }}</option>
            </select>
        </label>
    }
}

#[component]
pub fn LiveSliders(state: Workspace) -> impl IntoView {
    view! {
        <Slider label="Panel opacity" low=80 high=100 unit="%"
            value=Signal::derive(move || state.preferences.with(|p| p.appearance.panel_opacity.into()))
            change=Callback::new(move |v| state.preferences.update(|p| p.appearance.panel_opacity = v as u8)) state/>
        <Slider label="Detail zoom" low=100 high=320 step=5 unit="%"
            value=Signal::derive(move || state.preferences.with(|p| p.appearance.zoom))
            change=Callback::new(move |v| state.preferences.update(|p| p.appearance.zoom = v)) state/>
    }
}

#[component]
pub fn LayoutSettings(state: Workspace) -> impl IntoView {
    view! {
        <div class="settings-stack">
            <div class="settings-fields">
                <label>"Wallpaper"
                    <select prop:value=move || state.preferences.with(|p| match p.appearance.wallpaper {
                        Wallpaper::Plain => "plain", Wallpaper::Artwork => "artwork", Wallpaper::Photo(_) => "photo"
                    }) on:change=move |ev| {
                        let next = event_target_value(&ev);
                        if next != "photo" {
                            state.appearance(|look| look.wallpaper = if next == "artwork" { Wallpaper::Artwork } else { Wallpaper::Plain });
                        }
                    }>
                        <option value="plain">"Plain"</option><option value="artwork">"Knight artwork"</option>
                        <option value="photo" disabled=true>"Uploaded photo"</option>
                    </select>
                </label>
                <label>"Upload wallpaper (up to 512 KiB)"
                    <input type="file" accept="image/jpeg,image/png,image/webp" on:change=move |ev| {
                        let input = event_target::<web_sys::HtmlInputElement>(&ev);
                        let Some(file) = input.files().and_then(|files| files.get(0)) else { return; };
                        input.set_value("");
                        let kind = file.type_();
                        if !matches!(kind.as_str(), "image/jpeg" | "image/png" | "image/webp")
                            || file.size() == 0.0 || file.size() > MAX_WALLPAPER_BYTES as f64 {
                            state.status.set("Choose a JPEG, PNG, or WebP image up to 512 KiB.".into());
                            return;
                        }
                        spawn_local(async move {
                            match JsFuture::from(file.array_buffer()).await {
                                Ok(buffer) => {
                                    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                                    let valid = match kind.as_str() {
                                        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
                                        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                                        "image/webp" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
                                        _ => false,
                                    };
                                    if !valid {
                                        let _ = state.status.try_set("The wallpaper does not match its image format.".into());
                                        return;
                                    }
                                    let photo = Wallpaper::Photo(format!("data:{kind};base64,{}", STANDARD.encode(bytes)));
                                    if state.preferences.try_update(|p| p.appearance.wallpaper = photo).is_some() {
                                        state.persist();
                                    }
                                }
                                Err(_) => { let _ = state.status.try_set("Could not read the wallpaper.".into()); }
                            }
                        });
                    }/>
                </label>
            </div>
            <Slider label="Wallpaper opacity" low=0 high=45 unit="%"
                value=Signal::derive(move || state.preferences.with(|p| p.appearance.wallpaper_opacity.into()))
                change=Callback::new(move |v| state.preferences.update(|p| p.appearance.wallpaper_opacity = v as u8)) state/>
            <LiveSliders state/>
            <Slider label="Panel spacing" low=8 high=24 unit=" px"
                value=Signal::derive(move || state.preferences.with(|p| p.appearance.gap.into()))
                change=Callback::new(move |v| state.preferences.update(|p| p.appearance.gap = v as u8)) state/>
            <p class="eyebrow">"New Tab blocks"</p>
            <div class="block-list">
                <For each=move || state.preferences.with(|p| p.appearance.tiles.clone())
                    key=|tile| (tile.block.css(), tile.size as u8, tile.visible)
                    children=move |tile| {
                        let block = tile.block;
                        let position = move || state.preferences.with(|p| p.appearance.tiles.iter().position(|t| t.block == block).unwrap_or(0));
                        view! {
                            <div class="block-row">
                                <label class="block-label"><input type="checkbox" checked=tile.visible disabled=block == Block::Clock
                                    on:change=move |ev| state.appearance(|look| {
                                        if let Some(tile) = look.tiles.iter_mut().find(|t| t.block == block) {
                                            tile.visible = event_target_checked(&ev);
                                        }
                                    })/>{block.label()}</label>
                                <select aria-label=format!("{} size", block.label())
                                    prop:value=match tile.size { BlockSize::Standard => "standard", BlockSize::Wide => "wide", BlockSize::Tall => "tall" }
                                    on:change=move |ev| state.appearance(|look| {
                                        if let Some(tile) = look.tiles.iter_mut().find(|t| t.block == block) {
                                            tile.size = match event_target_value(&ev).as_str() {
                                                "wide" => BlockSize::Wide, "tall" => BlockSize::Tall, _ => BlockSize::Standard
                                            };
                                        }
                                    })>
                                    <option value="standard">"Standard"</option><option value="wide">"Wide"</option><option value="tall">"Tall"</option>
                                </select>
                                <button class="home-button" type="button" aria-label=format!("Move {} up", block.label())
                                    disabled=move || position() == 0
                                    on:click=move |_| state.appearance(|look| look.move_tile(block, -1))>"↑"</button>
                                <button class="home-button" type="button" aria-label=format!("Move {} down", block.label())
                                    disabled=move || position() + 1 == Block::ALL.len()
                                    on:click=move |_| state.appearance(|look| look.move_tile(block, 1))>"↓"</button>
                            </div>
                        }
                    }/>
            </div>
            <button type="button" class="home-button" on:click=move |_| state.appearance(|look| *look = Appearance::default())>"Reset appearance"</button>
        </div>
    }
}

#[component]
pub fn PresetSettings(state: Workspace) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let restoring = RwSignal::new(false);
    let backing_up = RwSignal::new(false);
    view! {
        <div class="settings-stack">
            <form class="preset-form" on:submit=move |ev| {
                ev.prevent_default();
                let name_value: String = name.get_untracked().trim().chars().take(48).collect();
                if name_value.is_empty() { return; }
                let mut presets = state.presets.get_untracked();
                if let Some(existing) = presets.iter_mut().find(|item| item.name == name_value) {
                    existing.settings = state.preferences.get_untracked();
                } else if presets.len() < MAX_PRESETS {
                    presets.push(Preset { name: name_value, settings: state.preferences.get_untracked() });
                } else {
                    state.status.set(format!("Remove a preset before saving another. The limit is {MAX_PRESETS}."));
                    return;
                }
                if state.save_presets(presets) {
                    name.set(String::new());
                    state.status.set("Preset saved on this device.".into());
                }
            }>
                <label>"Preset name"
                    <input required=true maxlength="48" placeholder="e.g. Evening" prop:value=move || name.get()
                        on:input=move |ev| name.set(event_target_value(&ev))/>
                </label>
                <button type="submit" class="home-button">"Save preset"</button>
            </form>
            <p class="quiet">"Saving with the same name replaces that preset."</p>
            <Show when=move || state.presets.with(Vec::is_empty)>
                <p class="quiet">"No presets saved on this device."</p>
            </Show>
            <ul class="preset-list">
                {move || state.presets.with(|presets| presets.iter().enumerate().map(|(index, preset)| {
                    let title = preset.name.clone();
                    view! {
                        <li><span>{title}</span>
                            <button type="button" class="home-button" on:click=move |_| {
                                if let Some(settings) = state.presets.with_untracked(|saved| saved.get(index).map(|p| p.settings.clone())) {
                                    state.preferences.set(settings);
                                    state.persist();
                                    state.status.set("Preset applied.".into());
                                }
                            }>"Apply"</button>
                            <button type="button" class="home-button" on:click=move |_| {
                                let mut presets = state.presets.get_untracked();
                                if index < presets.len() {
                                    presets.remove(index);
                                    if state.save_presets(presets) { state.status.set("Preset removed.".into()); }
                                }
                            }>"Remove"</button>
                        </li>
                    }
                }).collect::<Vec<_>>())}
            </ul>
            <Show when=|| !auth::is_native()>
                <section class="drive-settings" aria-label="Google Drive backup">
                    <p class="eyebrow">"Google Drive"</p>
                    <p class="quiet">"Back up your named presets in Ellipse’s private app folder. Google will ask for access."</p>
                    <div class="home-button-group">
                        <button type="button" class="home-button" disabled=move || state.drive_busy.get() || state.presets.with(Vec::is_empty)
                            on:click=move |_| { restoring.set(false); backing_up.set(true); }>"Back up presets"</button>
                        <button type="button" class="home-button" disabled=move || state.drive_busy.get()
                            on:click=move |_| { backing_up.set(false); restoring.set(true); }>"Restore from Drive"</button>
                    </div>
                    <Show when=move || backing_up.get() || restoring.get()>
                        <p>{move || if backing_up.get() { "Replace the current Ellipse backup in Google Drive?" } else { "Replace presets saved on this device with the Drive backup?" }}</p>
                        <div class="home-button-group">
                            <button type="button" class="home-button" on:click=move |_| {
                                state.backup(backing_up.get_untracked());
                                backing_up.set(false);
                                restoring.set(false);
                            }>"Continue with Google"</button>
                            <button type="button" class="home-button" on:click=move |_| { restoring.set(false); backing_up.set(false); }>"Cancel"</button>
                        </div>
                    </Show>
                </section>
            </Show>
        </div>
    }
}
