use super::{date_at, readable_duration, shift, today};
use crate::notes::{NotesState, Section, now};
use ellipse_core::notes::{
    SessionKind, label,
    planner::{Exam, Profile, Progress, Task, Topic, Track},
};
use leptos::prelude::*;
use std::collections::BTreeMap;

#[component]
pub(super) fn Syllabus(state: NotesState, section: RwSignal<Option<Section>>) -> impl IntoView {
    let profile = Memo::new(move |_| state.vault.with(|v| v.planner.profile));
    let edit = RwSignal::new(false);
    let notice = RwSignal::new(String::new());
    view! {
        <section class="planner-content study-page">
            <div class="planner-title">
                <h1>"Study"</h1>
                <Show when=move ||profile.with(Option::is_some)>
                    <button type="button" on:click=move |_|edit.update(|open|*open = !*open)>
                        {move ||if edit.get(){"Close track settings"}else{"Change exam track"}}
                    </button>
                </Show>
            </div>
            <Show when=move ||profile.with(Option::is_none)||edit.get()>
                {move || view!{<TrackForm state existing=profile.get() edit notice/>}}
            </Show>
            {move || profile.get().map(|profile|view!{
                <ExamTopics state profile section notice/>
            })}
            <StudyAnalytics state notice/>
            <p role="status">{move ||notice.get()}</p>
        </section>
    }
}

#[component]
fn TrackForm(
    state: NotesState,
    existing: Option<Profile>,
    edit: RwSignal<bool>,
    notice: RwSignal<String>,
) -> impl IntoView {
    let now = today();
    let track = RwSignal::new(existing.map(|value| value.track));
    let year =
        RwSignal::new(existing.map_or(now.year + u16::from(now.month >= 8), |value| value.year));
    let valid = RwSignal::new(true);
    view! {
        <form class="planner-form" aria-label="Exam track settings" on:submit=move |e|{
            e.prevent_default();
            let Some(track)=track.get_untracked() else{return};
            let year=year.get_untracked();
            if !valid.get_untracked(){return;}
            let result=state.edit_productivity(|vault|{vault.planner.profile=Some(Profile{track,year});Ok(())});
            if result.is_ok(){edit.set(false);}
            notice.set(result.err().unwrap_or_default());
        }>
            <h2>"Exam track"</h2>
            <fieldset class="planner-controls">
                <legend>"Choose one track"</legend>
                {[(Track::Main,"JEE Main only"),(Track::Both,"JEE Main + Advanced"),(Track::Neet,"NEET UG")]
                    .map(|(selection,title)|view!{<label class="planner-check">
                        <input type="radio" name="exam-track" required prop:checked=move ||track.get()==Some(selection)
                            on:change=move |_|track.set(Some(selection))/>
                        {title}
                    </label>})}
            </fieldset>
            <label>"Target year"<input type="number" min=2020 max=2200 required prop:value=move ||year.get().to_string()
                on:input=move |e|{
                    let parsed=event_target_value(&e).parse::<u16>().ok().filter(|year|(2020..=2200).contains(year));
                    valid.set(parsed.is_some());
                    if let Some(value)=parsed{year.set(value);}
                }/></label>
            <button type="submit" class="notes-primary" disabled=move ||track.with(Option::is_none)||!valid.get()>"Save track"</button>
            <p>"Changing tracks keeps your topics, tasks and recorded sessions."</p>
        </form>
    }
}

fn source(exam: Exam) -> &'static str {
    match exam {
        Exam::Main => "https://jeemain.nta.nic.in/",
        Exam::Advanced => "https://jeeadv.ac.in/",
        Exam::Neet => "https://neet.nta.nic.in/",
    }
}

