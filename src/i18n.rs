//! ProxySend - idiomas de la aplicacion (v2.3.0).
//!
//! La app viene en ESPANOL por defecto y se puede cambiar a INGLES en la
//! pagina Ajustes del proxy, tarjeta Preferencias. El idioma elegido se
//! guarda en settings.json (campo "language": "es"/"en") y se aplica a TODO:
//! menus, paginas, avisos (toast), bandeja del sistema, informe diagnostico,
//! mensajes del registro nuevo y la consola (CLI).
//!
//! Funcionamiento:
//!   - Las claves de traduccion son los textos en espanol originales.
//!     `tr("Texto")` devuelve el texto tal cual en espanol y su traduccion
//!     en ingles; si una clave no esta en la tabla, cae al espanol (nunca
//!     se rompe la interfaz).
//!   - Para los textos con valores (`format!`) se usa `tf("Plantilla {0}",
//!     &[valor])`: traduce la plantilla y sustituye los marcadores {0} {1}...
//!   - El idioma vive en una variable global atomica: la UI, la bandeja, el
//!     temporizador y los trabajos en segundo plano leen el mismo valor.

use std::sync::atomic::{AtomicU8, Ordering};

static LANG: AtomicU8 = AtomicU8::new(0); // 0 = espanol (por defecto), 1 = ingles

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Lang {
    Es,
    En,
}

pub fn set(lang: Lang) {
    LANG.store(if lang == Lang::En { 1 } else { 0 }, Ordering::SeqCst);
}

pub fn get() -> Lang {
    if LANG.load(Ordering::SeqCst) == 1 { Lang::En } else { Lang::Es }
}

pub fn is_en() -> bool {
    get() == Lang::En
}

/// Acepta "es"/"en" (y variantes); cualquier otra cosa cae al espanol.
pub fn set_code(code: &str) {
    let c = code.trim().to_lowercase();
    set(if c == "en" || c == "en_us" || c == "english" { Lang::En } else { Lang::Es });
}

/// Sincroniza el idioma global con el ajuste guardado (la app arranca asi).
pub fn sync_from_settings() {
    let s = crate::store::load_settings();
    set_code(s.language.as_deref().unwrap_or("es"));
}

/// Traduccion estatica: la clave es el texto en espanol.
pub fn tr(key: &'static str) -> &'static str {
    if !is_en() {
        return key;
    }
    en(key).unwrap_or(key)
}

/// Plantilla con marcadores {0} {1} {2}...: traduce y sustituye los valores.
pub fn tf(key: &'static str, args: &[&str]) -> String {
    let mut s = tr(key).to_string();
    for (i, a) in args.iter().enumerate() {
        let token = format!("{{{}}}", i);
        if s.contains(&token) {
            s = s.replace(&token, a);
        }
    }
    s
}

/// Mes abreviado segun el idioma (0 = enero).
pub fn mes(m: usize) -> &'static str {
    const ES: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
    const EN: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let i = m.min(11);
    if is_en() { EN[i] } else { ES[i] }
}

/// Etiqueta del escritorio detectado segun el idioma (para el registro).
pub fn de_label(de: crate::sysops::DetectedDesktop) -> &'static str {
    use crate::sysops::DetectedDesktop as D;
    match de {
        D::Gnome => "GNOME",
        D::Cosmic => "COSMIC",
        D::Kde => "KDE",
        D::Xfce => "XFCE",
        D::Other => tr("otro"),
    }
}

