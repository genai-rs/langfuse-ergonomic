#!/usr/bin/env bash
# Run the live tests against a throwaway self-hosted Langfuse v4 (docker/langfuse/compose.yml).
#
# Usage: scripts/live-tests.sh [all|up|test|down] [v3|v4 ...]
#   all  (default) start the stack, run the live tests for each API version, stop the stack
#   up   start the stack and wait until it is healthy
#   test run the live tests against a running stack
#   down stop the stack and delete its data
# API versions default to "v3 v4". Set KEEP_STACK=1 to leave the stack running after `all`.
# LANGFUSE_IMAGE_TAG overrides the pinned Langfuse version, LANGFUSE_PORT the host port (3000).
set -euo pipefail

cd "$(dirname "$0")/.."
compose=(docker compose -f docker/langfuse/compose.yml)
port="${LANGFUSE_PORT:-3000}"
base_url="http://localhost:${port}"
live_tests=(--test integration_test --test generation_usage_e2e)

up() {
  "${compose[@]}" up -d --quiet-pull
  echo "Waiting for Langfuse at ${base_url} ..."
  local deadline=$((SECONDS + ${LANGFUSE_STARTUP_TIMEOUT:-300}))
  # The public health endpoint answers once migrations ran; the authenticated project call
  # proves the headless initialization created the test keys.
  until curl -fsS -u pk-lf-live-test:sk-lf-live-test "${base_url}/api/public/projects" >/dev/null 2>&1; do
    if ((SECONDS > deadline)); then
      echo "Langfuse did not become ready in time" >&2
      "${compose[@]}" ps >&2
      "${compose[@]}" logs --tail 100 langfuse-web langfuse-worker >&2
      exit 1
    fi
    sleep 2
  done
  echo "Langfuse is ready"
}

run_tests() {
  (($#)) || set -- v3 v4
  for version in "$@"; do
    echo "=== Live tests with LANGFUSE_API_VERSION=${version} ==="
    LANGFUSE_PUBLIC_KEY=pk-lf-live-test \
      LANGFUSE_SECRET_KEY=sk-lf-live-test \
      LANGFUSE_BASE_URL="${base_url}" \
      LANGFUSE_API_VERSION="${version}" \
      cargo test "${live_tests[@]}" -- --ignored --test-threads 4
  done
}

down() {
  "${compose[@]}" down -v --remove-orphans
}

command="${1:-all}"
(($#)) && shift
case "${command}" in
  up) up ;;
  test) run_tests "$@" ;;
  down) down ;;
  all)
    [[ "${KEEP_STACK:-0}" == 1 ]] || trap down EXIT
    up
    run_tests "$@"
    ;;
  *)
    echo "usage: $0 [all|up|test|down] [v3|v4 ...]" >&2
    exit 2
    ;;
esac
