#!/usr/bin/env python3
"""Verify exported browser fixtures with Poppler, independent of the exporter."""
from pathlib import Path
import subprocess
import sys


def run(*args: str) -> bytes:
    return subprocess.check_output(args)


def raster(pdf: Path, page: int, dpi: int) -> tuple[int, int, bytes]:
    image = run("pdftoppm", "-f", str(page), "-l", str(page), "-r", str(dpi), str(pdf))
    kind, dimensions, depth, rgb = image.split(b"\n", 3)
    width, height = (int(n) for n in dimensions.split())
    assert kind == b"P6" and depth == b"255" and len(rgb) == width * height * 3
    return width, height, rgb


def verify_annotations(pdf: Path) -> None:
    text = run("pdftotext", str(pdf), "-")
    assert b"Ellipse field notes" in text and b"Second page evidence" in text
    info = run("pdfinfo", str(pdf))
    assert any(line.split() == [b"Pages:", b"2"] for line in info.splitlines())
    width, height, rgb = raster(pdf, 2, 96)
    assert width > height, "The rotated page should be landscape"
    navy = gold = 0
    for r, g, b in zip(rgb[0::3], rgb[1::3], rgb[2::3], strict=True):
        navy += r < 65 and g < 85 and 40 < b < 105 and b - r > 10
        gold += r > 230 and 195 < g < 244 and b < 202 and r - b > 35
    assert navy > 100 and gold > 100, (navy, gold)
    print(f"Poppler rendered the {width}×{height} rotated page with {navy} ink pixels and {gold} highlight pixels.")


def verify_stationery(pdf: Path) -> None:
    info = run("pdfinfo", str(pdf))
    assert any(line.split() == [b"Pages:", b"2"] for line in info.splitlines())
    for number, channel in [(1, 0), (2, 2)]:
        width, height, rgb = raster(pdf, number, 144)
        assert (width, height) == (1224, 1584)

        def pixel(x: int, y: int) -> bytes:
            offset = (y * width + x) * 3
            return rgb[offset:offset + 3]

        # Cornell's cue column is unruled. The note column and summary have lines.
        assert pixel(200, 96) == pixel(200, 100)
        for x, y in [(800, 96), (318, 200), (200, 1283)]:
            assert any(
                (color := pixel(x + dx, y + dy))[channel] - color[1] > 40
                for dx in range(-1, 2) for dy in range(-1, 2)
            ), (number, x, y)
    print("Poppler rendered the Cornell divisions and writing lines with independent colors on both pages.")


if __name__ == "__main__":
    verify_annotations(Path(sys.argv[1]))
    if len(sys.argv) > 2:
        verify_stationery(Path(sys.argv[2]))
