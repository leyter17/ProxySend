//! ProxySend - operaciones reales sobre el sistema (equivalente a system.js).
//!   1. Ajustes de proxy del escritorio (GNOME/COSMIC via gsettings, KDE via kioslaverc)
//!   2. Variables de entorno del sistema (/etc/environment, con pkexec)
//!   3. Variables de entorno de las terminales (~/.bashrc y ~/.zshrc si existe)
//!   4. Autoarranque (~/.config/autostart/proxysend.desktop)

use crate::core;
use crate::store::{home_dir, ProxyConfig};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const ETC_ENV: &str = "/etc/environment";
pub const BACKUP: &str = "/etc/environment.proxysend.bak";

#[derive(Debug, Clone, Default)]
pub struct OpResult {
    pub ok: bool,
    pub detail: String,
}

impl OpResult {
    pub fn ok(detail: impl Into<String>) -> Self {
        Self { ok: true, detail: detail.into() }
    }
    pub fn fail(detail: impl Into<String>) -> Self {
        Self { ok: false, detail: detail.into() }
    }
}

pub struct RunOut {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

fn run(cmd: &str, args: &[&str]) -> RunOut {
    run_with_env(cmd, args, false, 30_000)
}

/// visible para pruebas: run_with_env con timeout corto (verifica el kill)
pub(crate) fn run_with_env(cmd: &str, args: &[&str], clean_proxy_env: bool, timeout_ms: u64) -> RunOut {
    let mut command = Command::new(cmd);
    command.args(args);
    if clean_proxy_env {
        // prueba de conexion: no heredar el proxy del entorno
        for k in [
            "http_proxy", "https_proxy", "all_proxy", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY",
            "ftp_proxy", "FTP_PROXY", "no_proxy", "NO_PROXY",
        ] {
            command.env_remove(k);
        }
    }
    let mut child = match command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => return RunOut { ok: false, stdout: String::new(), stderr: e.to_string() },
    };
    // la salida se lee en hilos aparte para no bloquear por el tamano de los
    // pipes (un hijo que escriba mucho no debe colgar la lectura)
    let so = child.stdout.take();
    let se = child.stderr.take();
    let t_out = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut s) = so {
            use std::io::Read;
            let _ = s.read_to_string(&mut buf);
        }
        buf
    });
    let t_err = std::thread::spawn(move || {
        let mut buf = String::new();
        if let Some(mut s) = se {
            use std::io::Read;
            let _ = s.read_to_string(&mut buf);
        }
        buf
    });
    // espera activa con deadline: al agotar el timeout se MATA al hijo
    // (antes solo se abandonaba: el proceso y el hilo quedaban colgados)
    let started = std::time::Instant::now();
    let deadline = std::time::Duration::from_millis(timeout_ms);
    let mut timed_out = false;
    let mut status: Option<std::process::ExitStatus> = None;
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                status = Some(st);
                break;
            }
            Ok(None) => {
                if started.elapsed() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = t_out.join();
                let _ = t_err.join();
                return RunOut { ok: false, stdout: String::new(), stderr: e.to_string() };
            }
        }
    }
    let stdout = t_out.join().unwrap_or_default();
    let stderr = t_err.join().unwrap_or_default();
    if timed_out {
        return RunOut { ok: false, stdout, stderr: "timeout".to_string() };
    }
    RunOut {
        ok: status.map(|s| s.success()).unwrap_or(false),
        stdout,
        stderr,
    }
}

fn err_text(r: &RunOut) -> String {
    let s = r.stderr.trim();
    if !s.is_empty() {
        return s.lines().last().unwrap_or(s).to_string();
    }
    s.to_string()
}

/// Traduce los errores crudos de pkexec/PolicyKit a un mensaje claro en
/// espanol. El mas comun: el usuario cerro o cancelo la ventana de la
/// contrasena ("Request dismissed"), que antes salia en el informe como un
/// error tecnico en ingles.
fn friendly_pkexec_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("dismissed")
        || lower.contains("cancel")
        || lower.contains("no authentication agent")
        || lower.contains("interaction not allowed")
    {
        crate::i18n::tr("Se cancelo la ventana de contraseña de administrador: no se pudo escribir /etc/environment. Escritorio y terminales se aplican igual; si tambien quieres el nivel de sistema, vuelve a intentarlo y escribe tu contraseña cuando salga la ventana.").to_string()
    } else {
        raw.to_string()
    }
}

