use super::{
    NotesState, now,
    productivity::{date_at, readable_duration, shift, today},
};
use ellipse_core::notes::{Cycle, Id, SessionKind};
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
pub fn StudyPanel(
    state: NotesState,
    book: Option<Id>,
    #[prop(optional)] full: bool,
) -> impl IntoView {
    let task = RwSignal::new(if full {
        state.focus_task.get_untracked()
    } else {
        None
    });
    let topic = RwSignal::new(if full {
        state.focus_topic.get_untracked()
    } else {
        None
    });
    let document = RwSignal::new(book);
    let subject = RwSignal::new(state.vault.with_untracked(|v| {
        if let Some(task) = task
            .get_untracked()
            .and_then(|id| v.planner.tasks.iter().find(|task| task.id == id))
        {
            document.set(task.book);
            topic.set(task.topic);
            return task.subject.clone();
        }
        if let Some(topic) = topic
            .get_untracked()
            .and_then(|id| v.planner.topics.iter().find(|topic| topic.id == id))
        {
            return topic.subject.label().into();
        }
        book.and_then(|id| v.book(id).map(|b| b.subject.clone()))
            .unwrap_or_default()
    }));
    let minutes = RwSignal::new(25u16);
    let automatic = RwSignal::new(false);
    let rounds = RwSignal::new(4u8);
    let rest_minutes = RwSignal::new(5u16);
    let kind = RwSignal::new(SessionKind::Focus);
    let clock = RwSignal::new(state.timer_now());
    let notice = RwSignal::new(String::new());
    if let Ok(timer) = set_interval_with_handle(
        move || {
            if clock.try_get_untracked().is_some() {
                clock.set(state.timer_now());
            }
        },
        Duration::from_millis(500),
    ) {
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
    let today_totals = Memo::new(move |_| {
        let selected = today();
        let end = date_at(selected, 1).get_time() as i64;
        let starts = [
            date_at(selected, 0),
            date_at(shift(selected, -i32::from(selected.weekday())), 0),
            date_at(ellipse_core::notes::planner::Day { day: 1, ..selected }, 0),
        ];
        state.vault.with(|v| {
            starts.map(|date| {
                v.study
                    .history
                    .iter()
                    .filter(|session| session.kind != SessionKind::Break)
                    .map(|session| session.milliseconds_in(date.get_time() as i64, end))
                    .sum::<i64>()
            })
        })
    });
    let start = move || {
        let minutes = minutes.get_untracked();
        let kind = if minutes == 0 {
            SessionKind::Stopwatch
        } else {
            kind.get_untracked()
        };
        let cycle = if full && automatic.get_untracked() && kind == SessionKind::Focus {
            Some(Cycle {
                focus_minutes: minutes,
                break_minutes: rest_minutes.get_untracked(),
                remaining: rounds.get_untracked(),
            })
        } else {
            None
        };
        let result = state.edit_productivity(|vault| {
            if !vault.study.start(
                &subject.get_untracked(),
                document.get_untracked(),
                (minutes > 0).then_some(minutes),
                now(),
            ) {
                return Err("Finish the current session before starting another.");
            }
            if let Some(session) = vault.study.active.as_mut() {
                session.kind = kind;
                session.task = task.get_untracked();
                session.topic = topic.get_untracked();
            }
            // The controls bound every parameter before the active session is created.
            if let Some(cycle) = cycle {
                vault.study.enable_cycle(cycle)?;
            }
            Ok(())
        });
        clock.set(state.timer_now());
        notice.set(result.err().unwrap_or_default());
    };
    view! {
        {(!full).then(||view!{<h2>"Study"</h2>})}
        <Show when=move ||active.with(Option::is_none) fallback=move ||view!{
            <p class="focus-kind">{move ||active.with(|s|s.as_ref().map(|s|s.kind.label()))}</p>
            <output class="notes-timer" aria-label="Session time" aria-live="off">{active_display}</output>
            <p>{move ||active.with(|s|s.as_ref().map(|s|if s.subject.is_empty(){"Independent study".into()}else{s.subject.clone()}))}</p>
            <Show when=move ||active.with(|s|s.as_ref().is_some_and(|s|s.target_ms.is_some_and(|target|s.elapsed(clock.get())>=target)))>
                <p>"Target reached. Finish to record this session."</p>
            </Show>
            <Show when=move ||state.vault.with(|v|v.study.cycle.is_some())>
                <p>{move ||state.vault.with(|v|v.study.cycle.map(|cycle|format!("{} focus rounds remaining",cycle.remaining)))}</p>
            </Show>
            <div class="notes-row focus-controls">
                <button type="button" on:click=move |_|{
                    let time=state.timer_now();
                    let result=state.edit_productivity(|v| {
                        let session=v.study.active.as_mut().ok_or("The session has ended.")?;
                        if session.running_since.is_some(){session.pause(time);}else{session.resume(time);}
                        Ok(())
                    });
                    clock.set(state.timer_now());
                    notice.set(result.err().unwrap_or_default());
                }>{move ||if active.with(|s|s.as_ref().is_some_and(|s|s.running_since.is_some())){"Pause"}else{"Resume"}}</button>
                <button class="notes-primary" type="button" on:click=move |_|{
                    let time=state.timer_now();
                    let result=state.edit_productivity(|vault|{
                        vault.study.finish(time).then_some(()).ok_or("The session has ended.")
                    });
                    notice.set(result.err().unwrap_or_default());
                }>"Finish and log"</button>
                {full.then(||view!{<button type="button" on:click=move |_|{
                    if window().confirm_with_message("Discard the active phase? Completed phases will remain in your log.").unwrap_or(false){
                        let result=state.edit_productivity(|vault|{vault.study.cancel(); Ok(())});
                        notice.set(result.err().unwrap_or_default());
                    }
                }>"Discard active phase"</button>})}
            </div>
        }>
            {full.then(||view!{<div class="focus-preview" aria-hidden="true">{move || if minutes.get()==0 {"00:00:00".into()}else{duration(i64::from(minutes.get()) * 60_000)}}</div>})}
            <div class="focus-setup">
                <label>"Subject"<input maxlength=120 prop:value=move ||subject.get()
                    on:input=move |event|subject.set(event_target_value(&event))/></label>
                {full.then(||view!{<SessionLinks state task topic document subject/>})}
                <div class="notes-row focus-controls">
                    {[
                        ("Focus 25 min",25,SessionKind::Focus),("Deep work 50 min",50,SessionKind::Focus),
                        ("Deep work 90 min",90,SessionKind::Focus),("Short break 5 min",5,SessionKind::Break),
                        ("Long break 10 min",10,SessionKind::Break),("Stopwatch",0,SessionKind::Stopwatch),
                    ].into_iter().map(|(label,min,mode)|view!{<button type="button" on:click=move |_|{
                        minutes.set(min);kind.set(mode);
                    } aria-pressed=move ||(minutes.get()==min&&kind.get()==mode).to_string()>{label}</button>}).collect::<Vec<_>>()}
                </div>
                <label>"Minutes (0 for stopwatch)"<input type="number" min=0 max=540 prop:value=move ||minutes.get().to_string()
                    on:input=move |e|{if let Ok(value)=event_target_value(&e).parse::<u16>(){
                        minutes.set(value.min(540));
                        if value==0{kind.set(SessionKind::Stopwatch);}
                        else if kind.get_untracked()==SessionKind::Stopwatch{kind.set(SessionKind::Focus);}
                    }}/></label>
                {full.then(||view!{
                    <Show when=move || { kind.get()==SessionKind::Focus && minutes.get()>0 }>
                        <details class="focus-links" open=move ||automatic.get()>
                            <summary>"Pomodoro cycle"</summary>
                            <label class="notes-row"><input type="checkbox" prop:checked=move ||automatic.get()
                                on:change=move |e|automatic.set(event_target_checked(&e))/>"Advance focus and breaks automatically"</label>
                            <div class="planner-form-grid">
                                <label>"Focus rounds"<input type="number" min=1 max=12 prop:value=move ||rounds.get().to_string()
                                    on:change=move |e|{if let Ok(value)=event_target_value(&e).parse::<u8>(){rounds.set(value.clamp(1,12));}}/></label>
                                <label>"Break minutes"<input type="number" min=1 max=120 prop:value=move ||rest_minutes.get().to_string()
                                    on:change=move |e|{if let Ok(value)=event_target_value(&e).parse::<u16>(){rest_minutes.set(value.clamp(1,120));}}/></label>
                            </div>
                        </details>
                    </Show>
                })}
                <button type="button" class="notes-primary" on:click=move |_|start()>"Start timer"</button>
            </div>
        </Show>
        <p role="status">{move ||notice.get()}</p>
        <Show when=move ||book.is_none() && !full>
            <div class="notes-study-summary">
                {move || ["Today","This week","This month"].into_iter().zip(today_totals.get()).map(|(label,ms)|view!{
                    <div><span>{label}</span><strong>{readable_duration(ms)}</strong></div>
                }).collect::<Vec<_>>()}
            </div>
            <h3>"Recorded sessions"</h3>
            {move ||state.vault.with(|v|{
                let mut subjects=BTreeMap::<String,i64>::new();
                for record in v.study.history.iter().filter(|record|record.kind!=SessionKind::Break){
                    let key=if record.subject.is_empty(){"No subject".into()}else{record.subject.clone()};
                    *subjects.entry(key).or_default()+=record.elapsed(record.finished.unwrap_or(record.started));
                }
                subjects.into_iter().map(|(subject,ms)|view!{<p>{format!("{subject} · {}",readable_duration(ms))}</p>}).collect::<Vec<_>>()
            })}
            {move ||state.vault.with(|v|v.study.history.iter().rev().take(25).map(|record|{
                let time=js_sys::Date::new(&(record.started as f64).into()).to_locale_string("default",&js_sys::Object::new()).as_string().unwrap_or_default();
                view!{<p class="notes-session">{format!("{} · {} · {} · {}",record.subject,record.kind.label(),time,readable_duration(record.elapsed(record.finished.unwrap_or(record.started))))}</p>}
            }).collect::<Vec<_>>())}
        </Show>
    }
}

#[component]
fn SessionLinks(
    state: NotesState,
    task: RwSignal<Option<Id>>,
    topic: RwSignal<Option<Id>>,
    document: RwSignal<Option<Id>>,
    subject: RwSignal<String>,
) -> impl IntoView {
    let (tasks, topics, books) = state.vault.with_untracked(|v| {
        (
            v.planner
                .tasks
                .iter()
                .map(|t| (t.id, t.title.clone()))
                .collect::<Vec<_>>(),
            v.planner
                .topics
                .iter()
                .map(|t| (t.id, format!("{} · {}", t.exam.label(), t.title)))
                .collect::<Vec<_>>(),
            v.books
                .iter()
                .filter(|b| b.disposition == ellipse_core::notes::Disposition::Active)
                .map(|b| (b.id, b.title.clone()))
                .collect::<Vec<_>>(),
        )
    });
    let value = |id: Option<Id>| id.map(|id| id.to_string()).unwrap_or_default();
    view! {
        <details class="focus-links" open=task.get_untracked().is_some()||topic.get_untracked().is_some()>
            <summary>"Link this session"</summary>
            <div class="planner-form-grid">
                <label>"Linked task"<select aria-label="Linked task" prop:value=move ||value(task.get())
                    on:change=move |e|{
                        let selected=event_target_value(&e).parse().ok();
                        task.set(selected);
                        state.vault.with_untracked(|vault|{
                            if let Some(entry)=selected.and_then(|id|vault.planner.tasks.iter().find(|task|task.id==id)){
                                document.set(entry.book);topic.set(entry.topic);subject.set(entry.subject.clone());
                            }
                        });
                    }>
                    <option value="">"None"</option>
                    {tasks.into_iter().map(|(id,name)|view!{<option value=id>{name}</option>}).collect::<Vec<_>>()}
                </select></label>
                <label>"Linked topic"<select aria-label="Linked topic" prop:value=move ||value(topic.get())
                    on:change=move |e|{
                        let selected=event_target_value(&e).parse().ok();
                        topic.set(selected);
                        state.vault.with_untracked(|vault|{
                            if let Some(entry)=selected.and_then(|id|vault.planner.topics.iter().find(|topic|topic.id==id)){
                                subject.set(entry.subject.label().into());
                            }
                        });
                    }>
                    <option value="">"None"</option>
                    {topics.into_iter().map(|(id,name)|view!{<option value=id>{name}</option>}).collect::<Vec<_>>()}
                </select></label>
                {state.local.then(move ||view!{<label>"Linked notebook or PDF"<select aria-label="Linked notebook or PDF"
                    prop:value=move ||value(document.get()) on:change=move |e|document.set(event_target_value(&e).parse().ok())>
                    <option value="">"None"</option>
                    {books.into_iter().map(|(id,name)|view!{<option value=id>{name}</option>}).collect::<Vec<_>>()}
                </select></label>})}
            </div>
        </details>
    }
}
