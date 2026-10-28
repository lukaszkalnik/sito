"""Tiny synthetic RAW files for tests: the TIFF layouts of each RAW format,
holding small JPEG previews instead of sensor data.

Used by scripts/make-fixtures.py; writes into fixtures/raw/.
"""

from __future__ import annotations

import io
import struct
from dataclasses import dataclass
from pathlib import Path

import piexif
from PIL import Image

ASCII, SHORT, LONG, UNDEFINED = 2, 3, 4, 7
TYPE_SIZE = {ASCII: 1, SHORT: 2, LONG: 4, UNDEFINED: 1}


@dataclass(frozen=True)
class IfdRef:
    """The offset of another IFD of the same structure."""

    name: str


@dataclass(frozen=True)
class BlobRef:
    """The offset of a named blob of the same structure."""

    name: str


@dataclass(frozen=True)
class BlobLen:
    """The length of a named blob."""

    name: str


Value = int | list[int] | str | bytes | IfdRef | BlobRef | BlobLen
Entry = tuple[int, int, Value]


def build(order: str, header: bytes, ifds: dict[str, list[Entry]], blobs: dict[str, bytes]) -> bytes:
    """A TIFF structure: `header`, then the IFDs (in order, chained), then blobs.

    Offsets are relative to the start of the returned bytes. The first IFD
    follows the header; each IFD's next pointer leads to the following one
    unless its name starts with `_` (sub-IFDs, which are not chained).
    """
    end = "<" if order == "II" else ">"
    names = list(ifds)
    ifd_at: dict[str, int] = {}
    at = len(header)
    for name in names:
        ifd_at[name] = at
        at += 2 + 12 * len(ifds[name]) + 4
    # Out-of-line values and blobs follow the IFDs.
    data = bytearray()
    data_start = at
    blob_at: dict[str, int] = {}
    for name, blob in blobs.items():
        blob_at[name] = data_start + len(data)
        data += blob
        data += b"\0" * (len(data) % 2)

    def field(kind: int, value: Value) -> tuple[int, bytes]:
        if isinstance(value, IfdRef):
            return 1, struct.pack(end + "I", ifd_at[value.name])
        if isinstance(value, BlobRef):
            return 1, struct.pack(end + "I", blob_at[value.name])
        if isinstance(value, BlobLen):
            return 1, struct.pack(end + "I", len(blobs[value.name]))
        if isinstance(value, str):
            raw = value.encode() + b"\0"
        elif isinstance(value, bytes):
            raw = value
        else:
            values = value if isinstance(value, list) else [value]
            code = {SHORT: "H", LONG: "I"}[kind]
            raw = b"".join(struct.pack(end + code, v) for v in values)
        count = len(raw) // TYPE_SIZE[kind]
        if len(raw) <= 4:
            return count, raw.ljust(4, b"\0")
        offset = data_start + len(data)
        data.extend(raw)
        data.extend(b"\0" * (len(data) % 2))
        return count, struct.pack(end + "I", offset)

    out = bytearray(header)
    chained = [n for n in names if not n.startswith("_")]
    for name in names:
        entries = sorted(ifds[name], key=lambda e: e[0])
        out += struct.pack(end + "H", len(entries))
        for tag, kind, value in entries:
            count, raw = field(kind, value)
            out += struct.pack(end + "HHI", tag, kind, count) + raw
        following = chained[chained.index(name) + 1] if name in chained[:-1] else None
        out += struct.pack(end + "I", ifd_at[following] if following else 0)
    assert len(out) == data_start
    return bytes(out + data)


def jpeg(width: int, height: int, color: tuple[int, int, int], exif: bytes | None = None) -> bytes:
    out = io.BytesIO()
    image = Image.new("RGB", (width, height), color)
    if exif is None:
        image.save(out, "JPEG", quality=80)
    else:
        image.save(out, "JPEG", quality=80, exif=exif)
    return out.getvalue()


def tiff_header(order: str, magic: bytes, extra: bytes = b"") -> bytes:
    """`order`, `magic` and the first IFD's offset, right after `extra`."""
    end = "<" if order == "II" else ">"
    return order.encode() + magic + struct.pack(end + "I", 8 + len(extra)) + extra


PREVIEW = jpeg(64, 48, (200, 120, 40))
THUMB = jpeg(16, 12, (40, 40, 40))
DATE = "2024:06:10 10:00:00"