/// true si el detalle corresponde a una cancelacion del usuario (para guardarla
/// como aviso en el registro en vez de como error). Vale para los dos idiomas
/// (el mensaje de cancelacion se genera en el idioma activo).
pub fn is_cancelled_detail(detail: &str) -> bool {
    detail.starts_with("Se cancelo")
        || detail.starts_with("The administrator password window was cancelled")
}

pub fn tool_exists(cmd: &str) -> bool {
    let r = run("/bin/sh", &["-c", &format!("command -v {}", cmd)]);
    r.ok && !r.stdout.trim().is_empty()
}

/// Ejecuta un comando externo y devuelve la salida (para el informe y otras
/// consultas desde fuera del modulo).
pub fn run_public(cmd: &str, args: &[&str]) -> RunOut {
    run(cmd, args)
}

/* ------------------------------------------------------------------ */
/* 1. Escritorio GNOME/COSMIC (gsettings)                              */
/* ------------------------------------------------------------------ */

fn unquote(v: &str) -> String {
    let s = v.trim();
    if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn gnome_supported() -> bool {
    let r = run("gsettings", &["get", "org.gnome.system.proxy", "mode"]);
    r.ok
}

fn gnome_get(schema: &str, key: &str) -> Option<String> {
    let r = run("gsettings", &["get", schema, key]);
    if r.ok {
        Some(unquote(&r.stdout))
    } else {
        None
    }
}

fn gnome_set(schema: &str, key: &str, value: impl std::fmt::Display) -> RunOut {
    run("gsettings", &["set", schema, key, &value.to_string()])
}

fn sval(v: &str) -> String {
    format!("'{}'", v.replace('\'', ""))
}

pub fn gnome_apply(config: &ProxyConfig) -> OpResult {
    if !gnome_supported() {
        return OpResult::fail(
            crate::i18n::tr("Este sistema no expone org.gnome.system.proxy (gsettings); el escritorio no se puede configurar desde aqui.").to_string(),
        );
    }
    let s = "org.gnome.system.proxy";
    if config.protocol == "socks5" {
        gnome_set(&format!("{}.http", s), "host", sval(""));
        gnome_set(&format!("{}.http", s), "port", "0");
        gnome_set(&format!("{}.https", s), "host", sval(""));
        gnome_set(&format!("{}.https", s), "port", "0");
        gnome_set(&format!("{}.socks", s), "host", sval(&config.host));
        gnome_set(&format!("{}.socks", s), "port", config.port);
    } else if config.same_for_all {
        gnome_set(&format!("{}.http", s), "host", sval(&config.host));
        gnome_set(&format!("{}.http", s), "port", config.port);
        gnome_set(&format!("{}.https", s), "host", sval(&config.host));
        gnome_set(&format!("{}.https", s), "port", config.port);
        gnome_set(&format!("{}.socks", s), "host", sval(""));
        gnome_set(&format!("{}.socks", s), "port", "0");
    } else {
        gnome_set(&format!("{}.http", s), "host", sval(&config.http.host));
        gnome_set(&format!("{}.http", s), "port", config.http.port);
        gnome_set(&format!("{}.https", s), "host", sval(&config.https.host));
        gnome_set(&format!("{}.https", s), "port", config.https.port);
        gnome_set(&format!("{}.socks", s), "host", sval(""));
        gnome_set(&format!("{}.socks", s), "port", "0");
    }
    if config.protocol != "socks5" && config.auth {
        gnome_set(&format!("{}.http", s), "use-authentication", "true");
        gnome_set(&format!("{}.http", s), "authentication-user", sval(&config.username));
        gnome_set(&format!("{}.http", s), "authentication-password", sval(&config.password));
    } else {
        gnome_set(&format!("{}.http", s), "use-authentication", "false");
        gnome_set(&format!("{}.http", s), "authentication-user", sval(""));
        gnome_set(&format!("{}.http", s), "authentication-password", sval(""));
    }
    let ignore: Vec<String> = if config.ignore_hosts.is_empty() {
        core::DEFAULT_IGNORE_HOSTS.iter().map(|s| s.to_string()).collect()
    } else {
        config.ignore_hosts.clone()
    };
    let list = ignore.iter().map(|h| format!("'{}'", h)).collect::<Vec<_>>().join(", ");
    gnome_set(s, "ignore-hosts", format!("[{}]", list));
    let r = gnome_set(s, "mode", sval("manual"));
    if !r.ok {
        return OpResult::fail(crate::i18n::tf("No se pudo activar el proxy del escritorio: {0}", &[&err_text(&r)]));
    }
    OpResult::ok(crate::i18n::tr("Proxy del escritorio (GNOME) configurado en modo manual."))
}

pub fn gnome_disable() -> OpResult {
    if !gnome_supported() {
        return OpResult::fail(crate::i18n::tr("gsettings no disponible."));
    }
    let r = run("gsettings", &["set", "org.gnome.system.proxy", "mode", "'none'"]);
    if r.ok {
        OpResult::ok(crate::i18n::tr("Proxy del escritorio desactivado (modo none)."))
    } else {
        OpResult::fail(crate::i18n::tf("No se pudo desactivar el proxy del escritorio: {0}", &[&err_text(&r)]))
    }
}

pub fn gnome_reset() -> OpResult {
    if !gnome_supported() {
        return OpResult::fail(crate::i18n::tr("gsettings no disponible."));
    }
    for schema in [
        "org.gnome.system.proxy.http",
        "org.gnome.system.proxy.https",
        "org.gnome.system.proxy.socks",
    ] {
        for key in [
            "host", "port", "use-authentication", "authentication-user", "authentication-password",
        ] {
            // los fallos se ignoran: algunas claves no existen en cada esquema
            run("gsettings", &["reset", schema, key]);
        }
    }
    run("gsettings", &["reset", "org.gnome.system.proxy", "ignore-hosts"]);
    let r = run("gsettings", &["set", "org.gnome.system.proxy", "mode", "'none'"]);
    if r.ok {
        OpResult::ok(crate::i18n::tr("Ajustes de proxy del escritorio restablecidos a los valores por defecto."))
    } else {
        OpResult::fail(crate::i18n::tr("No se pudo restablecer el modo del proxy."))
    }
}

/* ------------------------------------------------------------------ */
/* 1b. Escritorio KDE (archivo kioslaverc de KIO)                      */
/* ------------------------------------------------------------------ */

pub fn kde_rc_path() -> PathBuf {
    home_dir().join(".config").join("kioslaverc")
}

fn read_kde_rc() -> String {
    fs::read_to_string(kde_rc_path()).unwrap_or_default()
}

/// Devuelve las claves del grupo [Proxy Settings]
pub fn parse_kde_group(text: &str) -> std::collections::BTreeMap<String, String> {
    let mut map = std::collections::BTreeMap::new();
    let mut in_group = false;
    for raw in text.split('\n') {
        let line = raw.trim();
        if line.starts_with('[') {
            in_group = line == "[Proxy Settings]";
            continue;
        }
        if !in_group {
            continue;
        }
        if let Some(i) = line.find('=') {
            if i > 0 {
                map.insert(line[..i].to_string(), line[i + 1..].to_string());
            }
        }
    }
    map
}

/// Sustituye (o crea) el grupo [Proxy Settings] conservando el resto del archivo
fn write_kde_group(entries: &[(String, String)]) -> OpResult {
    let original = read_kde_rc();
    let mut out: Vec<String> = Vec::new();
    let mut in_group = false;
    let mut injected = false;
    for line in original.split('\n') {
        let is_header = line.trim_start().starts_with('[');
        if is_header {
            if in_group && !injected {
                out.push("[Proxy Settings]".to_string());
                for (k, v) in entries {
                    out.push(format!("{}={}", k, v));
                }
                injected = true;
            }
            in_group = line.trim() == "[Proxy Settings]";
            if in_group {
                continue;
            }
            out.push(line.to_string());
            continue;
        }
        if in_group {
            continue;
        }
        out.push(line.to_string());
    }
    if in_group && !injected {
        out.push("[Proxy Settings]".to_string());
        for (k, v) in entries {
            out.push(format!("{}={}", k, v));
        }
        injected = true;
    }
    if !injected {
        while out.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
            out.pop();
        }
        if !out.is_empty() {
            out.push(String::new());
        }
        out.push("[Proxy Settings]".to_string());
        for (k, v) in entries {
            out.push(format!("{}={}", k, v));
        }
    }
    match fs::write(kde_rc_path(), out.join("\n")) {
        Ok(_) => OpResult::ok(""),
        Err(e) => OpResult::fail(crate::i18n::tf("No se pudo escribir {0}: {1}", &[&kde_rc_path().display().to_string(), &e.to_string()])),
    }
}

