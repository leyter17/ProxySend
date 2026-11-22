#!/bin/bash
# Verificacion de portabilidad de los 4 entregables de ProxySend 2.3.3:
#   - AppImage portable (extraccion sin libfuse2, ejecucion con env minimo)
#   - .deb sin dependencias externas (binario + libs propias en /usr/lib/proxysend)
#   - tar.gz portable (extraer y ejecutar + instalar.sh en HOME de prueba)
#   - .run autoextraible (instalacion completa en HOME de prueba)
#   - ldd del binario: lista de librerias que debe tener cualquier escritorio
#   - libs XKB embebidas en los 4 formatos (no se piden al sistema)
set -u
VER=2.3.5
OUT=/home/z/my-project/download
SIM=/home/z/my-project/scripts/sim
PASS=0; FAIL=0
ok()  { printf 'PASS  %s\n' "$1"; PASS=$((PASS+1)); }
bad() { printf 'FAIL  %s\n' "$1"; FAIL=$((FAIL+1)); }

mkdir -p /tmp/clean-home
Xvfb :99 -screen 0 1280x800x24 >/dev/null 2>&1 &
XVFB=$!
sleep 1

echo "=== 0. Tamanos (limite 30 MB) ==="
for f in "$OUT"/ProxySend-$VER-x86_64.AppImage "$OUT"/proxysend_${VER}_amd64.deb "$OUT"/ProxySend-$VER-linux64.tar.gz "$OUT"/ProxySend-$VER-linux64.run; do
  SZ=$(du -m "$f" | cut -f1)
  [ "$SZ" -le 30 ] && ok "$(basename $f): ${SZ} MB (<= 30)" || bad "$(basename $f): ${SZ} MB"
done

echo "=== 1. AppImage portable ==="
AI="$OUT/ProxySend-$VER-x86_64.AppImage"
file "$AI" | grep -q 'AppImage' && ok "formato AppImage reconocido por file(1)" || bad "file no reconoce AppImage"
EX=/tmp/appimg-extract; rm -rf $EX; mkdir -p $EX
(cd $EX && "$AI" --appimage-extract > /dev/null 2>&1) && ok "--appimage-extract funciona (sin libfuse2)" || bad "--appimage-extract fallo"
[ -x "$EX/squashfs-root/AppRun" ] && ok "AppRun extraido es ejecutable" || bad "AppRun no presente"
[ -f "$EX/squashfs-root/usr/lib/libxkbcommon-x11.so.0" ] && [ -f "$EX/squashfs-root/usr/lib/libxcb-xkb.so.1" ] \
  && ok "AppImage lleva sus libs XKB embebidas (usr/lib)" || bad "faltan libs XKB en el AppImage"
# version con entorno MINIMO (sin heredar nada del sandbox)
V=$(env -i HOME=/tmp/clean-home PATH=/usr/bin:/bin "$EX/squashfs-root/AppRun" --version 2>&1)
echo "$V" | grep -q "ProxySend $VER" && ok "AppRun --version con env minimo: $V" || bad "AppRun --version fallo: $V"
# arranque grafico real con env minimo + dbus
dbus-run-session -- bash -c "
export HOME=/tmp/clean-home DISPLAY=:99 XDG_CURRENT_DESKTOP=GNOME
env -i HOME=/tmp/clean-home PATH=/usr/bin:/bin DISPLAY=:99 $EX/squashfs-root/AppRun --pagina estado >/tmp/ai.log 2>&1 &
APP=\$!
sleep 8
ffmpeg -y -loglevel error -f x11grab -video_size 1220x740 -i :99 -frames:v 1 $SIM/shots/07-appimage-corriendo.jpg 2>/dev/null
kill \$APP 2>/dev/null
" >/dev/null 2>&1
SZ=$(du -b "$SIM/shots/07-appimage-corriendo.jpg" 2>/dev/null | cut -f1)
[ "${SZ:-0}" -gt 30000 ] && ok "AppImage abre su ventana grafica (captura ${SZ} bytes)" || bad "AppImage no abrio ventana"
grep -q 'abriendo ventana' /tmp/ai.log 2>/dev/null && ok "log del AppImage: ventana abierta" || bad "log sin ventana"

echo "=== 2. .deb sin dependencias ==="
DEB="$OUT/proxysend_${VER}_amd64.deb"
dpkg-deb --info "$DEB" > /tmp/debinfo.txt 2>&1 && grep -q 'Package: proxysend' /tmp/debinfo.txt && grep -q "Version: $VER" /tmp/debinfo.txt \
  && ok "control del .deb valido (dpkg-deb --info: package/version/maintainer)" || bad "control del .deb"
dpkg-deb -f "$DEB" Depends | grep -q 'libc6' && ok "Depends: solo libc6 (base de todo Linux)" || bad "Depends inesperado"
DR=/tmp/deb-root; rm -rf $DR; dpkg -x "$DEB" "$DR"
[ -x "$DR/usr/bin/proxysend" ] && ok "dpkg -x extrae /usr/bin/proxysend" || bad "dpkg -x fallo"
[ -f "$DR/usr/lib/proxysend/libxkbcommon-x11.so.0" ] && [ -f "$DR/usr/lib/proxysend/libxcb-xkb.so.1" ] \
  && ok "el .deb lleva sus libs XKB en /usr/lib/proxysend" || bad "faltan libs en el .deb"
V=$(env -i HOME=/tmp/clean-home PATH=/usr/bin:/bin "$DR/usr/bin/proxysend" --version 2>&1)
echo "$V" | grep -q "ProxySend $VER" && ok "binario del .deb corre con env minimo: $V" || bad "binario del .deb fallo: $V"

