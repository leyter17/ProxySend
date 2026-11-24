# ProxySend
Gestor de proxy para Linux
=====================================================================
 
SI TE GUSTA ESTE PROYECTO Y QUIERES AYUDARNOS A SEGUIR DESARROLLANDO POR FAVOR DEJA UNA DONACION EN BITCOIN A ESTA DIRECCION:
bc1qac0vp996wqaq8z2gujyzcg7699lcmmtul8e8qtrkqdk4pg48wq8s3nj8hh

---------------------------------------------------------------------
1. QUE ES PROXYSEND
---------------------------------------------------------------------

ProxySend es una aplicacion de escritorio para Linux que te deja
cambiar el proxy de tu PC con un clic, con una interfaz inspirada en
LocalSend: barra lateral con navegacion, tarjetas redondeadas, tema
claro/oscuro y un boton grande redondo para activar o desactivar.

La idea es simple:

  - Agregar un proxy (HTTP, HTTPS o SOCKS5) cuando lo necesites.
  - Guardar todos los proxies que uses en una biblioteca y activar
    el que quieras, a gusto personal, sin volver a escribir nada.
  - Activarlo o desactivarlo cuando quieras, sin tocar archivos a mano.
  - Quitarlo por completo y dejar los valores por defecto del sistema.
  - Elegir DONDE se aplica: escritorio, sistema y/o terminales.

La app no navega por Internet ni envia datos a ningun servidor: solo
cambia la configuracion de tu equipo.

