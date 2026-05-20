#!/usr/bin/env bash
# Publish all 28 @brainwires/* packages to JSR in dependency order.
#
# Usage:
#   ! ./deno/scripts/publish-v0.11.0.sh           # interactive (browser auth)
#   ! ./deno/scripts/publish-v0.11.0.sh --dry-run # dry run all packages
#
# JSR auth: pass --token <jsr_token> as the FIRST arg to skip the browser
# flow, or set JSR_TOKEN and the script will forward it.
#
# Tombstones are published separately from a release/0.10.2-tombstones
# branch — see deno/tombstones/README.md.

set -euo pipefail

cd "$(dirname "$0")/.."

DRY_RUN=""
TOKEN_ARG=""

for arg in "$@"; do
  case "$arg" in
    --dry-run) DRY_RUN="--dry-run --allow-dirty" ;;
    --token=*) TOKEN_ARG="--token=${arg#--token=}" ;;
    --token)   shift; TOKEN_ARG="--token=$1" ;;
  esac
done

if [ -z "$TOKEN_ARG" ] && [ -n "${JSR_TOKEN:-}" ]; then
  TOKEN_ARG="--token=$JSR_TOKEN"
fi

# Dependency tiers — publish in order so each package's deps are already on JSR.
TIER_0=("core")
TIER_1=("a2a" "call-policy" "finetune" "mcp-client" "permission" "provider" \
        "provider-speech" "reasoning" "session" "storage" "telemetry")
TIER_2=("knowledge" "mcp-server" "memory" "prompting" "rag" "stores" "tool-runtime")
TIER_3=("eval" "inference" "mdap" "network" "seal" "skills" "tool-builtins")
TIER_4=("agent")
TIER_TRANSITIONAL=("tools")

publish_pkg() {
  local pkg="$1"
  echo ""
  echo "=== publishing @brainwires/$pkg ==="
  (cd "packages/$pkg" && deno publish $DRY_RUN $TOKEN_ARG)
}

publish_tier() {
  local name="$1"; shift
  echo ""
  echo "### Tier: $name ###"
  for pkg in "$@"; do
    publish_pkg "$pkg"
  done
}

publish_tier "0  (zero deps)"                      "${TIER_0[@]}"
publish_tier "1  (depends on core)"                "${TIER_1[@]}"
publish_tier "2  (depends on tier 1)"              "${TIER_2[@]}"
publish_tier "3  (depends on tier 2)"              "${TIER_3[@]}"
publish_tier "4  (depends on tier 3)"              "${TIER_4[@]}"
publish_tier "transitional barrel"                 "${TIER_TRANSITIONAL[@]}"

echo ""
echo "=== all packages published. tag deno-v0.11.0 next. ==="
echo "  git tag -a deno-v0.11.0 -m 'Deno port v0.11.0' && git push origin deno-v0.11.0"
echo ""
echo "For the 7 tombstones (providers/permissions/agents/mcp/resilience/training/tools)"
echo "at 0.10.2, see deno/tombstones/README.md and use a separate release branch."
