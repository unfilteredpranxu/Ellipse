use super::io;
use leptos::{html, prelude::*};
use send_wrapper::SendWrapper;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

type PdfSignal = RwSignal<Option<Rc<io::PdfHandle>>, LocalStorage>;

#[component]
pub(super) fn PdfNavigator(
    pdf: PdfSignal,
    page_index: Memo<usize>,
    page_total: Memo<usize>,
    navigate: Callback<usize>,
) -> impl IntoView {
    let results = RwSignal::new(None::<Result<Vec<io::OutlineEntry>, String>>);
    let requested = RwSignal::new(false);
    let previews = Memo::new(move |_| {
        let total = page_total.get();
        let index = page_index.get();
        let begin = index.saturating_sub(2).min(total.saturating_sub(5));
        (begin..(begin + 5).min(total)).collect::<Vec<_>>()
    });
    Effect::new(move |_| {
        if requested.get()
            && let Some(handle) = pdf.get()
        {
            spawn_local(async move {
                let value = io::typed(io::pdf_outline(&handle.0)).await;
                let _ = results.try_set(Some(value));
            });
        }
    });
    view! {
        <div class="pdf-jump"><JumpField page_index page_total navigate/></div>
        <div class="notes-pdf-previews" aria-label="Nearby PDF pages">
            <For each=move ||previews.get() key=|index|*index children=move |index|view!{
                <Thumbnail pdf index page_index navigate/>
            }/>
        </div>
        <details class="notes-pdf-outline" on:toggle=move |_|requested.set(true)>
            <summary>"PDF outline"</summary>
            {move || match results.get() {
                Some(Ok(entries)) if entries.is_empty() => view!{<p>"No internal outline entries."</p>}.into_any(),
                Some(Ok(entries)) => view!{
                    <ul>{entries.into_iter().map(|entry|{
                        let indent=format!("--outline-depth:{}",entry.depth.min(6));
                        let title=entry.title;
                        view!{<li style=indent>{
                            if let Some(number)=entry.page {
                                view!{<button type="button" aria-current=move ||(page_index.get()+1==number).then_some("page")
                                    on:click=move |_|navigate.run(number-1)>{title}</button>}.into_any()
                            } else { view!{<span>{title}</span>}.into_any() }
                        }</li>}
                    }).collect::<Vec<_>>()}</ul>
                }.into_any(),
                Some(Err(error)) => view!{<p role="status">{error}</p>}.into_any(),
                None => view!{<p role="status">"Reading outline…"</p>}.into_any(),
            }}
        </details>
    }
}

#[component]
fn JumpField(
    page_index: Memo<usize>,
    page_total: Memo<usize>,
    navigate: Callback<usize>,
) -> impl IntoView {
    let input = NodeRef::<html::Input>::new();
    let jump = move || {
        if let Some(field) = input.get_untracked()
            && let Ok(number) = field.value().parse::<usize>()
            && (1..=page_total.get_untracked()).contains(&number)
        {
            navigate.run(number - 1);
        }
    };
    view! {
        <label>"Page"<input node_ref=input type="number" aria-label="Jump to PDF page" min=1
            max=move ||page_total.get().to_string() prop:value=move ||(page_index.get()+1).to_string()
            on:keydown=move |e: web_sys::KeyboardEvent|if e.key()=="Enter"{e.prevent_default();jump();}/></label>
        <button type="button" on:click=move |_|jump()>"Go"</button>
    }
}

#[component]
fn Thumbnail(
    pdf: PdfSignal,
    index: usize,
    page_index: Memo<usize>,
    navigate: Callback<usize>,
) -> impl IntoView {
    let canvas = NodeRef::<html::Canvas>::new();
    let failed = RwSignal::new(false);
    Effect::new(move |_| {
        if let (Some(handle), Some(node)) = (pdf.get(), canvas.get()) {
            let cleanup = SendWrapper::new(node.clone());
            on_cleanup(move || io::cancel_pdf_thumbnail(&cleanup));
            spawn_local(async move {
                if io::result(io::render_pdf_thumbnail(&handle.0, index as u32 + 1, &node))
                    .await
                    .is_err()
                {
                    let _ = failed.try_set(true);
                }
            });
        }
    });
    view! {
        <button type="button" class="notes-pdf-thumbnail" aria-label=format!("Preview page {}",index+1)
            aria-current=move ||(page_index.get()==index).then_some("page") on:click=move |_|navigate.run(index)>
            <canvas node_ref=canvas aria-hidden="true" width=1 height=1></canvas>
            <span>{index+1}</span>
            <Show when=move ||failed.get()><span class="notes-pdf-thumbnail-error">"Preview unavailable"</span></Show>
        </button>
    }
}