---------------------------------------------------------------------
2. QUE PUEDE HACER (funciones)
---------------------------------------------------------------------

  * Activar y desactivar el proxy con un clic desde el boton grande
    de la pantalla "Estado" (se pone azul cuando esta activo).

  * INDICADOR DEL PROXY VIGENTE: siempre visible en la barra lateral
    (punto verde + nombre del proxy activo y su direccion, o "Sin
    proxy - conexion directa" si no hay ninguno) y en la tarjeta
    "Proxy vigente en este momento" de la pantalla Estado. Sabras en
    todo momento que proxy esta vigente, aunque la app acabe de
    abrirse o la active desde la bandeja.

  * BIBLIOTECA DE PROXIES: pantalla "Mis proxies" donde almacenas
    varios proxies con nombre (casa, trabajo, VPN...) y activas el que
    quieras con un boton. Cada uno se puede Editar, Probar y Eliminar,
    y el que esta aplicado se marca con "En uso". Solo hay uno activo
    a la vez. El boton "Nuevo proxy" limpia el formulario para agregar
    otro.

  * Agregar un proxy nuevo o quitarlo y dejar los valores por defecto
    del sistema con el boton "Restablecer valores por defecto"
    (la biblioteca de proxies se conserva).

  * Protocolos HTTP, HTTPS y SOCKS5, con selector tipo segmentado y
    opcion "usar el mismo proxy para todos los protocolos" o uno
    distinto para HTTP y HTTPS.

  * Proxies con usuario y contraseña (los guarda solo en tu perfil
    local, no se envian a ningun sitio).

  * Tres niveles de aplicacion, activables por separado con
    interruptores en la pantalla Estado:

      1) Escritorio: cambia los ajustes de red del escritorio.
         - GNOME (y escritorios con gsettings): usa el esquema
           org.gnome.system.proxy. Chrome, Firefox, Chromium y las
           apps de GNOME lo aplican al instante.
         - COSMIC (System76): el escritorio se detecta automaticamente
           y usa el mismo mecanismo gsettings; sus aplicaciones (GTK)
           respetan el proxy al instante. COSMIC todavia no trae
           ajuste de proxy propio: ProxySend lo suple, con un aviso
           especifico en la pantalla Estado.
         - KDE: escribe el archivo kioslaverc de KIO (ProxyType,
           httpProxy, httpsProxy, NoProxyFor), el mismo que usa el
           modulo de proxy de Plasma; las apps de KDE lo leen al
           abrirse. SOCKS5 no se guarda en kioslaverc: usa los niveles
           Sistema o Terminal para SOCKS5.
         - XFCE (y LXQt/LXDE): usan el mismo mecanismo gsettings: las
           apps GTK y Firefox lo respetan a traves de GIO, porque
           XFCE no trae ajustes de proxy propios (comprobado en la
           documentacion de Xfce). El nivel muestra "Escritorio
           (XFCE)" con un consejo especifico.
         - Otros escritorios: si existe gsettings se usa; si no, el
           nivel aparece como "no disponible" con una explicacion y el
           resto de niveles sigue funcionando. El nivel muestra siempre
           el nombre del escritorio detectado (GNOME, COSMIC, KDE,
           XFCE...).

      2) Sistema (/etc/environment): escribe http_proxy, https_proxy,
         no_proxy (y sus variantes en mayusculas, mas all_proxy para
         SOCKS5) para todo el equipo. Pide tu contraseña una vez
         (por polkit/pkexec) y surte efecto al volver a iniciar sesion.

      3) Terminales (~/.bashrc): agrega un bloque claramente marcado
         con "export http_proxy=..." que las terminales nuevas leen al
         instante. Si existe ~/.zshrc tambien lo escribe ahi.

  * Excepciones: lista de direcciones que NO pasan por el proxy
    (por defecto: localhost, 127.0.0.0/8, ::1). Editable, una por
    linea o separadas por coma.

  * Compatibilidad universal: funciona en cualquier distribucion
    Linux moderna (Debian, Ubuntu, Mint, Pop!_OS, Fedora, Arch,
    openSUSE, Void, NixOS...) en sesiones X11 y Wayland (incluido
    COSMIC). Con soporte para GNOME, COSMIC, KDE, XFCE, MATE,
    Cinnamon, Budgie y otros.

  * Probar el proxy: botones "Probar proxy" y "Probar sin proxy" que
    consultan https://api.ipify.org con el motor de red interno de la
    app (TLS incluido, en Rust puro) y te muestran la IP publica de
    salida, el codigo HTTP y la latencia, con curl como respaldo.
    Tambien puedes probar cualquier proxy de la biblioteca sin
    activarlo.

  * BANDEJA DEL SISTEMA: icono en la bandeja con menu para mostrar la
    app, ACTIVAR o DESACTIVAR el proxy y salir, sin abrir la ventana.
    El menu responde SIEMPRE (verificado a fondo en v2.3.4), la app
    avisa del resultado de Activar/Desactivar y Salir cierra el proceso
    AL INSTANTE. Implementado con StatusNotifierItem sobre D-Bus (sin
    libappindicator ni GTK). "Mostrar ProxySend" des-minimiza de verdad
    la ventana en X11 y la reabre en primer plano en Wayland. La app ya
    NO se oculta en la bandeja al cerrar ni al minimizar (esas opciones
    se retiraron en v2.3.4): cerrar cierra y minimizar solo minimiza.

  * AUTOARRANQUE: casilla en Ajustes > Preferencias para que ProxySend
    se abra al iniciar sesion y vuelva a aplicar el proxy guardado al
    escritorio y a las terminales. Crea el lanzador en
    ~/.config/autostart/proxysend.desktop. El nivel Sistema no
    necesita re-aplicarse al arrancar (ya es permanente), por eso no
    pide contraseña al inicio.

  * RECONEXION AUTOMATICA (temporizador): casilla en Ajustes (abajo de
    Preferencias) para que ProxySend verifique periodicamente que el
    proxy activo sigue funcionando (cada 1, 5, 10, 30 o 60 minutos, a tu
    eleccion). Si el proxy deja de responder, la app lo vuelve a aplicar
    sola en los niveles escritorio y terminales, lo anota en el registro
    y avisa en la pantalla Estado ("Reconexion automatica: ...").
    En Estado, la tarjeta del proxy vigente muestra la linea con la
    ultima verificacion, y el boton "Verificar ahora" hace la comprobacion
    al momento. El nivel Sistema no se repite al reconectar:
    /etc/environment ya es permanente y repetirlo pediria contrasena.

  * EXPORTACION DE LA BIBLIOTECA COMPLETA: botones "Exportar biblioteca"
    e "Importar biblioteca" en la pantalla "Mis proxies". El archivo
    ~/ProxySend-biblioteca-<fecha>.json incluye TODOS los perfiles
    guardados, los ajustes (niveles, tema, autoarranque,
    reconexion automatica, idioma), el perfil activo y el historial.
    Importarla restaura los perfiles y ajustes; NO cambia el proxy que
    esta aplicado en el sistema (despues activas el que quieras). Si el
    archivo se importa dos veces, el historial no se duplica.

  * BOTON DE INFORME CON LOG COMPLETO: nueva pantalla "Informe" con:
      - Resumen del diagnostico: cuantas informaciones, avisos y errores
        hay registrados, el ultimo error y el escritorio detectado.
      - "Generar informe completo": escribe
        ~/ProxySend-informe-<fecha>.txt con 10 secciones: sistema y
        escritorio, herramientas externas presentes, estado de los tres
        niveles, configuracion vigente, biblioteca de proxies,
        reconexion automatica, variables de entorno, resumen de errores,
        registro completo de eventos e historial. Sin contrasenas.
      - Registro de eventos en vivo con filtro "Todos" / "Avisos y
        errores"; cada evento tiene nivel, categoria y fecha.
    Todo lo que sucede en la app queda anotado en
    ~/.config/ProxySend/eventos.log (tambien los errores inesperados,
    capturados aunque la app se cierre).

  * Historial: registro con fecha y hora de cada activacion,
    desactivacion, restablecimiento, guardado en la biblioteca,
    prueba, exportacion e importacion. Con boton para limpiarlo.

  * Exportar / Importar la configuracion en un archivo JSON para
    copiarla a otra maquina o hacer copia de seguridad. Exportar
    guarda en ~/proxysend-config.json; Importar abre un selector de
    archivos si tu sistema tiene zenity o kdialog (si no, busca el
    mismo archivo de exportacion). Tambien existe el comando
    "proxysend --importar archivo.json".

  * Tema claro y oscuro con interruptor en la parte baja de la barra
    lateral (se recuerda entre sesiones).

  * Copia de seguridad automatica: la primera vez que ProxySend
    escribe /etc/environment crea /etc/environment.proxysend.bak con
    el contenido original.

