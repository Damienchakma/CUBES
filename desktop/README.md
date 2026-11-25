# CUBES desktop launcher

CUBES Editor is a native desktop application. This folder adds double-clickable launchers for XFCE
and other freedesktop-compatible desktops, plus the cube visual identity.

Install or refresh with:

```bash
desktop/install-desktop-app.sh
```

The script installs:

- `CUBES Editor` with the new cube icon.
- `Vela Editor (legacy)` pointing at the same binary so existing habits and launchers keep working.
- Launchers in `~/.local/bin` for both names.

Find **CUBES Editor** in your application menu. With no arguments it opens a welcome screen. To open
a project from a terminal:

```bash
~/.local/bin/cubes-editor-desktop --project /path/to/project
```

The launcher points at `target/debug/vela-editor`. Rerun `cargo build -p vela-editor` after code
changes; no reinstall is needed.
