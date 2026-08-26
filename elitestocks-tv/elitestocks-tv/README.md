# EliteStocks TV

A Windows desktop IPTV player built with **Tauri 2** (Rust) + a Netflix-style
HTML/CSS/JS UI, signing in with **Xtream Codes** and playing everything through
a real, embedded **mpv** (with the [uosc](https://github.com/tomasklaen/uosc)
on-screen controller, themed to the app's orange/black look).

Categories: **Live TV**, **Movies**, **TV Shows** (no games/other extras).

## How it's built

- `src/` — the app UI: custom black titlebar, Xtream Codes sign-in screen, and
  a Netflix-style browse screen (hero banner, horizontal rows, category rail
  for Live TV, a details modal with season/episode picker for series).
- `src-tauri/` — the Rust backend:
  - `xtream.rs` — talks to your Xtream Codes `player_api.php` (login,
    categories, live/VOD/series listings, series info) over plain HTTP.
  - `mpv.rs` — launches `mpv.exe` full-screen and drives it over its JSON IPC
    protocol (play/pause/seek/track switching all go through this).
  - `main.rs` — wires both up as Tauri commands the frontend calls.
- `mpv-config/` — the mpv/uosc "portable_config" (skin + keybindings: 10s
  skip, subtitle/audio menus, playlist-driven "next episode") that CI copies
  next to the bundled `mpv.exe`.
- `.github/workflows/build-windows.yml` — builds the whole thing on a
  `windows-latest` GitHub Actions runner: installs Rust/Node, pulls mpv via
  Chocolatey, downloads uosc, assembles `src-tauri/mpv/`, then runs
  `tauri build` to produce an NSIS installer (`.exe`) and an MSI.

## Why mpv runs as its own full-screen window (not embedded pixel-for-pixel
into the WebView)

Truly compositing a native video surface *underneath* a transparent WebView2
layer requires low-level COM/Win32 work that's brittle and effectively
impossible to verify without iterating on a real Windows machine. Instead this
app uses the same proven approach as mpv-based players like mpv.net/Celluloid:
clicking Play hides the app window and launches mpv full-screen with its own
GPU-accelerated render surface, using **uosc** for the on-screen UI (progress
bar with a red/orange playhead, skip ±10s, play/pause, Episodes/playlist menu,
Audio & Subtitles menus, Next Episode) styled to match the layout you shared.
When you back out of mpv, the app window reappears automatically. Functionally
this gives you the same experience shown in your reference screenshot, using
real mpv playback (all codecs, subtitle formats, and hardware decoding it
supports) rather than a browser `<video>` tag.

If you'd rather have pixel-perfect HTML controls drawn *over* a truly embedded
video surface, that's a follow-up project (WebView2 transparent-background +
Win32 child-window reparenting) best done and tested directly on a Windows
box — I can help build that iteratively if you want to go there next.

## Building it yourself

You don't need a Windows machine — push this repo to GitHub and either:

1. Push to `main`, or
2. Go to **Actions → Build EliteStocks TV (Windows) → Run workflow**.

The workflow uploads `EliteStocksTV-windows-installers` (contains the `.exe`
NSIS installer and `.msi`) as a build artifact you can download and run.

### Building locally (if you do have Windows)

```powershell
# Prereqs: Rust (rustup, MSVC toolchain), Node.js 20+, mpv on PATH or copied
# into src-tauri/mpv/mpv.exe with a portable_config folder (see mpv-config/).
npm install
npm run dev     # dev mode
npm run build   # produces the installer in src-tauri/target/release/bundle/
```

## Notes

- **Credentials**: "Remember me" stores your Xtream Codes server/username/
  password in the OS-local browser storage of the app (not synced anywhere).
  Uncheck it if you don't want that.
- **Theming**: further tweak colors/layout in `src/css/style.css` (app chrome)
  and `mpv-config/script-opts/uosc.conf` (in-player controls) — see uosc's own
  README for the full, version-matched option list.
- **Content responsibility**: this app is a generic Xtream Codes client, like
  many IPTV players. It doesn't include or endorse any specific content
  provider — you're responsible for only connecting it to services you're
  authorized to use.
