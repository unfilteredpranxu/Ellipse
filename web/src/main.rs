mod artwork_editor;
mod auth;
mod clock;
mod home;
mod media;
mod notes;
mod settings;

use auth::Session;
use leptos::prelude::*;
use notes::Section;

#[component]
fn App() -> impl IntoView {
    let session = RwSignal::new(Session::Checking);
    let message = RwSignal::new(String::new());
    auth::observe(session, message);
    let section = RwSignal::new(None::<Section>);
    let open = Memo::new(move |_| section.with(Option::is_some));
    let open_section = Callback::new(move |destination| section.set(Some(destination)));
    let close_notes = Callback::new(move |()| section.set(None));

    move || match session.get() {
        Session::Checking => view! {
            <main class="session-loading" aria-label="Ellipse">
                <span class="ellipse-mark" aria-hidden="true"></span>
                <p role="status">"Checking your Google session…"</p>
            </main>
        }
        .into_any(),
        Session::SignedOut if auth::is_native() && open.get() => view! {
            <notes::Notes local=true section back=close_notes/>
        }
        .into_any(),
        Session::SignedOut => view! { <SignIn message open_section/> }.into_any(),
        Session::SignedIn(account) if open.get() => view! {
            <notes::Notes local=auth::is_native() account=account.uid section back=close_notes/>
        }
        .into_any(),
        Session::SignedIn(account) => view! {
            <home::Home account=account message=message open_section/>
        }
        .into_any(),
    }
}

#[component]
fn SignIn(message: RwSignal<String>, open_section: Callback<Section>) -> impl IntoView {
    let pairing = RwSignal::new(false);
    let response = NodeRef::<leptos::html::Textarea>::new();
    view! {
        <main class="screen" aria-label="Ellipse welcome">
            <div class="hero-wrap">
                <img class="hero-art" src="/assets/ellipse-knight-clean.jpg"
                    alt="Black and white stippled fantasy knight with sword"
                    fetchpriority="high"/>
            </div>
            <section class="welcome-pane">
                <header class="topbar">
                    <div class="brand">
                        <span class="brand-mark" aria-hidden="true"></span>
                        <span>"Ellipse"</span>
                    </div>
                </header>
                <div class="welcome-copy">
                    <h1>"Welcome back"</h1>
                    <p>"Sign in to pick up where you left off."</p>
                </div>
                <div class="bottom">
                        <button class="signin-button" type="button" on:click=move |_| { pairing.set(auth::is_native()); auth::ellipse_login(); }>
                            <svg class="google-mark" width="21" height="21" viewBox="0 0 48 48" aria-hidden="true">
                                <path fill="#EA4335" d="M24 9.5c3.54 0 6.71 1.22 9.21 3.6l6.85-6.85C35.9 2.38 30.47 0 24 0 14.62 0 6.51 5.38 2.56 13.22l7.98 6.19C12.43 13.72 17.74 9.5 24 9.5z"/>
                                <path fill="#4285F4" d="M46.98 24.55c0-1.57-.15-3.09-.38-4.55H24v9.02h12.94c-.58 2.96-2.26 5.48-4.78 7.18l7.73 6C44.4 38.04 46.98 31.9 46.98 24.55z"/>
                                <path fill="#FBBC05" d="M10.53 28.59c-.48-1.44-.75-2.97-.75-4.59l-7.98-6.2C.92 16.46 0 20.12 0 24s.92 7.54 2.56 10.78l7.97-6.19z"/>
                                <path fill="#34A853" d="M24 48c6.48 0 11.93-2.13 15.91-5.8l-7.73-6c-2.15 1.45-4.92 2.3-8.18 2.3-6.26 0-11.57-4.22-13.47-9.91l-7.98 6.19C6.51 42.62 14.62 48 24 48z"/>
                            </svg>
                            <span>"Continue with Google"</span>
                        </button>
                    <Show when=move || pairing.get()>
                        <form on:submit=move |event| {
                            event.prevent_default();
                            if let Some(field) = response.get_untracked() {
                                let code = field.value();
                                field.set_value("");
                                let _ = auth::complete_native_login(&code);
                            }
                        }>
                            <label>"Sign-in response"
                                <textarea node_ref=response required maxlength=16384 autocomplete="off" spellcheck="false"></textarea>
                            </label>
                            <button type="submit">"Finish Google sign-in"</button>
                        </form>
                    </Show>
                    <Show when=auth::is_native>
                        <button type="button" class="signin-button local-notes-button" on:click=move |_|open_section.run(Section::Notes)>
                            "Open Notes on this device"
                        </button>
                        <nav class="local-tools" aria-label="Tools on this device">
                            {Section::PRODUCTIVITY.map(|section|view!{
                                <button type="button" on:click=move |_|open_section.run(section)>{section.label()}</button>
                            })}
                        </nav>
                        <button type="button" class="local-diagnostics" on:click=move |_|{
                            message.set("Preparing local diagnostics…".into());
                            wasm_bindgen_futures::spawn_local(async move {
                                let result=notes::diagnostics().await;
                                let _=message.try_set(result.map(|path|format!("Diagnostics saved to {path}"))
                                    .unwrap_or_else(|error|error));
                            });
                        }>"Save local diagnostics"</button>
                    </Show>
                    <p class="notice" role="status" aria-live="polite">{move || message.get()}</p>
                </div>
            </section>
        </main>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App /> });
}
