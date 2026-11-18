use js_sys::{Function, Reflect};
use leptos::prelude::*;
use send_wrapper::SendWrapper;
use wasm_bindgen::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Account {
    pub uid: String,
    pub name: String,
    pub email: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Session {
    Checking,
    SignedOut,
    SignedIn(Account),
}

#[wasm_bindgen(raw_module = "/auth.js")]
extern "C" {
    #[wasm_bindgen(js_name = subscribeAuth)]
    fn subscribe_auth(callback: &Function) -> Function;
    #[wasm_bindgen(js_name = ellipseLogin)]
    pub fn ellipse_login();
    #[wasm_bindgen(js_name = ellipseLogout)]
    pub fn ellipse_logout();
    #[wasm_bindgen(js_name = isNative)]
    pub fn is_native() -> bool;
    #[wasm_bindgen(js_name = drivePresets)]
    pub fn drive_presets(uid: &str, payload: Option<String>) -> js_sys::Promise;
}

pub fn observe(session: RwSignal<Session>, message: RwSignal<String>) {
    let callback = Closure::<dyn FnMut(JsValue)>::new(move |detail| {
        let field = |key: &str| {
            Reflect::get(&detail, &JsValue::from_str(key))
                .ok()
                .and_then(|value| value.as_string())
                .unwrap_or_default()
        };
        let status = field("type");
        let state = match status.as_str() {
            "checking" => Some(Session::Checking),
            "signed_out" | "unavailable" => Some(Session::SignedOut),
            "signed_in" => {
                let uid = field("uid");
                if uid.is_empty() {
                    Some(Session::SignedOut)
                } else {
                    Some(Session::SignedIn(Account {
                        uid,
                        name: field("name"),
                        email: field("email"),
                    }))
                }
            }
            _ => None,
        };
        if let Some(state) = state
            && session.get_untracked() != state
        {
            session.set(state);
        }
        message.set(if status == "busy" {
            "Opening Google…".into()
        } else {
            field("message")
        });
    });
    let unsubscribe = subscribe_auth(callback.as_ref().unchecked_ref());
    let subscription = SendWrapper::new((unsubscribe, callback));
    on_cleanup(move || {
        let (unsubscribe, _callback) = subscription.take();
        let _ = unsubscribe.call0(&JsValue::UNDEFINED);
    });
}
