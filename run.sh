#!/usr/bin/env bash
# Build and bring up the SocialLink stack (ui + api + mongo).
#
# Usage:
#   ./run.sh [up|down|stop|restart|logs|ps] [extra compose args...]
#
# Options:
#   --foreground   Run "up" in the foreground (stream logs) instead of detached.
#   --no-build     Bring services up without rebuilding images.
#   -h, --help     Show this help.
#
# Env overrides:
#   COMPOSE_CMD    Force a compose command, e.g. "docker compose".
#   COMPOSE_FILE   Force a compose file path.
#   UI_PORT        Host port the UI is published on (default 3000).
#   UI_TLS_ENABLED Serve the UI over HTTPS/HTTP-2 (default false). When true,
#                  put a cert/key in ./certs (or set UI_CERT_DIR). See
#                  DEVELOPMENT.md and certs/README.md.
set -euo pipefail

cd "$(dirname "$0")"

UI_PORT="${UI_PORT:-3000}"
# Scheme shown in the post-up hints; the UI serves https when TLS is enabled.
case "${UI_TLS_ENABLED:-false}" in
  1|true|TRUE|yes|on) UI_SCHEME="https" ;;
  *) UI_SCHEME="http" ;;
esac

usage() {
  awk 'NR==1 { next } /^#/ { sub(/^# ?/, ""); print; next } { exit }' "$0"
}

# Pick the first available compose runtime.
detect_compose() {
  if [ -n "${COMPOSE_CMD:-}" ]; then printf '%s' "$COMPOSE_CMD"; return 0; fi
  if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
    printf 'docker compose'; return 0
  fi
  if command -v docker-compose >/dev/null 2>&1; then printf 'docker-compose'; return 0; fi
  if command -v podman >/dev/null 2>&1 && podman compose version >/dev/null 2>&1; then
    printf 'podman compose'; return 0
  fi
  if command -v podman-compose >/dev/null 2>&1; then printf 'podman-compose'; return 0; fi
  return 1
}

CMD="up"
DETACH="-d"
BUILD=1
FILE="${COMPOSE_FILE:-docker-compose.yml}"
EXTRA=()

while [ $# -gt 0 ]; do
  case "$1" in
    up|down|stop|restart|logs|ps) CMD="$1" ;;
    --foreground) DETACH="" ;;
    --no-build) BUILD=0 ;;
    -h|--help) usage; exit 0 ;;
    *) EXTRA+=("$1") ;;
  esac
  shift
done

if ! COMPOSE="$(detect_compose)"; then
  echo "error: no compose runtime found." >&2
  echo "       install one of: docker (compose v2), docker-compose, podman compose, podman-compose" >&2
  exit 1
fi

if [ ! -f "$FILE" ]; then
  echo "error: compose file not found: $FILE" >&2
  exit 1
fi

echo ">> compose: $COMPOSE  (file: $FILE)"

# shellcheck disable=SC2086
run() { echo "+ $COMPOSE -f $FILE $*"; $COMPOSE -f "$FILE" "$@"; }

case "$CMD" in
  up)
    [ "$BUILD" -eq 1 ] && run build ${EXTRA[@]+"${EXTRA[@]}"}
    if [ -n "$DETACH" ]; then
      run up -d ${EXTRA[@]+"${EXTRA[@]}"}
      echo
      echo ">> stack is up."
      echo "   UI:    ${UI_SCHEME}://localhost:${UI_PORT}"
      echo "   Admin: ${UI_SCHEME}://localhost:${UI_PORT}/admin/login"
      echo "          credentials come from config/local/social-link.yaml and secret env vars"
      echo ">> follow logs:  ./run.sh logs -f"
      echo ">> tear down:    ./run.sh down"
    else
      run up ${EXTRA[@]+"${EXTRA[@]}"}
    fi
    ;;
  down)    run down ${EXTRA[@]+"${EXTRA[@]}"} ;;
  stop)    run stop ${EXTRA[@]+"${EXTRA[@]}"} ;;
  restart) run restart ${EXTRA[@]+"${EXTRA[@]}"} ;;
  logs)    run logs ${EXTRA[@]+"${EXTRA[@]}"} ;;
  ps)      run ps ${EXTRA[@]+"${EXTRA[@]}"} ;;
esac