---------------------------------------------------------------------
3. INSTALACION (elige la opcion para tu sistema)
---------------------------------------------------------------------

3.1 Instalador universal de UN solo archivo (cualquier Linux)
    [RECOMENDADO: cero dependencias]

    No requiere root, ni libfuse2, ni instalar paquetes: todo lo que
    la app necesita va dentro del archivo .run.

    1) Ejecutalo (doble clic > Ejecutar, o desde la terminal):

         sh ProxySend-2.4.0-linux64.run

    2) Listo: aparece "ProxySend" en el menu de aplicaciones y la
       carpeta queda en ~/.local/share/ProxySend.

    Otras opciones del instalador:

         sh ProxySend-2.4.0-linux64.run --extraer DIR
             (solo extraer los archivos en DIR, modo portatil)
         sh ProxySend-2.4.0-linux64.run --desinstalar
             (quitar la aplicacion y la entrada del menu)

3.2 Paquete portable universal (.tar.gz, cualquier Linux)

    Extraer y ejecutar, sin instalar nada:

         tar xzf ProxySend-2.4.0-linux64.tar.gz
         cd ProxySend-2.4.0-linux64
         ./ProxySend/proxysend

    Opcional: sh instalar.sh crea la entrada del menu y
    sh desinstalar.sh la quita. El LEEME.txt incluido lo explica todo.

3.3 Version portatil (AppImage)

    1) Dale permisos de ejecucion (solo la primera vez):

         chmod +x ProxySend-2.4.0-x86_64.AppImage

    2) Ejecutala con doble clic o desde la terminal:

         ./ProxySend-2.4.0-x86_64.AppImage

    Si tu sistema no trae libfuse2 (error "AppImages require FUSE"),
    lanzala asi (comprobado: funciona sin instalar nada):

         APPIMAGE_EXTRACT_AND_RUN=1 ./ProxySend-2.4.0-x86_64.AppImage

    o usa las opciones 3.1 / 3.2, que no necesitan FUSE para nada.