/// Tabla de traduccion al ingles. Cada clave es EXACTAMENTE el texto en
/// espanol que aparece en el codigo. Las claves que faltan se muestran en
/// espanol (comportamiento seguro por defecto).
pub fn en(key: &str) -> Option<&'static str> {
    let found = match key {
        /* ---------- titulos de pagina y menu lateral ---------- */
        "Estado" => "Status",
        "Ajustes del proxy" => "Proxy settings",
        "Mis proxies" => "My proxies",
        "Historial" => "History",
        "Informe" => "Report",
        "Informe y diagnostico" => "Report and diagnostics",
        "Acerca de" => "About",
        "Acerca de ProxySend" => "About ProxySend",
        "Gestor de proxy para Linux" => "Proxy manager for Linux",
        "Tema claro" => "Light theme",
        "Tema oscuro" => "Dark theme",
        "Proxy activo" => "Active proxy",
        "Sin proxy" => "No proxy",
        "conexion directa" => "direct connection",
        "proxy en uso" => "proxy in use",

        /* ---------- etiquetas de actividad (historial) ---------- */
        "Proxy activado" => "Proxy enabled",
        "Proxy desactivado" => "Proxy disabled",
        "Valores por defecto restaurados" => "Defaults restored",
        "Configuracion guardada" => "Configuration saved",
        "Prueba de conexion" => "Connection test",
        "Configuracion exportada" => "Configuration exported",
        "Configuracion importada" => "Configuration imported",
        "Historial limpiado" => "History cleared",
        "Proxy guardado en la biblioteca" => "Proxy saved to the library",
        "Proxy eliminado de la biblioteca" => "Proxy removed from the library",
        "Proxy de la biblioteca activado" => "Library proxy enabled",
        "Biblioteca completa exportada" => "Full library exported",
        "Biblioteca completa importada" => "Full library imported",
        "Reconexion automatica" => "Automatic reconnection",
        "Informe diagnostico" => "Diagnostic report",
        "Autoarranque" => "Autostart",
        "Bandeja del sistema" => "System tray",
        "Actividad" => "Activity",
        "Idioma cambiado" => "Language changed",
        "Idioma cambiado a espanol" => "Language changed to Spanish",
        "Idioma cambiado a ingles" => "Language changed to English",

        /* ---------- mensajes de la interfaz (toasts y avisos) ---------- */
        "Configuracion importada. Revisala y guarda." => "Configuration imported. Review it and save.",
        "No se pudo completar la operacion." => "The operation could not be completed.",
        "sin conexion a traves del proxy." => "no connection through the proxy.",
        "Configuracion exportada a {0}" => "Configuration exported to {0}",
        "Informe diagnostico generado: {0}" => "Diagnostic report generated: {0}",
        "Informe completo generado en {0}" => "Full report generated at {0}",
        "Biblioteca completa exportada: {0}" => "Full library exported: {0}",
        "Biblioteca completa exportada a {0}" => "Full library exported to {0}",
        "El proxy responde: {0}" => "The proxy responds: {0}",
        "Verificacion: {0}" => "Check: {0}",
        "\"{0}\" funciona. IP de salida: {1} - {2} s" => "\"{0}\" works. Exit IP: {1} - {2} s",
        "\"{0}\" fallo: {1}" => "\"{0}\" failed: {1}",

        /* ---------- pagina Estado ---------- */
        "Cargando..." => "Loading...",
        "PROXY VIGENTE EN ESTE MOMENTO" => "PROXY IN USE RIGHT NOW",
        "Configuracion actual" => "Current configuration",
        "Ninguno - la conexion es directa" => "None - the connection is direct",
        "No hay ningun proxy aplicado: el trafico sale directo a internet." => {
            "No proxy is applied: traffic goes straight to the internet."
        }
        "VIGENTE" => "IN USE",
        "SIN PROXY" => "NO PROXY",
        "Reconexion automatica activada: primera verificacion en menos de {0} min" => {
            "Automatic reconnection enabled: first check in less than {0} min"
        }
        "Reconexion automatica: el proxy responde ({0}, {1})" => "Automatic reconnection: the proxy responds ({0}, {1})",
        "Reconexion automatica: {0}, {1}" => "Automatic reconnection: {0}, {1}",
        "Reconexion automatica: verificacion en curso..." => "Automatic reconnection: check in progress...",
        "Configurado: {0}" => "Configured: {0}",
        "Todavia no hay un proxy configurado. Ve a Ajustes para agregar uno." => {
            "There is still no proxy configured. Go to Settings to add one."
        }
        " - Perfil: {0}" => " - Profile: {0}",
        "Pulsa para desactivar" => "Press to disable",
        "Pulsa para activar" => "Press to enable",
        "Configura un proxy primero" => "Configure a proxy first",
        "Donde se aplica" => "Where it is applied",
        "Sistema (/etc/environment)" => "System (/etc/environment)",
        "Terminales (~/.bashrc)" => "Terminals (~/.bashrc)",
        "Editar configuracion" => "Edit configuration",
        "Restablecer valores por defecto" => "Restore defaults",
        "activo" => "active",
        "inactivo" => "inactive",
        "no disponible" => "not available",
        "Desactivando proxy..." => "Disabling proxy...",
        "Activando proxy..." => "Enabling proxy...",
        "Proxy activado en los niveles seleccionados." => "Proxy enabled on the selected levels.",
        "Primero configura un proxy en Ajustes." => "First configure a proxy in Settings.",

        /* ---------- pagina Ajustes del proxy ---------- */
        "Tipo de proxy" => "Proxy type",
        "Usar el mismo proxy para todos los protocolos" => "Use the same proxy for all protocols",
        "Direccion del servidor" => "Server address",
        "Servidor HTTP" => "HTTP server",
        "Servidor HTTPS" => "HTTPS server",
        "Puerto" => "Port",
        "El proxy necesita usuario y contraseña" => "The proxy needs a username and password",
        "Usuario" => "Username",
        "Contraseña" => "Password",
        "Excepciones (no pasan por el proxy)" => "Exceptions (they do not go through the proxy)",
        "Una por linea o separadas por coma. Por defecto: localhost, 127.0.0.0/8 y ::1." => {
            "One per line or comma-separated. Default: localhost, 127.0.0.0/8 and ::1."
        }
        "Guardar configuracion" => "Save configuration",
        "Guardar y activar" => "Save and enable",
        "Actualizar en la biblioteca" => "Update in the library",
        "Guardar en la biblioteca" => "Save to the library",
        "Probar proxy" => "Test proxy",
        "Probar sin proxy" => "Test without proxy",
        "Exportar configuracion" => "Export configuration",
        "Importar configuracion" => "Import configuration",
        "Estas editando un proxy guardado: usa \"Actualizar en la biblioteca\" para conservar los cambios, o desmarca la casilla al guardar para crear una copia nueva." => {
            "You are editing a saved proxy: use \"Update in the library\" to keep the changes, or uncheck the box when saving to create a new copy."
        }
        "Probando la conexion..." => "Testing the connection...",
        "Funciona. IP publica de salida: {0} - HTTP {1} - {2} s" => "It works. Public exit IP: {0} - HTTP {1} - {2} s",
        "No se pudo conectar a traves del proxy." => "Could not connect through the proxy.",
        "Preferencias" => "Preferences",
        "Abrir ProxySend al iniciar sesion" => "Open ProxySend at login",
        "Al iniciar sesion se vuelve a aplicar el proxy guardado en escritorio y terminales." => {
            "At login the saved proxy is applied again on desktop and terminals."
        }
        // v2.3.4: las opciones de bandeja al cerrar o minimizar se retiraron;
        // sus traducciones tambien. Queda solo autostart e idioma en Preferencias.
        "Idioma de la aplicacion" => "App language",
        "Cambia entre espanol e ingles; se guarda y se aplica en el momento a toda la app." => {
            "Switch between Spanish and English; it is saved and applied to the whole app immediately."
        }
        "Un temporizador comprueba cada cierto tiempo que el proxy activo siga funcionando. Si deja de responder, ProxySend lo vuelve a aplicar solo en escritorio y terminales y lo deja anotado en el Informe. El temporizador solo esta en marcha mientras esta opcion este activada: al abrir la app con la opcion apagada no arranca ningun temporizador." => {
            "A timer checks from time to time that the active proxy still works. If it stops responding, ProxySend re-applies it on desktop and terminals and logs it in the Report. The timer only runs while this option is enabled: opening the app with the option off starts no timer."
        }
        "Frecuencia de la verificacion" => "Check frequency",
        "1 hora" => "1 hour",
        "Verificando el proxy..." => "Checking the proxy...",
        "Verificando la conexion a traves del proxy..." => "Checking the connection through the proxy...",
        "Todavia no se ha hecho ninguna verificacion automatica. Pulsa \"Verificar ahora\" para comprobarlo ya." => {
            "No automatic check has been done yet. Press \"Check now\" to check right away."
        }
        "Ultima verificacion ({0}): {1}" => "Last check ({0}): {1}",
        "Verificacion en curso..." => "Check in progress...",
        "Verificar ahora" => "Check now",

        /* ---------- pagina Mis proxies ---------- */
        "Nuevo proxy" => "New proxy",
        "Configura el proxy nuevo y pulsa \"Guardar en la biblioteca\"." => {
            "Configure the new proxy and press \"Save to the library\"."
        }
        "Actualizar" => "Refresh",
        "Biblioteca completa (perfiles + ajustes + historial):" => "Full library (profiles + settings + history):",
        "Exportar biblioteca" => "Export library",
        "Importar biblioteca" => "Import library",
        "Exportando la biblioteca..." => "Exporting the library...",
        "Importando la biblioteca..." => "Importing the library...",
        "Todavia no hay proxies guardados." => "There are still no saved proxies.",
        "Configura uno en Ajustes y pulsa \"Guardar en la biblioteca\", o crea uno con el boton \"Nuevo proxy\"." => {
            "Configure one in Settings and press \"Save to the library\", or create one with the \"New proxy\" button."
        }
        "En uso" => "In use",
        "Eliminar" => "Delete",
        "Probar" => "Test",
        "Editar" => "Edit",
        "Activar" => "Enable",
        "Probando \"{0}\" a traves del proxy..." => "Testing \"{0}\" through the proxy...",
        "Editando \"{0}\". Cambia lo que quieras y pulsa \"Actualizar en la biblioteca\"." => {
            "Editing \"{0}\". Change what you want and press \"Update in the library\"."
        }
        "Activando \"{0}\"..." => "Enabling \"{0}\"...",

        /* ---------- pagina Historial ---------- */
        "Limpiar historial" => "Clear history",
        "Historial limpiado." => "History cleared.",
        "Todavia no hay actividad registrada." => "There is no recorded activity yet.",

        /* ---------- pagina Informe ---------- */
        "Resumen del diagnostico" => "Diagnostic summary",
        "{0} informaciones" => "{0} info",
        "{0} avisos" => "{0} warnings",
        "{0} errores" => "{0} errors",
        "No hay ningun error registrado: todo ha ido bien." => "No errors recorded: everything went fine.",
        "Ultimo error [{0}]: {1}" => "Last error [{0}]: {1}",
        "Escritorio detectado: {0} - version {1} - eventos guardados en ~/.config/ProxySend/eventos.log" => {
            "Desktop detected: {0} - version {1} - events saved in ~/.config/ProxySend/eventos.log"
        }
        "Informe completo para detectar errores" => "Full report to find errors",
        "Genera un archivo de texto con TODO lo que sucede en la app: sistema y escritorio detectado, herramientas disponibles, estado de los tres niveles, biblioteca de proxies, reconexion automatica, historial y el registro completo de eventos con avisos y errores. No incluye contrasenas (se muestran como ****)." => {
            "Generates a text file with EVERYTHING that happens in the app: system and detected desktop, available tools, status of the three levels, proxy library, automatic reconnection, history and the full event log with warnings and errors. It does not include passwords (they are shown as ****)."
        }
        "Generar informe completo" => "Generate full report",
        "Generando el informe..." => "Generating the report...",
        "Limpiar registro" => "Clear log",
        "Registro de eventos limpiado." => "Event log cleared.",
        "Registro de eventos limpiado" => "Event log cleared",
        "Registro de eventos (log completo)" => "Event log (full log)",
        "Todos" => "All",
        "Avisos y errores" => "Warnings and errors",
        "Ningun aviso ni error registrado hasta ahora." => "No warnings or errors recorded so far.",
        "Todavia no hay eventos registrados." => "No events recorded yet.",
        "Verificaciones de calidad de la version" => "Version quality checks",
        "Comprobaciones hechas sobre la version {0} antes de publicarla. El mismo listado sale en el informe completo (seccion 11)." => {
            "Checks made on version {0} before publishing it. The same list appears in the full report (section 11)."
        }
        "Todos los campos de escritura (servidor, puerto, usuario, contrasena, excepciones y nombre al guardar) tienen fondo y borde resaltado." => {
            "All text fields (server, port, username, password, exceptions and the save name) have highlighted background and border."
        }
        "Tamano e intensidad de los textos afinados: se lee todo bien en tema claro, incluidos los menus, manteniendo el tema azul." => {
            "Text size and intensity fine-tuned: everything reads well in the light theme, including the menus, keeping the blue theme."
        }
        "Botones Actualizar de Mis proxies, Historial e Informe verificados: recargan los datos al pulsarlos." => {
            "Refresh buttons of My proxies, History and Report verified: they reload the data when pressed."
        }
        "Activar un proxy SIN contrasena de administrador: se explica el fallo en pantalla y queda como aviso en el registro (no pasa en silencio)." => {
            "Enabling a proxy WITHOUT the administrator password: the failure is explained on screen and logged as a warning (never silent)."
        }
        "Activar sin ningun nivel marcado: rechazado con mensaje claro." => {
            "Enabling with no level selected: rejected with a clear message."
        }
        "Licencia cambiada de MIT a GNU GPL v3 en Acerca de, en Cargo.toml y en el copyright del paquete .deb (texto completo en COPYING.txt)." => {
            "License changed from MIT to GNU GPL v3 in About, in Cargo.toml and in the .deb package copyright (full text in COPYING.txt)."
        }
        "Temporizador de reconexion automatica reparado: ya no arranca al ejecutar la app; solo corre al activar la opcion y se detiene de verdad al desactivarla." => {
            "Automatic reconnection timer fixed: it no longer starts when the app runs; it only runs when the option is enabled and truly stops when disabled."
        }
        "Idioma espanol/ingles incorporado: la app arranca en espanol y se cambia en Preferencias; se guarda y aplica a menus, bandeja, informe y consola." => {
            "Spanish/English language built in: the app starts in Spanish and switches in Preferences; it is saved and applied to menus, tray, report and console."
        }

        /* ---------- pagina Acerca de ---------- */
        "Version {0} - gestor de proxy para Linux" => "Version {0} - proxy manager for Linux",
        "Cambia el proxy de tu PC con un clic: agrega, guarda varios proxies en una biblioteca y activa el que quieras a gusto personal, sin tocar archivos a mano. La interfaz se inspira en LocalSend." => {
            "Change your PC's proxy with one click: add, save several proxies in a library and enable the one you want, without editing files by hand. The interface is inspired by LocalSend."
        }
        "Creado por: Leyter Vázquez . V" => "Created by: Leyter Vázquez . V",
        "Autor y desarrollador de ProxySend" => "Author and developer of ProxySend",
        "Detalles tecnicos" => "Technical details",
        "Motor grafico propio en Rust (egui): sin Electron ni Chromium, la app pesa poco y arranca rapido." => {
            "Own graphics engine in Rust (egui): no Electron or Chromium, the app is small and starts fast."
        }
        "Ejecutable en cualquier distribucion Linux moderna, en X11 y Wayland (incluido COSMIC)." => {
            "Runs on any modern Linux distribution, on X11 and Wayland (including COSMIC)."
        }
        "Escritorios soportados: GNOME, KDE (kioslaverc), COSMIC y XFCE, con deteccion automatica del resto." => {
            "Supported desktops: GNOME, KDE (kioslaverc), COSMIC and XFCE, with automatic detection for the rest."
        }
        "Indicador del proxy vigente siempre visible: la barra lateral y la pagina Estado muestran que proxy se esta usando, o si no hay ninguno." => {
            "Always-visible indicator of the proxy in use: the sidebar and the Status page show which proxy is being used, or if there is none."
        }
        "Reconexion automatica: un temporizador comprueba el proxy activo cada 1/5/10/30/60 minutos y lo re-aplica si deja de responder." => {
            "Automatic reconnection: a timer checks the active proxy every 1/5/10/30/60 minutes and re-applies it if it stops responding."
        }
        "Biblioteca completa exportable e importable: perfiles, ajustes e historial en un archivo JSON (copia de seguridad)." => {
            "Full library exportable and importable: profiles, settings and history in a JSON file (backup)."
        }
        "Informe diagnostico con el log completo de la app: sistema, escritorio, herramientas, niveles, errores y eventos." => {
            "Diagnostic report with the full app log: system, desktop, tools, levels, errors and events."
        }
        "Bandeja del sistema via StatusNotifierItem (D-Bus), sin libappindicator." => {
            "System tray via StatusNotifierItem (D-Bus), without libappindicator."
        }
        "Prueba de conexion con motor de red interno TLS, con curl como respaldo." => {
            "Connection test with the internal TLS network engine, with curl as fallback."
        }
        "Configuracion e historial en ~/.config/ProxySend; todo funciona en local, sin telemetria." => {
            "Configuration and history in ~/.config/ProxySend; everything works locally, no telemetry."
        }
        "Licencia y privacidad" => "License and privacy",
        "Licencia GNU GPL v3. La app no envia datos a ningun servidor: solo cambia la configuracion de tu equipo (las pruebas de IP que tu pulsas consultan api.ipify.org)." => {
            "GNU GPL v3 license. The app does not send data to any server: it only changes your computer's settings (the IP tests you trigger query api.ipify.org)."
        }

        /* ---------- modales ---------- */
        "Eliminar proxy" => "Delete proxy",
        "Nombre del proxy" => "Proxy name",
        "Ejemplos: Casa - 3128, Trabajo - Squid, VPN del club" => "Examples: Home - 3128, Work - Squid, Club VPN",
        "Guardar como proxy nuevo (no actualizar el actual)" => "Save as a new proxy (do not update the current one)",
        "Se eliminara \"{0}\" de la biblioteca. El proxy que este aplicado en el sistema no se desactiva (para eso usa Estado)." => {
            "\"{0}\" will be removed from the library. The proxy currently applied on the system is not disabled (use Status for that)."
        }
        "Cancelar" => "Cancel",
        "Guardar" => "Save",
        "Actualizar el proxy de la biblioteca" => "Update the library proxy",
        "Actualizaras los datos de \"{0}\" con la configuracion actual del formulario." => {
            "You will update the data of \"{0}\" with the current form configuration."
        }
        "La configuracion actual del formulario se guardara con este nombre." => {
            "The current form configuration will be saved with this name."
        }
        "Escribe un nombre para el proxy." => "Type a name for the proxy.",

        /* ---------- trabajos en segundo plano (UI) ---------- */
        "Proxy desactivado: el sistema vuelve a la conexion directa." => {
            "Proxy disabled: the system goes back to the direct connection."
        }
        "Proxy desactivado en todos los niveles" => "Proxy disabled on all levels",
        "Valores por defecto restaurados y proxy quitado (la biblioteca se conserva)" => {
            "Defaults restored and proxy removed (the library is kept)"
        }
        "Proxy quitado: todo restablecido a los valores por defecto." => {
            "Proxy removed: everything restored to the defaults."
        }
        "Configuracion guardada. Usa el boton grande de Estado para activarla." => {
            "Configuration saved. Use the big Status button to enable it."
        }
        "Proxy guardado y activado." => "Proxy saved and enabled.",
        "{0} (actualizado) - {1}" => "{0} (updated) - {1}",
        "Proxy \"{0}\" guardado en la biblioteca." => "Proxy \"{0}\" saved to the library.",
        "Proxy \"{0}\" eliminado de la biblioteca" => "Proxy \"{0}\" removed from the library",
        "Proxy \"{0}\" eliminado de la biblioteca." => "Proxy \"{0}\" removed from the library.",
        "Ese proxy ya no esta en la biblioteca." => "That proxy is no longer in the library.",
        "Proxy \"{0}\" activado en los niveles seleccionados." => "Proxy \"{0}\" enabled on the selected levels.",
        "Salida por el proxy: IP {0} en {1} s" => "Exit through the proxy: IP {0} in {1} s",
        "Autoarranque activado" => "Autostart enabled",
        "Autoarranque desactivado" => "Autostart disabled",
        "Autoarranque activado." => "Autostart enabled.",
        "Autoarranque desactivado." => "Autostart disabled.",
        "No se pudo escribir el archivo: {0}" => "Could not write the file: {0}",
        "No se pudo leer {0}: {1}" => "Could not read {0}: {1}",
        "Archivo JSON invalido: {0}" => "Invalid JSON file: {0}",
        "El archivo no contiene una configuracion valida: {0}" => "The file does not contain a valid configuration: {0}",
        "Importar configuracion de ProxySend" => "Import ProxySend configuration",
        "Importar biblioteca completa de ProxySend" => "Import ProxySend full library",
        "No se encontro ninguna biblioteca (archivo ProxySend-biblioteca-*.json) en tu carpeta personal, ni zenity/kdialog para buscarla. Genera primero una con Exportar biblioteca." => {
            "No library found (a ProxySend-biblioteca-*.json file) in your home folder, and no zenity/kdialog to look for one. Generate one first with Export library."
        }
        "Biblioteca importada desde {0}: {1} proxies, {2} eventos de historial y los ajustes restaurados." => {
            "Library imported from {0}: {1} proxies, {2} history events and the settings restored."
        }
        " Avisos: {0}" => " Warnings: {0}",
        "Temporizador activado: cada {0} min" => "Timer enabled: every {0} min",
        "Temporizador desactivado: cada {0} min" => "Timer disabled: every {0} min",
        "Reconexion automatica activada: cada {0} minutos" => "Automatic reconnection enabled: every {0} minutes",
        "Reconexion automatica desactivada" => "Automatic reconnection disabled",
        "Reconexion automatica activada: verificacion cada {0} min." => {
            "Automatic reconnection enabled: check every {0} min."
        }
        "Reconexion automatica desactivada." => "Automatic reconnection disabled.",

        /* ---------- textos auxiliares de la UI ---------- */
        "Ejemplo: {0}" => "Example: {0}",
        "Ejemplo: 3128" => "Example: 3128",
        "con usuario {0}" => "with user {0}",
        "HTTP y HTTPS separados" => "HTTP and HTTPS separate",
        "{0} excepcion" => "{0} exception",
        "{0} excepciones" => "{0} exceptions",
        "{0} (ajustes de KDE)" => "{0} (KDE settings)",
        "{0} (ajustes de GNOME)" => "{0} (GNOME settings)",
        "{0} (solo niveles sistema y terminal)" => "{0} (system and terminal levels only)",
        "otro" => "other",
        "activado" => "enabled",
        "desactivado" => "disabled",
        "re-aplicado" => "re-applied",

        /* ---------- bandeja del sistema ---------- */
        "Proxy activado ({0})" => "Proxy enabled ({0})",
        "Mostrar ProxySend" => "Show ProxySend",
        "Activar proxy" => "Enable proxy",
        "Desactivar proxy" => "Disable proxy",
        "Salir" => "Quit",
        "Proxy desactivado desde la bandeja del sistema" => "Proxy disabled from the system tray",
        "Proxy activado desde la bandeja del sistema - {0}" => "Proxy enabled from the system tray - {0}",
        "Bandeja del sistema lista (StatusNotifierItem)" => "System tray ready (StatusNotifierItem)",
        "Bandeja del sistema no disponible: {0}" => "System tray not available: {0}",

        /* ---------- bandeja del sistema (v2.3.1) ---------- */
        "No se pudo activar el proxy desde la bandeja: {0}" => "Could not enable the proxy from the tray: {0}",
        "No se pudo desactivar el proxy desde la bandeja: {0}" => "Could not disable the proxy from the tray: {0}",
        "No hay ningun proxy guardado todavia: agrega uno en la pagina \"Mis proxies\"." => {
            "There is no saved proxy yet: add one on the \"My proxies\" page."
        }
        "Salida de la aplicacion desde la bandeja del sistema" => "Application exit requested from the system tray",
        "Salir desde la bandeja: proceso terminado al instante" => "Quit from the tray: process ended instantly",
        // v2.3.4: verificaciones de la revision de la bandeja
        "Menus de la bandeja del sistema reparados y probados por D-Bus: Mostrar ProxySend, Activar/Desactivar proxy y Salir responden siempre, con la ventana visible o minimizada." => {
            "System tray menus repaired and tested over D-Bus: Show ProxySend, Enable/Disable proxy and Quit always respond, with the window visible or minimized."
        }
        "Activar/Desactivar proxy desde la bandeja muestra el resultado: aviso en pantalla con el detalle de cada nivel y constancia en el registro." => {
            "Enable/Disable proxy from the tray reports the result: on-screen notice with the detail of each level and a record in the log."
        }
        "Las opciones de bandeja al cerrar o minimizar se quitaron de Ajustes del proxy (v2.3.4): cerrar la ventana cierra ProxySend y minimizar solo minimiza, sin ocultar nada." => {
            "The tray options on close or minimize were removed from Proxy settings (v2.3.4): closing the window quits ProxySend and minimizing only minimizes, with nothing hidden."
        }
        "Sin rastros de la funcionalidad retirada: casillas, ajustes guardados, registros, botones y traducciones fuera; los ajustes viejos con esas claves se leen sin error y se descartan." => {
            "No traces of the removed feature: checkboxes, saved settings, logs, buttons and translations gone; old settings with those keys load without errors and are discarded."
        }
        "Mostrar ProxySend reparado a fondo: en X11 la ventana minimizada vuelve a pantalla porque ahora se des-minimiza de verdad (antes solo se pedia foco y la ventana seguia abajo); en Wayland se reabre en primer plano con una ventana nueva." => {
            "Show ProxySend deeply repaired: on X11 the minimized window comes back on screen because it is now truly un-minimized (before only focus was requested and the window stayed down); on Wayland it reopens in the foreground with a new window."
        }
        "Registro de la bandeja sincero: el informe cuenta la accion real de cada momento (mostrar, activar, desactivar, salir) y ya no anuncia ocultaciones que la app ya no hace." => {
            "Honest tray log: the report states the action actually taken at each moment (show, enable, disable, quit) and no longer claims hide actions the app no longer performs."
        }
        "Simulacion real en X11: al minimizar la ventana por el gestor de ventanas queda abajo de forma normal y Mostrar ProxySend la devuelve a pantalla con foco; no hay ocultacion a la bandeja." => {
            "Real X11 simulation: minimizing the window through the window manager keeps it down as usual and Show ProxySend brings it back to screen with focus; there is no tray hiding."
        }
        "Mostrar ProxySend verificado por D-Bus (dbusmenu) con la ventana minimizada: la ventana vuelve a aparecer; tambien responde la segunda instancia." => {
            "Show ProxySend verified over D-Bus (dbusmenu) with the window minimized: the window comes back; the second instance also works."
        }
        "Salir desde la bandeja termina el proceso AL INSTANTE (medido en simulacion con la ventana visible y minimizada): sin esperas ni cierre garantizado de respaldo." => {
            "Quit from the tray ends the process INSTANTLY (measured in simulation with the window visible and minimized): no waits and no guaranteed fallback close."
        }
        "Activar y Desactivar proxy desde la bandeja verificados por D-Bus: el nivel de escritorio se aplica de verdad (gsettings) y el resultado queda en historial, registro e informe." => {
            "Enable and Disable proxy from the tray verified over D-Bus: the desktop level is really applied (gsettings) and the result is recorded in history, log and report."
        }
        // v2.3.5: verificaciones de la revision a fondo
        "Revision a fondo de la version 2.3.5: recorrido completo del codigo fuente, cero funciones sin uso, cero dependencias sin usar y clippy en silencio total (los 11 avisos de estilo de antes, corregidos)." => {
            "Deep review of version 2.3.5: full pass over the source code, zero unused functions, zero unused dependencies and clippy totally silent (the previous 11 style warnings, fixed)."
        }
        "Regresion re-verificada en la simulacion real de la 2.3.5: los menus de la bandeja (Mostrar ProxySend, Activar/Desactivar proxy y Salir) responden al instante y los 4 entregables vuelven a pasar las 28 comprobaciones de portabilidad." => {
            "Regression re-verified in the 2.3.5 real simulation: the tray menus (Show ProxySend, Enable/Disable proxy and Quit) respond instantly and the 4 deliverables pass the 28 portability checks again."
        }
        "Revision a fondo de la 2.3.5: cero funciones muertas, cero dependencias sin usar y clippy en silencio total." => {
            "Deep review of 2.3.5: zero dead functions, zero unused dependencies and clippy totally silent."
        }
        "Regresion re-verificada en simulacion real: menus de la bandeja al instante y las 28 comprobaciones de portabilidad pasadas de nuevo." => {
            "Regression re-verified in real simulation: tray menus respond instantly and the 28 portability checks pass again."
        }
        // v2.4.0: interfaz azul, revision de codigo muerto y simulacion E2E
        "Nueva interfaz azul: barra lateral azul marino con iconos dibujados, boton de encendido rediseñado y tarjetas con sombra suave; tema claro y oscuro reafinados en azul." => {
            "New blue interface: navy-blue sidebar with drawn icons, redesigned power button and cards with soft shadows; light and dark themes re-tuned in blue."
        }
        "Nueva interfaz azul (v2.4.0): barra lateral azul marino con iconos dibujados, boton de encendido rediseñado y tarjetas con sombra suave; tema claro y oscuro reafinados en azul." => {
            "New blue interface (v2.4.0): navy-blue sidebar with drawn icons, redesigned power button and cards with soft shadows; light and dark themes re-tuned in blue."
        }
        // v2.4.0: revision a fondo post-entrega (errores reparados)
        "Revision a fondo post-entrega de la 2.4.0: el proceso externo se elimina al agotar el timeout (antes quedaba colgado), el handle de la bandeja es reemplazable en el paso autoarranque a ventana, ajustes e historial se escriben de forma atomica y sin panics de Instant al arrancar." => {
            "In-depth post-release review of 2.4.0: the external process is killed when the timeout expires (it used to hang), the tray handle is replaceable on the autostart-to-window handoff, settings and history are written atomically and no Instant arithmetic panics at startup."
        }
        "Revision a fondo post-entrega: timeout que elimina al proceso externo colgado, handle de bandeja reemplazable, escritura atomica de ajustes e historial y aritmetica de Instant sin panics." => {
            "In-depth post-release review: timeout that kills the hung external process, replaceable tray handle, atomic settings and history writes and Instant arithmetic without panics."
        }
        "Arranque automatico: el proxy activo no tiene niveles de escritorio ni terminal que aplicar." => {
            "Autostart: the active proxy has no desktop or terminal levels to apply."
        }
        "Todos los niveles quedaron desactivados: el proxy ya no esta activado." => {
            "All levels were switched off: the proxy is no longer active."
        }
        "Revision de codigo muerto con escaneo automatizado de funciones, constantes, dependencias y claves de traduccion: cero restos en toda la base de codigo." => {
            "Dead code review with automated scanning of functions, constants, dependencies and translation keys: zero leftovers across the codebase."
        }
        "Revision de codigo muerto con escaneo automatizado de la 2.4.0: funciones, constantes, dependencias y claves de traduccion verificadas una por una; cero restos." => {
            "Dead code review with automated scanning in 2.4.0: functions, constants, dependencies and translation keys checked one by one; zero leftovers."
        }
        "Simulacion E2E real en X11 con bandeja por D-Bus y capturas de pantalla: arranque, cambio de tema, navegacion, activacion del proxy y salida verificados." => {
            "Real E2E simulation on X11 with D-Bus tray and screenshots: startup, theme switch, navigation, proxy activation and exit verified."
        }
        "Simulacion E2E real en X11 de la 2.4.0 con bandeja por D-Bus y capturas de pantalla: arranque, cambio de tema, navegacion, activacion del proxy y salida verificados." => {
            "Real E2E simulation on X11 in 2.4.0 with D-Bus tray and screenshots: startup, theme switch, navigation, proxy activation and exit verified."
        }
        // v2.3.3: verificacion de portabilidad de los 4 entregables
        "Portabilidad verificada en los 4 entregables (28 comprobaciones): AppImage se extrae y corre sin libfuse2, el .deb solo declara libc6, el tar.gz instala en el HOME de prueba y el .run se autoextrae e instala." => {
            "Portability verified on all 4 deliverables (28 checks): the AppImage extracts and runs without libfuse2, the .deb declares only libc6, the tar.gz installs into a test HOME and the .run self-extracts and installs."
        }
        "Cero dependencias externas: el binario solo enlaza glibc (libc, libm, libgcc) y las 2 librerias XKB que usa van embebidas en los 4 formatos; el resto es base de cualquier escritorio Linux." => {
            "Zero external dependencies: the binary links only glibc (libc, libm, libgcc) and the 2 XKB libraries it uses are embedded in all 4 formats; the rest is base of any Linux desktop."
        }
        "Abrir ProxySend cuando ya esta corriendo trae la ventana existente al frente (la peticion de la segunda instancia ahora se atiende)." => {
            "Opening ProxySend while it is already running brings the existing window to the front (the second-instance request is now handled)."
        }
        "Revision a fondo del codigo completo: compilacion limpia sin ninguna advertencia del compilador (antes 38), codigo muerto eliminado y una clave de traduccion duplicada corregida." => {
            "Full deep code review: clean build with no compiler warnings (previously 38), dead code removed and one duplicated translation key fixed."
        }
        "Icono de la bandeja sincronizado: al activar o desactivar el proxy desde la ventana, el icono y su tooltip se refrescan al momento (antes quedaba con el estado viejo)." => {
            "Tray icon kept in sync: enabling or disabling the proxy from the window now refreshes the tray icon and its tooltip right away (previously it kept the old state)."
        }

        /* ---------- bandeja del sistema (v2.3.2) ---------- */
        "Ventana reabierta en primer plano desde la bandeja del sistema" => {
            "Window reopened in the foreground from the system tray"
        }
        "Ventana mostrada en pantalla" => "Window shown on screen",

        /* ---------- operaciones (ops.rs) ---------- */
        "Validacion rechazada: {0}" => "Validation rejected: {0}",
        "No hay ningun nivel seleccionado (escritorio, sistema ni terminal). Marca al menos un nivel en la pagina Estado y vuelve a intentarlo." => {
            "No level is selected (desktop, system or terminal). Tick at least one level on the Status page and try again."
        }
        "Nivel {0}: ok ({1})" => "Level {0}: ok ({1})",
        "Nivel {0}: cancelado - {1}" => "Level {0}: cancelled - {1}",
        "Nivel {0}: fallo - {1}" => "Level {0}: failed - {1}",
        "Proxy aplicado: {0}" => "Proxy applied: {0}",
        "Ningun nivel se aplico: el proxy NO quedo activado. El detalle de cada nivel esta arriba y en el Informe." => {
            "No level was applied: the proxy was NOT enabled. The detail of each level is above and in the Report."
        }
        "Nivel desconocido." => "Unknown level.",
        "Nivel \"{0}\" {1}" => "Level \"{0}\" {1}",
        "Nivel {0} {1}: ok" => "Level {0} {1}: ok",
        "Nivel {0} {1}: cancelado - {2}" => "Level {0} {1}: cancelled - {2}",
        "Nivel {0} {1}: fallo - {2}" => "Level {0} {1}: failed - {2}",
        "Valores por defecto restaurados (la biblioteca se conserva)" => "Defaults restored (the library is kept)",
        "Arranque automatico: no habia proxy guardado para aplicar." => "Autostart: there was no saved proxy to apply.",
        "Arranque automatico: proxy guardado re-aplicado en escritorio y terminales." => {
            "Autostart: saved proxy re-applied on desktop and terminals."
        }
        "Arranque automatico: re-aplicado con avisos - {0}" => "Autostart: re-applied with warnings - {0}",
        "Arranque automatico: proxy re-aplicado sin errores" => "Autostart: proxy re-applied without errors",
        "Arranque automatico: {0}" => "Autostart: {0}",
        "ProxySend {0} iniciado ({1}) - escritorio {2}" => "ProxySend {0} started ({1}) - desktop {2}",
        "ventana" => "window",
        "autoarranque" => "autostart",

        /* ---------- sysops: escritorio GNOME ---------- */
        "Este sistema no expone org.gnome.system.proxy (gsettings); el escritorio no se puede configurar desde aqui." => {
            "This system does not expose org.gnome.system.proxy (gsettings); the desktop cannot be configured from here."
        }
        "Proxy del escritorio (GNOME) configurado en modo manual." => "Desktop proxy (GNOME) set to manual mode.",
        "No se pudo activar el proxy del escritorio: {0}" => "Could not enable the desktop proxy: {0}",
        "gsettings no disponible." => "gsettings not available.",
        "Proxy del escritorio desactivado (modo none)." => "Desktop proxy disabled (none mode).",
        "No se pudo desactivar el proxy del escritorio: {0}" => "Could not disable the desktop proxy: {0}",
        "Ajustes de proxy del escritorio restablecidos a los valores por defecto." => {
            "Desktop proxy settings restored to their defaults."
        }
        "No se pudo restablecer el modo del proxy." => "Could not reset the proxy mode.",

        /* ---------- sysops: escritorio KDE ---------- */
        "No se pudo escribir {0}: {1}" => "Could not write {0}: {1}",
        "Proxy del escritorio KDE sin activar (ProxyType {0})." => "KDE desktop proxy not enabled (ProxyType {0}).",
        "KDE no guarda proxys SOCKS5 en kioslaverc. Usa los niveles Sistema y Terminal para SOCKS5." => {
            "KDE does not store SOCKS5 proxies in kioslaverc. Use the System and Terminal levels for SOCKS5."
        }
        "Proxy del escritorio (KDE) guardado en kioslaverc. Las apps de KDE lo leen al abrirse." => {
            "Desktop proxy (KDE) saved in kioslaverc. KDE apps read it when they open."
        }
        " KDE pedira el usuario y contraseña del proxy cuando los necesite." => {
            " KDE will ask for the proxy username and password when it needs them."
        }
        "Proxy del escritorio (KDE) desactivado." => "Desktop proxy (KDE) disabled.",
        "Ajustes de proxy del escritorio KDE eliminados de kioslaverc." => {
            "KDE desktop proxy settings removed from kioslaverc."
        }
        "Escritorio (COSMIC)" => "Desktop (COSMIC)",
        "Escritorio (KDE)" => "Desktop (KDE)",
        "Escritorio (GNOME)" => "Desktop (GNOME)",
        "Escritorio (XFCE)" => "Desktop (XFCE)",
        "Escritorio" => "Desktop",
        "Este escritorio no permite configurar el proxy desde la app (ni gsettings ni KDE kioslaverc). Usa los niveles Sistema y Terminal: funcionan igual de bien." => {
            "This desktop does not allow configuring the proxy from the app (neither gsettings nor KDE kioslaverc). Use the System and Terminal levels: they work just as well."
        }
        "Este escritorio no permite configurar el proxy desde la app. Usa los niveles Sistema y Terminal." => {
            "This desktop does not allow configuring the proxy from the app. Use the System and Terminal levels."
        }
        "Este escritorio no permite configurar el proxy desde la app." => {
            "This desktop does not allow configuring the proxy from the app."
        }
        " Nota: tu escritorio es XFCE; Firefox y las apps GNOME respetan estos ajustes, el resto de programas usan los niveles Sistema y Terminal." => {
            " Note: your desktop is XFCE; Firefox and GNOME apps respect these settings, other programs use the System and Terminal levels."
        }
        " Nota: COSMIC no trae ajuste de proxy propio, pero sus apps son GTK y usan estas mismas preferencias: se aplican al instante." => {
            " Note: COSMIC has no proxy setting of its own, but its apps are GTK and use these same preferences: they apply instantly."
        }
        "gsettings o el esquema org.gnome.system.proxy no esta disponible en este sistema." => {
            "gsettings or the org.gnome.system.proxy schema is not available on this system."
        }
        "modo {0} (sin proxy)" => "mode {0} (no proxy)",
        "desconocido" => "unknown",
        "manual sin servidores definidos" => "manual with no servers defined",
        " · usuario {0}" => " · user {0}",

        /* ---------- sysops: nivel sistema y terminales ---------- */
        "Se cancelo la ventana de contraseña de administrador: no se pudo escribir /etc/environment. Escritorio y terminales se aplican igual; si tambien quieres el nivel de sistema, vuelve a intentarlo y escribe tu contraseña cuando salga la ventana." => {
            "The administrator password window was cancelled: /etc/environment could not be written. Desktop and terminals are applied anyway; if you also want the system level, try again and type your password when the window appears."
        }
        "No se encontro pkexec (PolicyKit). Instalalo con: sudo apt install policykit-1" => {
            "pkexec (PolicyKit) not found. Install it with: sudo apt install policykit-1"
        }
        "No se pudo preparar el archivo temporal: {0}" => "Could not prepare the temporary file: {0}",
        "Variables de proxy escritas en /etc/environment (se aplican al volver a iniciar sesion)." => {
            "Proxy variables written to /etc/environment (they apply when you log in again)."
        }
        "/etc/environment ya esta sin proxy." => "/etc/environment is already proxy-free.",
        "Variables de proxy eliminadas de /etc/environment." => "Proxy variables removed from /etc/environment.",
        "Variables exportadas en {0} (activas en las terminales nuevas)." => {
            "Variables exported in {0} (active in new terminals)."
        }
        "Bloque de ProxySend eliminado de las terminales." => "ProxySend block removed from the terminals.",
        "Las terminales ya estaban sin proxy." => "The terminals were already proxy-free.",
        "Vuelve a aplicar el proxy guardado al iniciar sesion" => "Re-applies the saved proxy at login",
        "Autoarranque activado: {0}" => "Autostart enabled: {0}",
        "No se pudo crear {0}: {1}" => "Could not create {0}: {1}",
        "ProxySend abrira al iniciar sesion y aplicara el proxy guardado." => {
            "ProxySend will open at login and apply the saved proxy."
        }

        /* ---------- sysops: prueba con curl ---------- */
        "No se pudo conectar (curl no disponible)." => "Could not connect (curl not available).",
        "No se pudo conectar: {0}" => "Could not connect: {0}",
        "Respuesta inesperada de curl." => "Unexpected curl response.",

        /* ---------- prueba de conexion (nettest) ---------- */
        "respuesta HTTP {0}" => "HTTP response {0}",
        "respuesta vacia" => "empty response",
        "Fallo de conexion: {0}" => "Connection failed: {0}",
        "Prueba con proxy" => "Test with proxy",
        "Prueba directa" => "Direct test",
        "{0}: IP {1} - HTTP {2} - {3} s" => "{0}: IP {1} - HTTP {2} - {3} s",

        /* ---------- biblioteca completa (libexport) ---------- */
        "Este archivo no es una biblioteca completa de ProxySend (usa Importar configuracion para archivos de un solo proxy)." => {
            "This file is not a full ProxySend library (use Import configuration for single-proxy files)."
        }
        "La biblioteca no es valida: {0}" => "The library is not valid: {0}",
        "\"{0}\" se descarto por tener datos invalidos" => "\"{0}\" was discarded for having invalid data",
        "Biblioteca importada: {0} proxies, {1} eventos de historial ({2})" => {
            "Library imported: {0} proxies, {1} history events ({2})"
        }
        "Importacion completa desde {0}: {1} proxies validos, {2} historial" => {
            "Full import from {0}: {1} valid proxies, {2} history"
        }
        "La biblioteca decia tener un proxy activo pero sin configuracion; el sistema actual no se toco." => {
            "The library said it had an active proxy but no configuration; the current system was not touched."
        }

        /* ---------- reconexion automatica (watcher) ---------- */
        "No hay un proxy configurado para verificar." => "There is no proxy configured to check.",
        "El proxy no esta activado: nada que reconectar." => "The proxy is not enabled: nothing to reconnect.",
        "OK - IP {0} - {1} s" => "OK - IP {0} - {1} s",
        "El proxy VOLVIO A RESPONDER despues del fallo: {0}" => "The proxy RESPONDED AGAIN after the failure: {0}",
        "Verificacion automatica correcta: {0}" => "Automatic check succeeded: {0}",
        "FALLO ({0}) - proxy re-aplicado en {1}" => "FAILURE ({0}) - proxy re-applied on {1}",
        "escritorio: {0}" => "desktop: {0}",
        "terminales: {0}" => "terminals: {0}",
        "ningun nivel configurado" => "no level configured",
        "sin respuesta del proxy" => "no response from the proxy",
        "Temporizador de reconexion automatica iniciado" => "Automatic reconnection timer started",
        "Temporizador de reconexion automatica detenido" => "Automatic reconnection timer stopped",
        "hace {0} s" => "{0} s ago",
        "hace {0} min" => "{0} min ago",
        "hace {0} h {1} min" => "{0} h {1} min ago",

        /* ---------- validacion (core.rs) ---------- */
        "El protocolo debe ser http, https o socks5." => "The protocol must be http, https or socks5.",
        "El host no es valido (sin espacios, comillas ni simbolos raros)." => {
            "The host is not valid (no spaces, quotes or odd symbols)."
        }
        "El puerto debe ser un numero entre 1 y 65535." => "The port must be a number between 1 and 65535.",
        "El host HTTP no es valido." => "The HTTP host is not valid.",
        "El puerto HTTP debe estar entre 1 y 65535." => "The HTTP port must be between 1 and 65535.",
        "El host HTTPS no es valido." => "The HTTPS host is not valid.",
        "El puerto HTTPS debe estar entre 1 y 65535." => "The HTTPS port must be between 1 and 65535.",
        "Falta el usuario del proxy." => "The proxy username is missing.",
        "Hay una excepcion (host excluido) con caracteres no validos." => {
            "There is an exception (excluded host) with invalid characters."
        }

        /* ---------- consola (main.rs) ---------- */
        "OK  IP {0}  HTTP {1}  {2} s" => "OK  IP {0}  HTTP {1}  {2} s",
        "FALLO  HTTP {0}  respuesta inesperada" => "FAILURE  HTTP {0}  unexpected response",
        "FALLO  {0}" => "FAILURE  {0}",
        "fallo - {0}" => "failed - {0}",
        "Informe generado: {0}" => "Report generated: {0}",
        "Informe generado desde CLI: {0}" => "Report generated from CLI: {0}",
        "Proxy desactivado con proxysend --quitar" => "Proxy disabled with proxysend --quitar",
        "La configuracion no es valida: {0}" => "The configuration is not valid: {0}",

        /* ---------- informe diagnostico (report.rs) ---------- */
        "INFORME DE DIAGNOSTICO DE PROXYSEND" => "PROXYSEND DIAGNOSTIC REPORT",
        "Generado: {0} (hora local)" => "Generated: {0} (local time)",
        "Version de la app: {0}" => "App version: {0}",
        "Usuario: {0} - HOME: {1}" => "User: {0} - HOME: {1}",
        "Linux (sin /etc/os-release)" => "Linux (no /etc/os-release)",
        "1. SISTEMA Y ESCRITORIO" => "1. SYSTEM AND DESKTOP",
        "Sistema operativo : {0}" => "Operating system  : {0}",
        "Nucleo            : {0}" => "Kernel            : {0}",
        "Sesion            : {0}" => "Session           : {0}",
        "(sin definir)" => "(not set)",
        "XDG_CURRENT_DESKTOP={0} - sesion {1}" => "XDG_CURRENT_DESKTOP={0} - session {1}",
        "Escritorio detectado: {0}{1}" => "Detected desktop: {0}{1}",
        "Nivel escritorio configurable: {0}" => "Desktop level configurable: {0}",
        "si" => "yes",
        "no" => "no",
        "Detalle: {0}" => "Detail: {0}",
        "2. HERRAMIENTAS EXTERNAS (para saber que depende de que)" => "2. EXTERNAL TOOLS (to know what depends on what)",
        "ajustes de proxy en GNOME/COSMIC/XFCE" => "proxy settings on GNOME/COSMIC/XFCE",
        "escribir /etc/environment (nivel sistema)" => "writing /etc/environment (system level)",
        "respaldo de la prueba de conexion" => "connection test fallback",
        "ventana para elegir archivos al importar" => "file picker window when importing",
        "ventana para elegir archivos al importar (KDE)" => "file picker window when importing (KDE)",
        "terminales Bash (~/.bashrc)" => "Bash terminals (~/.bashrc)",
        "terminales Zsh (~/.zshrc)" => "Zsh terminals (~/.zshrc)",
        "solo para pruebas, no lo usa la app" => "for tests only, the app does not use it",
        "[presente ]" => "[present  ]",
        "[FALTA    ]" => "[MISSING  ]",
        "Nota: si una herramienta falta, la app sigue funcionando con los otros niveles." => {
            "Note: if a tool is missing, the app keeps working with the other levels."
        }
        "3. ESTADO DE LOS TRES NIVELES" => "3. STATUS OF THE THREE LEVELS",
        "proxy ACTIVO" => "proxy ACTIVE",
        "sin proxy" => "no proxy",
        "  Escritorio : {0}{1}" => "  Desktop  : {0}{1}",
        "  Sistema    : {0}{1}" => "  System   : {0}{1}",
        "  Terminales : {0}{1}" => "  Terminal : {0}{1}",
        "Niveles seleccionados por el usuario: escritorio={0} sistema={1} terminal={2}" => {
            "Levels selected by the user: desktop={0} system={1} terminal={2}"
        }
        " (aviso: /etc/environment se puede leer, el nivel escritorio no)" => {
            " (warning: /etc/environment can be read, the desktop level cannot)"
        }
        "4. CONFIGURACION VIGENTE" => "4. CURRENT CONFIGURATION",
        "  No hay ninguna configuracion guardada." => "  No configuration saved.",
        "  Proxy activado : {0}" => "  Proxy enabled : {0}",
        "SI" => "YES",
        "  Perfil en uso  : {0}" => "  Profile in use : {0}",
        "(sin biblioteca)" => "(no library)",
        "  Resumen        : {0}" => "  Summary        : {0}",
        "  Autenticacion  : usuario={0} contrasena={1}" => "  Authentication : user={0} password={1}",
        "  Excepciones    : {0}" => "  Exceptions     : {0}",
        "(por defecto: localhost, 127.0.0.0/8, ::1)" => "(default: localhost, 127.0.0.0/8, ::1)",
        "5. BIBLIOTECA DE PROXIES" => "5. PROXY LIBRARY",
        "  (vacia)" => "  (empty)",
        "  <== ACTIVO" => "  <== ACTIVE",
        "6. RECONEXION AUTOMATICA" => "6. AUTOMATIC RECONNECTION",
        "activado (en marcha solo con esta opcion activada), cada {0} min" => {
            "enabled (running only while this option is on), every {0} min"
        }
        "desactivado (no hay ningun temporizador en marcha)" => "disabled (no timer is running)",
        "  Todavia no se ha hecho ninguna verificacion automatica." => "  No automatic check has been done yet.",
        "  Ultima verificacion: {0} - {1}" => "  Last check: {0} - {1}",
        "7. VARIABLES DE ENTORNO DEL PROCESO (con credenciales ocultas)" => {
            "7. PROCESS ENVIRONMENT VARIABLES (credentials hidden)"
        }
        "  (ninguna: el proceso se abrio sin variables de proxy)" => {
            "  (none: the process started without proxy variables)"
        }
        "8. RESUMEN DE ERRORES Y AVISOS" => "8. ERRORS AND WARNINGS SUMMARY",
        "  Eventos registrados en total: {0} ({1} info, {2} avisos, {3} errores)" => {
            "  Total events recorded: {0} ({1} info, {2} warnings, {3} errors)"
        }
        "  No hay ningun error registrado." => "  No errors recorded.",
        "  Ultimo error: {0}" => "  Last error: {0}",
        "9. REGISTRO COMPLETO DE EVENTOS (LOG COMPLETO)" => "9. FULL EVENT LOG",
        "  (vacio)" => "  (empty)",
        "10. HISTORIAL DE ACTIVIDAD (compatibilidad con versiones anteriores)" => {
            "10. ACTIVITY HISTORY (backward compatibility)"
        }
        "11. VERIFICACIONES DE CALIDAD DE LA VERSION" => "11. VERSION QUALITY CHECKS",
        "  Comprobaciones hechas sobre esta version antes de publicarla:" => {
            "  Checks made on this version before publishing it:"
        }
        "Campos de escritura con fondo y borde resaltado: servidor, puerto, usuario, contrasena, excepciones y el nombre al guardar en la biblioteca" => {
            "Text fields with highlighted background and border: server, port, username, password, exceptions and the name when saving to the library"
        }
        "Tamano e intensidad de los textos afinados para leerlo todo bien en tema claro (incluidos los menus de la barra lateral)" => {
            "Text size and intensity fine-tuned so everything reads well in the light theme (including the sidebar menus)"
        }
        "Colores revisados eligiendo la combinacion de mejor lectura, manteniendo el tema azul de la app" => {
            "Colors reviewed choosing the most readable combination, keeping the app's blue theme"
        }
        "Botones Actualizar de Mis proxies, Historial e Informe verificados: recargan los datos al pulsarlos" => {
            "Refresh buttons of My proxies, History and Report verified: they reload the data when pressed"
        }
        "Activar un proxy SIN poner la contrasena de administrador: se explica el fallo en pantalla y queda anotado como aviso (no se queda silencioso)" => {
            "Enabling a proxy WITHOUT the administrator password: the failure is explained on screen and logged as a warning (never silent)"
        }
        "Activar un proxy sin ningun nivel marcado: rechazado con mensaje claro (antes quedaba 'activo' sin hacer nada)" => {
            "Enabling a proxy with no level selected: rejected with a clear message (before it stayed 'active' doing nothing)"
        }
        "Licencia cambiada de MIT a GNU GPL v3: en la pagina Acerca de, en Cargo.toml, en el copyright del paquete .deb y en COPYING.txt del codigo fuente" => {
            "License changed from MIT to GNU GPL v3: on the About page, in Cargo.toml, in the .deb package copyright and in COPYING.txt of the source code"
        }
        "Temporizador de reconexion automatica reparado: ya NO arranca al ejecutar la app con la opcion apagada; arranca solo al activar la opcion y se detiene de verdad al desactivarla" => {
            "Automatic reconnection timer fixed: it NO LONGER starts when the app runs with the option off; it starts only when the option is enabled and truly stops when disabled"
        }
        "Idioma espanol/ingles incorporado: la app arranca en espanol y el cambio en Preferencias se aplica a toda la interfaz, la bandeja, el informe y la consola" => {
            "Spanish/English language built in: the app starts in Spanish and the switch in Preferences applies to the whole interface, tray, report and console"
        }
        "Pruebas unitarias (cargo test) mas simulaciones de interfaz y de flujo con contrasena cancelada" => {
            "Unit tests (cargo test) plus interface and flow simulations with a cancelled password"
        }
        "12. PANIC CAPTURADO (error inesperado registrado)" => "12. CAUGHT PANIC (unexpected error logged)",
        "Fin del informe. Este archivo NO contiene contrasenas: se muestran como ****." => {
            "End of report. This file contains NO passwords: they are shown as ****."
        }
        "No se pudo escribir el informe: {0}" => "Could not write the report: {0}",

        /* ---------- ayuda de consola (main.rs) ---------- */
        "ProxySend {0} - gestor de proxy para Linux\n\nUso:\n  proxysend                 abre la aplicacion\n  proxysend --autostart     aplica el proxy guardado al iniciar sesion (sin ventana)\n  proxysend --aplicar       aplica el proxy guardado y sale\n  proxysend --quitar        quita el proxy configurado en todos los niveles y sale\n  proxysend --informe       genera el informe diagnostico completo y sale\n  proxysend --verificar     verifica el proxy activo ahora mismo (reconexion) y sale\n  proxysend --pagina X      abre en una pagina (estado|ajustes|proxies|historial|informe|acerca)\n  proxysend --oscuro        inicia con el tema oscuro (no se guarda)\n  proxysend --importar F    importa una configuracion JSON al formulario\n  proxysend --probar URL    prueba una conexion a traves de un proxy y sale (URL como\n                            http://host:puerto, socks5://host:puerto o \"directo\" para\n                            probar sin proxy)\n  proxysend --version       muestra la version\n" => {
            "ProxySend {0} - proxy manager for Linux\n\nUsage:\n  proxysend                 opens the application\n  proxysend --autostart     applies the saved proxy at login (no window)\n  proxysend --aplicar       applies the saved proxy and exits\n  proxysend --quitar        removes the configured proxy from all levels and exits\n  proxysend --informe       generates the full diagnostic report and exits\n  proxysend --verificar     checks the active proxy right now (reconnection) and exits\n  proxysend --pagina X      opens on a page (estado|ajustes|proxies|historial|informe|acerca)\n  proxysend --oscuro        starts with the dark theme (not saved)\n  proxysend --importar F    imports a JSON configuration into the form\n  proxysend --probar URL    tests a connection through a proxy and exits (URL like\n                            http://host:port, socks5://host:port or \"directo\" to\n                            test without a proxy)\n  proxysend --version       shows the version\n"
        }

        /* ---------- libexport: nombre de archivo de la biblioteca ---------- */
        _ => return None,
    };
    Some(found)
}
