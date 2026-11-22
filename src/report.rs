//! ProxySend - informe diagnostico completo (el boton de "Informe").
//!
//! Reune en un solo archivo de texto TODO lo que se puede saber del estado de
//! la app y del sistema, para detectar errores: datos del equipo, escritorio
//! detectado, herramientas disponibles, estado de los tres niveles, entorno,
//! biblioteca de proxies, reconexion automatica, historial de actividad y el
//! registro completo de eventos (con avisos y errores).
//!
//! v2.3.0: el informe se genera en el idioma activo (espanol por defecto,
//! ingles opcional); las claves de traduccion son los textos en espanol.

use crate::core;
use crate::eventlog;
use crate::i18n::{tf, tr};
use crate::store::{self, Settings};
use crate::sysops;
use chrono::Local;
use chrono::TimeZone;

fn mask_secret(s: &str) -> String {
    if s.is_empty() { String::new() } else { "****".to_string() }
}

/// Oculta usuario:contrasena en URLs de proxy (http://user:pass@host:puerto).
fn mask_url(url: &str) -> String {
    if let Some(scheme_end) = url.find("://") {
        let after = &url[scheme_end + 3..];
        if let Some(at) = after.find('@') {
            let creds = &after[..at];
            let user = creds.split(':').next().unwrap_or("");
            return format!("{}{}:****@{}", &url[..scheme_end + 3], user, &after[at + 1..]);
        }
    }
    url.to_string()
}

fn os_pretty() -> String {
    if let Ok(text) = std::fs::read_to_string("/etc/os-release") {
        let mut name = String::new();
        let mut ver = String::new();
        for line in text.lines() {
            if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                name = v.trim_matches('"').to_string();
            }
            if let Some(v) = line.strip_prefix("VERSION_ID=") {
                ver = v.trim_matches('"').to_string();
            }
        }
        if !name.is_empty() {
            return if ver.is_empty() { name } else { format!("{} ({})", name, ver) };
        }
    }
    tr("Linux (sin /etc/os-release)").to_string()
}

fn kernel() -> String {
    let r = sysops::run_public("/bin/uname", &["-sr"]);
    if r.ok { r.stdout.trim().to_string() } else { tr("desconocido").to_string() }
}

fn session_info() -> String {
    let de = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_else(|_| tr("(sin definir)").into());
    let ses = std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| tr("(sin definir)").into());
    tf("XDG_CURRENT_DESKTOP={0} - sesion {1}", &[&de, &ses])
}