fn remove_kde_group() -> OpResult {
    let original = read_kde_rc();
    if !original.contains("[Proxy Settings]") {
        return OpResult::ok("");
    }
    let mut out: Vec<String> = Vec::new();
    let mut in_group = false;
    for line in original.split('\n') {
        if line.trim_start().starts_with('[') {
            in_group = line.trim() == "[Proxy Settings]";
            if in_group {
                continue;
            }
            out.push(line.to_string());
            continue;
        }
        if in_group {
            continue;
        }
        out.push(line.to_string());
    }
    match fs::write(kde_rc_path(), out.join("\n")) {
        Ok(_) => OpResult::ok(""),
        Err(e) => OpResult::fail(crate::i18n::tf("No se pudo escribir {0}: {1}", &[&kde_rc_path().display().to_string(), &e.to_string()])),
    }
}

fn kde_fmt_proxy(v: &str) -> String {
    let mut parts = v.split_whitespace();
    let host = parts.next().unwrap_or("");
    if host.is_empty() {
        return String::new();
    }
    match parts.next() {
        Some(port) => format!("{}:{}", host, port),
        None => host.to_string(),
    }
}

pub fn kde_status() -> (bool, Option<String>, String) {
    let map = parse_kde_group(&read_kde_rc());
    let active = map.get("ProxyType").map(|v| v == "1").unwrap_or(false)
        && map.get("httpProxy").map(|v| !v.trim().is_empty()).unwrap_or(false);
    if active {
        let mut v = format!("HTTP {}", kde_fmt_proxy(map.get("httpProxy").map(|s| s.as_str()).unwrap_or("")));
        if let Some(https) = map.get("httpsProxy").filter(|s| !s.trim().is_empty()) {
            v += &format!(" · HTTPS {}", kde_fmt_proxy(https));
        }
        let valor = Some(v.clone());
        return (true, valor, format!("{} (kioslaverc)", v));
    }
    (
        false,
        None,
        crate::i18n::tf(
            "Proxy del escritorio KDE sin activar (ProxyType {0}).",
            &[&map.get("ProxyType").cloned().unwrap_or_else(|| "0".to_string())],
        ),
    )
}

