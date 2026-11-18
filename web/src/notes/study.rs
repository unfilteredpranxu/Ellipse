use super::{NotesState, now};
use ellipse_core::notes::Id;
use leptos::{leptos_dom::helpers::set_interval_with_handle, prelude::*};
use std::{collections::BTreeMap, time::Duration};

fn duration(ms: i64) -> String {
    let seconds = ms.max(0) / 1000;
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}

#[component]
pub fn StudyPanel(state: NotesState, book: Option<Id>) -> impl IntoView {
    let subject = RwSignal::new(
        book.and_then(|id| {
            state
                .vault
                .with_untracked(|v| v.book(id).map(|b| b.subject.clone()))
        })
        .unwrap_or_default(),
    );
    let minutes = RwSignal::new(25u16);
    let clock = RwSignal::new(now());
    if let Ok(timer) =
        set_interval_with_handle(move || clock.set(now()), Duration::from_millis(500))
    {
        on_cleanup(move || timer.clear());
    }
    let active = Memo::new(move |_| state.vault.with(|v| v.study.active.clone()));
    let active_display = move || {
        let time = clock.get();
        active.with(|session| {
            session
                .as_ref()
                .map(|session| {
                    let elapsed = session.elapsed(time);
                    duration(
                        session
                            .target_ms
                            .map_or(elapsed, |target| (target - elapsed).max(0)),
                    )
                })
                .unwrap_or_default()
        })
    };
    let today = move || {
        let date = js_sys::Date::new(&(clock.get() as f64).into());
        let (year, month, day) = (
            date.get_full_year(),
            date.get_month() as i32,
            date.get_date() as i32,
        );
        // Construct local calendar boundaries separately so DST is respected.
        let midnight = |m, d| js_sys::Date::new_with_year_month_day(year, m, d).get_time() as i64;
        state.vault.with(|v| {
            let totals = [
                midnight(month, day),
                midnight(month, day - (date.get_day() as i32 + 6) % 7),
                midnight(month, 1),
            ];
            let end = midnight(month, day + 1);
            totals.map(|start| {
                v.study
                    .history
                    .iter()
                    .map(|session| session.milliseconds_in(start, end))
                    .sum::<i64>()
            })
        })
    };
    view! {
        <h2>"Study"</h2>
        <Show when=move ||active.with(Option::is_none) fallback=move ||view!{
            <output class="notes-timer">{active_display}</output>
            <p>{move ||active.with(|s|s.as_ref().map(|s|if s.subject.is_empty(){"Independent study".into()}else{s.subject.clone()}))}</p>
            <div class="notes-row">
                <button type="button" on:click=move |_|state.edit(|v|if let Some(session)=&mut v.study.active{
                    if session.running_since.is_some(){session.pause(now());}else{session.resume(now());}
                })>{move ||if active.with(|s|s.as_ref().is_some_and(|s|s.running_since.is_some())){"Pause"}else{"Resume"}}</button>
                <button type="button" on:click=move |_|state.edit(|vault|{vault.study.finish(now());})>"Finish and log"</button>
            </div>
        }>
            <label>"Subject"<input maxlength=120 prop:value=move ||subject.get() on:input=move |event|subject.set(event_target_value(&event))/></label>
            <div class="notes-row">
                {[("Focus 25 min",25),("Deep work 50 min",50),("Short break 5 min",5),("Stopwatch",0)].into_iter()
                    .map(|(label,min)|view!{<button type="button" on:click=move |_|minutes.set(min)
                        aria-pressed=move ||(minutes.get()==min).to_string()>{label}</button>}).collect::<Vec<_>>()}
            </div>
            <label>"Minutes (0 for stopwatch)"<input type="number" min=0 max=540 prop:value=move ||minutes.get().to_string()
                on:input=move |e|{if let Ok(value)=event_target_value(&e).parse::<u16>(){minutes.set(value.min(540));}}/></label>
            <button type="button" class="notes-primary" on:click=move |_|{
                let minutes=minutes.get_untracked();
                state.edit(|v|{v.study.start(&subject.get_untracked(),book,(minutes>0).then_some(minutes),now());});
            }>"Start timer"</button>
        </Show>
        <Show when=move ||book.is_none()>
            <div class="notes-study-summary">
                {move || ["Today","This week","This month"].into_iter().zip(today()).map(|(label,ms)|view!{
                    <div><span>{label}</span><strong>{duration(ms)}</strong></div>
                }).collect::<Vec<_>>()}
            </div>
            <h3>"Completed sessions"</h3>
            <p class="notes-subtle">"Elapsed timers report time recorded, including time away from the app."</p>
            {move ||state.vault.with(|v|{
                let mut subjects=BTreeMap::<String,i64>::new();
                for record in &v.study.history{
                    let key=if record.subject.is_empty(){"No subject".into()}else{record.subject.clone()};
                    *subjects.entry(key).or_default()+=record.elapsed(record.finished.unwrap_or(record.started));
                }
                subjects.into_iter().map(|(subject,ms)|view!{<p>{format!("{subject} · {}",duration(ms))}</p>}).collect::<Vec<_>>()
            })}
            {move ||state.vault.with(|v|v.study.history.iter().rev().take(25).map(|record|{
                let time=js_sys::Date::new(&(record.started as f64).into()).to_locale_string("default",&js_sys::Object::new()).as_string().unwrap_or_default();
                view!{<p class="notes-session">{format!("{} · {} · {}",record.subject,time,duration(record.elapsed(record.finished.unwrap_or(record.started))))}</p>}
            }).collect::<Vec<_>>())}
        </Show>
    }
}
