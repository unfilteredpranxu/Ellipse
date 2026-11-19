use super::{date_at, localized, readable_duration, shift, today};
use crate::notes::{NotesState, Section, now};
use ellipse_core::notes::{
    Disposition, SessionKind, label,
    planner::{Day, Priority, Repeat, Task},
};
use leptos::prelude::*;

#[derive(Clone, Copy, Eq, PartialEq)]
enum Scale {
    Day,
    Week,
    Month,
}

impl Scale {
    fn label(self) -> &'static str {
        match self {
            Self::Day => "Day",
            Self::Week => "Week",
            Self::Month => "Month",
        }
    }
}

#[component]
pub(super) fn Calendar(state: NotesState, section: RwSignal<Option<Section>>) -> impl IntoView {
    let selected = RwSignal::new(today());
    let scale = RwSignal::new(Scale::Month);
    let draft = RwSignal::new(None::<Task>);
    let notice = RwSignal::new(String::new());
    let direction = move |step: i32| {
        let day = selected.get_untracked();
        let target = match scale.get_untracked() {
            Scale::Day => shift(day, step),
            Scale::Week => shift(day, step * 7),
            Scale::Month => {
                let first = Day { day: 1, ..day };
                if step > 0 {
                    shift(first, i32::from(first.month_days()))
                } else {
                    Day {
                        day: 1,
                        ..shift(first, -1)
                    }
                }
            }
        };
        selected.set(target);
    };
    let scheduled = Memo::new(move |_| {
        let day = selected.get();
        let mut scheduled = state.vault.with(|v| {
            v.planner
                .tasks
                .iter()
                .filter(|task| task.occurs(day))
                .cloned()
                .collect::<Vec<_>>()
        });
        scheduled.sort_by_key(|task| {
            (
                task.time.unwrap_or(u16::MAX),
                task.priority != Priority::High,
                task.id,
            )
        });
        scheduled
    });
    let save = Callback::new(move |task: Task| {
        let day = task.day;
        match state.edit_productivity(move |vault| vault.planner.save_task(task).map(|_| ())) {
            Ok(()) => {
                selected.set(day);
                draft.set(None);
                notice.set(String::new());
            }
            Err(error) => notice.set(error),
        }
    });
    view! {
        <section class="planner-content">
            <div class="planner-title">
                <h1>"Calendar"</h1>
                <button class="notes-primary" type="button"
                    on:click=move |_|draft.set(Some(Task::new(selected.get_untracked(),"",now())))>
                    "Add task"
                </button>
            </div>
            <div class="planner-datebar">
                <div class="planner-controls" aria-label="Calendar view">
                    {[Scale::Day,Scale::Week,Scale::Month].map(|mode|view!{
                        <button type="button" aria-pressed=move ||(scale.get()==mode).to_string()
                            on:click=move |_|scale.set(mode)>{mode.label()}</button>
                    })}
                </div>
                <div class="planner-controls">
                    <button type="button" aria-label="Previous period" on:click=move |_|direction(-1)>"←"</button>
                    <button type="button" on:click=move |_|selected.set(today())>"Today"</button>
                    <button type="button" aria-label="Next period" on:click=move |_|direction(1)>"→"</button>
                    <input type="date" aria-label="Calendar date" prop:value=move ||selected.get().to_string()
                        on:change=move |event|if let Ok(day)=event_target_value(&event).parse(){selected.set(day);}/>
                </div>
            </div>
            <div class="planner-calendar-layout">
                <Show when=move ||scale.get()!=Scale::Day>
                    <section class="planner-month" aria-label="Calendar grid">
                        <h2>{move ||{
                            let day=selected.get();
                            let month=localized(&date_at(day,0),"month","long");
                            format!("{month} {}",day.year)
                        }}</h2>
                        <div class="planner-grid">
                            {(0..7).map(|weekday| {
                                let monday=Day {year:2024,month:1,day:1};
                                view!{<span class="planner-weekday">{localized(&date_at(monday,weekday),"weekday","short")}</span>}
                            }).collect::<Vec<_>>()}
                            {move || {
                                let day=selected.get();
                                let (anchor,count)=if scale.get()==Scale::Month {
                                    let first=Day {day:1,..day};
                                    let before=i32::from(first.weekday());
                                    (shift(first,-before), ((before+i32::from(first.month_days())+6)/7)*7)
                                } else {(shift(day,-i32::from(day.weekday())),7)};
                                (0..count).map(|offset|{
                                    let cell=shift(anchor,offset);
                                    let (total,has_exam)=state.vault.with(|v|(
                                        v.planner.tasks.iter().filter(|task|task.occurs(cell)).count(),
                                        v.planner.custom_exams_on(cell).next().is_some(),
                                    ));
                                    view!{<button type="button" class:other-month=cell.month!=day.month
                                        aria-label=cell.to_string() aria-pressed=(cell==day).to_string()
                                        on:click=move |_|selected.set(cell)>
                                        <span>{cell.day}</span>
                                        { (total>0).then(||view!{<small aria-label=format!("{total} tasks")>{total}</small>}) }
                                        {has_exam.then(||view!{<span class="planner-exam-dot" aria-label="Custom exam date"></span>})}
                                    </button>}
                                }).collect::<Vec<_>>()
                            }}
                        </div>
                    </section>
                </Show>
                <section class="planner-agenda" aria-label="Selected day">
                    <h2>{move || localized(&date_at(selected.get(),0),"dateStyle","full")}</h2>
                    <ul class="planner-exam-list">
                        {move ||{
                            let day=selected.get();
                            state.vault.with(|vault|vault.planner.custom_exams_on(day)
                                .map(|exam|view!{<li>{format!("{} · Custom date",exam.label())}</li>})
                                .collect::<Vec<_>>())
                        }}
                    </ul>
                    <p>{move ||{
                        let key=selected.get().to_string();
                        scheduled.with(|tasks|format!("{} of {} tasks complete",
                            tasks.iter().filter(|task|task.completions.contains_key(&key)).count(),tasks.len()))
                    }}</p>
                    <div class="planner-tasks">
                        {move || scheduled.get().into_iter().map(|task|view!{
                            <TaskRow state task day=selected.get() draft notice section/>
                        }).collect::<Vec<_>>()}
                        <Show when=move ||scheduled.with(Vec::is_empty)><p>"No tasks scheduled."</p></Show>
                    </div>
                    <h3>"Recorded sessions"</h3>
                    {move ||{
                        let day=selected.get();
                        let start=date_at(day,0).get_time() as i64;
                        let end=date_at(day,1).get_time() as i64;
                        let rows=state.vault.with(|vault|vault.study.history.iter().filter_map(|entry|{
                            let ms=entry.milliseconds_in(start,end);
                            (ms>0).then(|| (entry.subject.clone(),entry.kind,ms))
                        }).collect::<Vec<_>>());
                        if rows.is_empty(){return view!{<p>"No sessions recorded for this date."</p>}.into_any();}
                        let total=rows.iter().filter(|(_,kind,_)|*kind!=SessionKind::Break).map(|(_,_,ms)|*ms).sum();
                        view!{
                            <p>{format!("{} focused",readable_duration(total))}</p>
                            <ul>{rows.into_iter().map(|(subject,kind,ms)|view!{
                                <li>{format!("{} · {} · {}",if subject.is_empty(){"Independent".into()}else{subject},kind.label(),readable_duration(ms))}</li>
                            }).collect::<Vec<_>>()}</ul>
                        }.into_any()
                    }}
                </section>
            </div>
            {move ||{
                let current=today();
                let overdue=state.vault.with(|vault|vault.planner.tasks.iter()
                    .filter(|task|task.overdue(current)).map(|task|(task.day,task.title.clone())).collect::<Vec<_>>());
                (!overdue.is_empty()).then(||view!{
                    <details class="planner-overdue">
                        <summary>{format!("Overdue one-off tasks ({})",overdue.len())}</summary>
                        <ul>{overdue.into_iter().map(|(day,title)|view!{
                            <li><button type="button" on:click=move |_|selected.set(day)>
                                {format!("{title} · {day}")}
                            </button></li>
                        }).collect::<Vec<_>>()}</ul>
                    </details>
                })
            }}
            <p role="status">{move ||notice.get()}</p>
            {move ||draft.get().map(|task|view!{
                <TaskForm state task save cancel=Callback::new(move |()|draft.set(None))/>
            })}
        </section>
    }
}

