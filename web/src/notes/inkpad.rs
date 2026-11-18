use ellipse_core::notes::{Page, Pen, Point, Stroke, outline};
use leptos::{
    ev, html,
    leptos_dom::helpers::{request_animation_frame, window_event_listener},
    prelude::*,
};
use std::collections::BTreeSet;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, PointerEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Ink,
    Whole,
    Partial,
    Lasso,
    SelectBox,
    Text,
    Line,
    Rectangle,
    Ellipse,
    Arrow,
    Pan,
}

impl Tool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ink => "Ink",
            Self::Whole => "Stroke eraser",
            Self::Partial => "Partial eraser",
            Self::Lasso => "Lasso",
            Self::SelectBox => "Rectangle selection",
            Self::Text => "Text",
            Self::Line => "Line",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Arrow => "Arrow",
            Self::Pan => "Pan",
        }
    }
}

pub struct Gesture {
    pub tool: Tool,
    pub stroke: Stroke,
}

struct Drawing {
    id: i32,
    tool: Tool,
    stroke: Stroke,
}

fn context(canvas: &HtmlCanvasElement) -> Option<CanvasRenderingContext2d> {
    use wasm_bindgen::JsCast;
    canvas.get_context("2d").ok()??.dyn_into().ok()
}

fn path(ctx: &CanvasRenderingContext2d, points: &[Point]) {
    if let Some(first) = points.first() {
        ctx.begin_path();
        ctx.move_to(f64::from(first.x), f64::from(first.y));
        for p in &points[1..] {
            ctx.line_to(f64::from(p.x), f64::from(p.y));
        }
    }
}

fn paint_stroke(ctx: &CanvasRenderingContext2d, stroke: &Stroke) {
    path(ctx, &outline(stroke));
    ctx.close_path();
    ctx.set_fill_style_str(&stroke.pen.color.hex());
    ctx.set_global_alpha(f64::from(stroke.pen.opacity));
    ctx.fill();
    ctx.set_global_alpha(1.);
}

fn paint_paper(ctx: &CanvasRenderingContext2d, page: &Page) {
    let paper = &page.paper;
    ctx.set_fill_style_str(&paper.background.hex());
    ctx.fill_rect(0., 0., f64::from(paper.width), f64::from(paper.height));
    ctx.set_fill_style_str(&paper.pattern.hex());
    paper.pattern_rectangles(|[x, y, width, height]| ctx.fill_rect(x, y, width, height));
}

fn setup(canvas: &HtmlCanvasElement, page: &Page) -> Option<CanvasRenderingContext2d> {
    let rect = canvas.get_bounding_client_rect();
    if rect.width() < 1. || rect.height() < 1. {
        return None;
    }
    let ratio = window().device_pixel_ratio().min(3.);
    let scale = (ratio * rect.width() / f64::from(page.paper.width))
        .min((4_000_000. / (f64::from(page.paper.width) * f64::from(page.paper.height))).sqrt());
    let width = (f64::from(page.paper.width) * scale).ceil() as u32;
    let height = (f64::from(page.paper.height) * scale).ceil() as u32;
    if canvas.width() != width {
        canvas.set_width(width);
    }
    if canvas.height() != height {
        canvas.set_height(height);
    }
    let ctx = context(canvas)?;
    let _ = ctx.set_transform(scale, 0., 0., scale, 0., 0.);
    ctx.clear_rect(
        0.,
        0.,
        f64::from(page.paper.width),
        f64::from(page.paper.height),
    );
    Some(ctx)
}

