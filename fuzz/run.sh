#!/usr/bin/env bash
# Runs one fuzz target with its seeds, dictionary and limits.
# Usage: fuzz/run.sh <target> [seconds]   (default: 600 seconds)
set -euo pipefail

target=${1:?usage: fuzz/run.sh <target> [seconds]}
seconds=${2:-600}
cd "$(dirname "$0")"
fixtures=../fixtures

dict=dict/tiff.dict
# From an input of 64 KB, any single allocation over this is a bug; the
# largest legitimate one is a 64 MB preview block in a file that large.
malloc_limit_mb=256
timeout=10
case "$target" in
  tiff_reader) seeds=("$fixtures/tiff" "$fixtures/raw") ;;
  raw_preview) seeds=("$fixtures/raw" "$fixtures/exif") ;;
  metadata) seeds=("$fixtures/tiff" "$fixtures/raw" "$fixtures/exif" "$fixtures/cameras") ;;
  orientation) seeds=("$fixtures/exif" "$fixtures/cameras") ;;
  sidecar)
    seeds=("$fixtures/takeout")
    dict=dict/json.dict
    ;;
  decode)
    seeds=("$fixtures/exif" "$fixtures/cameras" "$fixtures/tiff")
    # Just above the app's TIFF memory limit (decode::TIFF_MEMORY_LIMIT).
    malloc_limit_mb=1100
    # A JPEG up to decode::JPEG_MEMORY_LIMIT (about 170 MP) is legitimate and
    # takes about 25 seconds under the fuzzer's instrumentation; only an input
    # beyond that is a hang. Its inputs are decoded, never skipped.
    timeout=60
    ;;
  *)
    echo "unknown target: $target" >&2
    exit 2
    ;;
esac

# The first corpus folder is where the fuzzer keeps what it finds; the
# fixtures are only read.
mkdir -p "corpus/$target"
exec cargo +nightly fuzz run "$target" "corpus/$target" "${seeds[@]}" -- \
  -dict="$dict" \
  -max_total_time="$seconds" \
  -timeout="$timeout" \
  -max_len=65536 \
  -rss_limit_mb=2048 \
  -malloc_limit_mb="$malloc_limit_mb"