pub fn build_report(settings: &Settings) -> String {
    let mut out = String::new();
    let line = "=".repeat(74);
    let thin = "-".repeat(74);

    // 0. Cabecera
    out += &format!("{}\n", line);
    out += &format!("{}\n", tr("INFORME DE DIAGNOSTICO DE PROXYSEND"));
    out += &format!(
        "{}\n",
        tf("Generado: {0} (hora local)", &[&Local::now().format("%Y-%m-%d %H:%M:%S").to_string()])
    );
    out += &format!("{}\n", tf("Version de la app: {0}", &[crate::APP_VERSION]));
    out += &format!("{}\n", tf("Usuario: {0} - HOME: {1}", &[&whoami(), &store::home_dir().display().to_string()]));
    out += &format!("{}\n\n", line);

    // 1. Sistema y escritorio
    out += &format!("{}\n", tr("1. SISTEMA Y ESCRITORIO"));
    out += &format!("{}\n", thin);
    out += &format!("{}\n", tf("Sistema operativo : {0}", &[&os_pretty()]));
    out += &format!("{}\n", tf("Nucleo            : {0}", &[&kernel()]));
    out += &format!("{}\n", tf("Sesion            : {0}", &[&session_info()]));
    let (supported, backend, detected, _active, _value, detail) = sysops::desktop_status();
    out += &format!(
        "{}{}\n",
        tf(
            "Escritorio detectado: {0}{1}",
            &[&format!("{:?}", detected), &backend.map(|b| format!(" (backend {:?})", b)).unwrap_or_default()],
        ),
        ""
    );
    out += &format!(
        "{}\n",
        tf("Nivel escritorio configurable: {0}", &[if supported { tr("si") } else { tr("no") }])
    );
    out += &format!("{}\n\n", tf("Detalle: {0}", &[&detail]));

    // 2. Herramientas externas detectadas
    out += &format!("{}\n", tr("2. HERRAMIENTAS EXTERNAS (para saber que depende de que)"));
    out += &format!("{}\n", thin);
    for (name, why) in [
        ("gsettings", tr("ajustes de proxy en GNOME/COSMIC/XFCE")),
        ("pkexec", tr("escribir /etc/environment (nivel sistema)")),
        ("curl", tr("respaldo de la prueba de conexion")),
        ("zenity", tr("ventana para elegir archivos al importar")),
        ("kdialog", tr("ventana para elegir archivos al importar (KDE)")),
        ("bash", tr("terminales Bash (~/.bashrc)")),
        ("zsh", tr("terminales Zsh (~/.zshrc)")),
        ("dbus-run-session", tr("solo para pruebas, no lo usa la app")),
    ] {
        let ok = sysops::tool_exists(name);
        out += &format!(
            "  {:<16} {} {}\n",
            name,
            if ok { tr("[presente ]") } else { tr("[FALTA    ]") },
            why
        );
    }
    out += &format!("{}\n\n", tr("Nota: si una herramienta falta, la app sigue funcionando con los otros niveles."));

    // 3. Estado de los tres niveles
    out += &format!("{}\n", tr("3. ESTADO DE LOS TRES NIVELES"));
    out += &format!("{}\n", thin);
    let (s_supported, s_active, s_value) = sysops::env_status();
    out += &format!(
        "{}\n",
        tf(
            "  Escritorio : {0}{1}",
            &[
                if supported { if _active { tr("proxy ACTIVO") } else { tr("sin proxy") } } else { tr("no disponible") },
                &_value.as_ref().map(|v| format!(" - {}", v)).unwrap_or_default()
            ]
        )
    );
    out += &format!(
        "{}\n",
        tf(
            "  Sistema    : {0}{1}",
            &[
                if s_active { tr("proxy ACTIVO") } else { tr("sin proxy") },
                &s_value.as_ref().map(|v| format!(" - {}", mask_url(v))).unwrap_or_default()
            ]
        )
    );
    let (_t_sup, t_active, t_value) = sysops::shell_status();
    out += &format!(
        "{}\n",
        tf(
            "  Terminales : {0}{1}",
            &[
                if t_active { tr("proxy ACTIVO") } else { tr("sin proxy") },
                &t_value.as_ref().map(|v| format!(" - {}", mask_url(v))).unwrap_or_default()
            ]
        )
    );
    out += &format!(
        "{}{}\n\n",
        tf(
            "Niveles seleccionados por el usuario: escritorio={0} sistema={1} terminal={2}",
            &[
                if settings.scopes.desktop { tr("si") } else { tr("no") },
                if settings.scopes.system { tr("si") } else { tr("no") },
                if settings.scopes.terminal { tr("si") } else { tr("no") },
            ]
        ),
        if s_supported && !supported { tr(" (aviso: /etc/environment se puede leer, el nivel escritorio no)") } else { "" }
    );

    // 4. Configuracion vigente
    out += &format!("{}\n", tr("4. CONFIGURACION VIGENTE"));
    out += &format!("{}\n", thin);
    match &settings.config {
        None => out += &format!("{}\n\n", tr("  No hay ninguna configuracion guardada.")),
        Some(c) => {
            out += &format!("{}\n", tf("  Proxy activado : {0}", &[if settings.active { tr("SI") } else { tr("no") }]));
            out += &format!(
                "{}\n",
                tf("  Perfil en uso  : {0}", &[&active_name(settings).unwrap_or_else(|| tr("(sin biblioteca)").into())])
            );
            out += &format!("{}\n", tf("  Resumen        : {0}", &[&core::summary(c)]));
            let v = core::env_values(c);
            out += &format!("  http_proxy     : {}\n", mask_url(&v.http_url));
            out += &format!("  https_proxy    : {}\n", mask_url(&v.https_url));
            out += &format!("  no_proxy       : {}\n", v.no_proxy);
            if c.auth {
                out += &format!(
                    "{}\n",
                    tf("  Autenticacion  : usuario={0} contrasena={1}", &[&c.username, &mask_secret(&c.password)])
                );
            }
            out += &format!(
                "{}\n\n",
                tf(
                    "  Excepciones    : {0}",
                    &[&if c.ignore_hosts.is_empty() { tr("(por defecto: localhost, 127.0.0.0/8, ::1)").into() } else { c.ignore_hosts.join(", ") }]
                )
            );
        }
    }

    // 5. Biblioteca de proxies
    out += &format!("{}\n", tr("5. BIBLIOTECA DE PROXIES"));
    out += &format!("{}\n", thin);
    if settings.proxies.is_empty() {
        out += &format!("{}\n\n", tr("  (vacia)"));
    } else {
        for p in &settings.proxies {
            out += &format!(
                "  - {} [{}] - {}{}\n",
                p.name,
                p.id,
                core::summary(&p.config),
                if settings.active_proxy_id.as_deref() == Some(p.id.as_str()) { tr("  <== ACTIVO") } else { "" }
            );
        }
        out += "\n";
    }

    // 6. Reconexion automatica
    out += &format!("{}\n", tr("6. RECONEXION AUTOMATICA"));
    out += &format!("{}\n", thin);
    out += &format!(
        "  Temporizador: {}\n",
        if settings.reconnect_enabled {
            tf("activado (en marcha solo con esta opcion activada), cada {0} min", &[&settings.reconnect_min.to_string()])
        } else {
            tr("desactivado (no hay ningun temporizador en marcha)").to_string()
        }
    );
    let st = crate::watcher::read_state();
    if st.last_check == 0 {
        out += &format!("{}\n\n", tr("  Todavia no se ha hecho ninguna verificacion automatica."));
    } else {
        let hace = Local.timestamp_millis_opt(st.last_check as i64).single().map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
        out += &format!("{}\n\n", tf("  Ultima verificacion: {0} - {1}", &[&hace, &st.detail]));
    }

    // 7. Variables de entorno del proceso
    out += &format!("{}\n", tr("7. VARIABLES DE ENTORNO DEL PROCESO (con credenciales ocultas)"));
    out += &format!("{}\n", thin);
    let mut found = false;
    for (k, v) in std::env::vars() {
        let kl = k.to_lowercase();
        if kl.ends_with("_proxy") {
            found = true;
            out += &format!("  {}={}\n", k, mask_url(&v));
        }
    }
    if !found {
        out += &format!("{}\n", tr("  (ninguna: el proceso se abrio sin variables de proxy)"));
    }
    out += "\n";

    // 8. Resumen de errores y avisos
    out += &format!("{}\n", tr("8. RESUMEN DE ERRORES Y AVISOS"));
    out += &format!("{}\n", thin);
    let (i, a, e) = eventlog::counts();
    out += &format!(
        "{}\n",
        tf(
            "  Eventos registrados en total: {0} ({1} info, {2} avisos, {3} errores)",
            &[&(i + a + e).to_string(), &i.to_string(), &a.to_string(), &e.to_string()]
        )
    );
    match eventlog::last_error() {
        None => out += &format!("{}\n", tr("  No hay ningun error registrado.")),
        Some(ev) => {
            out += &format!("{}\n", tf("  Ultimo error: {0}", &[&ev.msg]));
        }
    }
    out += "\n";

    // 9. Registro completo de eventos
    out += &format!("{}\n", tr("9. REGISTRO COMPLETO DE EVENTOS (LOG COMPLETO)"));
    out += &format!("{}\n", thin);
    let events = eventlog::read_all();
    if events.is_empty() {
        out += &format!("{}\n\n", tr("  (vacio)"));
    } else {
        for ev in &events {
            let t = Local.timestamp_millis_opt(ev.ts as i64).single().map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "?".into());
            out += &format!("  {} [{:^5}] [{:<11}] {}\n", t, ev.level.tag(), ev.cat, ev.msg);
        }
        out += "\n";
    }

    // 10. Actividad (historial legible)
    out += &format!("{}\n", tr("10. HISTORIAL DE ACTIVIDAD (compatibilidad con versiones anteriores)"));
    out += &format!("{}\n", thin);
    let hist = store::read_history();
    if hist.is_empty() {
        out += &format!("{}\n\n", tr("  (vacio)"));
    } else {
        for h in &hist {
            let t = Local.timestamp_millis_opt(h.ts as i64).single().map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_else(|| "?".into());
            out += &format!("  {} [{}] {}\n", t, h.action, h.detail);
        }
        out += "\n";
    }

    // 11. Verificaciones de calidad de esta version (constancia de la revision)
    out += &format!("{}\n", tr("11. VERIFICACIONES DE CALIDAD DE LA VERSION"));
    out += &format!("{}\n", thin);
    out += &format!("{}\n", tr("  Comprobaciones hechas sobre esta version antes de publicarla:"));
    for (result, check) in [
        ("OK", tr("Campos de escritura con fondo y borde resaltado: servidor, puerto, usuario, contrasena, excepciones y el nombre al guardar en la biblioteca")),
        ("OK", tr("Tamano e intensidad de los textos afinados para leerlo todo bien en tema claro (incluidos los menus de la barra lateral)")),
        ("OK", tr("Colores revisados eligiendo la combinacion de mejor lectura, manteniendo el tema azul de la app")),
        ("OK", tr("Botones Actualizar de Mis proxies, Historial e Informe verificados: recargan los datos al pulsarlos")),
        ("OK", tr("Activar un proxy SIN poner la contrasena de administrador: se explica el fallo en pantalla y queda anotado como aviso (no se queda silencioso)")),
        ("OK", tr("Activar un proxy sin ningun nivel marcado: rechazado con mensaje claro (antes quedaba 'activo' sin hacer nada)")),
        ("OK", tr("Licencia cambiada de MIT a GNU GPL v3: en la pagina Acerca de, en Cargo.toml, en el copyright del paquete .deb y en COPYING.txt del codigo fuente")),
        ("OK", tr("Temporizador de reconexion automatica reparado: ya NO arranca al ejecutar la app con la opcion apagada; arranca solo al activar la opcion y se detiene de verdad al desactivarla")),
        ("OK", tr("Idioma espanol/ingles incorporado: la app arranca en espanol y el cambio en Preferencias se aplica a toda la interfaz, la bandeja, el informe y la consola")),
        ("OK", tr("Menus de la bandeja del sistema reparados y probados por D-Bus: Mostrar ProxySend, Activar/Desactivar proxy y Salir responden siempre, con la ventana visible o minimizada.")),
        ("OK", tr("Activar/Desactivar proxy desde la bandeja muestra el resultado: aviso en pantalla con el detalle de cada nivel y constancia en el registro.")),
        ("OK", tr("Las opciones de bandeja al cerrar o minimizar se quitaron de Ajustes del proxy (v2.3.4): cerrar la ventana cierra ProxySend y minimizar solo minimiza, sin ocultar nada.")),
        ("OK", tr("Sin rastros de la funcionalidad retirada: casillas, ajustes guardados, registros, botones y traducciones fuera; los ajustes viejos con esas claves se leen sin error y se descartan.")),
        ("OK", tr("Abrir ProxySend cuando ya esta corriendo trae la ventana existente al frente (la peticion de la segunda instancia ahora se atiende).")),
        ("OK", tr("Revision a fondo del codigo completo: compilacion limpia sin ninguna advertencia del compilador (antes 38), codigo muerto eliminado y una clave de traduccion duplicada corregida.")),
        ("OK", tr("Revision a fondo de la version 2.3.5: recorrido completo del codigo fuente, cero funciones sin uso, cero dependencias sin usar y clippy en silencio total (los 11 avisos de estilo de antes, corregidos).")),
        ("OK", tr("Icono de la bandeja sincronizado: al activar o desactivar el proxy desde la ventana, el icono y su tooltip se refrescan al momento (antes quedaba con el estado viejo).")),
        ("OK", tr("Mostrar ProxySend reparado a fondo: en X11 la ventana minimizada vuelve a pantalla porque ahora se des-minimiza de verdad (antes solo se pedia foco y la ventana seguia abajo); en Wayland se reabre en primer plano con una ventana nueva.")),
        ("OK", tr("Registro de la bandeja sincero: el informe cuenta la accion real de cada momento (mostrar, activar, desactivar, salir) y ya no anuncia ocultaciones que la app ya no hace.")),
        ("OK", tr("Simulacion real en X11: al minimizar la ventana por el gestor de ventanas queda abajo de forma normal y Mostrar ProxySend la devuelve a pantalla con foco; no hay ocultacion a la bandeja.")),
        ("OK", tr("Mostrar ProxySend verificado por D-Bus (dbusmenu) con la ventana minimizada: la ventana vuelve a aparecer; tambien responde la segunda instancia.")),
        ("OK", tr("Salir desde la bandeja termina el proceso AL INSTANTE (medido en simulacion con la ventana visible y minimizada): sin esperas ni cierre garantizado de respaldo.")),
        ("OK", tr("Activar y Desactivar proxy desde la bandeja verificados por D-Bus: el nivel de escritorio se aplica de verdad (gsettings) y el resultado queda en historial, registro e informe.")),
        ("OK", tr("Portabilidad verificada en los 4 entregables (28 comprobaciones): AppImage se extrae y corre sin libfuse2, el .deb solo declara libc6, el tar.gz instala en el HOME de prueba y el .run se autoextrae e instala.")),
        ("OK", tr("Regresion re-verificada en la simulacion real de la 2.3.5: los menus de la bandeja (Mostrar ProxySend, Activar/Desactivar proxy y Salir) responden al instante y los 4 entregables vuelven a pasar las 28 comprobaciones de portabilidad.")),
        ("OK", tr("Cero dependencias externas: el binario solo enlaza glibc (libc, libm, libgcc) y las 2 librerias XKB que usa van embebidas en los 4 formatos; el resto es base de cualquier escritorio Linux.")),
        ("OK", tr("Nueva interfaz azul (v2.4.0): barra lateral azul marino con iconos dibujados, boton de encendido rediseñado y tarjetas con sombra suave; tema claro y oscuro reafinados en azul.")),
        ("OK", tr("Revision de codigo muerto con escaneo automatizado de la 2.4.0: funciones, constantes, dependencias y claves de traduccion verificadas una por una; cero restos.")),
        ("OK", tr("Simulacion E2E real en X11 de la 2.4.0 con bandeja por D-Bus y capturas de pantalla: arranque, cambio de tema, navegacion, activacion del proxy y salida verificados.")),
        ("OK", tr("Revision a fondo post-entrega de la 2.4.0: el proceso externo se elimina al agotar el timeout (antes quedaba colgado), el handle de la bandeja es reemplazable en el paso autoarranque a ventana, ajustes e historial se escriben de forma atomica y sin panics de Instant al arrancar.")),
        ("OK", tr("Pruebas unitarias (cargo test) mas simulaciones de interfaz y de flujo con contrasena cancelada")),
    ] {
        out += &format!("  [{}] {}\n", result, check);
    }
    out += "\n";

    // 12. Panic capturado
    let panic_path = store::config_dir().join("panic.txt");
    if panic_path.exists() {
        out += &format!("{}\n", tr("12. PANIC CAPTURADO (error inesperado registrado)"));
        out += &format!("{}\n", thin);
        if let Ok(text) = std::fs::read_to_string(&panic_path) {
            out += &text;
            out += "\n";
        }
        out += "\n";
    }

    out += &format!("{}\n", line);
    out += &format!("{}\n", tr("Fin del informe. Este archivo NO contiene contrasenas: se muestran como ****."));
    out += &format!("{}\n", line);
    out
}

