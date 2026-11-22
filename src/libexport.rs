//! ProxySend - exportacion e importacion de la biblioteca completa.
//!
//! La biblioteca completa incluye: todos los proxies guardados, los ajustes de
//! la app (niveles, tema, autoarranque, reconexion automatica), el perfil
//! activo y el historial de actividad. Es una copia de seguridad que se puede
//! restaurar en otra maquina o despues de formatear.
//!
//! El archivo es JSON con extension .json y guarda las contrasenas tal cual
//! (es una copia de seguridad local): el informe diagnostico, en cambio,
//! siempre las oculta.

use crate::store::{self, Settings};
use chrono::Local;

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LibraryPayload {
    pub app: String,
    pub tipo: String,
    pub version: String,
    pub exportado: u64,
    pub exportado_iso: String,
    pub ajustes: AjustesPayload,
    pub activa: bool,
    pub active_proxy_id: Option<String>,
    pub config_actual: Option<store::ProxyConfig>,
    pub proxies: Vec<store::SavedProxy>,
    pub historial: Vec<store::HistoryEntry>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AjustesPayload {
    pub scopes: store::Scopes,
    pub theme: Option<String>,
    pub autostart: bool,
    // v2.3.4: las claves closeToTray/minimizeToTray de bibliotecas viejas se
    // ignoran al leer (serde descarta los campos desconocidos) y ya no se
    // escriben en las bibliotecas nuevas.
    pub reconnect_enabled: bool,
    pub reconnect_min: u64,
    pub language: Option<String>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Exporta la biblioteca completa a ~/ProxySend-biblioteca-<fecha>.json
pub fn export_library(settings: &Settings) -> Result<String, String> {
    let payload = LibraryPayload {
        app: "ProxySend".to_string(),
        tipo: "biblioteca".to_string(),
        version: crate::APP_VERSION.to_string(),
        exportado: now_ms(),
        exportado_iso: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        ajustes: AjustesPayload {
            scopes: settings.scopes,
            theme: settings.theme.clone(),
            autostart: settings.autostart,
            reconnect_enabled: settings.reconnect_enabled,
            reconnect_min: settings.reconnect_min,
            language: settings.language.clone(),
        },
        activa: settings.active,
        active_proxy_id: settings.active_proxy_id.clone(),
        config_actual: settings.config.clone(),
        proxies: settings.proxies.clone(),
        historial: store::read_history(),
    };
    let json = serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?;
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let path = store::home_dir().join(format!("ProxySend-biblioteca-{}.json", stamp));
    std::fs::write(&path, json).map_err(|e| crate::i18n::tf("No se pudo escribir el archivo: {0}", &[&e.to_string()]))?;
    Ok(path.to_string_lossy().to_string())
}

pub struct ImportResult {
    pub proxies: usize,
    pub historial: usize,
    pub advertencias: Vec<String>,
    pub path: String,
}

/// Importa una biblioteca completa. Los proxies guardados se sustituyen por
/// los del archivo; el estado del sistema (proxy aplicado ahora mismo) no se
/// toca: para usar un proxy importado hay que activarlo desde la app.
pub fn import_library(path: &str) -> Result<ImportResult, String> {
    use crate::i18n::{tf, tr};
    let text = std::fs::read_to_string(path)
        .map_err(|e| tf("No se pudo leer {0}: {1}", &[path, &e.to_string()]))?;
    let data: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| tf("Archivo JSON invalido: {0}", &[&e.to_string()]))?;
    if data.get("tipo").and_then(|t| t.as_str()) != Some("biblioteca") {
        return Err(
            tr("Este archivo no es una biblioteca completa de ProxySend (usa Importar configuracion para archivos de un solo proxy).")
                .to_string(),
        );
    }
    let mut payload: LibraryPayload =
        serde_json::from_value(data).map_err(|e| tf("La biblioteca no es valida: {0}", &[&e.to_string()]))?;

    let mut s = store::load_settings();
    let mut advertencias: Vec<String> = Vec::new();

    // validar cada proxy importado
    let mut validos: Vec<store::SavedProxy> = Vec::new();
    for p in payload.proxies.drain(..) {
        if crate::core::validate_config_errors(&p.config).is_empty() {
            validos.push(p);
        } else {
            advertencias.push(tf("\"{0}\" se descarto por tener datos invalidos", &[&p.name]));
        }
    }
    let n_proxies = validos.len();
    s.proxies = validos;

    // ajustes restaurables sin tocar el sistema
    s.scopes = payload.ajustes.scopes;
    if let Some(t) = &payload.ajustes.theme {
        s.theme = Some(t.clone());
    }
    s.reconnect_enabled = payload.ajustes.reconnect_enabled;
    s.reconnect_min = payload.ajustes.reconnect_min;
    s.autostart = payload.ajustes.autostart;
    // v2.3.0: el idioma viene en la biblioteca y se restaura tambien
    if payload.ajustes.language.is_some() {
        s.language = payload.ajustes.language.clone();
    }
    // el proxy activo del archivo solo se recuerda si su perfil existe
    if let Some(id) = &payload.active_proxy_id {
        if s.proxies.iter().any(|p| &p.id == id) {
            s.active_proxy_id = Some(id.clone());
        }
    }
    store::save_settings(&s);

    // historial: se agrega al existente (sin duplicar por marca de tiempo)
    let mut actuales = store::read_history();
    let mut n_hist = 0;
    for h in payload.historial {
        if !actuales.iter().any(|x| x.ts == h.ts && x.detail == h.detail) {
            actuales.push(h);
            n_hist += 1;
        }
    }
    actuales.sort_by_key(|e| std::cmp::Reverse(e.ts));
    actuales.truncate(200);
    let dir = store::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(&actuales) {
        // escritura atomica (la bandeja y la UI pueden escribir el historial a la vez)
        let _ = store::write_file_atomic(&store::history_file(), &json);
    }

    store::log_history(
        "importar-biblioteca",
        &tf("Biblioteca importada: {0} proxies, {1} eventos de historial ({2})", &[&n_proxies.to_string(), &n_hist.to_string(), path]),
    );
    crate::eventlog::info(
        "biblioteca",
        tf("Importacion completa desde {0}: {1} proxies validos, {2} historial", &[path, &n_proxies.to_string(), &n_hist.to_string()]),
    );
    if payload.config_actual.is_none() && payload.activa {
        advertencias.push(
            tr("La biblioteca decia tener un proxy activo pero sin configuracion; el sistema actual no se toco.").into(),
        );
    }
    Ok(ImportResult {
        proxies: n_proxies,
        historial: n_hist,
        advertencias,
        path: path.to_string(),
    })
}

/// Busca la biblioteca mas reciente en HOME (para importar sin ventanita).
pub fn latest_library_in_home() -> Option<String> {
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    let rd = std::fs::read_dir(store::home_dir()).ok()?;
    for entry in rd.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("ProxySend-biblioteca-") && name.ends_with(".json") {
            if let Ok(meta) = entry.metadata() {
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if best.as_ref().map(|(t, _)| mtime >= *t).unwrap_or(true) {
                    best = Some((mtime, entry.path()));
                }
            }
        }
    }
    best.map(|(_, p)| p.to_string_lossy().to_string())
}