3.4 Paquete .deb (Debian, Ubuntu, Linux Mint, Pop!_OS...)

    Instalacion recomendada:

         sudo apt install ./proxysend_2.4.0_amd64.deb

    o con dpkg:

         sudo dpkg -i proxysend_2.4.0_amd64.deb
         sudo apt -f install    # solo si avisa de dependencias

    Despues de instalar, la app aparece en el menu de aplicaciones
    como "ProxySend" (categoria Red). Tambien puedes lanzarla desde
    la terminal con el comando: proxysend

---------------------------------------------------------------------
4. COMO USARLA (paso a paso)
---------------------------------------------------------------------

4.1 Agregar un proxy

    1) Abre ProxySend y ve a "Ajustes del proxy" en la barra lateral.
    2) Elige el tipo de proxy: HTTP, HTTPS o SOCKS5.
    3) Escribe la direccion del servidor (host o IP) y el puerto.
       Si tu proxy usa una direccion distinta para HTTP y para HTTPS,
       desmarca "Usar el mismo proxy para todos los protocolos" y
       rellena los dos grupos de campos.
    4) Si el proxy pide credenciales, marca "El proxy necesita
       usuario y contraseña" y rellenalos.
    5) Revisa las excepciones (direcciones sin proxy) y ajustalas.
    6) Pulsa "Guardar configuracion" para guardarla sin activarla,
       o "Guardar y activar" para dejarla funcionando de una vez.

4.2 Guardar varios proxies y activarlos a gusto personal

    1) Configura un proxy en Ajustes y pulsa "Guardar en la
       biblioteca". Ponle un nombre claro (ej: "Casa - 3128",
       "Trabajo - Squid", "VPN del club").
    2) Repite con todos los proxies que uses. Apareceran en la
       pantalla "Mis proxies".
    3) En "Mis proxies", pulsa "Activar" en el que quieras usar: se
       aplica a los niveles marcados en Estado (Escritorio, Sistema,
       Terminales) y el proxy anterior se sustituye. La tarjeta se
       marca con "En uso" y en Estado veras "Perfil: <nombre>".
    4) "Editar" carga el proxy en Ajustes para modificarlo (con
       "Actualizar en la biblioteca" se guarda sobre el mismo);
       "Probar" comprueba si funciona sin activarlo; "Eliminar" lo
       quita de la biblioteca (con confirmacion).
    5) Para desactivar el proxy por completo usa el boton grande de
       "Estado"; para borrarlo todo usa "Restablecer valores por
       defecto" (la biblioteca se conserva).

4.3 Activar o desactivar a gusto personal

    - En la pantalla "Estado" pulsa el boton grande redondo:
      se activa (azul) o se desactiva (gris).
    - Con los interruptores de la seccion "Donde se aplica" eliges si
      el proxy se aplica al Escritorio, al Sistema y/o a las
      Terminales. Si el proxy ya esta activo y mueves un interruptor,
      el cambio se aplica en ese nivel al momento.
    - Tambien puedes activarlo/desactivarlo desde el icono de la
      bandeja del sistema, sin abrir la ventana.

4.4 Quitar el proxy y dejar los valores por defecto

    1) En "Estado" pulsa "Restablecer valores por defecto".
    2) ProxySend desactiva el proxy del escritorio, restaura los
       valores por defecto del escritorio (gsettings reset en GNOME;
       quita el grupo en KDE), quita las variables de /etc/environment
       y borra el bloque de ~/.bashrc y ~/.zshrc. Tambien olvida la
       configuracion activa, PERO conserva tu biblioteca de proxies.
    3) Si mas adelante quieres volver a usarlo, activalo desde
       "Mis proxies" o configura uno nuevo en "Ajustes".

4.5 Comprobar que funciona

    - En "Ajustes del proxy", pulsa "Probar proxy": te muestra la IP
      publica de salida a traves del proxy y la latencia.
    - Pulsa "Probar sin proxy" para comparar con tu conexion directa.
    - En "Mis proxies", el boton "Probar" de cada tarjeta hace lo
      mismo para ese proxy, sin necesidad de activarlo.

