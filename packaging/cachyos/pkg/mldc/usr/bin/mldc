#!/usr/bin/env bash

set -euo pipefail

app_bin="/usr/lib/mldc/mldc"
skip_root_prompt="${MLDC_NO_ROOT_PROMPT:-0}"

if [[ "${XDG_SESSION_TYPE:-}" == "wayland" ]]; then
  export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
fi

if [[ "${EUID}" -ne 0 && "${skip_root_prompt}" != "1" && "${MLDC_DEV_MODE:-}" != "fake" && -z "${MLDC_LAUNCHED_VIA_PKEXEC:-}" ]]; then
  if command -v pkexec >/dev/null 2>&1; then
    if pkexec env \
      MLDC_LAUNCHED_VIA_PKEXEC=1 \
      WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-}" \
      DISPLAY="${DISPLAY:-}" \
      WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-}" \
      XAUTHORITY="${XAUTHORITY:-}" \
      XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-}" \
      DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-}" \
      XDG_CURRENT_DESKTOP="${XDG_CURRENT_DESKTOP:-}" \
      DESKTOP_SESSION="${DESKTOP_SESSION:-}" \
      XDG_SESSION_TYPE="${XDG_SESSION_TYPE:-}" \
      "$app_bin" "$@"; then
      exit 0
    fi
  fi
fi

exec "$app_bin" "$@"
