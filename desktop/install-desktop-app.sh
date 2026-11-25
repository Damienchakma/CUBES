#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BINARY="$ROOT/target/debug/vela-editor"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"

if [[ ! -x "$BINARY" ]]; then
  echo "Building CUBES Editor..."
  (cd "$ROOT" && cargo build -p vela-editor)
fi

mkdir -p "$DATA_DIR/applications" "$DATA_DIR/icons" "$BIN_DIR"

cat > "$BIN_DIR/cubes-editor-desktop" <<EOF
#!/usr/bin/env bash
exec "$BINARY" "\$@"
EOF
cp "$BIN_DIR/cubes-editor-desktop" "$BIN_DIR/vela-editor-desktop"
chmod +x "$BIN_DIR/cubes-editor-desktop" "$BIN_DIR/vela-editor-desktop"

cp "$ROOT/desktop/cubes-dark.svg" "$DATA_DIR/icons/cubes-editor.svg"
cp "$ROOT/desktop/vela-editor.svg" "$DATA_DIR/icons/vela-editor.svg"

cat > "$DATA_DIR/applications/cubes-editor.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=CUBES Editor
GenericName=3D Creation Studio
Comment=Create games and animations with CUBES
Exec=$BIN_DIR/cubes-editor-desktop
Icon=$DATA_DIR/icons/cubes-editor.svg
Terminal=false
Categories=Development;
Keywords=cubes;vela;game;engine;3d;editor;animation;
StartupNotify=true
EOF

cat > "$DATA_DIR/applications/vela-editor.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Vela Editor (legacy)
GenericName=3D Scene Editor
Comment=Legacy launcher for the CUBES Editor
Exec=$BIN_DIR/vela-editor-desktop
Icon=$DATA_DIR/icons/vela-editor.svg
Terminal=false
Categories=Development;
Keywords=vela;cubes;legacy;3d;editor;
StartupNotify=true
EOF

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$DATA_DIR/applications" >/dev/null 2>&1 || true
fi

echo "Installed CUBES Editor to $DATA_DIR/applications/cubes-editor.desktop"
echo "Legacy Vela Editor launcher preserved at $DATA_DIR/applications/vela-editor.desktop"
echo "Launch it from your desktop application menu as 'CUBES Editor'."
