"""Generate small JPEG fixtures with known EXIF metadata into fixtures/exif/.

Run from the repo root:
    uv run --with pillow --with piexif scripts/make-fixtures.py

The files are committed; rerun only when the fixture set changes and update
fixtures/README.md accordingly.
"""

from __future__ import annotations

from pathlib import Path

import piexif
from PIL import Image

OUT = Path(__file__).resolve().parent.parent / "fixtures" / "exif"

# EXIF tag numbers not exposed by name in every piexif version.
OFFSET_TIME_ORIGINAL = 36881


def write(name: str, color: tuple[int, int, int], exif: dict[str, dict[int, object]] | None) -> None:
    image = Image.new("RGB", (16, 12), color)
    path = OUT / name
    if exif is None:
        image.save(path, "JPEG", quality=80)
    else:
        image.save(path, "JPEG", quality=80, exif=piexif.dump(exif))
    print(f"wrote {path.relative_to(OUT.parent.parent)}")


def camera(make: str, model: str) -> dict[int, object]:
    return {piexif.ImageIFD.Make: make.encode(), piexif.ImageIFD.Model: model.encode()}


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)

    write(
        "pixel_2024-06-01_12-00-00.jpg",
        (200, 60, 60),
        {
            "0th": camera("Google", "Pixel 8"),
            "Exif": {piexif.ExifIFD.DateTimeOriginal: b"2024:06:01 12:00:00"},
        },
    )
    write(
        "pixel_2024-06-01_12-00-05.jpg",
        (60, 200, 60),
        {
            "0th": camera("Google", "Pixel 8"),
            "Exif": {piexif.ExifIFD.DateTimeOriginal: b"2024:06:01 12:00:05"},
        },
    )
    write(
        "offset_2024-06-02_08-30-00+0200.jpg",
        (60, 60, 200),
        {
            "0th": camera("samsung", "SM-S911B"),
            "Exif": {
                piexif.ExifIFD.DateTimeOriginal: b"2024:06:02 08:30:00",
                OFFSET_TIME_ORIGINAL: b"+02:00",
            },
        },
    )
    write(
        "rotated_orientation-6.jpg",
        (200, 200, 60),
        {
            "0th": {**camera("Google", "Pixel 8"), piexif.ImageIFD.Orientation: 6},
            "Exif": {piexif.ExifIFD.DateTimeOriginal: b"2024:05:31 18:15:00"},
        },
    )
    write(
        "datetime-only_2024-05-30_10-00-00.jpg",
        (60, 200, 200),
        {"0th": {piexif.ImageIFD.DateTime: b"2024:05:30 10:00:00"}},
    )
    write(
        "broken-date.jpg",
        (200, 60, 200),
        {"Exif": {piexif.ExifIFD.DateTimeOriginal: b"0000:00:00 00:00:00"}},
    )
    write("no-exif.jpg", (120, 120, 120), None)


if __name__ == "__main__":
    main()
