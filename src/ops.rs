//! ProxySend - operaciones de alto nivel compartidas entre la UI, la bandeja y el CLI.

use crate::core;
use crate::store::{self, ProxyConfig, Scopes, Settings};
use crate::sysops::{self, OpResult};
use std::collections::BTreeMap;

/// Resultado de aplicar/quitar, con el detalle por nivel (para los avisos).
#[derive(Debug, Clone, Default)]
pub struct OpResults {
    pub results: BTreeMap<String, OpResult>,
    pub summary: String,
}

impl OpResults {
    pub fn failures(&self) -> Vec<String> {
        self.results.values().filter(|r| !r.ok).map(|r| r.detail.clone()).collect()
    }
    pub fn ok(&self) -> bool {
        self.results.values().all(|r| r.ok)
    }
}

pub fn apply_proxy_config(settings: &mut Settings, config: &ProxyConfig, scopes: Option<&Scopes>, proxy_id: Option<&str>) -> OpResults {
    use crate::i18n::{tf, tr};
    let v = core::validate_config_errors(config);
    if !v.is_empty() {
        let mut r = OpResults::default();
        r.results.insert(
            "validacion".to_string(),
            OpResult { ok: false, detail: v.join(" ") },
        );
        crate::eventlog::error("aplicar", tf("Validacion rechazada: {0}", &[&v.join(" ")]));
        return r;
    }
    if let Some(sc) = scopes {
        settings.scopes = *sc;
    }
    if let Some(id) = proxy_id {
        settings.active_proxy_id = Some(id.to_string());
    }
    settings.config = Some(config.clone());

    // v2.2.2: si el usuario no tiene marcado NINGUN nivel, no se aplica nada
    // y se explica en claro (antes quedaba "activado" sin hacer nada).
    if !settings.scopes.desktop && !settings.scopes.system && !settings.scopes.terminal {
        let detalle = tr("No hay ningun nivel seleccionado (escritorio, sistema ni terminal). Marca al menos un nivel en la pagina Estado y vuelve a intentarlo.").to_string();
        crate::eventlog::error("aplicar", &detalle);
        let mut r = OpResults::default();
        r.results.insert("validacion".to_string(), OpResult::fail(detalle));
        return r;
    }

    let mut results: BTreeMap<String, OpResult> = BTreeMap::new();
    if settings.scopes.desktop {
        results.insert("desktop".to_string(), sysops::desktop_apply(config));
    }
    if settings.scopes.system {
        results.insert("system".to_string(), sysops::env_apply(config));
    }
    if settings.scopes.terminal {
        results.insert("terminal".to_string(), sysops::shell_apply(config));
    }
    // v2.2.2: el proxy solo queda "activo" si al menos un nivel se aplico de
    // verdad. Antes se marcaba activo aunque todos los niveles hubieran
    // fallado (por ejemplo al cancelar la contraseña de administrador con
    // solo el nivel sistema marcado), y el Estado mentia.
    let any_ok = results.values().any(|r| r.ok);
    settings.active = any_ok;
    store::save_settings(settings);
    let summary = core::summary(config);
    for (nivel, res) in &results {
        if res.ok {
            crate::eventlog::info("aplicar", tf("Nivel {0}: ok ({1})", &[nivel, &res.detail]));
        } else if sysops::is_cancelled_detail(&res.detail) {
            // el usuario cerro la ventana de contraseña: aviso, no error
            crate::eventlog::aviso("aplicar", tf("Nivel {0}: cancelado - {1}", &[nivel, &res.detail]));
        } else {
            crate::eventlog::error("aplicar", tf("Nivel {0}: fallo - {1}", &[nivel, &res.detail]));
        }
    }
    if any_ok {
        crate::eventlog::info("aplicar", tf("Proxy aplicado: {0}", &[&summary]));
    } else {
        crate::eventlog::error("aplicar", tr("Ningun nivel se aplico: el proxy NO quedo activado. El detalle de cada nivel esta arriba y en el Informe."));
    }
    OpResults { results, summary }
}

pub fn remove_proxy_everywhere(settings: &mut Settings) -> OpResults {
    use crate::i18n::{tf, tr};
    let mut results: BTreeMap<String, OpResult> = BTreeMap::new();
    results.insert("desktop".to_string(), sysops::desktop_disable());
    results.insert("system".to_string(), sysops::env_clean());
    results.insert("terminal".to_string(), sysops::shell_clean());
    settings.active = false;
    settings.active_proxy_id = None;
    store::save_settings(settings);
    for (nivel, res) in &results {
        if res.ok {
            crate::eventlog::info("desactivar", tf("Nivel {0}: ok ({1})", &[nivel, &res.detail]));
        } else if sysops::is_cancelled_detail(&res.detail) {
            crate::eventlog::aviso("desactivar", tf("Nivel {0}: cancelado - {1}", &[nivel, &res.detail]));
        } else {
            crate::eventlog::error("desactivar", tf("Nivel {0}: fallo - {1}", &[nivel, &res.detail]));
        }
    }
    crate::eventlog::info("desactivar", tr("Proxy desactivado en todos los niveles"));
    crate::watcher::reset_state();
    OpResults { results, summary: String::new() }
}

