#!/usr/bin/env sh
set -eu
seconds=${1:-60}
case "$seconds" in ''|*[!0-9]*) echo 'Duration must be 1–3600 seconds' >&2; exit 1;; esac
[ "$seconds" -ge 1 ] && [ "$seconds" -le 3600 ]
mkdir -p validation/fuzz/corpus/protocols
python3 - <<'PY'
import json
from pathlib import Path
for i, frame in enumerate(json.loads(Path('validation/fuzz/seeds.json').read_text())):
    Path(f'validation/fuzz/corpus/protocols/seed-{i}').write_bytes(bytes.fromhex(frame['hex']))
PY
cargo +nightly-2026-07-20 fuzz run --fuzz-dir validation/fuzz protocols -- -max_total_time="$seconds" -seed=501 -max_len=65536 -rss_limit_mb=2048 -timeout=5
