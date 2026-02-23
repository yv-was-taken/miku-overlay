# Miku Sway Overlay

Animated Hatsune Miku sprite overlay for Sway/Wayland, positioned over the Helium browser's tab bar.

## Why

The "Sleeping Hatsune Miku AnimEdit" Firefox theme displays an animated Miku APNG sprite in the tab bar. Helium (Chromium-based) can't animate theme images (Chromium decodes to a single static `SkBitmap`). This overlay renders the animation natively via a wlr-layer-shell surface, tracked to Helium's window position via Sway IPC.

## How it works

- **APNG decoding**: `image` crate decodes the 22-frame APNG into pre-composited RGBA textures
- **Custom GdkPaintable**: GObject subclass drives frame-by-frame animation via `glib::timeout_add_local_once`
- **Layer-shell overlay**: GTK4 window using `gtk4-layer-shell` at the Overlay layer, click-through (empty input region)
- **Sway IPC tracking**: Background thread subscribes to window events, positions overlay at Helium's top-right corner
- **Fallback poll**: Re-queries sway tree every 2s to catch tiling changes that don't fire window events

## Install

```sh
nu install.nu
```

This builds the release binary, installs it to `~/.local/bin/miku-overlay`, and adds an `exec` line to your sway config so it starts on login. Reload sway or log out/in to activate.

## Usage

```sh
# Run directly (development)
cargo run

# Or build and run the release binary
cargo build --release
./target/release/miku-overlay
```

The overlay will:
- Appear when Helium is open (top-right of its window)
- Follow Helium when tiled/moved/resized
- Hide when Helium is fullscreen or closed
- Reappear when Helium exits fullscreen or reopens

## Sprite

`assets/miku_sprite.png` — 240x60 APNG, 22 frames, ~30fps, infinite loop. Sourced from the [Sleeping Hatsune Miku AnimEdit](https://addons.mozilla.org/en-US/firefox/addon/sleeping-hatsune-miku-animedit/) Firefox theme.

## Limitations

- Only supports Helium's **compact** tab layout. Classic and vertical layouts are not supported.

## Dependencies

- Rust stable
- GTK4 + `gtk4-layer-shell` (C library, install via `sudo pacman -S gtk4-layer-shell`)
- Sway (for IPC)