pub fn reset_defaults(settings: &mut Settings) -> OpResults {
    use crate::i18n::tr;
    let mut results: BTreeMap<String, OpResult> = BTreeMap::new();
    results.insert("desktop".to_string(), sysops::desktop_reset());
    results.insert("system".to_string(), sysops::env_clean());
    results.insert("terminal".to_string(), sysops::shell_clean());
    let prev = settings.clone();
    *settings = Settings {
        config: None,
        active: false,
        active_proxy_id: None,
        proxies: prev.proxies.clone(), // la biblioteca de proxies se conserva
        scopes: prev.scopes,
        theme: prev.theme.clone(),
        autostart: prev.autostart,
        reconnect_enabled: prev.reconnect_enabled,
        reconnect_min: prev.reconnect_min,
        language: prev.language.clone(), // v2.3.0: el idioma elegido se conserva
        version: prev.version.clone(),
    };
    store::save_settings(settings);
    crate::eventlog::info("restablecer", tr("Valores por defecto restaurados (la biblioteca se conserva)"));
    OpResults { results, summary: String::new() }
}

pub fn set_scope(settings: &mut Settings, scope: &str, enabled: bool) -> OpResult {
    use crate::i18n::{tf, tr};
    if !matches!(scope, "desktop" | "system" | "terminal") {
        return OpResult::fail(tr("Nivel desconocido."));
    }
    match scope {
        "desktop" => settings.scopes.desktop = enabled,
        "system" => settings.scopes.system = enabled,
        "terminal" => settings.scopes.terminal = enabled,
        _ => {}
    }
    let mut result = None;
    if settings.active {
        if let Some(config) = settings.config.clone() {
            if enabled {
                result = Some(match scope {
                    "desktop" => sysops::desktop_apply(&config),
                    "system" => sysops::env_apply(&config),
                    _ => sysops::shell_apply(&config),
                });
            } else {
                result = Some(match scope {
                    "desktop" => sysops::desktop_disable(),
                    "system" => sysops::env_clean(),
                    _ => sysops::shell_clean(),
                });
            }
            store::log_history(
                if enabled { "activar" } else { "desactivar" },
                &tf("Nivel \"{0}\" {1}", &[scope, if enabled { tr("activado") } else { tr("desactivado") }]),
            );
            if let Some(r) = &result {
                if r.ok {
                    crate::eventlog::info("niveles", tf("Nivel {0} {1}: ok", &[scope, if enabled { tr("activado") } else { tr("desactivado") }]));
                } else if sysops::is_cancelled_detail(&r.detail) {
                    crate::eventlog::aviso("niveles", tf("Nivel {0} {1}: cancelado - {2}", &[scope, if enabled { tr("activado") } else { tr("desactivado") }, &r.detail]));
                } else {
                    crate::eventlog::error("niveles", tf("Nivel {0} {1}: fallo - {2}", &[scope, if enabled { tr("activado") } else { tr("desactivado") }, &r.detail]));
                }
            }
        }
    }
    // honestidad de estado (v2.4.0): si el usuario apaga el ULTIMO nivel con
    // el proxy activo, cada interruptor ya desactivo su nivel en el sistema,
    // asi que "activo" dejaria de ser cierto. El Estado no debe mentir.
    if settings.active
        && !settings.scopes.desktop
        && !settings.scopes.system
        && !settings.scopes.terminal
    {
        settings.active = false;
        crate::eventlog::info(
            "niveles",
            tr("Todos los niveles quedaron desactivados: el proxy ya no esta activado."),
        );
    }
    store::save_settings(settings);
    result.unwrap_or_else(|| OpResult::ok(""))
}

/// Aplicar el proxy guardado al iniciar sesion (modo --autostart).
pub fn apply_on_autostart() {
    use crate::i18n::{tf, tr};
    let settings = store::load_settings();
    if !settings.active {
        store::log_history("autostart", tr("Arranque automatico: no habia proxy guardado para aplicar."));
        return;
    }
    let config = match settings.config.clone() {
        Some(c) => c,
        None => {
            store::log_history("autostart", tr("Arranque automatico: no habia proxy guardado para aplicar."));
            return;
        }
    };
    // /etc/environment ya es persistente: no se toca aqui para no pedir contraseña al arrancar
    let mut results: Vec<OpResult> = Vec::new();
    if settings.scopes.desktop {
        results.push(sysops::desktop_apply(&config));
    }
    if settings.scopes.terminal {
        results.push(sysops::shell_apply(&config));
    }
    if results.is_empty() {
        // v2.4.0: antes se registraba "re-aplicado sin errores" aunque no
        // hubiera ningun nivel de escritorio ni terminal que aplicar.
        store::log_history(
            "autostart",
            tr("Arranque automatico: el proxy activo no tiene niveles de escritorio ni terminal que aplicar."),
        );
        return;
    }
    let failed: Vec<String> = results.iter().filter(|r| !r.ok).map(|r| r.detail.clone()).collect();
    store::log_history(
        "autostart",
        &if failed.is_empty() {
            tr("Arranque automatico: proxy guardado re-aplicado en escritorio y terminales.").to_string()
        } else {
            tf("Arranque automatico: re-aplicado con avisos - {0}", &[&failed.join(" ")])
        },
    );
    if failed.is_empty() {
        crate::eventlog::info("autostart", tr("Arranque automatico: proxy re-aplicado sin errores"));
    } else {
        for f in &failed {
            crate::eventlog::error("autostart", tf("Arranque automatico: {0}", &[f]));
        }
    }
}

/// Registra el inicio de la app en el log de eventos.
pub fn log_app_start(contexto: &str) {
    let de = sysops::detect_desktop_env();
    crate::eventlog::info(
        "app",
        crate::i18n::tf(
            "ProxySend {0} iniciado ({1}) - escritorio {2}",
            &[crate::APP_VERSION, contexto, crate::i18n::de_label(de)],
        ),
    );
}
