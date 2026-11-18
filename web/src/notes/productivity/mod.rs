mod calendar;
mod syllabus;

use super::{NotesState, SaveState, Section, study::StudyPanel};
use ellipse_core::notes::planner::Day;
use leptos::prelude::*;

pub(super) fn today() -> Day {
    let date = js_sys::Date::new_0();
    Day {
        year: date.get_full_year() as u16,
        month: (date.get_month() + 1) as u8,
        day: date.get_date() as u8,
    }
}

pub(super) fn date_at(day: Day, offset: i32) -> js_sys::Date {
    js_sys::Date::new_with_year_month_day(
        u32::from(day.year),
        i32::from(day.month) - 1,
        i32::from(day.day) + offset,
    )
}

pub(super) fn shift(day: Day, offset: i32) -> Day {
    let date = date_at(day, offset);
    Day::new(
        date.get_full_year() as u16,
        (date.get_month() + 1) as u8,
        date.get_date() as u8,
    )
    .unwrap_or(day)
}

pub(super) fn localized(date: &js_sys::Date, field: &str, style: &str) -> String {
    let format = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&format, &field.into(), &style.into());
    date.to_locale_date_string("default", &format.into())
        .as_string()
        .unwrap_or_default()
}

pub(super) fn readable_duration(ms: i64) -> String {
    let seconds = ms.max(0) / 1000;
    if seconds >= 3600 {
        format!("{} h {} min", seconds / 3600, seconds % 3600 / 60)
    } else if seconds >= 60 {
        format!("{} min {} s", seconds / 60, seconds % 60)
    } else {
        format!("{seconds} s")
    }
}

#[component]
pub(super) fn Productivity(
    state: NotesState,
    selected: Section,
    section: RwSignal<Option<Section>>,
    back: Callback<()>,
) -> impl IntoView {
    view! {
        <main class:planner-page=true class:planner-focus=selected==Section::Focus aria-label=selected.label()>
            <header class="planner-nav">
                <button type="button" class="planner-brand" disabled=move ||state.local && !state.ready()
                    on:click=move |_|back.run(())>"← Ellipse"</button>
                <nav aria-label="Productivity">
                    {[Section::Notes, Section::Calendar, Section::Focus, Section::Study].map(|destination|view!{
                        <button type="button" aria-current=(destination==selected).then_some("page")
                            on:click=move |_|section.set(Some(destination))>{destination.label()}</button>
                    })}
                </nav>
                <span class="planner-save" role="status">{move ||match state.saving.get(){
                    SaveState::Clean if state.local => "Saved on this device".into(),
                    SaveState::Clean => "This browser only".into(),
                    SaveState::Saving => "Saving…".into(),
                    SaveState::Error => state.error.get(),
                }}</span>
                <Show when=move ||state.local && state.saving.get()==SaveState::Error>
                    <button type="button" on:click=move |_|state.flush()>"Retry save"</button>
                </Show>
            </header>
            {match selected {
                Section::Calendar => view!{<calendar::Calendar state section/>}.into_any(),
                Section::Focus => view!{
                    <section class="focus-room">
                        <h1>"Focus"</h1>
                        <StudyPanel state book=None full=true/>
                    </section>
                }.into_any(),
                Section::Study => view!{<syllabus::Syllabus state section/>}.into_any(),
                Section::Notes => ().into_any(),
            }}
        </main>
    }
}
