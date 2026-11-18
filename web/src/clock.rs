use crate::settings::Workspace;
use ellipse_core::home::{Alignment, DateStyle};
use leptos::{leptos_dom::helpers::set_interval_with_handle, prelude::*};
use std::time::Duration;

pub fn device_time() -> RwSignal<i64> {
    let seconds = || (js_sys::Date::now() / 1000.0) as i64;
    let now = RwSignal::new(seconds());
    if let Ok(timer) = set_interval_with_handle(
        move || {
            let next = seconds();
            if now.get_untracked() != next {
                now.set(next);
            }
        },
        Duration::from_millis(250),
    ) {
        on_cleanup(move || timer.clear());
    }
    now
}

#[component]
pub fn ClockFace(
    state: Workspace,
    now: RwSignal<i64>,
    #[prop(default = false)] compact: bool,
) -> impl IntoView {
    let fields = Memo::new(move |_| {
        let date = js_sys::Date::new(&((now.get() as f64 * 1000.0).into()));
        (
            date.get_hours(),
            date.get_minutes(),
            state
                .preferences
                .with(|p| p.appearance.clock.seconds)
                .then(|| date.get_seconds()),
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date(),
        )
    });
    let time = Memo::new(move |_| {
        let (hour, minute, second, ..) = fields.get();
        state
            .preferences
            .with(|p| p.appearance.clock.time(hour, minute, second.unwrap_or(0)))
    });
    let calendar = Memo::new(move |_| {
        let (_, _, _, year, month, day) = fields.get();
        (year, month, day)
    });
    let date = Memo::new(move |_| {
        let (year, month, day) = calendar.get();
        let noon = js_sys::Date::new_with_year_month_day(year, (month - 1) as i32, day as i32);
        state.preferences.with(|p| {
            let clock = &p.appearance.clock;
            if !clock.show_date {
                return None;
            }
            let options = js_sys::Object::new();
            let set = |key: &str, value: &str| {
                let _ = js_sys::Reflect::set(&options, &key.into(), &value.into());
            };
            match clock.date {
                DateStyle::Long => {
                    set("weekday", "long");
                    set("month", "long");
                }
                DateStyle::Short => {
                    set("weekday", "short");
                    set("month", "short");
                }
                DateStyle::Numeric => {
                    set("year", "numeric");
                    set("month", "2-digit");
                }
            }
            set("day", "numeric");
            let locales = js_sys::Array::new();
            Some(
                js_sys::Intl::DateTimeFormat::new(&locales, &options)
                    .format()
                    .call1(&wasm_bindgen::JsValue::UNDEFINED, &noon)
                    .ok()
                    .and_then(|date| date.as_string())
                    .unwrap_or_default(),
            )
        })
    });
    view! {
        <div class:clock-face=true class:clock-compact=compact
            class:clock-card-hidden=move || state.preferences.with(|p| !p.appearance.clock.card)
            style=move || state.preferences.with(|p| {
                let c = &p.appearance.clock;
                let position = |align| match align {
                    Alignment::Start => "start", Alignment::Center => "center", Alignment::End => "end",
                };
                format!("--clock-ink:{};--clock-opacity:{}%;--clock-blur:{}px;--clock-size:{};--clock-weight:{};--clock-x:{};--clock-y:{}",
                    c.color, c.opacity, c.blur, f64::from(c.size) / 100.0, c.weight, position(c.horizontal), position(c.vertical))
            })>
            <div class="clock-appearance" aria-hidden="true"></div>
            <div class="clock-reading">
                <time class="clock-digits" aria-label="Local time"
                    data-font=move || state.preferences.with(|p| p.appearance.clock.font.css())
                    class:clock-seconds=move || state.preferences.with(|p| p.appearance.clock.seconds)
                    class:clock-wide=move || time.with(|parts| parts.0.len() == 5)
                    datetime=move || {
                        let (hour, minute, second, ..) = fields.get();
                        if let Some(second) = second {format!("{hour:02}:{minute:02}:{second:02}")} else {format!("{hour:02}:{minute:02}")}
                    }>
                    {move || time.with(|parts| parts.0.clone())}
                </time>
                <div class="clock-meta">
                    {move || time.with(|parts| parts.1).map(|period| view! { <span class="clock-period">{period}</span> })}
                    {move || date.get().map(|date| view! { <span class="clock-date">{date}</span> })}
                </div>
            </div>
        </div>
    }
}
