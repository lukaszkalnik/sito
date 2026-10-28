"""Download a few real RAW files per format from raw.pixls.us into samples/raw/.

Run from the repo root:
    uv run scripts/get-raw-samples.py

Only samples the repository marks as CC0 (public domain) are taken: the two
smallest per format, plus one Olympus E-M body. Each download is checked
against its sha256 checksum. The files are not committed (samples/ is git-ignored); they are used by the
ignored test `cargo test --test raw_samples -- --ignored`.
"""

from __future__ import annotations

import hashlib
import json
import re
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "samples" / "raw"
LISTING = "https://raw.pixls.us/json/getrepository.php?set=all"
FORMATS = ["orf", "cr2", "nef", "arw", "rw2", "raf", "dng"]
PER_FORMAT = 2
CC0 = "creativecommons.org/publicdomain/zero/1.0"


@dataclass(frozen=True)
class Sample:
    make: str
    model: str
    url: str
    name: str
    sha256: str
    megabytes: float


def parse(row: list[str]) -> Sample | None:
    make, model, _mode, _mp, _, license_html, _date, file_html, *_ = row
    if CC0 not in license_html:
        return None
    link = re.search(r"href='([^']+)'>([^<]+)</a>", file_html)
    checksum = re.search(r"title='sha256 Checksum'>([0-9a-f]{64})<", file_html)
    size = re.search(r"\(([\d.]+)MB\)", file_html)
    if not (link and checksum and size):
        return None
    return Sample(make, model, link.group(1), link.group(2), checksum.group(1), float(size.group(1)))


def safe_name(name: str) -> str:
    """`name` without characters Windows forbids in file names (e.g. the `:`
    in `(4:3)`, which NTFS would take for an alternate data stream)."""
    return re.sub(r'[<>:"/\|?*]', "-", name)


def download(sample: Sample, path: Path) -> None:
    url = urllib.parse.quote(sample.url, safe=":/")
    with urllib.request.urlopen(url, timeout=300) as response:
        data = response.read()
    if hashlib.sha256(data).hexdigest() != sample.sha256:
        raise ValueError(f"checksum mismatch for {sample.name}")
    path.write_bytes(data)


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(LISTING, timeout=120) as response:
        rows = json.load(response)["data"]
    samples = [s for s in map(parse, rows) if s is not None]
    chosen: list[Sample] = []
    for ext in FORMATS:
        of_format = [s for s in samples if s.name.lower().endswith("." + ext)]
        chosen += sorted(of_format, key=lambda s: s.megabytes)[:PER_FORMAT]
    # The smallest samples are often old cameras; add the smallest Olympus
    # E-M body, the kind of camera Sito's user has.
    olympus = [s for s in samples if s.name.lower().endswith(".orf") and s.model.startswith("E-M")]
    chosen += sorted(olympus, key=lambda s: s.megabytes)[:1]

    sources = []
    for sample in chosen:
        path = OUT / safe_name(sample.name)
        if not path.exists():
            download(sample, path)
            print(f"downloaded {sample.name} ({sample.megabytes} MB)")
        sources.append(
            {
                "file": path.name,
                "camera": f"{sample.make} {sample.model}",
                "url": sample.url,
                "license": "CC0 1.0",
            }
        )
    (OUT / "SOURCES.json").write_text(json.dumps(sources, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