#[component]
fn ExamTopics(
    state: NotesState,
    profile: Profile,
    section: RwSignal<Option<Section>>,
    notice: RwSignal<String>,
) -> impl IntoView {
    let exam = RwSignal::new(profile.track.exams()[0]);
    let subject = RwSignal::new(exam.get_untracked().subjects()[0]);
    let unit = RwSignal::new(String::new());
    let title = RwSignal::new(String::new());
    let active_topics = Memo::new(move |_| {
        let exam = exam.get();
        let subject = subject.get();
        state.vault.with(|vault| {
            vault
                .planner
                .topics
                .iter()
                .filter(|topic| topic.exam == exam && topic.subject == subject)
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    view! {
        <section class="study-topics" aria-label="Personal syllabus">
            <h2>"Syllabus"</h2>
            <div class="exam-source-list">
                {profile.track.exams().iter().copied().map(|exam|view!{
                    <article>
                        <strong>{format!("{} {}",exam.label(),profile.year)}</strong>
                        <p>"Official date unavailable"</p>
                        <a href=source(exam) target="_blank" rel="noopener noreferrer">"Official exam site"</a>
                    </article>
                }).collect::<Vec<_>>()}
            </div>
            <p>"Official syllabus documents could not be verified. Topics added here are personal."</p>
            <div class="planner-controls" aria-label="Exam">
                {profile.track.exams().iter().copied().map(|value|view!{
                    <button type="button" aria-pressed=move ||(exam.get()==value).to_string() on:click=move |_|{
                        exam.set(value);subject.set(value.subjects()[0]);
                    }>{value.label()}</button>
                }).collect::<Vec<_>>()}
            </div>
            <div class="planner-controls" aria-label="Subject">
                {move ||exam.get().subjects().map(|value|view!{
                    <button type="button" aria-pressed=move ||(subject.get()==value).to_string()
                        on:click=move |_|subject.set(value)>{value.label()}</button>
                })}
            </div>
            <p>{move ||active_topics.with(|topics|format!("{} of {} personal topics complete",
                topics.iter().filter(|topic|topic.progress==Progress::Completed).count(), topics.len()))}</p>
            <form class="planner-form" aria-label="Personal topic details" on:submit=move |e|{
                e.prevent_default();
                let topic=Topic { id:0,exam:exam.get_untracked(),subject:subject.get_untracked(),
                    unit:label(&unit.get_untracked()),title:label(&title.get_untracked()),progress:Progress::NotStarted,weak:false,changed:now() };
                let result=state.edit_productivity(|vault|vault.planner.add_topic(topic));
                if result.is_ok(){title.set(String::new());}
                notice.set(result.err().unwrap_or_default());
            }>
                <div class="planner-form-grid">
                    <label>"Unit"<input required maxlength=120 prop:value=move ||unit.get()
                        on:input=move |e|unit.set(event_target_value(&e))/></label>
                    <label>"Topic"<input required maxlength=120 prop:value=move ||title.get()
                        on:input=move |e|title.set(event_target_value(&e))/></label>
                </div>
                <button type="submit" class="notes-primary">"Add personal topic"</button>
            </form>
            {move || {
                let mut groups=BTreeMap::<String,Vec<Topic>>::new();
                for topic in active_topics.get(){
                    groups.entry(topic.unit.clone()).or_default().push(topic);
                }
                groups.into_iter().map(|(unit,topics)|{
                    let complete=topics.iter().filter(|topic|topic.progress==Progress::Completed).count();
                    view!{<section class="study-unit">
                        <div class="planner-title"><h3>{unit}</h3><span>{format!("{complete} / {}",topics.len())}</span></div>
                        {topics.into_iter().map(|topic|view!{<TopicRow state topic section notice/>}).collect::<Vec<_>>()}
                    </section>}
                }).collect::<Vec<_>>()
            }}
        </section>
    }
}

#[component]
fn TopicRow(
    state: NotesState,
    topic: Topic,
    section: RwSignal<Option<Section>>,
    notice: RwSignal<String>,
) -> impl IntoView {
    let id = topic.id;
    let title = StoredValue::new(topic.title);
    let update = move |progress: Option<Progress>, weak: Option<bool>| {
        let result = state.edit_productivity(|vault| {
            let target = vault
                .planner
                .topics
                .iter_mut()
                .find(|topic| topic.id == id)
                .ok_or("The topic no longer exists.")?;
            if let Some(progress) = progress {
                target.progress = progress;
            }
            if let Some(weak) = weak {
                target.weak = weak;
            }
            target.changed = now();
            Ok(())
        });
        notice.set(result.err().unwrap_or_default());
    };
    view! {
        <article class="planner-task">
            <h4>{title.get_value()}</h4>
            <div class="planner-controls">
                <label>"Progress"<select aria-label=format!("Progress for {}",title.get_value()) prop:value=topic.progress.label()
                    on:change=move |e|update(Progress::ALL.into_iter().find(|value|value.label()==event_target_value(&e)),None)>
                    {Progress::ALL.map(|progress|view!{<option value=progress.label()>{progress.label()}</option>})}
                </select></label>
                <label class="planner-check"><input type="checkbox" prop:checked=move || topic.weak
                    on:change=move |e|update(None,Some(event_target_checked(&e))) />"Weak topic"</label>
                <button type="button" on:click=move |_|{
                    let mut task=Task::new(today(), &format!("Review {}",title.get_value()),now());
                    task.topic=Some(id);
                    task.subject=topic.subject.label().into();
                    let result=state.edit_productivity(|vault|vault.planner.save_task(task).map(|_|()));
                    if result.is_ok(){section.set(Some(Section::Calendar));}
                    else{notice.set(result.err().unwrap_or_default());}
                }>"Plan revision today"</button>
                <button type="button" on:click=move |_|{
                    state.focus_task.set(None);state.focus_topic.set(Some(id));section.set(Some(Section::Focus));
                }>"Focus on topic"</button>
            </div>
        </article>
    }
}

#[component]
fn StudyAnalytics(state: NotesState, notice: RwSignal<String>) -> impl IntoView {
    let goal = RwSignal::new(state.vault.with_untracked(|v| {
        v.planner
            .weekly_goal_minutes
            .map(|n| n.to_string())
            .unwrap_or_default()
    }));
    let today = today();
    let week = shift(today, -i32::from(today.weekday()));
    let weekly = Memo::new(move |_| {
        let from = date_at(week, 0).get_time() as i64;
        let end = date_at(week, 7).get_time() as i64;
        state.vault.with(|vault| {
            let mut focus = 0;
            let mut breaks = 0;
            let mut subjects = BTreeMap::<String, i64>::new();
            let mut days = [0i64; 7];
            for record in &vault.study.history {
                let ms = record.milliseconds_in(from, end);
                if record.kind == SessionKind::Break {
                    breaks += ms;
                    continue;
                }
                if ms == 0 {
                    continue;
                }
                focus += ms;
                *subjects
                    .entry(if record.subject.is_empty() {
                        "Independent".into()
                    } else {
                        record.subject.clone()
                    })
                    .or_default() += ms;
                for (offset, total) in days.iter_mut().enumerate() {
                    *total += record.milliseconds_in(
                        date_at(week, offset as i32).get_time() as i64,
                        date_at(week, offset as i32 + 1).get_time() as i64,
                    );
                }
            }
            (
                focus,
                breaks,
                subjects,
                days,
                vault.planner.weekly_goal_minutes,
            )
        })
    });
    view! {
        <section class="study-analytics">
            <h2>"This week"</h2>
            {move ||{
                let (focus,breaks,subjects,days,target)=weekly.get();
                view!{
                    <p>{format!("{} focused · {} in breaks",readable_duration(focus),readable_duration(breaks))}</p>
                    {target.map(|minutes|view!{<p>{format!("Focus goal · {minutes} min")}</p>})}
                    <dl>{days.into_iter().enumerate().map(|(offset,ms)|view!{
                        <div><dt>{super::localized(&date_at(week,offset as i32),"weekday","long")}</dt><dd>{readable_duration(ms)}</dd></div>
                    }).collect::<Vec<_>>()}</dl>
                    <h3>"By subject"</h3>
                    {subjects.is_empty().then(||view!{<p>"No completed focus sessions this week."</p>})}
                    <dl>{subjects.into_iter().map(|(name,ms)|view!{
                        <div><dt>{name}</dt><dd>{readable_duration(ms)}</dd></div>
                    }).collect::<Vec<_>>()}</dl>
                }
            }}
            <form class="planner-controls" on:submit=move |e|{
                e.prevent_default();
                let result=state.edit_productivity(|vault|{
                    vault.planner.weekly_goal_minutes=goal.with_untracked(|raw|
                        if raw.is_empty(){Ok(None)}else{raw.parse::<u16>().ok().filter(|n|(1..=10080).contains(n)).map(Some).ok_or("Enter a weekly goal from 1 to 10,080 minutes.")})?;
                    Ok(())
                });
                notice.set(result.err().unwrap_or_default());
            }>
                <label>"Weekly focus goal in minutes"<input type="number" min=1 max=10080 prop:value=move ||goal.get()
                    on:input=move |e|goal.set(event_target_value(&e))/></label>
                <button type="submit">"Save goal"</button>
            </form>
        </section>
    }
}