4.6 Autoarranque y bandeja (Preferencias)

    - Ajustes del proxy > Preferencias > "Abrir ProxySend al iniciar
      sesion": al activarla se crea ~/.config/autostart/proxysend-
      desktop. En cada inicio de sesion la app aplica el proxy
      guardado al escritorio y a las terminales sin abrir ventana
      (queda en la bandeja si la hay) y lo deja anotado en el
      Historial.
    - En Preferencias NO hay opciones de bandeja (se retiraron en
      v2.3.4): cerrar la ventana con la X cierra ProxySend y minimizar
      solo minimiza, como en cualquier app.
    - "Mostrar ProxySend" del menu de la bandeja des-minimiza la
      ventana (X11) o la reabre en primer plano (Wayland); tambien
      responde al clic en el icono y a una segunda instancia.
    - "Activar proxy" / "Desactivar proxy" del menu aplican el proxy
      guardado y avisan del resultado; "Salir" cierra la app al
      instante.

4.7 Activar la reconexion automatica (temporizador)

    1) Ve a "Ajustes del proxy", baja hasta la tarjeta "Reconexion
       automatica" y enciende el interruptor.
    2) Elige la frecuencia con los botones: 1 min, 5 min, 10 min,
       30 min o 1 hora.
    3) Desde ese momento ProxySend comprueba el proxy activo con esa
       frecuencia. Si deja de responder, lo vuelve a aplicar solo en
       escritorio y terminales, y lo anota en el registro.
    4) En la pantalla Estado veras una linea con la ultima
       verificacion ("Reconexion automatica: OK - IP x.x.x.x..." o el
       fallo correspondiente).
    5) El boton "Verificar ahora" (en Ajustes) hace la comprobacion al
       momento; tambien puedes usar el comando:
       proxysend --verificar

4.8 Exportar e importar la biblioteca completa

    1) Ve a "Mis proxies". Arriba estan los botones "Exportar
       biblioteca" e "Importar biblioteca".
    2) "Exportar biblioteca" crea ~/ProxySend-biblioteca-<fecha>.json
       con todos tus perfiles, los ajustes y el historial. Guarda ese
       archivo donde quieras (USB, nube...).
    3) "Importar biblioteca" abre un selector de archivos si tu sistema
       tiene zenity o kdialog; si no, busca automaticamente la
       biblioteca mas reciente en tu carpeta personal.
    4) Al importar se restauran los perfiles y los ajustes. El proxy
       que este aplicado en el sistema NO cambia: para usar uno de los
       perfiles importados, pulsa "Activar" en "Mis proxies".
    5) Importar el mismo archivo dos veces no duplica el historial.

4.9 Generar el informe con el log completo (para detectar errores)

    1) Ve a la pantalla "Informe" (barra lateral).
    2) El resumen muestra cuantas informaciones, avisos y errores hay
       registrados y el ultimo error.
    3) Pulsa "Generar informe completo": se crea
       ~/ProxySend-informe-<fecha>.txt con todo el diagnostico (sistema,
       escritorio detectado, herramientas presentes, estado de los tres
       niveles, biblioteca, reconexion, variables de entorno, resumen de
       errores, registro completo de eventos e historial). El archivo
       NO contiene contrasenas: se muestran como ****.
    4) En la misma pantalla puedes ver el registro de eventos en vivo y
       filtrarlo con "Todos" / "Avisos y errores".
    5) Desde la terminal: proxysend --informe

---------------------------------------------------------------------
5. DONDE SE GUARDA CADA COSA Y CUANDO SURTE EFECTO
---------------------------------------------------------------------

  Configuracion, biblioteca e historial de la app:
      ~/.config/ProxySend/settings.json   (incluye la biblioteca)
      ~/.config/ProxySend/history.json

  Escritorio GNOME y COSMIC: se cambia al instante con gsettings
      (Ajustes de GNOME > Red > Proxy mostrara lo mismo; en COSMIC
      las apps GTK lo aplican al instante porque leen gsettings).

  Escritorio KDE: se escribe al instante
      ~/.config/kioslaverc, grupo [Proxy Settings]
      (las apps de KDE lo leen al abrirse).

  /etc/environment: se escribe con permisos de administrador
      (pkexec pide tu contraseña). Requiere cerrar sesion y volver a
      entrar para que todas las apps del sistema lo tomen.
      Copia de seguridad del original:
      /etc/environment.proxysend.bak

  Terminales: el bloque marcado con
      # >>> ProxySend >>> ... # <<< ProxySend <<<
      en ~/.bashrc (y ~/.zshrc si existe). Se aplica al abrir una
      terminal NUEVA.

  Autoarranque: ~/.config/autostart/proxysend.desktop

  Registro de eventos (log completo de la app):
      ~/.config/ProxySend/eventos.log     (texto plano, 1 linea por evento)

  Estado de la reconexion automatica:
      ~/.config/ProxySend/reconnect.json  (ultima verificacion)

  Biblioteca completa exportada:
      ~/ProxySend-biblioteca-<fecha>.json

  Informes diagnostico:
      ~/ProxySend-informe-<fecha>.txt