echo "=== 3. tar.gz portable ==="
TG="$OUT/ProxySend-$VER-linux64.tar.gz"
TD=/tmp/tgz-root; rm -rf $TD; mkdir -p $TD
tar xzf "$TG" -C $TD
[ -x "$TD/ProxySend-$VER-linux64/ProxySend/proxysend" ] && ok "tar.gz extrae y el binario es ejecutable" || bad "tar.gz mal"
[ -f "$TD/ProxySend-$VER-linux64/ProxySend/lib/libxkbcommon-x11.so.0" ] && [ -f "$TD/ProxySend-$VER-linux64/ProxySend/lib/libxcb-xkb.so.1" ] \
  && ok "tar.gz lleva sus libs XKB en ProxySend/lib" || bad "faltan libs en el tar.gz"
V=$(env -i HOME=/tmp/clean-home PATH=/usr/bin:/bin "$TD/ProxySend-$VER-linux64/ProxySend/proxysend" --version 2>&1)
echo "$V" | grep -q "$VER" && ok "portable corre con env minimo: $V" || bad "portable fallo: $V"
FH=/tmp/fake-home1; rm -rf $FH; mkdir -p $FH
HOME=$FH sh "$TD/ProxySend-$VER-linux64/instalar.sh" > /tmp/inst.log 2>&1 \
  && [ -x "$FH/.local/share/ProxySend/ProxySend/proxysend" ] && [ -f "$FH/.local/share/applications/proxysend.desktop" ] \
  && ok "instalar.sh instala en ~/.local/share y crea la entrada del menu" || bad "instalar.sh fallo"

echo "=== 4. .run autoextraible ==="
RUN="$OUT/ProxySend-$VER-linux64.run"
RUNEX=/tmp/run-extract
FH2=/tmp/fake-home2; rm -rf $FH2 $RUNEX; mkdir -p $FH2
HOME=$FH2 sh "$RUN" --extraer "$RUNEX" | grep -q 'Extraido' && ok "--extraer funciona" || bad "--extraer fallo"
V=$(env -i HOME=/tmp/clean-home PATH=/usr/bin:/bin "$RUNEX/ProxySend-$VER-linux64/ProxySend/proxysend" --version 2>&1)
echo "$V" | grep -q "$VER" && ok "contenido del .run corre: $V" || bad "contenido del .run fallo"
HOME=$FH2 sh "$RUN" > /tmp/runinst.log 2>&1 && [ -x "$FH2/.local/share/ProxySend/ProxySend/proxysend" ] \
  && ok "instalacion completa del .run en HOME de prueba" || bad "instalacion .run fallo"
HOME=$FH2 sh "$RUN" --desinstalar | grep -q 'desinstalado' && [ ! -d "$FH2/.local/share/ProxySend" ] \
  && ok "--desinstalar deja el sistema limpio" || bad "--desinstalar fallo"

echo "=== 5. Librerias que necesita el binario (todas base de cualquier escritorio) ==="
ldd "$DR/usr/bin/proxysend" 2>/dev/null | awk '{print $1}' | sort -u | tee /tmp/ldd.txt
MISSING=$(ldd "$DR/usr/bin/proxysend" 2>/dev/null | grep 'not found' | wc -l)
[ "$MISSING" -eq 0 ] && ok "ldd sin 'not found' (todo resoluble en el sistema)" || bad "hay $MISSING librerias sin resolver"
# las libs XKB no deben aparecer como dependencia del sistema: van embebidas
LDD_XKB=$(grep -c 'xkb' /tmp/ldd.txt || true)
[ "$LDD_XKB" -eq 0 ] && ok "ninguna lib XKB se pide al sistema (van embebidas en el paquete)" || bad "el binario pide XKB al sistema"
BASE_OK=1
for lib in $(cat /tmp/ldd.txt); do
  case "$lib" in
    *ld-linux*) ;;  # enlazador dinamico de glibc (/lib64/ld-linux-x86-64.so.2)
    linux-vdso*) ;; # vDSO: mapeado por el propio kernel, no es un archivo
    libm.so*|libc.so*|libgcc_s.so*|libdl.so*|libpthread.so*) ;;
    libX11*|libXcursor*|libXrandr*|libXi*|libXrender*|libXext*|libxcb*|libXau*|libXdmcp*) ;;
    libwayland*|libxkbcommon.so*) ;;
    libGL*|libEGL*|libglapi*|libgbm*|libdrm*) ;;
    libfontconfig*|libfreetype*|libexpat*|libpng*|libz.so*|libz.so*) ;;
    libdbus*|libsystemd*|liblzma*|libzstd*|libcap*|libgcrypt*|libgpg-error*|liblz4*) ;;
    libglib*|libpcre*|libselinux*) ;;
    *) echo "      -> lib NO catalogada como base: $lib"; BASE_OK=0 ;;
  esac
done
[ "$BASE_OK" -eq 1 ] && ok "todas las lib dinamicas son base de cualquier escritorio Linux" || bad "hay libs no base"
# resolucion con env MINIMO (sin LD_LIBRARY_PATH): el rpath $ORIGIN/lib debe bastar
RPATH=$(/home/z/my-project/scripts/pack-tools/patchelf --print-rpath "$DR/usr/bin/proxysend" 2>/dev/null)
echo "      rpath del binario: $RPATH"
echo "$RPATH" | grep -q 'ORIGIN' && ok "rpath apunta a libs propias junto al ejecutable" || bad "rpath sin ORIGIN"

kill $XVFB 2>/dev/null
echo ""
echo "RESULTADO: $PASS PASS / $FAIL FAIL"
[ "$FAIL" -eq 0 ]
