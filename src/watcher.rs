//! ProxySend - temporizador de reconexion automatica.
//!
//! Un hilo en segundo plano comprueba periodicamente (segun el intervalo que
//! elija el usuario) si el proxy activo sigue funcionando:
//!   - Si responde: se anota en el registro y en el estado.
//!   - Si NO responde: se vuelve a aplicar el proxy en los niveles escritorio y
//!     terminales (el nivel sistema no se repite: /etc/environment ya es
//!     persistente y repetirlo pediria contrasena), se anota el fallo y la app
//!     lo muestra en la pagina Estado y en el Informe.
//!
//! CICLO DE VIDA (corregido en 2.2.4): el temporizador SOLO existe mientras la
//! opcion "Reconexion automatica" esta activada. Al ejecutar la app no se
//! arranca ningun temporizador si la opcion esta apagada (antes el hilo se
//! creaba siempre al abrir la app y dejaba la anotacion "Temporizador ...
//! iniciado" en el registro aunque la opcion estuviera apagada). Se arranca
//! al activar la opcion (o al abrir la app si ya estaba activada y guardada)
//! y se DETIENE de verdad al desactivarla.
//!
//! El estado de la ultima verificacion se guarda en
//! ~/.config/ProxySend/reconnect.json para que la interfaz lo lea aunque el
//! hilo corra en segundo plano.

use crate::eventlog;
use crate::nettest;
use crate::store;
use chrono::Local;
use chrono::TimeZone;
use std::sync::{Mutex, OnceLock};

/// Estado del temporizador en el proceso: generacion actual y hilo vivo.
/// Toda decision de arrancar/parar pasa por este mutex, asi no hay carreras
/// entre la UI, el autoarranque y el propio hilo.
struct TimerCell {
    gen: u64,
    handle: Option<std::thread::JoinHandle<()>>,
}

fn timer_cell() -> &'static Mutex<TimerCell> {
    static CELL: OnceLock<Mutex<TimerCell>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(TimerCell { gen: 0, handle: None }))
}