def camera(make: str, model: str, orientation: int = 1) -> list[Entry]:
    return [(0x010F, ASCII, make), (0x0110, ASCII, model), (0x0112, SHORT, orientation)]


def orf(with_preview: bool = True) -> bytes:
    """Olympus: `IIRO` magic; the preview sits in the maker note's CameraSettings,
    a small thumbnail in IFD1. Without preview, there is no thumbnail either."""
    settings: list[Entry] = []
    note_blobs: dict[str, bytes] = {}
    ifd1: dict[str, list[Entry]] = {}
    blobs: dict[str, bytes] = {}
    if with_preview:
        settings = [(0x0101, LONG, BlobRef("preview")), (0x0102, LONG, BlobLen("preview"))]
        note_blobs["preview"] = PREVIEW
        ifd1 = {"ifd1": [(0x0201, LONG, BlobRef("thumb")), (0x0202, LONG, BlobLen("thumb"))]}
        blobs = {"thumb": THUMB}
    else:
        settings = [(0x0100, LONG, 0)]
    note = build(
        "II",
        b"OLYMPUS\0II\x03\0",
        {"main": [(0x2020, LONG, IfdRef("_settings"))], "_settings": settings},
        note_blobs,
    )
    return build(
        "II",
        tiff_header("II", b"RO"),
        {
            "ifd0": [*camera("OLYMPUS IMAGING CORP.", "E-M10", 6), (0x8769, LONG, IfdRef("_exif"))],
            **ifd1,
            "_exif": [(0x9003, ASCII, DATE), (0x927C, UNDEFINED, note)],
        },
        blobs,
    )


def cr2() -> bytes:
    """Canon: IFD0 holds a JPEG-compressed image in one strip."""
    return build(
        "II",
        tiff_header("II", b"*\0", b"CR\x02\0\0\0\0\0"),
        {
            "ifd0": [
                *camera("Canon", "Canon EOS 80D", 8),
                (0x0103, SHORT, 6),
                (0x0111, LONG, BlobRef("preview")),
                (0x0117, LONG, BlobLen("preview")),
                (0x8769, LONG, IfdRef("_exif")),
            ],
            "_exif": [(0x9003, ASCII, DATE)],
        },
        {"preview": PREVIEW},
    )


def nef() -> bytes:
    """Nikon: big-endian; the preview is a JPEG in a sub-IFD."""
    return build(
        "MM",
        tiff_header("MM", b"\0*"),
        {
            "ifd0": [
                *camera("NIKON CORPORATION", "NIKON D750", 1),
                (0x014A, LONG, IfdRef("_sub")),
                (0x8769, LONG, IfdRef("_exif")),
            ],
            "_sub": [(0x0201, LONG, BlobRef("preview")), (0x0202, LONG, BlobLen("preview"))],
            "_exif": [(0x9003, ASCII, DATE)],
        },
        {"preview": PREVIEW},
    )


def arw() -> bytes:
    """Sony: the preview is IFD0's JPEG."""
    return build(
        "II",
        tiff_header("II", b"*\0"),
        {
            "ifd0": [
                *camera("SONY", "ILCE-7M3", 1),
                (0x0201, LONG, BlobRef("preview")),
                (0x0202, LONG, BlobLen("preview")),
                (0x8769, LONG, IfdRef("_exif")),
            ],
            "_exif": [(0x9003, ASCII, DATE)],
        },
        {"preview": PREVIEW},
    )


def rw2() -> bytes:
    """Panasonic: `IIU\\0` magic; the preview is the value of tag 0x002E."""
    return build(
        "II",
        tiff_header("II", b"U\0"),
        {
            "ifd0": [
                *camera("Panasonic", "DC-G9", 1),
                (0x002E, UNDEFINED, PREVIEW),
                (0x8769, LONG, IfdRef("_exif")),
            ],
            "_exif": [(0x9003, ASCII, DATE)],
        },
        {},
    )