pub fn kde_apply(config: &ProxyConfig) -> OpResult {
    if config.protocol == "socks5" {
        return OpResult::fail(
            crate::i18n::tr("KDE no guarda proxys SOCKS5 en kioslaverc. Usa los niveles Sistema y Terminal para SOCKS5.").to_string(),
        );
    }
    let ignore: Vec<String> = if config.ignore_hosts.is_empty() {
        core::DEFAULT_IGNORE_HOSTS.iter().map(|s| s.to_string()).collect()
    } else {
        config.ignore_hosts.clone()
    };
    let (http_val, https_val) = if config.same_for_all {
        (
            format!("{} {}", config.host, config.port),
            format!("{} {}", config.host, config.port),
        )
    } else {
        (
            format!("{} {}", config.http.host, config.http.port),
            format!("{} {}", config.https.host, config.https.port),
        )
    };
    let noproxy = ignore.join(",");
    let entries: Vec<(String, String)> = vec![
        ("ProxyType".to_string(), "1".to_string()),
        ("httpProxy".to_string(), http_val),
        ("httpsProxy".to_string(), https_val),
        ("NoProxyFor".to_string(), noproxy),
        ("ReversedException".to_string(), "false".to_string()),
    ];
    let r = write_kde_group(&entries);
    if !r.ok {
        return r;
    }
    let mut detail =
        crate::i18n::tr("Proxy del escritorio (KDE) guardado en kioslaverc. Las apps de KDE lo leen al abrirse.").to_string();
    if config.auth {
        detail += crate::i18n::tr(" KDE pedira el usuario y contraseña del proxy cuando los necesite.");
    }
    OpResult::ok(detail)
}

pub fn kde_disable() -> OpResult {
    let entries: Vec<(String, String)> = vec![
        ("ProxyType".to_string(), "0".to_string()),
        ("ReversedException".to_string(), "false".to_string()),
    ];
    let r = write_kde_group(&entries);
    if !r.ok {
        return r;
    }
    OpResult::ok(crate::i18n::tr("Proxy del escritorio (KDE) desactivado."))
}

