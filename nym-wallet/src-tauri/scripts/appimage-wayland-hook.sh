#!/usr/bin/env bash
# Reference only: Tauri AppImage bundling does not place /apprun-hooks/ into the final image (only /usr/ is staged).
# Wayland defaults and WEBKIT_DISABLE_DMABUF_RENDERER are applied in src/main.rs (configure_linux_wayland_defaults).

if [ -z "${WAYLAND_DISPLAY:-}" ]; then
  return 0 2>/dev/null || exit 0
fi

if [ -z "${LD_PRELOAD:-}" ]; then
  for lib_path in \
    /usr/lib/libwayland-client.so \
    /usr/lib64/libwayland-client.so \
    /usr/lib/x86_64-linux-gnu/libwayland-client.so
  do
    if [ -f "$lib_path" ]; then
      export LD_PRELOAD="$lib_path"
      break
    fi
  done
fi

export GDK_BACKEND="${GDK_BACKEND:-wayland}"
export GDK_SCALE="${GDK_SCALE:-1}"
export GDK_DPI_SCALE="${GDK_DPI_SCALE:-0.8}"

# Reduces WebKit DMA-BUF / EGL failures on some rolling Mesa + Wayland stacks. Set WEBKIT_DISABLE_DMABUF_RENDERER=0 to opt out.
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
