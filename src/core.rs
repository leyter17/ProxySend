//! ProxySend - logica pura de configuracion de proxy (equivalente a proxy-core.js).

pub const DEFAULT_IGNORE_HOSTS: [&str; 3] = ["localhost", "127.0.0.0/8", "::1"];

/// Claves de proxy que la app gestiona en los archivos de entorno.
pub const PROXY_KEYS: [&str; 6] = [
    "http_proxy", "https_proxy", "ftp_proxy", "socks_proxy", "all_proxy", "no_proxy",
];

pub const BEGIN_MARKER: &str = "# >>> ProxySend >>>";
pub const END_MARKER: &str = "# <<< ProxySend <<<";

pub const PROTOCOLS: [&str; 3] = ["http", "https", "socks5"];

pub fn scheme_for_protocol(protocol: &str) -> &'static str {
    match protocol {
        "https" => "https://",
        "socks5" => "socks5://",
        _ => "http://",
    }
}

/// Equivalente a encodeURIComponent de JavaScript (para usuario y contraseña en URLs).
pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(b, b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')');
        if keep {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

pub fn encoded_user(username: &str, password: &str) -> String {
    if username.is_empty() {
        return String::new();
    }
    let u = encode_uri_component(username);
    if password.is_empty() {
        format!("{}@", u)
    } else {
        format!("{}:{}@", u, encode_uri_component(password))
    }
}

/// Caracteres peligrosos en hosts, puertos y excepciones.
pub fn has_forbidden_chars(s: &str) -> bool {
    s.chars()
        .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '$' | '\\'))
}

fn port_ok(p: i64) -> bool {
    (1..=65535).contains(&p)
}

/// Errores de validacion (mensajes segun el idioma activo; espanol por defecto).
pub fn validate_config_errors(c: &crate::store::ProxyConfig) -> Vec<String> {
    use crate::i18n::tr;
    let mut errors = Vec::new();
    if !PROTOCOLS.contains(&c.protocol.as_str()) {
        errors.push(tr("El protocolo debe ser http, https o socks5.").to_string());
    }
    let host_ok = |h: &str| !h.trim().is_empty() && !has_forbidden_chars(h);
    if c.protocol == "socks5" || c.same_for_all {
        if !host_ok(&c.host) {
            errors.push(tr("El host no es valido (sin espacios, comillas ni simbolos raros).").to_string());
        }
        if !port_ok(c.port) {
            errors.push(tr("El puerto debe ser un numero entre 1 y 65535.").to_string());
        }
    } else {
        if !host_ok(&c.http.host) {
            errors.push(tr("El host HTTP no es valido.").to_string());
        }
        if !port_ok(c.http.port) {
            errors.push(tr("El puerto HTTP debe estar entre 1 y 65535.").to_string());
        }
        if !host_ok(&c.https.host) {
            errors.push(tr("El host HTTPS no es valido.").to_string());
        }
        if !port_ok(c.https.port) {
            errors.push(tr("El puerto HTTPS debe estar entre 1 y 65535.").to_string());
        }
    }
    if c.auth && c.username.trim().is_empty() {
        errors.push(tr("Falta el usuario del proxy.").to_string());
    }
    for h in &c.ignore_hosts {
        if has_forbidden_chars(h) {
            errors.push(tr("Hay una excepcion (host excluido) con caracteres no validos.").to_string());
            break;
        }
    }
    errors
}

/// Valores que se escriben en las variables de entorno.
pub struct EnvValues {
    pub http_url: String,
    pub https_url: String,
    pub no_proxy: String,
    pub is_socks: bool,
}

pub fn env_values(c: &crate::store::ProxyConfig) -> EnvValues {
    let ignore: Vec<String> = if c.ignore_hosts.is_empty() {
        DEFAULT_IGNORE_HOSTS.iter().map(|s| s.to_string()).collect()
    } else {
        c.ignore_hosts.clone()
    };
    let no_proxy = ignore.join(", ");
    let scheme = scheme_for_protocol(&c.protocol);
    let auth = if c.auth { encoded_user(&c.username, &c.password) } else { String::new() };
    if c.protocol == "socks5" || c.same_for_all {
        let u = format!("{}{}{}:{}", scheme, auth, c.host, c.port);
        EnvValues {
            is_socks: c.protocol == "socks5",
            http_url: u.clone(),
            https_url: u,
            no_proxy,
        }
    } else {
        EnvValues {
            is_socks: false,
            http_url: format!("{}{}{}:{}", scheme, auth, c.http.host, c.http.port),
            https_url: format!("{}{}{}:{}", scheme, auth, c.https.host, c.https.port),
            no_proxy,
        }
    }
}

