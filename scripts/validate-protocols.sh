#!/usr/bin/env sh
set -eu
# Local subprocess codecs only: never starts a network core or sends packets.
artifact=$(mktemp)
trap 'rm -f "$artifact"' EXIT
cargo run --quiet --locked -p ranterra-engine --example validation_frames > "$artifact"
(cd validation/free5gc && go run -mod=readonly . < "$artifact")

cargo build --quiet --locked -p ranterra-engine --example interop_endpoint
endpoint_path="$(pwd)/target/debug/examples/interop_endpoint"
(cd validation/free5gc && RANTERRA_INTEROP_ENDPOINT="$endpoint_path" go test -mod=readonly -count=1 -timeout=90s -v .)
