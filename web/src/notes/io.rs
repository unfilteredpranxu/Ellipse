use crate::media::error_text;
use ellipse_core::notes::{Book, Point, Vault, inverse_pdf, outline};
use js_sys::{Function, JSON, Promise};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(raw_module = "/notes-io.js")]
extern "C" {
    #[wasm_bindgen(js_name = invokeNotes)]
    fn invoke_notes(command: &str, args: &JsValue) -> Promise;
    #[wasm_bindgen(js_name = readLocalPdf)]
    pub fn read_local_pdf(file: &web_sys::File) -> Promise;
    #[wasm_bindgen(js_name = readStoredPdf)]
    pub fn read_stored_pdf(id: &str) -> Promise;
    #[wasm_bindgen(js_name = pdfInfo)]
    pub fn pdf_info(pdf: &JsValue) -> Promise;
    #[wasm_bindgen(js_name = closePdf)]
    fn close_pdf(pdf: &JsValue);
    #[wasm_bindgen(js_name = renderPdf)]
    pub fn render_pdf(pdf: &JsValue, page: u32, canvas: &web_sys::HtmlCanvasElement) -> Promise;
    #[wasm_bindgen(js_name = pdfOutline)]
    pub fn pdf_outline(pdf: &JsValue) -> Promise;
    #[wasm_bindgen(js_name = renderPdfThumbnail)]
    pub fn render_pdf_thumbnail(
        pdf: &JsValue,
        page: u32,
        canvas: &web_sys::HtmlCanvasElement,
    ) -> Promise;
    #[wasm_bindgen(js_name = cancelPdfThumbnail)]
    pub fn cancel_pdf_thumbnail(canvas: &web_sys::HtmlCanvasElement);
    #[wasm_bindgen(js_name = findPdf)]
    pub fn find_pdf(pdf: &JsValue, query: &str) -> Promise;
    #[wasm_bindgen(js_name = storePdf)]
    pub fn store_pdf(pdf: &JsValue) -> Promise;
    #[wasm_bindgen(js_name = downloadOriginal)]
    pub fn download_original(pdf: &JsValue, name: &str);
    #[wasm_bindgen(js_name = downloadNotebook)]
    pub fn download_notebook(json: &str, name: &str);
    #[wasm_bindgen(js_name = restoreNotes)]
    pub fn restore_notes(head: u32, file: &web_sys::File) -> Promise;
    #[wasm_bindgen(js_name = exportPdf)]
    fn export_pdf(pdf: &JsValue, spec: &JsValue, stationery: &Function, name: &str) -> Promise;
}

pub struct PdfHandle(pub JsValue);
impl Drop for PdfHandle {
    fn drop(&mut self) {
        close_pdf(&self.0);
    }
}

#[derive(Deserialize)]
pub struct PdfPage {
    pub width: f32,
    pub height: f32,
    pub matrix: [f32; 6],
}

#[derive(Clone, Deserialize)]
pub struct OutlineEntry {
    pub title: String,
    pub page: Option<usize>,
    pub depth: u8,
}

#[derive(Deserialize)]
pub struct SearchResult {
    #[serde(rename = "hasText")]
    pub has_text: bool,
    pub hits: Vec<Hit>,
}

#[derive(Clone, Deserialize)]
pub struct Hit {
    pub page: usize,
    pub text: String,
    pub runs: Vec<TextRun>,
}

#[derive(Clone, Deserialize)]
pub struct TextRun {
    pub highlight: [[f32; 2]; 2],
    pub underline: [[f32; 2]; 2],
    pub strike: [[f32; 2]; 2],
    pub width: f32,
}

#[derive(Deserialize)]
pub struct Snapshot {
    pub head: u32,
    pub vault: Vault,
    pub recovered: bool,
    #[serde(default)]
    pub restore_required: bool,
}

pub async fn result(promise: Promise) -> Result<JsValue, String> {
    JsFuture::from(promise)
        .await
        .map_err(|error| error_text(error, "Notes could not complete this operation."))
}