pub fn shape(stroke: &Stroke, tool: Tool) -> Stroke {
    let Some(&first) = stroke.points.first() else {
        return stroke.clone();
    };
    let last = *stroke.points.last().unwrap();
    let point = |x, y| Point { x, y, ..first };
    let (_, _, dx, dy) = (first.x, first.y, last.x - first.x, last.y - first.y);
    let points = match tool {
        Tool::Line => vec![first, last],
        Tool::Rectangle | Tool::SelectBox => vec![
            first,
            point(last.x, first.y),
            last,
            point(first.x, last.y),
            first,
        ],
        Tool::Ellipse => (0..=40)
            .map(|step| {
                let angle = std::f32::consts::TAU * step as f32 / 40.;
                point(
                    first.x + dx * 0.5 * (1. + angle.cos()),
                    first.y + dy * 0.5 * (1. + angle.sin()),
                )
            })
            .collect(),
        Tool::Arrow => {
            let angle = dy.atan2(dx);
            let length = dx.hypot(dy).clamp(1., 72.) * 0.30;
            let wing = |rotation: f32| {
                point(
                    last.x - length * (angle + rotation).cos(),
                    last.y - length * (angle + rotation).sin(),
                )
            };
            vec![first, last, wing(0.55), last, wing(-0.55)]
        }
        _ => return stroke.clone(),
    };
    Stroke {
        id: stroke.id,
        pen: stroke.pen,
        points,
    }
}