---------------------------------------------------------------------
7. SOLUCION DE PROBLEMAS
---------------------------------------------------------------------

  * El AppImage no abre y menciona FUSE / libfuse2:
    lanzalo con:
        APPIMAGE_EXTRACT_AND_RUN=1 ./ProxySend-2.4.0-x86_64.AppImage
    o usa el instalador universal (.run) o el portable (.tar.gz) de
    las secciones 3.1 / 3.2: no necesitan FUSE para nada.

  * Prueba de conexion falla pero el proxy si funciona:
    La prueba usa el motor de red interno de la app (con curl como
    respaldo). Revisa host, puerto y credenciales en Ajustes. Si el
    proxy es SOCKS5 con usuario y contraseña, la prueba usa curl
    (debe estar instalado), porque el motor interno no soporta esa
    combinacion.

  * No se aplica el nivel Sistema y sale un error de pkexec:
    - Necesitas pkexec y un agente de autenticacion polkit
      (policykit-1 en GNOME; polkit-kde-agent en KDE). Se instalan
      por defecto en la mayoria de escritorios.
    - Si cancelas la ventana de contraseña, ese nivel no se aplica
      pero los demas si; puedes reintentar con el interruptor.
    - Comprueba que existe: which pkexec

  * El nivel Escritorio aparece "no disponible":
    Tu escritorio no tiene gsettings ni es KDE. Los niveles Sistema
    y Terminal siguen funcionando; la mayoria de apps (Chrome,
    Firefox, curl...) respeta las variables de entorno.
    En KDE, recuerda que las apps leen kioslaverc al abrirse:
    si ya estaban abiertas, cierralas y vuelvelas a abrir.

  * Uso SOCKS5 y el nivel Escritorio no se aplica en KDE:
    Normal: KDE no guarda proxys SOCKS5 en kioslaverc. Marca los
    niveles Sistema y/o Terminal y desmarca el Escritorio.

  * Active el proxy y la terminal no cambia:
    Abre una terminal NUEVA (el bloque se lee al arrancar la shell).
    Si usas /etc/environment, ademas tienes que cerrar sesion y
    volver a entrar.

  * Active el proxy y el navegador no cambia:
    En GNOME se aplica al instante a Chrome, Chromium, Firefox y apps
    del sistema. Si usas solo el nivel Sistema, cierra sesion y
    vuelve a entrar.

  * El proxy pide usuario/contraseña al navegar y yo no los configure:
    Algunos proxies (especialmente SOCKS5) validan de otra forma.
    Activa la casilla de autenticacion en Ajustes. En KDE, la
    contraseña del proxy no se guarda en kioslaverc: las apps de KDE
    la pediran la primera vez.

  * No veo el icono de la bandeja:
    GNOME necesita la extension "AppIndicator" para mostrar iconos de
    bandeja (sudo apt install gnome-shell-extension-appindicator y
    activarla con Extensions). Si tu entorno no tiene bandeja, la app
    funciona igual abriendola normalmente.

  * Cerre la ventana y la app "sigue ahi":
    Si activaste "cerrar a la bandeja", la app sigue en el icono de
    la bandeja. Usa el menu de la bandeja > Salir para cerrarla del
    todo, o desactiva la casilla en Ajustes > Preferencias.

  * La reconexion automatica dice "FALLO" y re-aplica:
    Significa que el proxy activo no respondio a la verificacion
    (cayo, cambio de credenciales o no hay red). ProxySend ya volvio a
    aplicarlo en escritorio y terminales; cuando el proxy vuelva, la
    siguiente verificacion lo confirmara ("VOLVIO A RESPONDER"). Mira
    el detalle exacto en Estado o en la pantalla Informe.

  * Quiero saber por que algo fallo en la app:
    Abre la pantalla "Informe" y pulsa "Generar informe completo": el
    archivo incluye el log completo de eventos con avisos y errores.
    El log tambien queda en ~/.config/ProxySend/eventos.log.

  * Importe una biblioteca y el proxy no se activo:
    Es a proposito: importar restaura perfiles y ajustes pero no cambia
    el sistema. Ve a "Mis proxies" y pulsa "Activar" en el perfil que
    quieras usar.

  * Quiero verificar que no quedo nada del proxy:
    Pulsa "Restablecer valores por defecto"; ademas puedes revisar
    /etc/environment y el final de ~/.bashrc (el bloque marcado con
    "# >>> ProxySend >>>" desaparece al restablecer). En KDE tambien
    puedes mirar ~/.config/kioslaverc: el grupo [Proxy Settings]
    desaparece al restablecer.

