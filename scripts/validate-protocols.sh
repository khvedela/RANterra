#!/usr/bin/env sh
set -eu
# Local subprocess codecs only: never starts a network core or sends packets.
artifact=$(mktemp)
trap 'rm -f "$artifact"' EXIT
cargo run --quiet --locked -p ranterra-engine --example validation_frames > "$artifact"
(cd validation/free5gc && go run -mod=readonly . < "$artifact")