/// Resumen legible, tipo "HTTP 1.2.3.4:8080".
pub fn summary(c: &crate::store::ProxyConfig) -> String {
    let proto = if c.protocol == "socks5" { "SOCKS5".to_string() } else { c.protocol.to_uppercase() };
    let with_auth = if c.auth && !c.username.is_empty() { format!("{}@", c.username) } else { String::new() };
    if c.protocol == "socks5" || c.same_for_all {
        format!("{} {}{}:{}", proto, with_auth, c.host, c.port)
    } else {
        format!(
            "{} HTTP {}:{} / HTTPS {}:{}",
            proto, c.http.host, c.http.port, c.https.host, c.https.port
        )
    }
}

/// Contenido nuevo de /etc/environment conservando las lineas ajenas al proxy.
pub fn build_env_file_content(original: &str, values: Option<&EnvValues>) -> String {
    let mut kept: Vec<String> = Vec::new();
    for line in original.split('\n') {
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_lowercase();
            if PROXY_KEYS.contains(&key.as_str()) {
                continue;
            }
        }
        kept.push(line.to_string());
    }
    while kept.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        kept.pop();
    }
    if let Some(v) = values {
        kept.push(format!("http_proxy=\"{}\"", v.http_url));
        kept.push(format!("https_proxy=\"{}\"", v.https_url));
        kept.push(format!("HTTP_PROXY=\"{}\"", v.http_url));
        kept.push(format!("HTTPS_PROXY=\"{}\"", v.https_url));
        kept.push(format!("no_proxy=\"{}\"", v.no_proxy));
        kept.push(format!("NO_PROXY=\"{}\"", v.no_proxy));
        if v.is_socks {
            kept.push(format!("all_proxy=\"{}\"", v.http_url));
        }
    }
    if kept.is_empty() {
        String::new()
    } else {
        kept.join("\n") + "\n"
    }
}

/// Bloque export ... para ~/.bashrc (y ~/.zshrc si existe).
pub fn build_shell_block(values: Option<&EnvValues>) -> String {
    let v = match values {
        Some(v) => v,
        None => return String::new(),
    };
    let mut lines = vec![
        BEGIN_MARKER.to_string(),
        format!("export http_proxy=\"{}\"", v.http_url),
        format!("export https_proxy=\"{}\"", v.https_url),
    ];
    if v.is_socks {
        lines.push(format!("export all_proxy=\"{}\"", v.http_url));
    }
    lines.push(format!("export no_proxy=\"{}\"", v.no_proxy));
    lines.push(END_MARKER.to_string());
    lines.join("\n") + "\n"
}

/// Quita el bloque gestionado por ProxySend y, si se pasa uno nuevo, lo agrega al final.
pub fn apply_shell_block(content: &str, block: Option<&str>) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for line in content.split('\n') {
        if line.trim() == BEGIN_MARKER {
            skipping = true;
            continue;
        }
        if line.trim() == END_MARKER {
            skipping = false;
            continue;
        }
        if !skipping {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    let mut text = out;
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    if block.is_none() {
        let t = text.trim_end();
        return if t.is_empty() { String::new() } else { t.to_string() + "\n" };
    }
    let t = text.trim_end();
    if t.is_empty() {
        block.unwrap().to_string()
    } else {
        format!("{}\n{}", t, block.unwrap())
    }
}

pub fn has_shell_block(content: &str) -> bool {
    content.contains(BEGIN_MARKER)
}

/// "localhost\n127.0.0.0/8\n::1" o "localhost, 127.0.0.0/8" -> lista limpia y sin duplicados.
pub fn parse_ignore_hosts(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for part in text.split(['\n', ',']) {
        let s = part.trim();
        if !s.is_empty() && !out.iter().any(|x| x == s) {
            out.push(s.to_string());
        }
    }
    out
}