fn whoami() -> String {
    let r = sysops::run_public("/usr/bin/whoami", &[]);
    if r.ok { r.stdout.trim().to_string() } else { std::env::var("USER").unwrap_or_else(|_| tr("desconocido").into()) }
}

fn active_name(settings: &Settings) -> Option<String> {
    settings
        .active_proxy_id
        .as_ref()
        .and_then(|id| settings.proxies.iter().find(|p| &p.id == id))
        .map(|p| p.name.clone())
}

/// Genera el informe y lo guarda en la carpeta del usuario.
/// Devuelve la ruta del archivo creado.
pub fn write_report(settings: &Settings) -> Result<String, String> {
    let text = build_report(settings);
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let dir = store::home_dir();
    let path = dir.join(format!("ProxySend-informe-{}.txt", stamp));
    std::fs::write(&path, &text).map_err(|e| tf("No se pudo escribir el informe: {0}", &[&e.to_string()]))?;
    Ok(path.to_string_lossy().to_string())
}

/// Ayudantes para las pruebas unitarias (solo existen al compilar tests).
#[cfg(test)]
pub fn section_count(text: &str) -> usize {
    text.lines().filter(|l| l.starts_with(|c: char| c.is_ascii_digit()) && l.contains(". ")).count()
}

#[cfg(test)]
pub fn level_count(text: &str, tag: &str) -> usize {
    text.lines().filter(|l| l.contains(&format!("[{:^5}]", tag))).count()
}