#[component]
pub fn InkPad(
    page: Memo<Page>,
    pen: Memo<Pen>,
    tool: RwSignal<Tool>,
    touch: RwSignal<bool>,
    selected: RwSignal<BTreeSet<u32>>,
    is_pdf: bool,
    editable: bool,
    commit: Callback<Gesture>,
    zoom: Memo<u16>,
) -> impl IntoView {
    let base = NodeRef::<html::Canvas>::new();
    let overlay = NodeRef::<html::Canvas>::new();
    let live = StoredValue::new(None::<Drawing>);
    let queued = StoredValue::new(false);
    let resize = RwSignal::new(0u32);
    let handle = window_event_listener(ev::resize, move |_| {
        resize.update(|n| *n = n.wrapping_add(1))
    });
    on_cleanup(move || handle.remove());
    let paint = move || {
        let Some(canvas) = overlay.try_get().flatten() else {
            return;
        };
        page.with_untracked(|page| {
            if let Some(ctx) = setup(&canvas, page) {
                live.with_value(|active| {
                    if let Some(active) = active {
                        if matches!(
                            active.tool,
                            Tool::Ink | Tool::Line | Tool::Rectangle | Tool::Ellipse | Tool::Arrow
                        ) {
                            paint_stroke(&ctx, &shape(&active.stroke, active.tool));
                        } else {
                            path(&ctx, &shape(&active.stroke, active.tool).points);
                            ctx.set_stroke_style_str("#496781");
                            ctx.set_line_width(
                                if matches!(active.tool, Tool::Lasso | Tool::SelectBox) {
                                    1.5
                                } else {
                                    7.
                                },
                            );
                            ctx.set_global_alpha(0.55);
                            ctx.stroke();
                            ctx.set_global_alpha(1.);
                        }
                    }
                });
            }
        });
    };
    let schedule = move || {
        if queued.get_value() {
            return;
        }
        queued.set_value(true);
        request_animation_frame(move || {
            if queued.try_set_value(false).is_none() {
                paint();
            }
        });
    };
    Effect::new(move |_| {
        resize.track();
        zoom.track();
        let Some(canvas) = base.get() else { return };
        page.with(|page| {
            if let Some(ctx) = setup(&canvas, page) {
                if !is_pdf {
                    paint_paper(&ctx, page);
                }
                selected.with(|selected| {
                    for stroke in &page.ink {
                        paint_stroke(&ctx, stroke);
                        if selected.contains(&stroke.id) {
                            path(&ctx, &outline(stroke));
                            ctx.close_path();
                            ctx.set_stroke_style_str("#376f95");
                            ctx.set_line_width(1.2);
                            ctx.stroke();
                        }
                    }
                });
                for text in &page.text {
                    ctx.set_font(&format!(
                        "{} {} {}px sans-serif",
                        if text.italic { "italic" } else { "normal" },
                        if text.bold { "bold" } else { "normal" },
                        text.size
                    ));
                    ctx.set_fill_style_str(&text.color.hex());
                    for (line, value) in text.value.lines().enumerate() {
                        let _ = ctx.fill_text(
                            value,
                            f64::from(text.at.x),
                            f64::from(text.at.y) + line as f64 * f64::from(text.size) * 1.24,
                        );
                    }
                }
            }
        });
    });
    let position = move |event: &PointerEvent| {
        let rect = overlay.get_untracked()?.get_bounding_client_rect();
        if rect.width() < 1. || rect.height() < 1. {
            return None;
        }
        Some(page.with_untracked(|page| {
            Point {
                x: (((f64::from(event.client_x()) - rect.left()) / rect.width()) as f32
                    * page.paper.width)
                    .clamp(0., page.paper.width),
                y: (((f64::from(event.client_y()) - rect.top()) / rect.height()) as f32
                    * page.paper.height)
                    .clamp(0., page.paper.height),
                pressure: if event.pointer_type() == "pen" {
                    event.pressure().clamp(0., 1.)
                } else {
                    0.5
                },
                tilt: ((event.tilt_x() as f32).hypot(event.tilt_y() as f32) / 90.).min(1.),
                azimuth: (event.tilt_y() as f32).atan2(event.tilt_x() as f32),
            }
        }))
    };
    let finish = move |event: PointerEvent| {
        let Some(mut current) = live
            .try_update_value(|active| {
                if active
                    .as_ref()
                    .is_some_and(|state| state.id == event.pointer_id())
                {
                    active.take()
                } else {
                    None
                }
            })
            .flatten()
        else {
            return;
        };
        if let Some(canvas) = overlay.get_untracked() {
            let _ = canvas.release_pointer_capture(event.pointer_id());
        }
        paint();
        if event.type_() != "pointerup" {
            // Preserve sampled ink on interruption without applying a cancelled edit.
            if current.tool != Tool::Ink || current.stroke.points.len() < 2 {
                return;
            }
        } else if let Some(mut point) = position(&event) {
            if event.pointer_type() == "pen" && point.pressure == 0. {
                point.pressure = current.stroke.points.last().map_or(0.5, |p| p.pressure);
            }
            current.stroke.append(point);
        }
        commit.run(Gesture {
            tool: current.tool,
            stroke: shape(&current.stroke, current.tool),
        });
    };
    view! {
        <canvas class="notes-committed" node_ref=base aria-label="Page content"></canvas>
        <canvas class:notes-live=true node_ref=overlay aria-label="Writing surface"
            class:notes-draw=move ||editable && touch.get() && tool.get()!=Tool::Pan
            on:pointerdown=move |event:PointerEvent|{
                if !editable || tool.get_untracked()==Tool::Pan || event.button()!=0 && event.button()!=5
                    || event.pointer_type()=="touch" && !touch.get_untracked() || live.with_value(Option::is_some){return;}
                let Some(point)=position(&event) else{return};
                let Some(canvas)=overlay.get_untracked() else{return};
                event.prevent_default();
                let _=canvas.set_pointer_capture(event.pointer_id());
                let tool=if event.button()==5{Tool::Whole}else{tool.get_untracked()};
                live.set_value(Some(Drawing{id:event.pointer_id(),tool,stroke:Stroke{id:0,pen:pen.get_untracked(),points:vec![point]}}));
                schedule();
            }
            on:pointermove=move |event:PointerEvent|{
                if !live.with_value(|active|active.as_ref().is_some_and(|drawing|drawing.id==event.pointer_id())){return;}
                event.prevent_default();
                let samples=if js_sys::Reflect::get(&event,&"getCoalescedEvents".into()).is_ok_and(|f|f.is_function()){
                    event.get_coalesced_events()
                }else{js_sys::Array::new()};
                use wasm_bindgen::JsCast;
                live.update_value(|active|if let Some(drawing)=active{
                    for sample in samples.iter().filter_map(|sample|sample.dyn_into::<PointerEvent>().ok()).chain([event]){
                        if let Some(point)=position(&sample){drawing.stroke.append(point);}
                    }
                });
                schedule();
            }
            on:pointerup=finish on:pointercancel=finish on:lostpointercapture=finish/>
    }
}