fn decode<T: DeserializeOwned>(value: &JsValue) -> Result<T, String> {
    let json = JSON::stringify(value)
        .ok()
        .and_then(|raw| raw.as_string())
        .ok_or("Invalid Notes response.")?;
    serde_json::from_str(&json).map_err(|_| "Invalid Notes response.".into())
}

fn encode<T: Serialize>(value: &T) -> Result<JsValue, String> {
    let json = serde_json::to_string(value).map_err(|_| "Could not encode Notes.")?;
    JSON::parse(&json).map_err(|_| "Could not encode Notes.".into())
}

pub async fn typed<T: DeserializeOwned>(promise: Promise) -> Result<T, String> {
    decode(&result(promise).await?)
}

pub async fn load() -> Result<Snapshot, String> {
    typed(invoke_notes("notes_load", &js_sys::Object::new().into())).await
}

pub async fn diagnostics() -> Result<String, String> {
    typed(invoke_notes(
        "notes_diagnostics",
        &js_sys::Object::new().into(),
    ))
    .await
}

pub async fn save(head: u32, vault: &Vault) -> Result<u32, String> {
    let args = encode(&serde_json::json!({"head":head,"vault":vault}))?;
    typed(invoke_notes("notes_save", &args)).await
}

pub async fn backup(head: u32) -> Result<String, String> {
    let args = encode(&serde_json::json!({"head":head}))?;
    typed(invoke_notes("notes_backup", &args)).await
}

pub async fn export(book: &Book, pdf: Option<&PdfHandle>) -> Result<(), String> {
    let layer = book.pages.iter().enumerate().map(|(index, page)| {
        let matrix = book.pdf.as_ref().and_then(|pdf| pdf.transforms.get(index).copied())
            .unwrap_or([1.,0.,0.,-1.,0.,page.paper.height]);
        let position = |point:Point| inverse_pdf(matrix,point).unwrap_or(point);
        let ink: Vec<_> = page.ink.iter().map(|stroke| {
            let points: Vec<_> = outline(stroke).into_iter().map(|point| {
                let point = position(point); [point.x,point.y]
            }).collect();
            serde_json::json!({"points":points,"color":stroke.pen.color,"opacity":stroke.pen.opacity})
        }).collect();
        let a = position(Point::default());
        let b = position(Point { x: 1., ..Point::default() });
        let angle = (b.y-a.y).atan2(b.x-a.x).to_degrees();
        let text: Vec<_> = page.text.iter().flat_map(|text| {
            text.value.lines().enumerate().map(move |(line, value)| {
                let point = position(Point { y: text.at.y + line as f32*text.size*1.24, ..text.at });
                serde_json::json!({"x":point.x,"y":point.y,"value":value,"color":text.color,"size":text.size,"angle":angle,"bold":text.bold,"italic":text.italic})
            })
        }).collect();
        let stationery_key = format!("{:?}:{}:{}:{}",page.paper.template,page.paper.width,page.paper.height,page.paper.spacing);
        serde_json::json!({"width":page.paper.width,"height":page.paper.height,"background":page.paper.background,
            "pattern":page.paper.pattern,"stationery":stationery_key,"ink":ink,"text":text})
    }).collect::<Vec<_>>();
    let papers = book
        .pages
        .iter()
        .map(|page| page.paper.clone())
        .collect::<Vec<_>>();
    let stationery = Closure::<dyn Fn(u32) -> Result<JsValue, JsError>>::new(move |index: u32| {
        let paper = papers
            .get(index as usize)
            .ok_or_else(|| JsError::new("Invalid stationery page."))?;
        let mut rectangles = Vec::new();
        paper.pattern_rectangles(|rect| rectangles.push(rect));
        encode(&rectangles).map_err(|message| JsError::new(&message))
    });
    result(export_pdf(
        pdf.map_or(&JsValue::NULL, |pdf| &pdf.0),
        &encode(&layer)?,
        stationery.as_ref().unchecked_ref(),
        &format!("{}.pdf", book.title),
    ))
    .await?;
    Ok(())
}
