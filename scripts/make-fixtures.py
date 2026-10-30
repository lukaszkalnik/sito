"""Generate small JPEG fixtures with known EXIF metadata into fixtures/exif/.

Run from the repo root:
    uv run --with pillow --with piexif scripts/make-fixtures.py

The files are committed; rerun only when the fixture set changes and update
fixtures/README.md accordingly.
"""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

import piexif
from PIL import Image

import raw_fixtures

FIXTURES = Path(__file__).resolve().parent.parent / "fixtures"
OUT = FIXTURES / "exif"
TAKEOUT = FIXTURES / "takeout" / "Album"

# EXIF tag numbers not exposed by name in every piexif version.
OFFSET_TIME_ORIGINAL = 36881


def write(
    name: str,
    color: tuple[int, int, int],
    exif: dict[str, dict[int, object]] | None,
    folder: Path = OUT,
) -> None:
    image = Image.new("RGB", (16, 12), color)
    path = folder / name
    if exif is None:
        image.save(path, "JPEG", quality=80)
    else:
        image.save(path, "JPEG", quality=80, exif=piexif.dump(exif))
    print(f"wrote {path.relative_to(FIXTURES.parent)}")


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


def taken(text: str) -> dict[str, dict[int, object]]:
    return {"Exif": {piexif.ExifIFD.DateTimeOriginal: text.encode()}}


def sidecar(name: str, title: str, utc: str) -> None:
    """A Takeout date file; `utc` is `YYYY-MM-DD HH:MM:SS` in UTC."""
    moment = datetime.strptime(utc, "%Y-%m-%d %H:%M:%S").replace(tzinfo=timezone.utc)
    content = {
        "title": title,
        "description": "",
        "photoTakenTime": {"timestamp": str(int(moment.timestamp())), "formatted": utc + " UTC"},
        # Never read by Sito; here to check that it is ignored.
        "geoData": {"latitude": 50.29, "longitude": 18.67, "altitude": 0.0},
    }
    path = TAKEOUT / name
    path.write_text(json.dumps(content, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"wrote {path.relative_to(FIXTURES.parent)}")


def takeout() -> None:
    """An album of an unpacked Google Takeout, one case per file."""
    TAKEOUT.mkdir(parents=True, exist_ok=True)
    grey = (120, 120, 120)
    write("no-exif.jpg", grey, None, TAKEOUT)
    sidecar("no-exif.jpg.supplemental-metadata.json", "no-exif.jpg", "2024-06-01 10:00:00")
    write("IMG_1.jpg", (200, 60, 60), taken("2024:06:01 12:00:05"), TAKEOUT)
    sidecar("IMG_1.jpg.supplemental-metadata.json", "IMG_1.jpg", "2024-06-01 10:00:05")
    write("IMG_1(1).jpg", grey, None, TAKEOUT)
    sidecar("IMG_1.jpg.supplemental-metadata(1).json", "IMG_1.jpg", "2024-06-01 10:00:08")
    write("scan_1976.jpg", (200, 200, 60), taken("2006:03:04 18:22:10"), TAKEOUT)
    sidecar("scan_1976.jpg.supplemental-metadata.json", "scan_1976.jpg", "1976-07-01 10:00:00")
    write("reset_clock.jpg", (60, 200, 60), taken("2000:01:01 00:00:00"), TAKEOUT)
    sidecar("reset_clock.jpg.supplemental-metadata.json", "reset_clock.jpg", "2024-06-02 08:00:00")
    write("old_export.jpg", grey, None, TAKEOUT)
    sidecar("old_export.jpg.json", "old_export.jpg", "2024-06-03 08:00:00")
    long_name = "Screenshot_20240604-120000_Some_Long_App.jpg"
    write(long_name, grey, None, TAKEOUT)
    sidecar(long_name + ".supple.json", long_name, "2024-06-04 10:00:00")
    write("renamed.jpg", grey, None, TAKEOUT)
    sidecar("unrelated-name.json", "renamed.jpg", "2024-06-05 10:00:00")
    write("no-sidecar_reset-clock.jpg", (60, 60, 200), taken("2000:01:01 00:00:00"), TAKEOUT)
    (TAKEOUT / "metadane.json").write_text(
        json.dumps({"title": "Album", "description": ""}, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def cameras() -> None:
    """Two cameras shooting one moment, their photos interleaved in time."""
    folder = FIXTURES / "cameras"
    folder.mkdir(parents=True, exist_ok=True)
    shots = [
        ("pixel_12-00-00.jpg", "Google", "Pixel 8", "2024:06:01 12:00:00"),
        ("canon_12-00-02.jpg", "Canon", "Canon EOS R6", "2024:06:01 12:00:02"),
        ("pixel_12-00-04.jpg", "Google", "Pixel 8", "2024:06:01 12:00:04"),
        ("canon_12-00-06.jpg", "Canon", "Canon EOS R6", "2024:06:01 12:00:06"),
    ]
    for name, make, model, time in shots:
        exif = {"0th": camera(make, model), **taken(time)}
        write(name, (60, 60, 200) if make == "Canon" else (200, 60, 60), exif, folder)


def raw() -> None:
    """Synthetic RAW files, one per layout (see scripts/raw_fixtures.py)."""
    for path in raw_fixtures.write_all(FIXTURES / "raw"):
        print(f"wrote {path.relative_to(FIXTURES.parent)}")


def tiff() -> None:
    """Small TIFFs (scans): dates, orientation, bit depths, pages, a broken one."""
    folder = FIXTURES / "tiff"
    folder.mkdir(parents=True, exist_ok=True)

    def save(name: str, image: Image.Image, **options: object) -> Path:
        path = folder / name
        image.save(path, "TIFF", **options)
        print(f"wrote {path.relative_to(FIXTURES.parent)}")
        return path

    rgb = folder / "rgb8_orientation-6.tif"
    rgb.write_bytes(raw_fixtures.tiff_scan())
    print(f"wrote {rgb.relative_to(FIXTURES.parent)}")

    dated = Image.new("L", (40, 30), 128)
    dated_exif = dated.getexif()
    dated_exif[0x0132] = "2024:06:13 10:00:00"
    save("gray8_datetime-only.tif", dated, exif=dated_exif)
    save("gray16.tif", Image.new("I;16", (40, 30), 30000))
    save("cmyk.tif", Image.new("CMYK", (40, 30), (0, 128, 255, 0)))
    save("no-date.tif", Image.new("RGB", (40, 30), (60, 60, 200)))
    save(
        "two-pages.tif",
        Image.new("RGB", (40, 30), (255, 0, 0)),
        save_all=True,
        append_images=[Image.new("RGB", (60, 20), (0, 0, 255))],
    )
    broken = folder / "broken.tif"
    broken.write_bytes(rgb.read_bytes()[:60])
    print(f"wrote {broken.relative_to(FIXTURES.parent)}")


if __name__ == "__main__":
    main()
    cameras()
    takeout()
    raw()
    tiff()
