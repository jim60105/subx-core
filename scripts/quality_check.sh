#!/usr/bin/env bash
# subx-core/scripts/quality_check.sh
#
# The quality gate for subx-core standing on its own — the six invocations
# its own CI runs. Deliberately small: the superproject's
# scripts/quality_check.sh is a 356-line multi-OS script with shell-completion
# and doc-coverage extras; this crate needs none of that, and keeping one
# definition of the check set here (invoked through bash on every OS) is what
# makes the three test matrix entries actually equivalent.
#
# Usage: scripts/quality_check.sh [nextest-profile]   (default: default)

set -euo pipefail
cd "$(dirname "$0")/.."

PROFILE="${1:-default}"

cargo fmt -- --check
cargo check --all-features
cargo clippy --all-features -- -D warnings
cargo doc --all-features --no-deps --document-private-items
cargo test --doc --all-features
cargo nextest run --profile "$PROFILE" --features slow-tests
