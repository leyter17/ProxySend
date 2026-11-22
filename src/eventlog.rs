//! ProxySend - registro de eventos de la aplicacion (el "log completo").
//!
//! Todo lo que sucede en la app queda anotado aqui con nivel (info / aviso /
//! error), categoria y fecha: operaciones de aplicar/quitar, resultados de las
//! pruebas de conexion, bandeja, autoarranque, reconexion automatica,
//! exportacion/importacion, informes y cualquier error inesperado (incluidos
//! los panics, que se capturan con un hook).
//!
//! El registro se guarda en ~/.config/ProxySend/eventos.log (texto plano,
//! una linea por evento, campos separados por tabulaciones) y ademas se
//! mantiene en memoria para la pagina Informe de la interfaz.

use chrono::Local;
use chrono::TimeZone;
use std::fs;
use std::sync::Mutex;
use std::sync::OnceLock;

const MAX_LINES: usize = 4000;
const KEEP_LINES: usize = 3000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Level {
    Info,
    Aviso,
    Error,
}

impl Level {
    /// Etiqueta del nivel en el log: se muestra en el idioma activo (INFO /
    /// AVISO|WARN / ERROR) y al leer se aceptan las dos variantes para que los
    /// registros viejos sigan leyendose.
    pub fn tag(&self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Aviso => if crate::i18n::is_en() { "WARN" } else { "AVISO" },
            Level::Error => "ERROR",
        }
    }
    pub fn from_tag(t: &str) -> Level {
        match t {
            "AVISO" | "WARN" => Level::Aviso,
            "ERROR" => Level::Error,
            _ => Level::Info,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub ts: u64, // milisegundos desde epoch
    pub level: Level,
    pub cat: String,
    pub msg: String,
}

struct LogState {
    memory: Vec<Event>,
}

static STATE: OnceLock<Mutex<LogState>> = OnceLock::new();

fn state() -> &'static Mutex<LogState> {
    STATE.get_or_init(|| Mutex::new(LogState { memory: Vec::new() }))
}

pub fn log_file() -> std::path::PathBuf {
    crate::store::config_dir().join("eventos.log")
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Anota un evento en el log (memoria + archivo). Nunca aborta: si el archivo
/// no se puede escribir, el evento queda en memoria.
pub fn log(level: Level, cat: &str, msg: impl Into<String>) {
    let ev = Event { ts: now_ms(), level, cat: cat.to_string(), msg: msg.into() };
    {
        let mut st = state().lock().unwrap();
        st.memory.push(ev.clone());
        if st.memory.len() > MAX_LINES {
            let skip = st.memory.len() - KEEP_LINES;
            st.memory.drain(0..skip);
        }
    }
    append_to_file(&ev);
}

pub fn info(cat: &str, msg: impl Into<String>) {
    log(Level::Info, cat, msg);
}
pub fn aviso(cat: &str, msg: impl Into<String>) {
    log(Level::Aviso, cat, msg);
}
pub fn error(cat: &str, msg: impl Into<String>) {
    log(Level::Error, cat, msg);
}

fn append_to_file(ev: &Event) {
    let path = log_file();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    let iso = Local
        .timestamp_millis_opt(ev.ts as i64)
        .single()
        .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    let line = format!("{}\t{}\t{}\t{}\n", iso, ev.level.tag(), ev.cat, ev.msg.replace('\t', " "));
    // sin permisos de escritura no se debe colgar la app: se ignora el error
    let res = fs::OpenOptions::new().create(true).append(true).open(&path).and_then(|mut f| {
        use std::io::Write;
        f.write_all(line.as_bytes())
    });
    if res.is_err() {
        return;
    }
    // recorte periodico: si el archivo crece demasiado, se conservan las ultimas lineas
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > 512 * 1024 {
            if let Ok(text) = fs::read_to_string(&path) {
                let lines: Vec<&str> = text.lines().collect();
                let start = lines.len().saturating_sub(KEEP_LINES);
                let trimmed = lines[start..].join("\n") + "\n";
                let _ = fs::write(&path, trimmed);
            }
        }
    }
}

/// Todos los eventos disponibles: memoria de esta sesion + archivo de sesiones
/// anteriores (el archivo manda: es el historial completo).
///
/// El archivo guarda la fecha con precision de segundos y la memoria con
/// precision de milisegundos: para no duplicar (que salia dos veces en el
/// informe) se comparan los eventos por segundo redondeado + mensaje.
pub fn read_all() -> Vec<Event> {
    use std::collections::HashSet;
    let mut out: Vec<Event> = Vec::new();
    let mut seen: HashSet<(u64, String)> = HashSet::new();
    if let Ok(text) = fs::read_to_string(log_file()) {
        for line in text.lines().rev() {
            if let Some(ev) = parse_line(line) {
                // clave de deduplicacion: segundo (sin milisegundos) + mensaje
                let key = (ev.ts / 1000, format!("{}|{}", ev.level.tag(), ev.msg));
                if seen.insert(key) {
                    out.push(ev);
                }
            }
        }
    }
    // eventos de esta sesion que podrian no haberse escrito (sin permisos)
    if let Ok(st) = state().lock() {
        for ev in st.memory.iter().rev() {
            let key = (ev.ts / 1000, format!("{}|{}", ev.level.tag(), ev.msg));
            if seen.insert(key) {
                out.push(ev.clone());
            }
        }
    }
    out
}

fn parse_line(line: &str) -> Option<Event> {
    let parts: Vec<&str> = line.split('\t').collect();
    if parts.len() < 4 {
        return None;
    }
    let ts = chrono::NaiveDateTime::parse_from_str(parts[0], "%Y-%m-%d %H:%M:%S")
        .ok()
        .and_then(|nd| Local.from_local_datetime(&nd).single().map(|d| d.timestamp_millis() as u64))
        .unwrap_or(0);
    Some(Event {
        ts,
        level: Level::from_tag(parts[1]),
        cat: parts[2].to_string(),
        msg: parts[3..].join("\t"),
    })
}

/// (info, avisos, errores) de todo el registro.
pub fn counts() -> (usize, usize, usize) {
    let all = read_all();
    let e = all.iter().filter(|e| e.level == Level::Error).count();
    let a = all.iter().filter(|e| e.level == Level::Aviso).count();
    let i = all.len() - e - a;
    (i, a, e)
}

pub fn last_error() -> Option<Event> {
    read_all().into_iter().find(|e| e.level == Level::Error)
}

pub fn clear() {
    let _ = fs::write(log_file(), "");
    state().lock().unwrap().memory.clear();
    info("registro", crate::i18n::tr("Registro de eventos limpiado"));
}

/// Hook de panico: los errores inesperados quedan en el log y en un archivo
/// aparte (panic.txt) para poder diagnostidarlos desde el informe.
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("{}", info);
        error("panic", &msg);
        let path = crate::store::config_dir().join("panic.txt");
        let _ = fs::write(
            &path,
            format!("{}\n{}", Local::now().format("%Y-%m-%d %H:%M:%S"), msg),
        );
        default(info);
    }));
}
