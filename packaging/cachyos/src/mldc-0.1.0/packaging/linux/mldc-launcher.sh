#!/usr/bin/env bash

set -euo pipefail

app_bin="/usr/lib/mldc/mldc"

if [[ "${XDG_SESSION_TYPE:-}" == "wayland" ]]; then
  export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
fi

exec "$app_bin" "$@"
