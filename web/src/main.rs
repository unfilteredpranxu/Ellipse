use leptos::prelude::*;
use wasm_bindgen::{closure::Closure, prelude::*, JsCast};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window, js_name = ellipseLogin)]
    fn ellipse_login();
    #[wasm_bindgen(js_namespace = window, js_name = ellipseLogout)]
    fn ellipse_logout();
}

fn field(event: &web_sys::Event, key: &str) -> String {
    let Some(custom) = event.dyn_ref::<web_sys::CustomEvent>() else { return String::new(); };
    js_sys::Reflect::get(&custom.detail(), &JsValue::from_str(key))
        .ok().and_then(|v| v.as_string()).unwrap_or_default()
}

#[component]
fn App() -> impl IntoView {
    let (signed_in, set_signed_in) = signal(false);
    let (name, set_name) = signal(String::new());
    let (message, set_message) = signal(String::new());

    // Firebase is a small platform bridge; layout and UI state belong to Rust.
    let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |event| {
        match field(&event, "type").as_str() {
            "signed_in" => {
                set_signed_in.set(true);
                let label = field(&event, "name");
                let email = field(&event, "email");
                set_name.set(if label.is_empty() { email } else { label });
                set_message.set(String::new());
            },
            "signed_out" => {
                set_signed_in.set(false);
                set_message.set(String::new());
            },
            "error" => set_message.set(field(&event, "message")),
            "busy" => set_message.set("Opening Google…".to_owned()),
            _ => {}
        }
    });
    if let Some(window) = web_sys::window() {
        let _ = window.add_event_listener_with_callback("ellipse:auth", listener.as_ref().unchecked_ref());
        // Firebase can resolve before the Wasm runtime attaches its event listener.
        if let Ok(snapshot) = js_sys::Reflect::get(&window, &JsValue::from_str("ellipseAuthSnapshot")) {
            if !snapshot.is_undefined() && !snapshot.is_null() {
                let kind = js_sys::Reflect::get(&snapshot, &JsValue::from_str("type")).ok().and_then(|v| v.as_string()).unwrap_or_default();
                if kind == "signed_in" {
                    set_signed_in.set(true);
                    let label = js_sys::Reflect::get(&snapshot, &JsValue::from_str("name")).ok().and_then(|v| v.as_string()).unwrap_or_default();
                    set_name.set(label);
                }
            }
        }
    }
    listener.forget();

    view! {
        <main class="screen" aria-label="Ellipse welcome">
            <div class="hero-wrap"><img class="hero-art" src="/assets/ellipse-art.jpg" alt="Monochrome fantasy knight artwork" fetchpriority="high"/></div>
            <header class="topbar">
                <div class="brand"><span class="brand-mark" aria-hidden="true"></span><span>"Ellipse"</span></div>
                <span class="edition">"Personal space"</span>
            </header>
            <div class="bottom">
                <Show when=move || signed_in.get() fallback=move || view! {
                    <button class="signin-button" type="button" on:click=move |_| ellipse_login()>
                        <svg class="google-mark" width="21" height="21" viewBox="0 0 48 48" aria-hidden="true">
                            <path fill="#EA4335" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z"/>
                            <path fill="#4285F4" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6C44.4 38.04 46.98 31.9 46.98 24.55z"/>
                            <path fill="#FBBC05" d="M10.53 28.59c-.48-1.44-.75-2.97-.75-4.59s.27-3.15.75-4.59l-7.98-6.2C.92 16.46 0 20.12 0 24s.92 7.54 2.56 10.78l7.97-6.19z"/>
                            <path fill="#34A853" d="M24 48c6.48 0 11.93-2.13 15.91-5.8l-7.73-6c-2.15 1.45-4.92 2.3-8.18 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z"/>
                        </svg>
                        <span>"Continue with Google"</span>
                    </button>
                }>
                    <div class="signed-in">
                        <div>
                            <div class="account-name">{move || name.get()}</div>
                            <div class="account-label">"Connected with Google"</div>
                        </div>
                        <button class="signout" type="button" on:click=move |_| ellipse_logout()>"Sign out"</button>
                    </div>
                </Show>
                <p class="notice" role="status" aria-live="polite">{move || message.get()}</p>
            </div>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