def dng() -> bytes:
    """DNG: IFD0 is a JPEG preview strip; the raw data in a sub-IFD is not a JPEG."""
    return build(
        "II",
        tiff_header("II", b"*\0"),
        {
            "ifd0": [
                *camera("Google", "Pixel 8", 1),
                (0x00FE, LONG, 1),
                (0x0103, SHORT, 7),
                (0x0111, LONG, BlobRef("preview")),
                (0x0117, LONG, BlobLen("preview")),
                (0x014A, LONG, IfdRef("_raw")),
                (0x8769, LONG, IfdRef("_exif")),
            ],
            "_raw": [
                (0x00FE, LONG, 0),
                (0x0103, SHORT, 7),
                (0x0111, LONG, BlobRef("raw")),
                (0x0117, LONG, BlobLen("raw")),
            ],
            "_exif": [(0x9003, ASCII, DATE)],
        },
        {"preview": PREVIEW, "raw": b"\x12\x34" * 64},
    )


def raf() -> bytes:
    """Fujifilm: a header pointing to a JPEG that carries the EXIF."""
    exif = piexif.dump(
        {
            "0th": {piexif.ImageIFD.Make: b"FUJIFILM", piexif.ImageIFD.Model: b"X-T3"},
            "Exif": {piexif.ExifIFD.DateTimeOriginal: DATE.encode()},
        }
    )
    preview = jpeg(64, 48, (200, 120, 40), exif)
    header = bytearray(b"FUJIFILMCCD-RAW 0201FF383501".ljust(84, b"\0"))
    offset = 160
    header += struct.pack(">II", offset, len(preview))
    header = header.ljust(offset, b"\0")
    return bytes(header + preview)


def nef_maker_note() -> bytes:
    """Older Nikon: the preview is only in the maker note's preview IFD, whose
    offsets count from the maker note's own TIFF header."""
    inner = build(
        "MM",
        tiff_header("MM", b"\0*"),
        {
            "main": [(0x0011, LONG, IfdRef("_preview"))],
            "_preview": [(0x0201, LONG, BlobRef("preview")), (0x0202, LONG, BlobLen("preview"))],
        },
        {"preview": PREVIEW},
    )
    note = b"Nikon\0\x02\x10\0\0" + inner
    return build(
        "MM",
        tiff_header("MM", b"\0*"),
        {
            "ifd0": [*camera("NIKON CORPORATION", "NIKON D2H", 1), (0x8769, LONG, IfdRef("_exif"))],
            "_exif": [(0x9003, ASCII, DATE), (0x927C, UNDEFINED, note)],
        },
        {},
    )


def jpeg_named_raw() -> bytes:
    """A JPEG saved with a RAW extension, as some phones do."""
    exif = piexif.dump(
        {
            "0th": {piexif.ImageIFD.Make: b"samsung", piexif.ImageIFD.Model: b"SM-G973U"},
            "Exif": {piexif.ExifIFD.DateTimeOriginal: DATE.encode()},
        }
    )
    return jpeg(64, 48, (200, 120, 40), exif)


def write_all(folder: Path) -> list[Path]:
    folder.mkdir(parents=True, exist_ok=True)
    files = {
        "olympus.orf": orf(),
        "olympus-no-preview.ORF": orf(with_preview=False),
        "canon.cr2": cr2(),
        "nikon.nef": nef(),
        "sony.arw": arw(),
        "panasonic.rw2": rw2(),
        "google.dng": dng(),
        "fujifilm.raf": raf(),
        "nikon-maker-note.nef": nef_maker_note(),
        "samsung-jpeg.dng": jpeg_named_raw(),
    }
    written = []
    for name, content in files.items():
        path = folder / name
        path.write_bytes(content)
        written.append(path)
    return written


def tiff_scan(width: int = 40, height: int = 30) -> bytes:
    """An uncompressed 8-bit RGB TIFF with `DateTimeOriginal` in its Exif IFD,
    orientation 6 and a scanner as camera (Pillow writes no Exif IFD)."""
    pixels = bytes([200, 120, 40]) * (width * height)
    return build(
        "II",
        tiff_header("II", b"*\0"),
        {
            "ifd0": [
                (0x0100, SHORT, width),
                (0x0101, SHORT, height),
                (0x0102, SHORT, [8, 8, 8]),
                (0x0103, SHORT, 1),
                (0x0106, SHORT, 2),
                (0x0111, LONG, BlobRef("pixels")),
                (0x0115, SHORT, 3),
                (0x0116, SHORT, height),
                (0x0117, LONG, BlobLen("pixels")),
                *camera("EPSON", "Perfection V600", 6),
                (0x8769, LONG, IfdRef("_exif")),
            ],
            "_exif": [(0x9003, ASCII, "2024:06:12 09:00:00")],
        },
        {"pixels": pixels},
    )
