use super::NotesState;
use crate::media::{Library, Staged, error_text, frame_style, local_source_url};
use ellipse_core::{
    artwork::{Fit, Frame},
    home::Wallpaper,
    notes::{LibraryAppearance, LocalImage},
};
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn LocalPicture(
    library: Library,
    image: Signal<Option<LocalImage>>,
    class: &'static str,
    report: RwSignal<String>,
) -> impl IntoView {
    let source = Memo::new(move |_| {
        image.with(|image| {
            image.as_ref().map_or(Wallpaper::Plain, |entry| {
                Wallpaper::Local(entry.asset.clone())
            })
        })
    });
    let url = local_source_url(library, source, false, report);
    view! {
        {move ||url.get().map(|src|view!{
            <img class=class src=src alt="" draggable="false" decoding="async"
                style=move ||image.with(|value|value.as_ref().map(|image|frame_style(image.frame))) />
        })}
    }
}

#[component]
pub fn ImageChoice(
    library: Library,
    image: RwSignal<Option<LocalImage>>,
    label: &'static str,
) -> impl IntoView {
    let note = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let frame = Memo::new(move |_| {
        image.with(|entry| entry.as_ref().map(|image| image.frame).unwrap_or_default())
    });
    let update = move |change: fn(&mut Frame, u16), value: u16| {
        image.update(|item| {
            if let Some(item) = item {
                change(&mut item.frame, value);
                item.frame.normalize();
            }
        });
    };
    view! {
        <div class="notes-image-form">
            <div class="notes-image-preview">
                <LocalPicture library image=Signal::derive(move ||image.get()) class="notes-custom-image" report=note/>
            </div>
            <label class="notes-file">{label}
                <input type="file" accept="image/jpeg,image/png,image/webp,image/heic,.heic,.heif" disabled=move ||busy.get()
                    on:change=move |event|{
                        let input=event_target::<web_sys::HtmlInputElement>(&event);
                        let file=input.files().and_then(|f|f.get(0));
                        input.set_value("");
                        let Some(file)=file else{return};
                        busy.set(true);note.set("Preparing image…".into());
                        spawn_local(async move{
                            let loaded=async {
                                let staged=Staged::from_file(file).await.map_err(|e|error_text(e,"Could not decode the image."))?;
                                let pending=library.save(&staged).ok_or("This image picker is closed.")?;
                                let asset=super::io::result(pending).await?.as_string().ok_or("Invalid image reference.")?;
                                Ok::<_,String>(LocalImage{asset,frame:Frame::default()})
                            }.await;
                            if busy.try_set(false).is_some(){return;}
                            match loaded{
                                Ok(value)=>{image.set(Some(value));note.set(String::new());}
                                Err(error)=>note.set(error),
                            }
                        });
                    }/>
            </label>
            <Show when=move ||image.with(Option::is_some)>
                <div class="notes-image-controls">
                    <label>"Fit"<select prop:value=move ||if frame.get().fit==Fit::Cover{"fill"}else{"fit"}
                        on:change=move |e|update(|frame,value|frame.fit=if value==1{Fit::Contain}else{Fit::Cover},u16::from(event_target_value(&e)=="fit"))>
                        <option value="fill">"Fill and crop"</option><option value="fit">"Fit entire image"</option>
                    </select></label>
                    <label>"Horizontal position"<input type="range" min=0 max=100 prop:value=move ||frame.get().x.to_string()
                        on:input=move |e|{if let Ok(value)=event_target_value(&e).parse::<u16>(){update(|f,v|f.x=v.min(100) as u8,value);}}/></label>
                    <label>"Vertical position"<input type="range" min=0 max=100 prop:value=move ||frame.get().y.to_string()
                        on:input=move |e|{if let Ok(value)=event_target_value(&e).parse::<u16>(){update(|f,v|f.y=v.min(100) as u8,value);}}/></label>
                    <label>"Image zoom"<input type="range" min=100 max=300 prop:value=move ||frame.get().zoom.to_string()
                        on:input=move |e|{if let Ok(value)=event_target_value(&e).parse::<u16>(){update(|f,v|f.zoom=v,value);}}/></label>
                    <button type="button" on:click=move |_|image.set(None)>"Remove image"</button>
                </div>
            </Show>
            <p class="notes-subtle" role="status">{move ||note.get()}</p>
        </div>
    }
}

#[component]
pub fn AppearancePanel(state: NotesState, done: Callback<()>) -> impl IntoView {
    let current = state.vault.with_untracked(|v| v.appearance.clone());
    let wallpaper = RwSignal::new(current.wallpaper.clone());
    let look = RwSignal::new(current);
    let draft = move || {
        let mut next = look.get_untracked();
        next.wallpaper = wallpaper.get_untracked();
        next
    };
    on_cleanup(move || {
        let _ = state.library_preview.try_set(None);
    });
    view! {
        <h2>"Library appearance"</h2>
        <ImageChoice library=state.media image=wallpaper label="Choose wallpaper"/>
        <div class="notes-image-controls">
            { [("Wallpaper opacity",0,35),("Brightness",45,120),("Blur",0,14),("Frost",0,70)].into_iter().enumerate()
                .map(|(index,(label,min,max))|view!{
                    <label>{label}<input type="range" min=min max=max prop:value=move ||look.with(|look|{
                        match index{0=>look.opacity,1=>look.brightness,2=>look.blur,_=>look.frost}.to_string()
                    }) on:input=move |e|{
                        if let Ok(value)=event_target_value(&e).parse::<u8>(){
                            let value=value.clamp(min,max);
                            look.update(|look| match index{0=>look.opacity=value,1=>look.brightness=value,2=>look.blur=value,_=>look.frost=value});
                        }
                    }/></label>
                }).collect::<Vec<_>>() }
        </div>
        <div class="notes-row">
            <button type="button" on:click=move |_|{wallpaper.set(None);look.set(LibraryAppearance::default());}>"Reset appearance"</button>
            <button type="button" on:click=move |_|state.library_preview.set(Some(draft()))>"Preview wallpaper"</button>
            <button type="button" on:click=move |_|done.run(())>"Cancel"</button>
            <button class="notes-primary" type="button" on:click=move |_|{
                state.edit(|v|v.appearance=draft());done.run(());
            }>"Apply appearance"</button>
        </div>
    }
}
