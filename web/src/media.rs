use crate::settings::Workspace;
use ellipse_core::{
    artwork::{Fit, Frame, Palette, extract_palette},
    home::Wallpaper,
};
use js_sys::{Promise, Reflect, Uint8ClampedArray};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

pub const ART: &str = "/assets/ellipse-knight-clean.jpg";
pub const ART_ALT: &str = "Black-and-white stippled knight with raised sword";

#[wasm_bindgen(raw_module = "/media.js")]
extern "C" {
    #[wasm_bindgen(js_name = createLibrary)]
    fn create_library(uid: &str) -> JsValue;
    #[wasm_bindgen(js_name = stageImage)]
    fn stage_image(file: &web_sys::File) -> Promise;
    #[wasm_bindgen(js_name = releaseStage)]
    fn release_stage(stage: &JsValue);
    #[wasm_bindgen(js_name = saveImage)]
    fn save_image(library: &JsValue, stage: &JsValue) -> Promise;
    #[wasm_bindgen(js_name = leaseImage)]
    fn lease_image(library: &JsValue, id: &str) -> JsValue;
    #[wasm_bindgen(js_name = imageReady)]
    fn image_ready(lease: &JsValue) -> Promise;
    #[wasm_bindgen(js_name = releaseImage)]
    fn release_image(lease: &JsValue);
    #[wasm_bindgen(js_name = sampleImage)]
    fn sample_image(url: &str) -> Promise;
    #[wasm_bindgen(js_name = copyHex)]
    pub fn copy_hex(hex: &str) -> Promise;
}

#[derive(Clone, Copy)]
pub struct Library(StoredValue<SendWrapper<JsValue>>);

impl Library {
    pub fn new(uid: &str) -> Self {
        Self(StoredValue::new(SendWrapper::new(create_library(uid))))
    }

    pub fn save(self, stage: &Staged) -> Option<Promise> {
        self.0
            .try_with_value(|library| save_image(library, &stage.0))
    }
}

pub struct Staged(JsValue);

impl Staged {
    pub async fn from_file(file: web_sys::File) -> Result<Self, JsValue> {
        JsFuture::from(stage_image(&file)).await.map(Self)
    }

    pub fn url(&self) -> Option<String> {
        Reflect::get(&self.0, &"url".into()).ok()?.as_string()
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        release_stage(&self.0);
    }
}

pub fn error_text(error: JsValue, fallback: &str) -> String {
    if Reflect::get(&error, &"name".into())
        .ok()
        .and_then(|name| name.as_string())
        .as_deref()
        == Some("QuotaExceededError")
    {
        return "Image storage is full. Free up space on this device and try again.".into();
    }
    error
        .as_string()
        .or_else(|| Reflect::get(&error, &"message".into()).ok()?.as_string())
        .unwrap_or_else(|| fallback.into())
}

pub fn source_url(
    state: Workspace,
    source: Memo<Wallpaper>,
    fallback: bool,
    report: RwSignal<String>,
) -> RwSignal<Option<String>> {
    let url = RwSignal::new(None);
    Effect::new(move |_| match source.get() {
        Wallpaper::Plain => url.set(None),
        Wallpaper::Artwork => url.set(Some(ART.into())),
        Wallpaper::Photo(data) => url.set(Some(data)),
        Wallpaper::Local(id) => {
            url.set(if fallback { Some(ART.into()) } else { None });
            let Some(lease) = state
                .library
                .0
                .try_with_value(|library| lease_image(library, &id))
            else {
                return;
            };
            let pending = image_ready(&lease);
            let lease = SendWrapper::new(lease);
            let alive = Arc::new(AtomicBool::new(true));
            let guard = alive.clone();
            on_cleanup(move || {
                guard.store(false, Ordering::Relaxed);
                release_image(&lease);
            });
            spawn_local(async move {
                let result = JsFuture::from(pending).await;
                if !alive.load(Ordering::Relaxed) {
                    return;
                }
                match result {
                    Ok(value) => {
                        let _ = url.try_set(value.as_string());
                    }
                    Err(error) => {
                        let _ = report.try_set(error_text(error, "Could not load saved artwork."));
                    }
                }
            });
        }
    });
    url
}

pub fn frame_style(frame: Frame) -> String {
    let fit = match frame.fit {
        Fit::Cover => "cover",
        Fit::Contain => "contain",
    };
    format!(
        "object-fit:{fit};object-position:{}% {}%;transform-origin:{}% {}%;transform:scale({})",
        frame.x,
        frame.y,
        frame.x,
        frame.y,
        f64::from(frame.zoom) / 100.0
    )
}

pub async fn palette_from(url: String) -> Result<Palette, String> {
    let pixels = JsFuture::from(sample_image(&url))
        .await
        .map_err(|error| error_text(error, "Could not sample this wallpaper."))?;
    extract_palette(&Uint8ClampedArray::new(&pixels).to_vec())
        .ok_or_else(|| "This wallpaper has no visible colors to sample.".into())
}

pub fn tablet_tokens(palette: Palette, opacity: u8) -> String {
    let colors = palette.tokens();
    format!(
        "--canvas:{};--ink:{};--muted:{};--border:{};--active:{};--active-ink:{};--focus:{};--control:{};--tablet-surface:{};--wall-opacity:{}%",
        colors.canvas.hex(),
        colors.ink.hex(),
        colors.muted.hex(),
        colors.border.hex(),
        colors.selected.hex(),
        colors.selected_ink.hex(),
        colors.selected.hex(),
        colors.surface.hex(),
        colors.surface.hex(),
        opacity.min(10)
    )
}
