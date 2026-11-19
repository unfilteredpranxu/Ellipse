use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hkdf::Hkdf;
use p256::{
    ecdh::EphemeralSecret,
    elliptic_curve::{rand_core::OsRng, sec1::ToEncodedPoint},
    PublicKey,
};
use sha2::Sha256;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{State, WebviewWindow};
use tauri_plugin_opener::OpenerExt;

#[derive(Default)]
pub struct Pairing(Mutex<Option<(Instant, EphemeralSecret)>>);

fn check_window(window: &WebviewWindow) -> Result<(), &'static str> {
    if window.label() == "main"
        && window
            .url()
            .is_ok_and(|url| match (url.scheme(), url.host_str()) {
                ("tauri", Some("localhost")) | ("http" | "https", Some("tauri.localhost")) => true,
                ("http", Some("localhost" | "127.0.0.1")) => {
                    cfg!(debug_assertions) && url.port() == Some(5173)
                }
                _ => false,
            })
    {
        Ok(())
    } else {
        Err("Open sign-in in the installed Ellipse window.")
    }
}

#[tauri::command(async)]
pub fn begin_google_sign_in(
    window: WebviewWindow,
    app: tauri::AppHandle,
    pairing: State<Pairing>,
) -> Result<(), &'static str> {
    check_window(&window)?;
    let secret = EphemeralSecret::random(&mut OsRng);
    let key = URL_SAFE_NO_PAD.encode(secret.public_key().to_encoded_point(false).as_bytes());
    let mut pending = pairing
        .inner()
        .0
        .lock()
        .map_err(|_| "Restart Ellipse and try signing in again.")?;
    *pending = Some((Instant::now(), secret));
    if app
        .opener()
        .open_url(
            format!("https://ellipse-workspace.pages.dev/native-sign-in/#v1.{key}"),
            None::<&str>,
        )
        .is_err()
    {
        *pending = None;
        return Err("Could not open your browser. Try again.");
    }
    Ok(())
}

#[tauri::command]
pub fn finish_google_sign_in(
    window: WebviewWindow,
    pairing: State<Pairing>,
    response: &str,
) -> Result<String, &'static str> {
    check_window(&window)?;
    let invalid = "This response is invalid or belongs to another sign-in attempt.";
    let response = response.trim();
    if response.len() > 16_384 {
        return Err(invalid);
    }
    let mut parts = response.split('.');
    if parts.next() != Some("v1") {
        return Err(invalid);
    }
    let mut decode = || {
        URL_SAFE_NO_PAD
            .decode(parts.next().ok_or(invalid)?)
            .map_err(|_| invalid)
    };
    let peer = PublicKey::from_sec1_bytes(&decode()?).map_err(|_| invalid)?;
    let nonce: [u8; 12] = decode()?.try_into().map_err(|_| invalid)?;
    let ciphertext = decode()?;
    if parts.next().is_some() {
        return Err(invalid);
    }
    let mut pending = pairing.inner().0.lock().map_err(|_| invalid)?;
    let (started, secret) = pending
        .as_ref()
        .ok_or("Start Google sign-in again in Ellipse.")?;
    if started.elapsed() > Duration::from_secs(600) {
        *pending = None;
        return Err("This sign-in attempt expired. Open Google again.");
    }
    let shared = secret.diffie_hellman(&peer);
    let mut key = [0; 32];
    Hkdf::<Sha256>::new(None, shared.raw_secret_bytes().as_ref())
        .expand(b"Ellipse native Google pairing v1", &mut key)
        .map_err(|_| invalid)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| invalid)?;
    let plain = cipher
        .decrypt(&Nonce::from(nonce), ciphertext.as_slice())
        .map_err(|_| invalid)?;
    let token = String::from_utf8(plain).map_err(|_| invalid)?;
    *pending = None;
    Ok(token)
}