---------------------------------------------------------------------
8. DESINSTALACION
---------------------------------------------------------------------

  Version .deb:

      sudo apt remove proxysend
      (o sudo dpkg -r proxysend)

  Instalador universal (.run):

      sh ProxySend-2.4.0-linux64.run --desinstalar

  Paquete portable (.tar.gz) o AppImage: borra la carpeta o el
  archivo extraido y ya esta.

  Antes de desinstalar (o despues), deja el sistema limpio pulsando
  "Restablecer valores por defecto" en la app; si ya no puedes
  abrirla, borra a mano:

      - el bloque "# >>> ProxySend >>> ... # <<< ProxySend <<<" de
        ~/.bashrc (y ~/.zshrc si existe)
      - las lineas http_proxy/https_proxy/no_proxy (y mayusculas) de
        /etc/environment (con sudo)
      - el grupo "[Proxy Settings]" de ~/.config/kioslaverc si usaste
        el nivel Escritorio en KDE
      - opcionalmente: ~/.config/ProxySend,
        ~/.config/autostart/proxysend.desktop y
        /etc/environment.proxysend.bak

---------------------------------------------------------------------
9. PRIVACIDAD Y NOTAS TECNICAS
---------------------------------------------------------------------

  * Todo funciona en local: sin cuentas, sin telemetria, sin
    conexiones a servidores de terceros (salvo las pruebas de IP que
    TU pulsas, que van a api.ipify.org a traves del motor de red
    interno de la app, o de curl como respaldo).

  * La contraseña del proxy se guarda sin cifrar en
    ~/.config/ProxySend/settings.json (permisos 700 en la carpeta).
    Solo la leen procesos de tu usuario.

  * La aplicacion esta escrita en Rust con la interfaz egui. Es un
    unico binario de ~7 MB; los instaladores pesan entre 3 y 4 MB.

  * Requisitos reales: las librerias estandar de cualquier escritorio
    Linux moderno (glibc, Mesa/OpenGL para el renderizado y las
    librerias cliente de X11 o Wayland, presentes de serie en todos
    los escritorios). No necesita libfuse2, ni GTK, ni Node, ni
    ninguna dependencia de Electron. Las dos librerias XKB que la app
    usa en X11 (libxkbcommon-x11 y libxcb-xkb) van incluidas dentro de
    los cuatro paquetes: no hay que instalar nada extra, ni siquiera
    en sistemas X11 minimalistas.

  * Bandeja del sistema: protocolo StatusNotifierItem sobre D-Bus
    (el mismo que usan KDE, XFCE, COSMIC y GNOME con AppIndicator).
    En Wayland funciona sin XWayland.

  * La compilacion esta optimizada en tamano (LTO completo, binario
    despojado de simbolos), y las pruebas de la logica de proxy son
    23 pruebas automaticas incluidas en el codigo fuente.

  * Licencia GNU GPL v3: ProxySend es software libre; puedes
    redistribuirlo y modificarlo bajo los terminos de la GNU General
    Public License version 3 (texto oficial incluido en COPYING.txt
    del codigo fuente). Creado por Leyter Vázquez . V. "ProxySend" es
    un proyecto independiente; la interfaz se inspira en LocalSend
    pero no comparte codigo con el.

=====================================================================