pub fn kde_reset() -> OpResult {
    let r = remove_kde_group();
    if !r.ok {
        return r;
    }
    OpResult::ok(crate::i18n::tr("Ajustes de proxy del escritorio KDE eliminados de kioslaverc."))
}

/* ------------------------------------------------------------------ */
/* 1c. Nivel escritorio unificado (GNOME / COSMIC / KDE / otros)       */
/* ------------------------------------------------------------------ */

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectedDesktop {
    Gnome,
    Cosmic,
    Kde,
    Xfce,
    Other,
}

pub fn detect_desktop_env() -> DetectedDesktop {
    let xdg = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase();
    if xdg.contains("kde") {
        DetectedDesktop::Kde
    } else if xdg.contains("cosmic") {
        DetectedDesktop::Cosmic
    } else if xdg.contains("xfce") || xdg.contains("lxde") || xdg.contains("lxqt") {
        DetectedDesktop::Xfce
    } else if xdg.contains("gnome")
        || xdg.contains("unity")
        || xdg.contains("cinnamon")
        || xdg.contains("budgie")
        || xdg.contains("mate")
        || xdg.contains("pantheon")
    {
        DetectedDesktop::Gnome
    } else {
        DetectedDesktop::Other
    }
}

/// Elige el backend del escritorio: Gnome (gsettings) o Kde (kioslaverc)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Backend {
    Gnome,
    Kde,
}

pub fn desktop_backend() -> Option<Backend> {
    let de = detect_desktop_env();
    match de {
        DetectedDesktop::Kde => Some(Backend::Kde),
        DetectedDesktop::Cosmic => {
            if gnome_supported() {
                Some(Backend::Gnome)
            } else {
                None
            }
        }
        DetectedDesktop::Xfce => {
            if gnome_supported() {
                Some(Backend::Gnome)
            } else {
                None
            }
        }
        DetectedDesktop::Gnome => {
            if gnome_supported() {
                Some(Backend::Gnome)
            } else {
                None
            }
        }
        DetectedDesktop::Other => {
            if gnome_supported() {
                Some(Backend::Gnome)
            } else if kde_rc_path().exists() {
                Some(Backend::Kde)
            } else {
                None
            }
        }
    }
}

pub fn desktop_name(detected: DetectedDesktop, backend: Option<Backend>) -> &'static str {
    match detected {
        DetectedDesktop::Cosmic => crate::i18n::tr("Escritorio (COSMIC)"),
        DetectedDesktop::Kde => crate::i18n::tr("Escritorio (KDE)"),
        DetectedDesktop::Gnome => crate::i18n::tr("Escritorio (GNOME)"),
        DetectedDesktop::Xfce => crate::i18n::tr("Escritorio (XFCE)"),
        DetectedDesktop::Other => match backend {
            Some(Backend::Kde) => crate::i18n::tr("Escritorio (KDE)"),
            Some(Backend::Gnome) => crate::i18n::tr("Escritorio (GNOME)"),
            None => crate::i18n::tr("Escritorio"),
        },
    }
}

/// Estado del nivel Escritorio: (supported, backend, detected, active, value, detail)
pub fn desktop_status() -> (bool, Option<Backend>, DetectedDesktop, bool, Option<String>, String) {
    let de = detect_desktop_env();
    let backend = desktop_backend();
    match backend {
        None => (
            false,
            None,
            de,
            false,
            None,
            crate::i18n::tr("Este escritorio no permite configurar el proxy desde la app (ni gsettings ni KDE kioslaverc). Usa los niveles Sistema y Terminal: funcionan igual de bien.").to_string(),
        ),
        Some(Backend::Kde) => {
            let (active, value, detail) = kde_status();
            (true, backend, de, active, value, detail)
        }
        Some(Backend::Gnome) => {
            let (g_supported, active, value, mut detail) = gnome_status();
            if de == DetectedDesktop::Xfce {
                detail += crate::i18n::tr(" Nota: tu escritorio es XFCE; Firefox y las apps GNOME respetan estos ajustes, el resto de programas usan los niveles Sistema y Terminal.");
            }
            if de == DetectedDesktop::Cosmic {
                detail += crate::i18n::tr(" Nota: COSMIC no trae ajuste de proxy propio, pero sus apps son GTK y usan estas mismas preferencias: se aplican al instante.");
            }
            (g_supported, backend, de, active, value, detail)
        }
    }
}

