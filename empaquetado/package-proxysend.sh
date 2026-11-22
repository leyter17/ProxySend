#!/bin/bash
# Empaqueta ProxySend en los 4 formatos de descarga (todos < 30 MB).
# Incluye las 2 librerias graficas pequenas (libxkbcommon-x11, libxcb-xkb) para
# que los instaladores corran sin dependencias externas incluso en sistemas X11
# minimalistas; el resto (X11/Wayland, Mesa/GL, glibc) son base de cualquier
# escritorio Linux.
set -e

VER=2.4.0
ROOT=/home/z/my-project/work/fuente
BIN=$ROOT/target/release/proxysend
ICON=$ROOT/assets/icon.png
OUT=/home/z/my-project/download
DIST=$ROOT/dist
TOOLS=/home/z/my-project/scripts/pack-tools
MKSQ=$TOOLS/mksquashfs
RUNTIME=$TOOLS/runtime-x64
PATCHELF=$TOOLS/patchelf
LIBXKB=$TOOLS/libxkbcommon-x11.so.0.0.0
LIBXCBXKB=$TOOLS/libxcb-xkb.so.1.0.0

[ -x "$BIN" ] || { echo "Falta el binario. Compila antes: cargo build --release"; exit 1; }

# rpath del binario: busca librerias junto al ejecutable y en /usr/lib/proxysend
"$PATCHELF" --set-rpath '$ORIGIN/lib:$ORIGIN/../lib:/usr/lib/proxysend' "$BIN"

rm -rf "$DIST" "$OUT"/ProxySend-*.AppImage "$OUT"/proxysend_*.deb "$OUT"/ProxySend-*-linux64.tar.gz "$OUT"/ProxySend-*-linux64.run
mkdir -p "$DIST" "$OUT"

bundle_libs() { # $1=dir-lib
  mkdir -p "$1"
  cp "$LIBXKB" "$1/libxkbcommon-x11.so.0"
  cp "$LIBXCBXKB" "$1/libxcb-xkb.so.1"
  "$PATCHELF" --set-rpath '$ORIGIN' "$1/libxkbcommon-x11.so.0"
}

# ---------------------------------------------------------------------
# 1. AppImage (AppDir + mksquashfs + runtime)
# ---------------------------------------------------------------------
APPDIR=$DIST/AppDir
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" "$APPDIR/usr/share/icons/hicolor/256x256/apps"
cp "$BIN" "$APPDIR/usr/bin/proxysend"
bundle_libs "$APPDIR/usr/lib"
cp "$ICON" "$APPDIR/usr/share/icons/hicolor/256x256/apps/proxysend.png"
cp "$ICON" "$APPDIR/.DirIcon"

cat > "$APPDIR/proxysend.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ProxySend
Comment=Gestor de proxy para Linux con estilo LocalSend
Exec=proxysend
Icon=proxysend
Terminal=false
Categories=Network;Utility;
EOF
cp "$APPDIR/proxysend.desktop" "$APPDIR/usr/share/applications/proxysend.desktop"

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/bash
HERE="$(dirname "$(readlink -f "${0}")")"
exec "${HERE}/usr/bin/proxysend" "$@"
EOF
chmod +x "$APPDIR/AppRun" "$APPDIR/usr/bin/proxysend"

SQUASH=$DIST/proxysend.squashfs
# gzip: compresion soportada por TODOS los runtimes AppImage (tipo 2)
"$MKSQ" "$APPDIR" "$SQUASH" -noappend -comp gzip -all-root >/dev/null
cat "$RUNTIME" "$SQUASH" > "$OUT/ProxySend-$VER-x86_64.AppImage"
chmod +x "$OUT/ProxySend-$VER-x86_64.AppImage"
echo "AppImage listo"

# ---------------------------------------------------------------------
# 2. .deb (ar + tar.gz: control y data)
# ---------------------------------------------------------------------
DEB=$DIST/debroot
mkdir -p "$DEB/DEBIAN" "$DEB/usr/bin" "$DEB/usr/lib/proxysend" "$DEB/usr/share/applications" "$DEB/usr/share/icons/hicolor/256x256/apps" "$DEB/usr/share/doc/proxysend"
cp "$BIN" "$DEB/usr/bin/proxysend"
bundle_libs "$DEB/usr/lib/proxysend"
cp "$ICON" "$DEB/usr/share/icons/hicolor/256x256/apps/proxysend.png"
cp "$APPDIR/proxysend.desktop" "$DEB/usr/share/applications/proxysend.desktop"
chmod 755 "$DEB/usr/bin/proxysend"

