#!/usr/bin/env python3
"""Verify the rotated annotated browser fixture with Poppler, independent of the exporter."""
from pathlib import Path
import subprocess
import sys


def run(*args: str) -> bytes:
    return subprocess.check_output(args)


def main() -> None:
    pdf = Path(sys.argv[1])
    text = run("pdftotext", str(pdf), "-")
    assert b"Ellipse field notes" in text and b"Second page evidence" in text
    info = run("pdfinfo", str(pdf))
    assert any(line.split() == [b"Pages:", b"2"] for line in info.splitlines())
    image = run("pdftoppm", "-f", "2", "-l", "2", "-r", "96", str(pdf))
    kind, dimensions, depth, rgb = image.split(b"\n", 3)
    width, height = (int(n) for n in dimensions.split())
    assert kind == b"P6" and depth == b"255" and len(rgb) == width * height * 3
    assert width > height, "The rotated page should be landscape"
    navy = gold = 0
    for r, g, b in zip(rgb[0::3], rgb[1::3], rgb[2::3], strict=True):
        navy += r < 65 and g < 85 and 40 < b < 105 and b - r > 10
        gold += r > 230 and 195 < g < 244 and b < 202 and r - b > 35
    assert navy > 100 and gold > 100, (navy, gold)
    print(f"Poppler rendered the {width}×{height} rotated page with {navy} ink pixels and {gold} highlight pixels.")


if __name__ == "__main__":
    main()
