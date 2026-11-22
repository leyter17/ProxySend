//! ProxySend - bandeja del sistema con ksni (StatusNotifierItem sobre D-Bus,
//! implementado en Rust puro: sin libappindicator ni GTK).

use crate::ops;
use crate::store;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Aviso pendiente que la bandeja deja para la ventana (v2.3.1): el resultado
/// de Activar/Desactivar proxy desde el menu de la bandeja. La UI lo muestra
/// como toast y lo consume; si la ventana esta oculta se muestra al volver.
pub struct TrayNote {
    pub ok: bool,
    pub msg: String,
}

pub struct TrayShared {
    pub show: AtomicBool,
    pub dirty: AtomicBool,
    /// v2.3.4: la ventana NO esta en pantalla ahora mismo (minimizada por el
    /// usuario). Lo escribe la UI en cada frame y lo lee la decision de
    /// mostrar: X11 des-minimiza, Wayland reabre. (En v2.3.3 y antes tambien
    /// lo escribian las opciones de ocultar en la bandeja, retiradas en
    /// v2.3.4.)
    pub hidden: AtomicBool,
    /// v2.3.2: reapertura pendiente (Wayland). En Wayland el protocolo NO
    /// permite des-minimizar la ventana (winit lo ignora) ni darle foco,
    /// asi que "Mostrar ProxySend" cierra la ventana actual y main() abre una
    /// nueva en primer plano. Esta bandera le dice a main() que tras cerrar
    /// debe volver a abrir, no salir.
    pub restore: AtomicBool,
    note: Mutex<Option<TrayNote>>,
}

impl TrayShared {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            show: AtomicBool::new(false),
            dirty: AtomicBool::new(false),
            hidden: AtomicBool::new(false),
            restore: AtomicBool::new(false),
            note: Mutex::new(None),
        })
    }

    /// Deja un aviso para la ventana (sustituye al anterior si no se leyo).
    pub fn put_note(&self, ok: bool, msg: impl Into<String>) {
        *self.note.lock().unwrap() = Some(TrayNote { ok, msg: msg.into() });
    }

    /// Lee y consume el aviso pendiente (si lo hay).
    pub fn take_note(&self) -> Option<TrayNote> {
        self.note.lock().unwrap().take()
    }
}

/// Atiende la peticion de "mostrate" que escribe una segunda instancia
/// (abrir ProxySend cuando ya esta corriendo). v2.3.1: antes el archivo se
/// escribia pero nadie lo leia, y la ventana ya corriendo (u oculta en la
/// bandeja) no se mostraba nunca.
/// Devuelve true si habia una peticion pendiente (y la elimina).
pub fn consume_show_request() -> bool {
    let path = store::config_dir().join("show.request");
    std::fs::remove_file(&path).is_ok()
}

/* ---------- v2.3.2: decision unificada de "mostrar la ventana" ---------- */

/// Que hacer cuando la bandeja (o una segunda instancia) pide mostrar la
/// ventana. La eleccion depende del servidor grafico porque Wayland NO
/// permite a la app ocultar/reaparecer su ventana (winit 0.30 ignora
/// Visible() y Focus() en Wayland), pero X11 si.
pub enum ShowAction {
    /// No hay ninguna peticion pendiente.
    None,
    /// La ventana esta viva: hacerla visible y darle foco (X11; en Wayland
    /// solo sirve si sigue visible, el foco lo controla el compositor).
    Raise,
    /// Reabrir: cerrar la ventana actual y que main() abra una nueva en
    /// primer plano (es la unica via fiable en Wayland).
    Reopen,
}

/// Logica pura (sin entorno) para poder probarla con cargo test.
pub fn show_action_for(wayland: bool, hidden: bool) -> ShowAction {
    if wayland {
        if hidden {
            ShowAction::Reopen
        } else {
            // visible: el foco lo controla el compositor en Wayland; pedimos
            // el foco (no-op hoy) sin cerrar nada, para no perder el formulario.
            ShowAction::Raise
        }
    } else {
        // X11: la ventana se puede mapear y enfocar siempre.
        ShowAction::Raise
    }
}

/// ¿Estamos en Wayland? El mismo criterio que usa winit 0.30 para elegir
/// backend: WAYLAND_DISPLAY presente (o sesion declarada wayland), salvo que
/// WINIT_UNIX_BACKEND=x11 fuerce X11.
pub fn is_wayland() -> bool {
    if let Ok(v) = std::env::var("WINIT_UNIX_BACKEND") {
        if v.eq_ignore_ascii_case("x11") {
            return false;
        }
    }
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE").map(|v| v.eq_ignore_ascii_case("wayland")).unwrap_or(false)
}

/// Lee y consume UNA peticion de mostrar (bandeja o segunda instancia) y
/// decide la accion segun el servidor grafico y si la ventana esta escondida.
/// Lo usan igual el bucle de la interfaz y el hilo puente de main.rs: solo
/// uno de los dos consume cada peticion, y ambos aplican la misma decision.
pub fn take_show_action(shared: &Arc<TrayShared>) -> ShowAction {
    let pedida = shared.show.swap(false, Ordering::SeqCst) || consume_show_request();
    if !pedida {
        return ShowAction::None;
    }
    show_action_for(is_wayland(), shared.hidden.load(Ordering::SeqCst))
}