SIZE_KB=$(du -sk "$DEB/usr" | cut -f1)
cat > "$DEB/DEBIAN/control" <<EOF
Package: proxysend
Version: $VER
Section: net
Priority: optional
Architecture: amd64
Installed-Size: $SIZE_KB
Maintainer: Leyter Vázquez . V
Depends: libc6 (>= 2.17)
Description: Gestor de proxy para Linux con interfaz al estilo de LocalSend
 ProxySend guarda varios proxies (HTTP, HTTPS y SOCKS5) en una biblioteca
 y los activa a gusto personal en tres niveles: escritorio (GNOME, COSMIC,
 KDE, XFCE), sistema (/etc/environment) y terminales (~/.bashrc, ~/.zshrc).
 Incluye indicador del proxy vigente en la barra lateral y en Estado,
 reconexion automatica con temporizador configurable, exportacion e
 importacion de la biblioteca completa, informe diagnostico con el log
 completo de eventos y errores, bandeja del sistema, autoarranque,
 pruebas de conexion, historial, excepciones por proxy y temas
 claro/oscuro.
 .
 Escrito en Rust: binario pequeno, sin Electron ni dependencias de ejecucion.
Homepage: https://github.com/proxysend
EOF

cat > "$DEB/DEBIAN/postinst" <<'EOF'
#!/bin/sh
if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database >/dev/null 2>&1 || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
exit 0
EOF
cat > "$DEB/DEBIAN/prerm" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod 755 "$DEB/DEBIAN/postinst" "$DEB/DEBIAN/prerm"

cat > "$DEB/usr/share/doc/proxysend/copyright" <<'EOF'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: ProxySend
Source: https://github.com/proxysend

Files: *
Copyright: 2024-2026 Leyter Vázquez . V
License: GPL-3.0-only
 ProxySend es software libre: puedes redistribuirlo y modificarlo bajo
 los terminos de la GNU General Public License, version 3 unicamente,
 publicada por la Free Software Foundation.
 .
 Este programa se distribuye con la esperanza de que sea util, pero
 SIN NINGUNA GARANTIA; sin siquiera la garantia implicita de
 COMERCIABILIDAD o APTITUD PARA UN PROPOSITO PARTICULAR. Consulta la
 GNU General Public License para mas detalles.
 .
 Deberias haber recibido una copia de la GNU General Public License
 junto con este programa; en el codigo fuente va en COPYING.txt. Si
 no fue asi, consulta <https://www.gnu.org/licenses/>.
 .
 En sistemas Debian, el texto completo de la GNU General Public
 License version 3 tambien esta en "/usr/share/common-licenses/GPL-3".
EOF
# texto completo de la GPL-3 (COPYING.txt del codigo fuente), con el
# sangrado de un espacio que exige el formato machine-readable de Debian
echo " ." >> "$DEB/usr/share/doc/proxysend/copyright"
awk '{ if ($0 == "") print " ."; else print " " $0 }' "$ROOT/COPYING.txt" >> "$DEB/usr/share/doc/proxysend/copyright"

cd "$DEB"
# IMPORTANTE: el tar de control debe llevar los archivos en la raiz (sin carpeta DEBIAN/),
# asi lo exigen los instaladores Debian (dpkg -i, apt install).
tar czf "$DIST/control.tar.gz" -C DEBIAN control postinst prerm --owner=0 --group=0
tar czf "$DIST/data.tar.gz" usr --owner=0 --group=0
echo "2.0" > "$DIST/debian-binary"
ar r "$OUT/proxysend_${VER}_amd64.deb" "$DIST/debian-binary" "$DIST/control.tar.gz" "$DIST/data.tar.gz" >/dev/null 2>&1
echo "deb listo"

# ---------------------------------------------------------------------
# 3. Portable .tar.gz
# ---------------------------------------------------------------------
TGZ=$DIST/ProxySend-$VER-linux64
mkdir -p "$TGZ/ProxySend"
cp "$BIN" "$TGZ/ProxySend/proxysend"
bundle_libs "$TGZ/ProxySend/lib"
cp "$ICON" "$TGZ/ProxySend/icono.png"
chmod 755 "$TGZ/ProxySend/proxysend"

cat > "$TGZ/ProxySend/proxysend.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ProxySend
Comment=Gestor de proxy para Linux con estilo LocalSend
Exec=$HOME/.local/share/ProxySend/ProxySend/proxysend
Icon=$HOME/.local/share/ProxySend/ProxySend/icono.png
Terminal=false
Categories=Network;Utility;
EOF

