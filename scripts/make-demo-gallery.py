"""Build the demo gallery for the README screenshots into demo/ (git-ignored).

Run from the repo root:
    uv run --with pillow --with piexif scripts/make-demo-gallery.py

Downloads the stock photos listed in scripts/demo-gallery/sources.json (cached
in demo/originals/), turns them into a phone-like gallery with EXIF capture
times and cameras, and writes:

- demo/gallery/Camera/*.jpg: the photos, openable in Sito
- demo/thumbs/<id>.jpg: thumbnails like the app's thumbnail cache
- demo/manifest.json: the timeline as `get_timeline` returns it
- docs/images/demo-gallery/SOURCES.md: photo credits (committed)

Shots with an `edit` are variations of one photo (crop, exposure, blur), so a
single stock photo becomes a series. The output only depends on sources.json.
"""

from __future__ import annotations

import json
import urllib.request
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import piexif
from PIL import Image, ImageEnhance, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
SOURCES = ROOT / "scripts" / "demo-gallery" / "sources.json"
DEMO = ROOT / "demo"
ORIGINALS = DEMO / "originals"
GALLERY_DIR = "Camera"
GALLERY = DEMO / "gallery" / GALLERY_DIR
THUMBS = DEMO / "thumbs"
MANIFEST = DEMO / "manifest.json"
CREDITS = ROOT / "docs" / "images" / "demo-gallery" / "SOURCES.md"

PHOTO_EDGE = 1600
# Same as `DEFAULT_MAX_EDGE` in crates/core/src/thumbnail.rs.
THUMB_EDGE = 400
DOWNLOAD_EDGE = 2400

# EXIF tag numbers not exposed by name in every piexif version.
OFFSET_TIME_ORIGINAL = 36881

CAMERAS = {
    "pixel": ("Google", "Pixel 8"),
    "samsung": ("samsung", "SM-S911B"),
}


@dataclass(frozen=True)
class Edit:
    """A variation of a photo: zoom into the centre, shift, exposure, blur."""

    zoom: float = 1.0
    shift_x: float = 0.0
    shift_y: float = 0.0
    brightness: float = 1.0
    blur: float = 0.0

    @classmethod
    def from_json(cls, data: dict[str, float] | None) -> Edit:
        data = data or {}
        return cls(
            zoom=data.get("zoom", 1.0),
            shift_x=data.get("shiftX", 0.0),
            shift_y=data.get("shiftY", 0.0),
            brightness=data.get("brightness", 1.0),
            blur=data.get("blur", 0.0),
        )


@dataclass(frozen=True)
class Shot:
    """One photo of the gallery."""

    photo: str
    taken_at: datetime
    camera: str
    edit: Edit

    @property
    def file_name(self) -> str:
        """The file name the camera app would give it."""
        if self.camera == "pixel":
            return f"PXL_{self.taken_at:%Y%m%d_%H%M%S}000.jpg"
        return f"{self.taken_at:%Y%m%d_%H%M%S}.jpg"


def download(key: str, cdn: str) -> Path:
    path = ORIGINALS / f"{key}.jpg"
    if path.exists():
        return path
    url = f"https://images.unsplash.com/{cdn}?w={DOWNLOAD_EDGE}&q=90&fm=jpg"
    with urllib.request.urlopen(url, timeout=60) as response:
        path.write_bytes(response.read())
    print(f"downloaded {key}")
    return path


def apply(image: Image.Image, edit: Edit) -> Image.Image:
    width, height = image.size
    if edit.zoom != 1.0 or edit.shift_x or edit.shift_y:
        crop_w, crop_h = width / edit.zoom, height / edit.zoom
        left = (width - crop_w) / 2 + edit.shift_x * width
        top = (height - crop_h) / 2 + edit.shift_y * height
        left = min(max(left, 0), width - crop_w)
        top = min(max(top, 0), height - crop_h)
        box = (round(left), round(top), round(left + crop_w), round(top + crop_h))
        image = image.crop(box).resize((width, height), Image.Resampling.LANCZOS)
    if edit.brightness != 1.0:
        image = ImageEnhance.Brightness(image).enhance(edit.brightness)
    if edit.blur:
        image = image.filter(ImageFilter.GaussianBlur(edit.blur))
    return image