#[component]
fn TaskRow(
    state: NotesState,
    task: Task,
    day: Day,
    draft: RwSignal<Option<Task>>,
    notice: RwSignal<String>,
    section: RwSignal<Option<Section>>,
) -> impl IntoView {
    let id = task.id;
    let record = StoredValue::new(task.clone());
    let done = task.completions.contains_key(&day.to_string());
    let repeating = task.repeat != Repeat::Once;
    let book = task.book.filter(|id| {
        state.local
            && state.vault.with_untracked(|v| {
                v.book(*id)
                    .is_some_and(|book| book.disposition == Disposition::Active)
            })
    });
    let subtitle = [
        task.time
            .map(|value| format!("{:02}:{:02}", value / 60, value % 60)),
        (!task.subject.is_empty()).then_some(task.subject),
        task.estimate.map(|value| format!("{value} min")),
        (task.priority == Priority::High).then_some("High priority".into()),
        repeating.then_some(format!("{:?}", task.repeat)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    view! {
        <article class="planner-task">
            <label class="planner-check">
                <input type="checkbox" aria-label=format!("Complete {}",task.title) prop:checked=move || done
                    on:change=move |event|{
                        let done=event_target_checked(&event);
                        let error=state.edit_productivity(|vault|{
                            let task=vault.planner.tasks.iter_mut().find(|task|task.id==id).ok_or("The task no longer exists.")?;
                            task.set_done(day,done,now());
                            Ok(())
                        }).err();
                        if let Some(error)=error {
                            event_target::<web_sys::HtmlInputElement>(&event).set_checked(!done);
                            notice.set(error);
                        }
                    }/>
                <strong>{task.title}</strong>
            </label>
            {(!subtitle.is_empty()).then(||view!{<p class="planner-task-meta">{subtitle}</p>})}
            {(!task.details.is_empty()).then(||view!{<p>{task.details}</p>})}
            <ul class="planner-checklist">
                {task.checklist.items.iter().map(|item|{
                    let item_id=item.id;
                    let checked=task.checklist.checked(day,item_id);
                    let title=item.title.clone();
                    view!{<li><label class="planner-check">
                        <input type="checkbox" aria-label=format!("Checklist item {title}") prop:checked=move ||checked
                            on:change=move |event|{
                                let checked=event_target_checked(&event);
                                let result=state.edit_productivity(|vault|vault.planner.tasks.iter_mut()
                                    .find(|task|task.id==id).ok_or("The task no longer exists.")?
                                    .set_checklist_item(day,item_id,checked));
                                if let Err(error)=result {
                                    event_target::<web_sys::HtmlInputElement>(&event).set_checked(!checked);
                                    notice.set(error);
                                }
                            }/>
                        {title}
                    </label></li>}
                }).collect::<Vec<_>>()}
            </ul>
            <div class="planner-controls">
                <button type="button" on:click=move |_|draft.set(Some(record.get_value()))>"Edit task"</button>
                <button type="button" on:click=move |_|{
                    draft.set(Some(record.with_value(|task|task.duplicate_for(day,now()))));
                }>"Duplicate"</button>
                <button type="button" on:click=move |_|{
                    state.focus_task.set(Some(id));state.focus_topic.set(None);section.set(Some(Section::Focus));
                }>"Focus on task"</button>
                {book.map(|book|view!{<button type="button" on:click=move |_|{
                    state.open(book);section.set(Some(Section::Notes));
                }>"Open notebook"</button>})}
                {repeating.then(||view!{<button type="button" on:click=move |_|{
                    let error=state.edit_productivity(|vault|{
                        let task=vault.planner.tasks.iter_mut().find(|task|task.id==id).ok_or("The task no longer exists.")?;
                        task.skip(day);Ok(())
                    }).err();
                    notice.set(error.unwrap_or_default());
                }>"Skip this occurrence"</button>})}
                <button type="button" on:click=move |_|{
                    let prompt=if repeating{"Delete this task series and its occurrence history?"}else{"Delete this task?"};
                    if window().confirm_with_message(prompt).unwrap_or(false) {
                        let error=state.edit_productivity(|vault|{vault.planner.tasks.retain(|task|task.id!=id);Ok(())}).err();
                        notice.set(error.unwrap_or_default());
                    }
                }>{if repeating{"Delete series"}else{"Delete task"}}</button>
            </div>
        </article>
    }
}

#[component]
fn TaskForm(
    state: NotesState,
    task: Task,
    save: Callback<Task>,
    cancel: Callback<()>,
) -> impl IntoView {
    let new_item = RwSignal::new(String::new());
    let item_status = RwSignal::new(String::new());
    let draft = RwSignal::new(task);
    let repeat = draft.with_untracked(|t| format!("{:?}", t.repeat));
    let priority = draft.with_untracked(|t| format!("{:?}", t.priority));
    let books = state.vault.with_untracked(|v| {
        v.books
            .iter()
            .filter(|b| b.disposition == Disposition::Active)
            .map(|b| (b.id, b.title.clone()))
            .collect::<Vec<_>>()
    });
    let topics = state.vault.with_untracked(|v| {
        v.planner
            .topics
            .iter()
            .map(|t| (t.id, format!("{} · {}", t.exam.label(), t.title)))
            .collect::<Vec<_>>()
    });
    view! {
        <form class="planner-form" aria-label="Task details" on:submit=move |e|{
            e.prevent_default(); save.run(draft.get_untracked());
        }>
            <h2>{move ||if draft.with(|t|t.id==0){"New task"}else{"Edit task"}}</h2>
            <label>"Task title"<input required maxlength=120 prop:value=move ||draft.with(|t|t.title.clone())
                on:input=move |e|draft.update(|t|t.title=label(&event_target_value(&e)))/></label>
            <label>"Details"<textarea maxlength=950 prop:value=move ||draft.with(|t|t.details.clone())
                on:input=move |e|draft.update(|t|t.details=event_target_value(&e))></textarea></label>
            <div class="planner-form-grid">
                <label>"Task date"<input type="date" required prop:value=move ||draft.with(|t|t.day.to_string())
                    min="1900-01-01" max="2200-12-31" on:change=move |e|{
                        if let Ok(day)=event_target_value(&e).parse(){draft.update(|t|t.day=day);}
                    }/></label>
                <label>"Time"<input type="time" prop:value=move ||draft.with(|t|t.time.map(|v|format!("{:02}:{:02}",v/60,v%60)).unwrap_or_default())
                    on:change=move |e|{
                        let value=event_target_value(&e);
                        let time=value.split_once(':').and_then(|(h,m)|Some(h.parse::<u16>().ok()?*60+m.parse::<u16>().ok()?));
                        draft.update(|t|t.time=time);
                    }/></label>
                <label>"Subject"<input maxlength=120 prop:value=move ||draft.with(|t|t.subject.clone())
                    on:input=move |e|draft.update(|t|t.subject=label(&event_target_value(&e)))/></label>
                <label>"Estimated minutes"<input type="number" min=1 max=1440 prop:value=move ||draft.with(|t|t.estimate.map(|n|n.to_string()).unwrap_or_default())
                    on:input=move |e|draft.update(|t|t.estimate=event_target_value(&e).parse().ok())/></label>
                <label>"Priority"<select prop:value=priority on:change=move |e|draft.update(|t|t.priority=match event_target_value(&e).as_str(){
                    "High"=>Priority::High,"Low"=>Priority::Low,_=>Priority::Normal,
                })>
                    <option>"Normal"</option><option>"High"</option><option>"Low"</option>
                </select></label>
                <label>"Repeat"<select prop:value=repeat on:change=move |e|draft.update(|t|t.repeat=match event_target_value(&e).as_str(){
                    "Daily"=>Repeat::Daily,"Weekly"=>Repeat::Weekly,_=>Repeat::Once,
                })>
                    <option value="Once">"Does not repeat"</option><option>"Daily"</option><option>"Weekly"</option>
                </select></label>
                {state.local.then(move ||view!{<label>"Notebook or PDF"<select aria-label="Notebook or PDF"
                    prop:value=move ||draft.with(|t|t.book.map(|id|id.to_string()).unwrap_or_default())
                    on:change=move |e|draft.update(|t|t.book=event_target_value(&e).parse().ok())>
                    <option value="">"None"</option>
                    {books.into_iter().map(|(id,title)|view!{<option value=id>{title}</option>}).collect::<Vec<_>>()}
                </select></label>})}
                <label>"Personal topic"<select aria-label="Personal topic"
                    prop:value=move ||draft.with(|t|t.topic.map(|id|id.to_string()).unwrap_or_default())
                    on:change=move |e|draft.update(|t|t.topic=event_target_value(&e).parse().ok())>
                    <option value="">"None"</option>
                    {topics.into_iter().map(|(id,title)|view!{<option value=id>{title}</option>}).collect::<Vec<_>>()}
                </select></label>
            </div>
            <fieldset class="planner-checklist-editor">
                <legend>"Checklist"</legend>
                <ul class="planner-checklist">
                    {move ||draft.with(|task|task.checklist.items.clone()).into_iter().map(|item|{
                        let id=item.id;
                        let edit_label=format!("Edit checklist item {}",item.title);
                        view!{<li class="planner-controls">
                            <input aria-label=edit_label maxlength=120 prop:value=item.title on:change=move |e|{
                                let title=label(&event_target_value(&e));
                                if !title.is_empty(){
                                    draft.update(|task|if let Some(item)=task.checklist.items.iter_mut().find(|item|item.id==id){item.title=title;});
                                }
                            }/>
                            <button type="button" on:click=move |_|draft.update(|task|task.checklist.remove(id))>"Remove item"</button>
                        </li>}
                    }).collect::<Vec<_>>()}
                </ul>
                <div class="planner-controls">
                    <label>"New checklist item"<input maxlength=120 prop:value=move ||new_item.get()
                        on:input=move |e|new_item.set(event_target_value(&e))
                        on:keydown=move |e:web_sys::KeyboardEvent|if e.key()=="Enter"{e.prevent_default();}/></label>
                    <button type="button" on:click=move |_|{
                        let result=draft.try_update(|task|task.checklist.add(&new_item.get_untracked()));
                        if result==Some(Ok(())){new_item.set(String::new());}
                        item_status.set(result.and_then(Result::err).unwrap_or_default().into());
                    }>"Add checklist item"</button>
                </div>
                <p role="status">{move ||item_status.get()}</p>
            </fieldset>
            <div class="planner-controls">
                <button class="notes-primary" type="submit">"Save task"</button>
                <button type="button" on:click=move |_|cancel.run(())>"Cancel"</button>
            </div>
        </form>
    }
}