cat > "$TGZ/instalar.sh" <<'EOF'
#!/bin/bash
# Instala ProxySend en ~/.local/share/ProxySend y crea la entrada del menu
set -e
DEST="$HOME/.local/share/ProxySend"
mkdir -p "$DEST"
cp -r "$(cd "$(dirname "$0")" && pwd)/ProxySend" "$DEST/"
chmod 755 "$DEST/ProxySend/proxysend"
sed -i "s|Exec=.*|Exec=$DEST/ProxySend/proxysend|; s|Icon=.*|Icon=$DEST/ProxySend/icono.png|" "$DEST/ProxySend/proxysend.desktop"
mkdir -p "$HOME/.local/share/applications"
cp "$DEST/ProxySend/proxysend.desktop" "$HOME/.local/share/applications/proxysend.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true; fi
echo "ProxySend instalado en $DEST. Buscalo como 'ProxySend' en el menu de aplicaciones."
EOF

cat > "$TGZ/desinstalar.sh" <<'EOF'
#!/bin/bash
rm -rf "$HOME/.local/share/ProxySend"
rm -f "$HOME/.local/share/applications/proxysend.desktop"
if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "$HOME/.local/share/applications" >/dev/null 2>&1 || true; fi
echo "ProxySend desinstalado."
EOF

cat > "$TGZ/LEEME.txt" <<EOF
ProxySend $VER - version portable universal (cualquier Linux x86_64)

Ejecutar sin instalar:
    cd ProxySend-$VER-linux64
    ./ProxySend/proxysend

Instalar con entrada en el menu de aplicaciones:
    sh instalar.sh

Desinstalar:
    sh desinstalar.sh

No necesita ninguna aplicacion externa: solo las librerias base que trae
cualquier escritorio Linux moderno (glibc, X11 o Wayland y OpenGL/Mesa).
Las librerias XKB que usa las lleva consigo en ProxySend/lib.
Funciona en X11 y Wayland (GNOME, COSMIC, KDE, XFCE y otros).

La configuracion se guarda en ~/.config/ProxySend
Documentacion completa en el readme.txt del paquete de descarga.
EOF

chmod +x "$TGZ/instalar.sh" "$TGZ/desinstalar.sh" "$TGZ/ProxySend/proxysend"
tar czf "$OUT/ProxySend-$VER-linux64.tar.gz" -C "$DIST" "ProxySend-$VER-linux64"
echo "tar.gz listo"

# ---------------------------------------------------------------------
# 4. Instalador universal .run (script + tar.gz embebido)
# ---------------------------------------------------------------------
RUNOUT=$OUT/ProxySend-$VER-linux64.run
cat > "$DIST/run-header.sh" <<EOF
#!/bin/bash
# Instalador universal de ProxySend (un solo archivo, sin dependencias)
VER=$VER
DESC="\$HOME/.local/share/ProxySend"

SKIP=\$(awk '/^__PAYLOAD__\$/{print NR+1; exit}' "\$0")

case "\$1" in
  --extraer)
    DIR="\${2:-\$PWD/proxysend-extraido}"
    mkdir -p "\$DIR"
    tail -n +"\$SKIP" "\$0" | tar xzf - -C "\$DIR"
    echo "Extraido en \$DIR"
    exit 0
    ;;
  --desinstalar)
    rm -rf "\$DESC"
    rm -f "\$HOME/.local/share/applications/proxysend.desktop"
    rm -f "\$HOME/.config/autostart/proxysend.desktop"
    if command -v update-desktop-database >/dev/null 2>&1; then update-desktop-database "\$HOME/.local/share/applications" >/dev/null 2>&1 || true; fi
    echo "ProxySend desinstalado."
    exit 0
    ;;
esac

TMP=\$(mktemp -d)
tail -n +"\$SKIP" "\$0" | tar xzf - -C "\$TMP"
sh "\$TMP/ProxySend-\$VER-linux64/instalar.sh"
rm -rf "\$TMP"
echo ""
echo "Listo. Abre ProxySend desde el menu de aplicaciones o ejecuta:"
echo "  \$DESC/ProxySend/proxysend"
exit 0
__PAYLOAD__
EOF

cat "$DIST/run-header.sh" "$OUT/ProxySend-$VER-linux64.tar.gz" > "$RUNOUT"
chmod +x "$RUNOUT"
echo ".run listo"

echo "=== Tamanos ==="
ls -la "$OUT" | grep -E 'AppImage|deb|tar.gz|\.run' | awk '{printf "%8.2f MB  %s\n", $5/1048576, $9}'