def exif_for(shot: Shot) -> bytes:
    make, model = CAMERAS[shot.camera]
    taken = shot.taken_at.strftime("%Y:%m:%d %H:%M:%S").encode()
    return piexif.dump(
        {
            "0th": {piexif.ImageIFD.Make: make.encode(), piexif.ImageIFD.Model: model.encode()},
            "Exif": {
                piexif.ExifIFD.DateTimeOriginal: taken,
                OFFSET_TIME_ORIGINAL: b"+02:00",
            },
        }
    )


def write_shot(shot: Shot, original: Path, shot_id: int) -> None:
    with Image.open(original) as source:
        image = source.convert("RGB")
    image.thumbnail((PHOTO_EDGE, PHOTO_EDGE), Image.Resampling.LANCZOS)
    image = apply(image, shot.edit)
    image.save(GALLERY / shot.file_name, "JPEG", quality=88, exif=exif_for(shot))
    thumb = image.copy()
    thumb.thumbnail((THUMB_EDGE, THUMB_EDGE), Image.Resampling.LANCZOS)
    thumb.save(THUMBS / f"{shot_id}.jpg", "JPEG", quality=85)


def timeline_entry(shot: Shot, shot_id: int) -> dict[str, object]:
    """A `TimelineEntry` (ui/src/features/timeline/api.ts) with Windows paths."""
    return {
        "id": shot_id,
        "name": shot.file_name,
        "relativeDir": GALLERY_DIR,
        "relativePath": f"{GALLERY_DIR}\\{shot.file_name}",
        "takenAt": shot.taken_at.isoformat(),
        "fromFileDate": False,
    }


def write_credits(sources: dict[str, object]) -> None:
    license_ = sources["license"]
    photos = sources["photos"]
    assert isinstance(license_, dict) and isinstance(photos, dict)
    lines = [
        "# Demo gallery photo credits",
        "",
        "The README screenshots show a demo gallery built by `scripts/make-demo-gallery.py`",
        "from these photos on [Unsplash](https://unsplash.com). Some photos appear several",
        "times with small changes (crop, exposure, blur) to form a series.",
        "",
        f"License: [{license_['name']}]({license_['url']}) (terms checked {license_['checked']}).",
        f"{license_['summary']}",
        "",
        "Generated from `scripts/demo-gallery/sources.json`; do not edit by hand.",
        "",
        "| Photo | Author |",
        "|---|---|",
    ]
    for key, photo in photos.items():
        page = f"https://unsplash.com/photos/{photo['unsplash']}"
        profile = f"https://unsplash.com/@{photo['profile']}"
        lines.append(f"| [{key}]({page}) | [{photo['author']}]({profile}) |")
    CREDITS.parent.mkdir(parents=True, exist_ok=True)
    CREDITS.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"wrote {CREDITS.relative_to(ROOT)}")


def main() -> None:
    sources = json.loads(SOURCES.read_text(encoding="utf-8"))
    for directory in (ORIGINALS, GALLERY, THUMBS):
        directory.mkdir(parents=True, exist_ok=True)

    originals = {key: download(key, photo["cdn"]) for key, photo in sources["photos"].items()}
    shots = [
        Shot(
            photo=s["photo"],
            taken_at=datetime.fromisoformat(s["at"]),
            camera=s["camera"],
            edit=Edit.from_json(s.get("edit")),
        )
        for s in sources["shots"]
    ]
    # The timeline is newest first and ids follow its order.
    shots.sort(key=lambda shot: shot.taken_at, reverse=True)

    for stale in GALLERY.glob("*.jpg"):
        stale.unlink()
    for stale in THUMBS.glob("*.jpg"):
        stale.unlink()
    for shot_id, shot in enumerate(shots):
        write_shot(shot, originals[shot.photo], shot_id)

    manifest = [timeline_entry(shot, shot_id) for shot_id, shot in enumerate(shots)]
    MANIFEST.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {len(shots)} photos to {GALLERY.relative_to(ROOT)}")
    write_credits(sources)


if __name__ == "__main__":
    main()