/// Estado de gsettings: (supported, active, value, detail)
pub fn gnome_status() -> (bool, bool, Option<String>, String) {
    if !gnome_supported() {
        return (
            false,
            false,
            None,
            crate::i18n::tr("gsettings o el esquema org.gnome.system.proxy no esta disponible en este sistema.").to_string(),
        );
    }
    let mode = gnome_get("org.gnome.system.proxy", "mode").unwrap_or_default();
    let active = mode == "manual";
    if !active {
        return (
            true,
            false,
            None,
            crate::i18n::tf("modo {0} (sin proxy)", &[if mode.is_empty() { crate::i18n::tr("desconocido") } else { &mode }]),
        );
    }
    let read = |schema: &str| -> (String, i64) {
        let host = gnome_get(schema, "host").unwrap_or_default();
        let port = gnome_get(schema, "port").and_then(|p| p.parse::<i64>().ok()).unwrap_or(0);
        (host, port)
    };
    let (http_h, http_p) = read("org.gnome.system.proxy.http");
    let (https_h, https_p) = read("org.gnome.system.proxy.https");
    let (socks_h, socks_p) = read("org.gnome.system.proxy.socks");
    let auth_user = gnome_get("org.gnome.system.proxy.http", "authentication-user").unwrap_or_default();
    let use_auth = gnome_get("org.gnome.system.proxy.http", "use-authentication").as_deref() == Some("true");
    let mut parts: Vec<String> = Vec::new();
    if !http_h.is_empty() && http_p > 0 {
        parts.push(format!("HTTP {}:{}", http_h, http_p));
    }
    if !https_h.is_empty() && https_p > 0 {
        parts.push(format!("HTTPS {}:{}", https_h, https_p));
    }
    if !socks_h.is_empty() && socks_p > 0 {
        parts.push(format!("SOCKS {}:{}", socks_h, socks_p));
    }
    let mut value = if parts.is_empty() { crate::i18n::tr("manual sin servidores definidos").to_string() } else { parts.join(" · ") };
    if use_auth && !auth_user.is_empty() {
        value += &crate::i18n::tf(" · usuario {0}", &[&auth_user]);
    }
    (true, true, Some(value.clone()), value)
}

pub fn desktop_apply(config: &ProxyConfig) -> OpResult {
    match desktop_backend() {
        None => OpResult::fail(
            crate::i18n::tr("Este escritorio no permite configurar el proxy desde la app. Usa los niveles Sistema y Terminal.").to_string(),
        ),
        Some(Backend::Kde) => kde_apply(config),
        Some(Backend::Gnome) => gnome_apply(config),
    }
}

pub fn desktop_disable() -> OpResult {
    match desktop_backend() {
        None => OpResult::fail(crate::i18n::tr("Este escritorio no permite configurar el proxy desde la app.").to_string()),
        Some(Backend::Kde) => kde_disable(),
        Some(Backend::Gnome) => gnome_disable(),
    }
}

pub fn desktop_reset() -> OpResult {
    match desktop_backend() {
        None => OpResult::fail(crate::i18n::tr("Este escritorio no permite configurar el proxy desde la app.").to_string()),
        Some(Backend::Kde) => kde_reset(),
        Some(Backend::Gnome) => gnome_reset(),
    }
}

/* ------------------------------------------------------------------ */
/* 2. Variables de entorno del sistema (/etc/environment, con pkexec)  */
/* ------------------------------------------------------------------ */

pub fn read_etc_environment() -> String {
    fs::read_to_string(ETC_ENV).unwrap_or_default()
}

pub fn env_status() -> (bool, bool, Option<String>) {
    let text = read_etc_environment();
    let mut value: Option<String> = None;
    for line in text.split('\n') {
        let t = line.trim_start();
        for key in ["http_proxy", "HTTP_PROXY"] {
            if let Some(rest) = t.strip_prefix(key) {
                let rest = rest.trim_start();
                if let Some(v) = rest.strip_prefix('=') {
                    let v = v.trim().trim_matches('"');
                    if !v.is_empty() {
                        value = Some(v.to_string());
                    }
                }
            }
        }
    }
    let active = value.as_deref().map(|v| !v.is_empty()).unwrap_or(false);
    (true, active, value)
}