fn hilo_vivo(cell: &TimerCell) -> bool {
    cell.handle.as_ref().map(|h| !h.is_finished()).unwrap_or(false)
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct ReconnectState {
    /// marca de tiempo (ms) de la ultima verificacion
    pub last_check: u64,
    /// resultado: Some(true) ok, Some(false) fallo, None sin resultado
    pub ok: Option<bool>,
    pub detail: String,
    /// veces que se re-aplico el proxy por fallos desde el inicio
    pub reintentos: u64,
}

pub fn state_file() -> std::path::PathBuf {
    store::config_dir().join("reconnect.json")
}

pub fn read_state() -> ReconnectState {
    std::fs::read_to_string(state_file())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn write_state(st: &ReconnectState) {
    let dir = store::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(st) {
        // escritura atomica: el temporizador y "Verificar ahora" pueden
        // escribir a la vez; el rename evita un reconnect.json a medias
        let _ = store::write_file_atomic(&state_file(), &json);
    }
}

pub fn reset_state() {
    write_state(&ReconnectState::default());
}

/// Ejecuta UNA verificacion de reconexion ahora mismo (la usa el hilo
/// temporizador y el boton "Verificar ahora"). Devuelve (ok, detalle).
pub fn check_once() -> (bool, String) {
    use crate::i18n::{tf, tr};
    let settings = store::load_settings();
    let Some(config) = settings.config.clone() else {
        return (false, tr("No hay un proxy configurado para verificar.").to_string());
    };
    if !settings.active {
        return (false, tr("El proxy no esta activado: nada que reconectar.").to_string());
    }

    let r = nettest::test_connection(Some(&config), true);
    let mut st = read_state();
    st.last_check = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    if r.ok {
        let detalle = tf("OK - IP {0} - {1} s", &[&r.ip, &format!("{:.1}", r.seconds)]);
        // si antes fallaba y ahora responde, avisar de la recuperacion
        if st.ok == Some(false) {
            eventlog::info(
                "reconexion",
                tf("El proxy VOLVIO A RESPONDER despues del fallo: {0}", &[&detalle]),
            );
        } else {
            eventlog::info("reconexion", tf("Verificacion automatica correcta: {0}", &[&detalle]));
        }
        st.ok = Some(true);
        st.detail = detalle;
        write_state(&st);
        (true, st.detail.clone())
    } else {
        // re-aplicar el proxy en escritorio y terminales
        let mut resultados: Vec<String> = Vec::new();
        if settings.scopes.desktop {
            let rr = crate::sysops::desktop_apply(&config);
            resultados.push(tf("escritorio: {0}", &[if rr.ok { tr("re-aplicado") } else { &rr.detail }]));
        }
        if settings.scopes.terminal {
            let rr = crate::sysops::shell_apply(&config);
            resultados.push(tf("terminales: {0}", &[if rr.ok { tr("re-aplicado") } else { &rr.detail }]));
        }
        // el nivel sistema (/etc/environment) ya es persistente: no se repite
        let niveles_txt = if resultados.is_empty() {
            tr("ningun nivel configurado").to_string()
        } else {
            resultados.join("; ")
        };
        let detalle = tf(
            "FALLO ({0}) - proxy re-aplicado en {1}",
            &[if r.detail.is_empty() { tr("sin respuesta del proxy") } else { &r.detail }, &niveles_txt],
        );
        st.ok = Some(false);
        st.detail = detalle.clone();
        st.reintentos += 1;
        write_state(&st);
        eventlog::error("reconexion", &detalle);
        (false, detalle)
    }
}

/// Hilo temporizador: cada 15 segundos mira si toca verificar segun el
/// intervalo elegido. SOLO se arranca cuando la opcion "Reconexion automatica"
/// esta activada (desde la UI al pulsar el interruptor, o al abrir la app si
/// la opcion ya estaba activada en los ajustes guardados). Si ya hay un
/// temporizador vivo no se duplica.
pub fn spawn_timer() {
    let mut cell = timer_cell().lock().unwrap_or_else(|e| e.into_inner());
    if hilo_vivo(&cell) {
        return; // ya hay un temporizador
    }
    cell.gen += 1;
    let gen = cell.gen;
    // la anotacion de arranque se escribe AQUI (hilo llamador), no dentro del
    // hilo nuevo: asi queda ordenada con el resto del registro y las pruebas
    // son deterministas
    eventlog::info("reconexion", crate::i18n::tr("Temporizador de reconexion automatica iniciado"));
    cell.handle = Some(std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(15));
            // si alguien paro el temporizador, el hilo termina en su siguiente tic
            if timer_cell().lock().unwrap_or_else(|e| e.into_inner()).gen != gen {
                break;
            }
            // doble barrera: aunque el hilo siguiera vivo por error, nunca
            // verifica si la opcion de reconexion esta apagada
            let s = store::load_settings();
            if !s.reconnect_enabled || !s.active || s.config.is_none() {
                continue;
            }
            let intervalo = s.reconnect_min.max(1) * 60;
            let st = read_state();
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let last = st.last_check / 1000;
            if now.saturating_sub(last) >= intervalo {
                check_once();
            }
        }
    }));
}

/// Para el temporizador de reconexion de verdad (lo usa la UI al desactivar
/// la opcion). El hilo muere en su siguiente tic (como mucho 15 segundos
/// despues) y la parada queda anotada en el registro de eventos.
pub fn stop_timer() {
    let mut cell = timer_cell().lock().unwrap_or_else(|e| e.into_inner());
    let vivo = hilo_vivo(&cell);
    cell.gen += 1; // el hilo actual ve la generacion nueva y termina
    cell.handle = None;
    drop(cell);
    if vivo {
        eventlog::info("reconexion", crate::i18n::tr("Temporizador de reconexion automatica detenido"));
    }
}

/// Ajusta el temporizador a lo que digan los ajustes guardados: si la opcion
/// de reconexion automatica esta activada lo arranca (si no estaba ya) y si
/// esta desactivada lo para. Es la unica via de arranque al abrir la app.
pub fn sync_timer() {
    let s = store::load_settings();
    if s.reconnect_enabled {
        spawn_timer();
    } else {
        stop_timer();
    }
}

/// Formato amable "hace 3 min" para la interfaz.
pub fn hace_cuanto(ms: u64) -> String {
    use crate::i18n::tf;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let dif = now.saturating_sub(ms) / 1000;
    if dif < 60 {
        tf("hace {0} s", &[&dif.to_string()])
    } else if dif < 3600 {
        tf("hace {0} min", &[&(dif / 60).to_string()])
    } else {
        tf("hace {0} h {1} min", &[&(dif / 3600).to_string(), &((dif % 3600) / 60).to_string()])
    }
}

pub fn fecha(ms: u64) -> String {
    Local.timestamp_millis_opt(ms as i64)
        .single()
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}
