#!/usr/bin/env python3
"""Static acceptance checks for the M01 Ellipse preview."""
from pathlib import Path
import json

root = Path(__file__).resolve().parents[1]
page = (root / "site/index.html").read_text(encoding="utf-8")
manifest = json.loads((root / "site/manifest.webmanifest").read_text(encoding="utf-8"))
checks = {
    "Original embedded sign-in artwork": "data:image/jpeg;base64," in page,
    "Ellipse branding": "<title>Ellipse" in page,
    "First-screen markup": "Welcome back" in page and 'auth-screen' in page,
    "Local-only mode": "localGuestKey" in page,
    "PWA install manifest": 'rel="manifest"' in page and manifest.get("name", "").startswith("Ellipse"),
    "Offline shell": (root / "site/sw.js").is_file(),
    "No fake payment checkout": "checkout.stripe.com" not in page,
}
for name, ok in checks.items():
    print(f'{"PASS" if ok else "FAIL"}: {name}')
assert all(checks.values()), "M01 static acceptance failed"
print("M01 static acceptance: PASS")