fn write_etc_environment(content: &str) -> OpResult {
    if !tool_exists("pkexec") {
        return OpResult::fail(crate::i18n::tr("No se encontro pkexec (PolicyKit). Instalalo con: sudo apt install policykit-1").to_string());
    }
    let tmp = std::env::temp_dir().join(format!("proxysend-env-{}.new", std::process::id()));
    if let Err(e) = fs::write(&tmp, content) {
        return OpResult::fail(crate::i18n::tf("No se pudo preparar el archivo temporal: {0}", &[&e.to_string()]));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
    }
    if !PathBuf::from(BACKUP).exists() && PathBuf::from(ETC_ENV).exists() {
        let b = run_timeout("pkexec", &["/usr/bin/cp", ETC_ENV, BACKUP], 180_000);
        if !b.ok {
            let _ = fs::remove_file(&tmp);
            return OpResult::fail(friendly_pkexec_error(&err_text(&b)));
        }
    }
    let r = run_timeout("pkexec", &["/usr/bin/cp", tmp.to_str().unwrap_or("/tmp/proxysend-env.new"), ETC_ENV], 180_000);
    let _ = fs::remove_file(&tmp);
    if !r.ok {
        return OpResult::fail(friendly_pkexec_error(&err_text(&r)));
    }
    run_timeout("pkexec", &["/usr/bin/chmod", "644", ETC_ENV], 180_000);
    OpResult::ok("")
}

fn run_timeout(cmd: &str, args: &[&str], timeout_ms: u64) -> RunOut {
    run_with_env(cmd, args, false, timeout_ms)
}

pub fn env_apply(config: &ProxyConfig) -> OpResult {
    let values = core::env_values(config);
    let content = core::build_env_file_content(&read_etc_environment(), Some(&values));
    let r = write_etc_environment(&content);
    if !r.ok {
        return r;
    }
    OpResult::ok(crate::i18n::tr(
        "Variables de proxy escritas en /etc/environment (se aplican al volver a iniciar sesion).",
    ))
}

pub fn env_clean() -> OpResult {
    let original = read_etc_environment();
    let content = core::build_env_file_content(&original, None);
    if content == original {
        return OpResult::ok(crate::i18n::tr("/etc/environment ya esta sin proxy."));
    }
    let r = write_etc_environment(&content);
    if !r.ok {
        return r;
    }
    OpResult::ok(crate::i18n::tr("Variables de proxy eliminadas de /etc/environment."))
}

/* ------------------------------------------------------------------ */
/* 3. Terminales (~/.bashrc y ~/.zshrc si existe)                      */
/* ------------------------------------------------------------------ */

pub fn shell_files() -> Vec<PathBuf> {
    let mut files = vec![home_dir().join(".bashrc")];
    let zsh = home_dir().join(".zshrc");
    if zsh.exists() {
        files.push(zsh);
    }
    files
}

pub fn shell_status() -> (bool, bool, Option<String>) {
    let mut active = false;
    let mut value: Option<String> = None;
    for f in shell_files() {
        let text = match fs::read_to_string(&f) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if core::has_shell_block(&text) {
            active = true;
            for line in text.split('\n') {
                let t = line.trim_start();
                if let Some(rest) = t.strip_prefix("export") {
                    let rest = rest.trim_start();
                    if let Some(v) = rest.strip_prefix("http_proxy=") {
                        value = Some(v.trim_matches('"').to_string());
                    }
                }
            }
        }
    }
    (true, active, value)
}

pub fn shell_apply(config: &ProxyConfig) -> OpResult {
    let values = core::env_values(config);
    let block = core::build_shell_block(Some(&values));
    let files = shell_files();
    let mut names = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).unwrap_or_default();
        let _ = fs::write(f, core::apply_shell_block(&text, Some(&block)));
        names.push(format!("~/{}", f.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()));
    }
    OpResult::ok(crate::i18n::tf(
        "Variables exportadas en {0} (activas en las terminales nuevas).",
        &[&names.join(" y ")],
    ))
}

pub fn shell_clean() -> OpResult {
    let mut changed = false;
    for f in shell_files() {
        let text = match fs::read_to_string(&f) {
            Ok(t) => t,
            Err(_) => continue,
        };
        if !core::has_shell_block(&text) {
            continue;
        }
        let _ = fs::write(&f, core::apply_shell_block(&text, None));
        changed = true;
    }
    if changed {
        OpResult::ok(crate::i18n::tr("Bloque de ProxySend eliminado de las terminales."))
    } else {
        OpResult::ok(crate::i18n::tr("Las terminales ya estaban sin proxy."))
    }
}