/// Apaga el servicio de bandeja de forma ordenada (se usa en modo autoarranque
/// antes de abrir la ventana, que crea su propia bandeja).
pub fn shutdown_tray() {
    // se saca del slot y se apaga FUERA del bloqueo (shutdown espera al servicio)
    let h = tray_slot().lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(h) = h {
        h.shutdown();
    }
}

struct TrayState {
    active: bool,
    summary: String,
}

pub struct TrayModel {
    shared: Arc<TrayShared>,
    state: Mutex<TrayState>,
    icon_on: ksni::Icon,
    icon_off: ksni::Icon,
}

static TRAY_HANDLE: OnceLock<Mutex<Option<TrayHandle>>> = OnceLock::new();
pub type TrayHandle = ksni::blocking::Handle<TrayModel>;

/// Ranura del handle de bandeja vivo. Es REEMPLAZABLE (v2.4.0): en el paso
/// autoarranque -> "Mostrar ProxySend" la bandeja se apaga y se crea otra
/// nueva; con el OnceLock de antes el segundo handle se descartaba en
/// silencio y el icono nuevo quedaba sin refrescar nunca.
fn tray_slot() -> &'static Mutex<Option<TrayHandle>> {
    TRAY_HANDLE.get_or_init(|| Mutex::new(None))
}

fn with_tray_handle(f: impl FnOnce(&TrayHandle)) {
    let guard = tray_slot().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(h) = guard.as_ref() {
        f(h);
    }
}

fn rgba_to_argb32(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.as_chunks::<4>().0 {
        let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
        // ARGB32: alfa primero
        out.extend_from_slice(&[a, r, g, b]);
    }
    out
}

fn load_icon_bytes(on: bool) -> ksni::Icon {
    let png = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(png).expect("icono embebido");
    let mut tinted = img.to_rgba8();
    if on {
        // tinte teal (#00BFA5) cuando el proxy esta activo
        for px in tinted.pixels_mut() {
            let (r, g, b, a) = (px.0[0] as f32, px.0[1] as f32, px.0[2] as f32, px.0[3] as f32);
            if a > 0.0 {
                px.0[0] = (0.35 * r + 0.65 * 0.0).round() as u8;
                px.0[1] = (0.35 * g + 0.65 * 191.0).round() as u8;
                px.0[2] = (0.35 * b + 0.65 * 165.0).round() as u8;
            }
        }
    }
    let img_dyn = image::DynamicImage::ImageRgba8(tinted);
    let small = img_dyn.resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
    let rgba = small.to_rgba8();
    ksni::Icon {
        width: 32,
        height: 32,
        data: rgba_to_argb32(rgba.as_raw()),
    }
}

impl TrayModel {
    fn refresh_from_disk(&mut self) {
        let s = store::load_settings();
        let mut st = self.state.lock().unwrap();
        st.active = s.active;
        st.summary = s
            .config
            .as_ref()
            .map(crate::core::summary)
            .unwrap_or_default();
    }
}

impl ksni::Tray for TrayModel {
    fn id(&self) -> String {
        "proxysend".to_string()
    }

    fn title(&self) -> String {
        "ProxySend".to_string()
    }

