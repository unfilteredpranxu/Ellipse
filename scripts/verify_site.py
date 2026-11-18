#!/usr/bin/env python3
"""Check Google-only sign-in and Rust UI build inputs, without pretending OAuth ran."""
from pathlib import Path
import json
root=Path(__file__).resolve().parents[1]
site=(root/"site/index.html").read_text()
rust=(root/"web/src/main.rs").read_text()
auth=(root/"web/auth.js").read_text()
manifest=json.loads((root/"site/manifest.webmanifest").read_text())
checks={
"Artwork retained as a separate original JPEG":(root/"site/assets/ellipse-art.jpg").is_file(),
"No white box":'class="form-container"' not in site and "auth-panel" not in site,
"Google-only sign-in screen":"Continue with Google" in site and "Continue with Google" in rust,
"Firebase Google provider":"signInWithPopup" in auth and "GoogleAuthProvider" in auth,
"No password/OTP forms":'<input' not in site and "email-form" not in site,
"Rust Leptos UI":'leptos' in (root/"web/Cargo.toml").read_text() and 'fn App()' in rust,
"Tauri native shell":(root/"native/src-tauri/Cargo.toml").is_file(),
"PWA manifest":manifest["theme_color"]=="#030303",
}
for name,ok in checks.items():print(f'{"PASS" if ok else "FAIL"}: {name}')
assert all(checks.values()),"Ellipse M02 checks failed"