/* ------------------------------------------------------------------ */
/* 4. Autoarranque (~/.config/autostart/proxysend.desktop)             */
/* ------------------------------------------------------------------ */

pub fn autostart_file_path() -> PathBuf {
    home_dir().join(".config").join("autostart").join("proxysend.desktop")
}

pub fn autostart_exec() -> String {
    // En AppImage, APPIMAGE apunta al archivo original; en .deb usamos el binario instalado
    let target = std::env::var("APPIMAGE")
        .ok()
        .filter(|s| PathBuf::from(s).exists())
        .or_else(|| std::env::current_exe().ok().map(|p| p.to_string_lossy().to_string()))
        .unwrap_or_else(|| "proxysend".to_string());
    format!("\"{}\" --autostart", target)
}

/// Ayudante para las pruebas unitarias (solo existe al compilar tests).
#[cfg(test)]
pub fn autostart_installed() -> bool {
    fs::read_to_string(autostart_file_path())
        .map(|t| t.contains("[Desktop Entry]"))
        .unwrap_or(false)
}

pub fn set_autostart(enabled: bool) -> OpResult {
    let file = autostart_file_path();
    if enabled {
        let content = [
            "[Desktop Entry]",
            "Type=Application",
            "Name=ProxySend",
            // el comentario del archivo .desktop va en el idioma activo
            &format!("Comment={}", crate::i18n::tr("Vuelve a aplicar el proxy guardado al iniciar sesion")),
            &format!("Exec={}", autostart_exec()),
            "Terminal=false",
            "X-GNOME-Autostart-enabled=true",
            "Categories=Network;",
            "",
        ]
        .join("\n");
        match fs::create_dir_all(file.parent().unwrap_or(&home_dir())).and_then(|_| fs::write(&file, content)) {
            Ok(_) => {
                crate::eventlog::info("autoarranque", crate::i18n::tf("Autoarranque activado: {0}", &[&file.display().to_string()]));
                OpResult::ok(crate::i18n::tr("ProxySend abrira al iniciar sesion y aplicara el proxy guardado."))
            }
            Err(e) => {
                crate::eventlog::error("autoarranque", crate::i18n::tf("No se pudo crear {0}: {1}", &[&file.display().to_string(), &e.to_string()]));
                OpResult::fail(crate::i18n::tf("No se pudo crear {0}: {1}", &[&file.display().to_string(), &e.to_string()]))
            }
        }
    } else {
        let _ = fs::remove_file(&file);
        crate::eventlog::info("autoarranque", crate::i18n::tr("Autoarranque desactivado"));
        OpResult::ok(crate::i18n::tr("Autoarranque desactivado."))
    }
}

/* ------------------------------------------------------------------ */
/* Prueba de conexion con curl (respaldo)                              */
/* ------------------------------------------------------------------ */

pub fn test_proxy_curl(config: Option<&ProxyConfig>, use_proxy: bool) -> Result<crate::nettest::TestResult, String> {
    let mut args: Vec<String> = vec![
        "-sS".into(),
        "--connect-timeout".into(),
        "8".into(),
        "--max-time".into(),
        "25".into(),
        "-w".into(),
        "\n__%{http_code}__%{time_total}__".into(),
        "https://api.ipify.org".into(),
    ];
    if use_proxy {
        if let Some(c) = config {
            let v = core::env_values(c);
            args.insert(0, v.http_url);
            args.insert(0, "-x".into());
        }
    }
    let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let r = run_with_env("curl", &args_ref, true, 40_000);
    if !r.ok {
        let e = err_text(&r);
        if e.is_empty() {
            return Err(crate::i18n::tr("No se pudo conectar (curl no disponible).").to_string());
        }
        return Err(crate::i18n::tf("No se pudo conectar: {0}", &[&e]));
    }
    let mut lines = r.stdout.lines();
    let ip = lines.next().unwrap_or("").trim().to_string();
    let tail = r.stdout.trim_end();
    // formato: __code__seconds__
    let parts: Vec<&str> = tail.rsplit("__").collect();
    if parts.len() < 3 {
        return Err(crate::i18n::tr("Respuesta inesperada de curl.").to_string());
    }
    let seconds: f64 = parts[1].parse().unwrap_or(0.0);
    let code = parts[2].to_string();
    Ok(crate::nettest::TestResult {
        ok: code == "200",
        ip,
        code,
        seconds,
        detail: String::new(),
    })
}