    fn icon_name(&self) -> String {
        String::new()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        let active = self.state.lock().unwrap().active;
        vec![if active { self.icon_on.clone() } else { self.icon_off.clone() }]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let st = self.state.lock().unwrap();
        let desc = if st.active {
            if st.summary.is_empty() {
                crate::i18n::tr("Proxy activado").to_string()
            } else {
                crate::i18n::tf("Proxy activado ({0})", &[&st.summary])
            }
        } else {
            crate::i18n::tr("Proxy desactivado").to_string()
        };
        ksni::ToolTip {
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
            title: "ProxySend".to_string(),
            description: desc,
        }
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let active = self.state.lock().unwrap().active;
        let std_item = |label: String, activate: Box<dyn Fn(&mut TrayModel) + Send>| ksni::MenuItem::Standard(
            ksni::menu::StandardItem {
                label,
                enabled: true,
                visible: true,
                icon_name: String::new(),
                icon_data: Vec::new(),
                shortcut: Vec::new(),
                disposition: ksni::menu::Disposition::Normal,
                activate,
            },
        );
        vec![
            std_item(
                crate::i18n::tr("Mostrar ProxySend").to_string(),
                Box::new(|m: &mut Self| m.shared.show.store(true, Ordering::SeqCst)),
            ),
            std_item(
                if active { crate::i18n::tr("Desactivar proxy").to_string() } else { crate::i18n::tr("Activar proxy").to_string() },
                // v2.3.1: la accion informa su resultado. Antes el trabajo
                // corria en un hilo sin decir nada: si faltaba la contrasena
                // de administrador o fallaba un nivel, la bandeja parecia
                // "no hacer nada". Ahora deja un aviso para la ventana, lo
                // anota en el historial y refresca el icono.
                Box::new(|m: &mut Self| {
                    let shared = m.shared.clone();
                    std::thread::spawn(move || {
                        let mut s = store::load_settings();
                        if s.active {
                            let r = ops::remove_proxy_everywhere(&mut s);
                            let fallos: Vec<String> = r
                                .results
                                .iter()
                                .filter(|(_, x)| !x.ok)
                                .map(|(n, x)| format!("{n}: {}", x.detail))
                                .collect();
                            if fallos.is_empty() {
                                shared.put_note(true, crate::i18n::tr("Proxy desactivado desde la bandeja del sistema"));
                                store::log_history("desactivar", crate::i18n::tr("Proxy desactivado desde la bandeja del sistema"));
                            } else {
                                let msg = crate::i18n::tf("No se pudo desactivar el proxy desde la bandeja: {0}", &[&fallos.join(" | ")]);
                                shared.put_note(false, &msg);
                                store::log_history("desactivar", &msg);
                            }
                        } else if let Some(c) = s.config.clone() {
                            let r = ops::apply_proxy_config(&mut s, &c, None, None);
                            let fallos: Vec<String> = r
                                .results
                                .iter()
                                .filter(|(_, x)| !x.ok)
                                .map(|(n, x)| format!("{n}: {}", x.detail))
                                .collect();
                            if fallos.is_empty() {
                                let msg = crate::i18n::tf("Proxy activado desde la bandeja del sistema - {0}", &[&crate::core::summary(&c)]);
                                shared.put_note(true, &msg);
                                store::log_history("activar", &msg);
                            } else {
                                let msg = crate::i18n::tf("No se pudo activar el proxy desde la bandeja: {0}", &[&fallos.join(" | ")]);
                                shared.put_note(false, &msg);
                                store::log_history("activar", &msg);
                            }
                        } else {
                            // sin configuracion guardada: abrir la app para que
                            // se cree una, y avisarlo (antes se abria sin decir nada)
                            shared.put_note(false, crate::i18n::tr("No hay ningun proxy guardado todavia: agrega uno en la pagina \"Mis proxies\"."));
                            shared.show.store(true, Ordering::SeqCst);
                            return;
                        }
                        shared.dirty.store(true, Ordering::SeqCst);
                        with_tray_handle(|h| {
                            let _ = h.update(|m: &mut TrayModel| m.refresh_from_disk());
                        });
                    });
                }),
            ),
            ksni::MenuItem::Separator,
            std_item(
                crate::i18n::tr("Salir").to_string(),
                // v2.3.4: SALIR INSTANTANEO. Antes se pedia un cierre ordenado
                // de la ventana y se esperaba hasta 1.5 s con respaldo; si la
                // ventana estaba minimizada o sin pintar tardaba y el usuario
                // percibia la demora. Ahora la salida desde la bandeja deja
                // constancia y termina el proceso en el momento: no hay nada
                // que guardar pendiente (los ajustes y el historial se guardan
                // en cada operacion) y el sistema libera el bloqueo de
                // instancia y la sesion D-Bus al morir el proceso.
                Box::new(|m: &mut Self| {
                    crate::store::log_history(
                        "bandeja",
                        crate::i18n::tr("Salida de la aplicacion desde la bandeja del sistema"),
                    );
                    crate::eventlog::info(
                        "bandeja",
                        crate::i18n::tr("Salir desde la bandeja: proceso terminado al instante"),
                    );
                    let _ = m; // el modelo deja de importar: el proceso termina aqui
                    std::process::exit(0);
                }),
            ),
        ]
    }

    /// Clic en el icono de la bandeja (v2.3.2): pide mostrar la ventana; la
    /// decision de subir o reabrir la toma take_show_action segun el entorno.
    fn activate(&mut self, _x: i32, _y: i32) {
        self.shared.show.store(true, Ordering::SeqCst);
    }
}

/// Lanza la bandeja en un hilo propio. Devuelve true si el entorno tiene bandeja.
pub fn spawn_tray(shared: Arc<TrayShared>) -> bool {
    let model = TrayModel {
        shared,
        state: Mutex::new(TrayState { active: false, summary: String::new() }),
        icon_on: load_icon_bytes(true),
        icon_off: load_icon_bytes(false),
    };
    let mut model = model;
    model.refresh_from_disk();
    use ksni::blocking::TrayMethods;
    match model.spawn() {
        Ok(handle) => {
            // sustituye (o deja) el handle vivo en la ranura reemplazable
            *tray_slot().lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
            crate::eventlog::info("bandeja", crate::i18n::tr("Bandeja del sistema lista (StatusNotifierItem)"));
            true
        }
        Err(e) => {
            crate::eventlog::aviso("bandeja", crate::i18n::tf("Bandeja del sistema no disponible: {0}", &[&e.to_string()]));
            eprintln!("[proxysend] bandeja no disponible: {e}");
            false
        }
    }
}

pub fn notify_state_changed() {
    with_tray_handle(|h| {
        let _ = h.update(|m: &mut TrayModel| m.refresh_from_disk());
    });
}
