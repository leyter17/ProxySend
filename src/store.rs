//! ProxySend - modelo de datos y persistencia (~/.config/ProxySend, compatible con la v1.x).

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"));
    base.join("ProxySend")
}

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn history_file() -> PathBuf {
    config_dir().join("history.json")
}

/* ------------------------- Modelo ------------------------- */

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Endpoint {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProxyConfig {
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(default = "true_val")]
    pub same_for_all: bool,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: i64,
    #[serde(default)]
    pub http: Endpoint,
    #[serde(default)]
    pub https: Endpoint,
    #[serde(default)]
    pub auth: bool,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub ignore_hosts: Vec<String>,
}

fn default_protocol() -> String {
    "http".to_string()
}
fn true_val() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[derive(Default)]
pub struct SavedProxy {
    pub id: String,
    pub name: String,
    pub config: ProxyConfig,
    pub created_at: u64,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Scopes {
    pub desktop: bool,
    pub system: bool,
    pub terminal: bool,
}

impl Default for Scopes {
    fn default() -> Self {
        Self { desktop: true, system: true, terminal: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub config: Option<ProxyConfig>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub active_proxy_id: Option<String>,
    #[serde(default)]
    pub proxies: Vec<SavedProxy>,
    #[serde(default)]
    pub scopes: Scopes,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub autostart: bool,
    // v2.3.4: las opciones "cerrar a la bandeja" y "minimizar a la bandeja"
    // se retiraron por completo. Las claves viejas de settings.json
    // (closeToTray / minimizeToTray) se ignoran al leer: serde descarta los
    // campos desconocidos y no vuelven a escribirse.
    #[serde(default)]
    pub reconnect_enabled: bool,
    #[serde(default = "default_reconnect_min")]
    pub reconnect_min: u64,
    /// Idioma de la interfaz: None/"es" = espanol (por defecto), "en" = ingles (v2.3.0)
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

fn default_reconnect_min() -> u64 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryEntry {
    pub ts: u64,
    pub action: String,
    #[serde(default)]
    pub detail: String,
}

/* ------------------------- Persistencia ------------------------- */

/// Cerrojo de persistencia: la UI, la bandeja (hilo propio) y el temporizador
/// escriben los mismos archivos JSON. Sin este cerrojo dos hilos podian hacer
/// leer-modificar-escribir a la vez y perderse entradas de historial o ajustes.
fn persist_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Escritura ATOMICA: se escribe un archivo temporal unico en la misma carpeta
/// y se renombra encima del destino. Si el proceso muere a mitad de escritura,
/// el JSON vigente nunca queda a medias (el rename es atomico).
/// Nombre unico por llamada para que dos hilos nunca compartan el temporal.
pub(crate) fn write_file_atomic(path: &Path, data: &str) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let name = format!(
        ".{}.{}.{}.tmp",
        path.file_name().and_then(|s| s.to_str()).unwrap_or("archivo"),
        std::process::id(),
        n
    );
    let tmp = path.parent().unwrap_or(Path::new(".")).join(name);
    match fs::write(&tmp, data).and_then(|_| fs::rename(&tmp, path)) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            Err(e)
        }
    }
}

pub fn load_settings() -> Settings {
    match fs::read_to_string(settings_file()) {
        Ok(text) => serde_json::from_str::<Settings>(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save_settings(s: &Settings) {
    let _guard = persist_lock().lock().unwrap_or_else(|e| e.into_inner());
    let dir = config_dir();
    let _ = fs::create_dir_all(&dir);
    // permisos 700 en la carpeta de configuracion
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = write_file_atomic(&settings_file(), &json);
    }
}

pub fn read_history() -> Vec<HistoryEntry> {
    match fs::read_to_string(history_file()) {
        Ok(text) => serde_json::from_str::<Vec<HistoryEntry>>(&text).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub fn log_history(action: &str, detail: &str) {
    let _guard = persist_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut list = read_history();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    list.insert(0, HistoryEntry { ts, action: action.to_string(), detail: detail.to_string() });
    list.truncate(200);
    let _ = fs::create_dir_all(config_dir());
    if let Ok(json) = serde_json::to_string_pretty(&list) {
        let _ = write_file_atomic(&history_file(), &json);
    }
}

/// Nombre legible del intervalo de reconexion para la interfaz.
pub fn reconnect_min_options() -> [(u64, &'static str); 5] {
    [(1, "1 min"), (5, "5 min"), (10, "10 min"), (30, "30 min"), (60, "1 hora")]
}

pub fn clear_history() {
    let _guard = persist_lock().lock().unwrap_or_else(|e| e.into_inner());
    let _ = fs::create_dir_all(config_dir());
    let _ = write_file_atomic(&history_file(), "[]");
}

pub fn new_proxy_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let rand = fastrand_part();
    format!("px-{}{}", to_base36(n), rand)
}

fn to_base36(n: u128) -> String {
    const CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut n = n;
    let mut out = Vec::new();
    loop {
        out.push(CHARS[(n % 36) as usize]);
        n /= 36;
        if n == 0 {
            break;
        }
    }
    out.iter().rev().map(|b| *b as char).collect()
}

fn fastrand_part() -> String {
    // sin dependencias: semilla del reloj de alta resolucion
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ (d.as_millis() as u64) << 8)
        .unwrap_or(0);
    let a = t.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    to_base36(((a >> 16) % 99999) as u128).chars().take(5).collect()
}
