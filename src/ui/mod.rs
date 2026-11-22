//! ProxySend - interfaz grafica (egui) con estilo LocalSend: barra lateral,
//! tarjetas redondeadas, tema claro/oscuro y boton grande de accion.

use crate::core;
use crate::store::{self, HistoryEntry, ProxyConfig, SavedProxy, Settings};
use crate::sysops;
use crate::tray::TrayShared;
use chrono::Datelike;
use chrono::TimeZone;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub mod pages;
pub mod widgets;

// v2.3.2: contexto de egui publicado para el hilo puente de la bandeja
// (main.rs). Con el la bandeja puede mostrar la ventana y cerrar la app
// aunque el bucle de la interfaz no este pintando. En v2.3.2 es REEMPLAZABLE:
// en Wayland "Mostrar ProxySend" reabre la ventana (cierra y abre), y la
// ventana nueva trae un contexto nuevo que sustituye al viejo.
static BRIDGE_CTX: Mutex<Option<egui::Context>> = Mutex::new(None);

pub fn bridge_ctx() -> Option<egui::Context> {
    BRIDGE_CTX.lock().ok().and_then(|g| g.clone())
}

pub fn set_bridge_ctx(ctx: egui::Context) {
    if let Ok(mut g) = BRIDGE_CTX.lock() {
        *g = Some(ctx);
    }
}

pub fn action_label(action: &str) -> &'static str {
    let key = match action {
        "activar" => "Proxy activado",
        "desactivar" => "Proxy desactivado",
        "restablecer" => "Valores por defecto restaurados",
        "guardar" => "Configuracion guardada",
        "probar" => "Prueba de conexion",
        "exportar" => "Configuracion exportada",
        "importar" => "Configuracion importada",
        "limpiar" => "Historial limpiado",
        "guardar-biblioteca" => "Proxy guardado en la biblioteca",
        "eliminar-biblioteca" => "Proxy eliminado de la biblioteca",
        "activar-biblioteca" => "Proxy de la biblioteca activado",
        "exportar-biblioteca" => "Biblioteca completa exportada",
        "importar-biblioteca" => "Biblioteca completa importada",
        "reconexion" => "Reconexion automatica",
        "informe" => "Informe diagnostico",
        "autostart" => "Autoarranque",
        "bandeja" => "Bandeja del sistema",
        "idioma" => "Idioma cambiado",
        _ => "Actividad",
    };
    crate::i18n::tr(key)
}

pub fn action_color(action: &str) -> egui::Color32 {
    use egui::Color32;
    // v2.4.0 (paleta azul): mismos significados, tonos reafinados al azul
    match action {
        "activar" | "activar-biblioteca" => Color32::from_rgb(16, 168, 152),
        "desactivar" => Color32::from_rgb(100, 112, 132),
        "restablecer" => Color32::from_rgb(245, 158, 11),
        "eliminar-biblioteca" | "limpiar" => Color32::from_rgb(220, 38, 38),
        "probar" | "reconexion" => Color32::from_rgb(37, 99, 235),
        "informe" | "exportar-biblioteca" | "importar-biblioteca" => Color32::from_rgb(129, 90, 240),
        _ => Color32::from_rgb(37, 99, 235),
    }
}

pub fn format_time(ts: u64) -> String {
    match chrono::Local.timestamp_millis_opt(ts as i64).single() {
        Some(d) => format!(
            "{:02} {} {}  {}",
            d.day(),
            crate::i18n::mes((d.month0() as usize).min(11)),
            d.year(),
            d.format("%H:%M:%S")
        ),
        None => String::new(),
    }
}

/* ------------------------- Paleta azul (v2.4.0) ------------------------- */

/// Paleta "azul": barra lateral azul marino con degradado vertical, contenido
/// luminoso en azul y un azul brillante como color de marca. Los tonos de exito se
/// mantienen en verde-azulado (teal) para convivir en armonia con el azul.
#[derive(Clone, Copy)]
pub struct Palette {
    pub bg: egui::Color32,
    pub card: egui::Color32,
    pub card2: egui::Color32,
    pub line: egui::Color32,
    pub text: egui::Color32,
    pub soft: egui::Color32,
    pub faint: egui::Color32,
    pub accent: egui::Color32,
    /// azul claro de apoyo (degradados, resaltes sobre la barra lateral)
    pub accent_hi: egui::Color32,
    pub accent2: egui::Color32,
    pub accent_soft: egui::Color32,
    pub danger: egui::Color32,
    pub ok_text: egui::Color32,
    pub field_bg: egui::Color32,
    /// borde de las barras de escritura: resalta SUAVE los campos para que se vean
    pub field_line: egui::Color32,
    // barra lateral navy (degradado vertical)
    pub sidebar_top: egui::Color32,
    pub sidebar_bot: egui::Color32,
    /// relleno del elemento activo del menu (blanco translucido)
    pub nav_fill: egui::Color32,
    /// relleno al pasar el puntero por el menu
    pub nav_hover: egui::Color32,
    /// texto secundario sobre la barra lateral (blanco translucido)
    pub on_sidebar_soft: egui::Color32,
    /// sombra suave para tarjetas y botones principales (NONE en oscuro)
    pub shadow: egui::Shadow,
}

pub fn palette(dark: bool) -> Palette {
    if dark {
        Palette {
            bg: egui::Color32::from_rgb(11, 17, 31),
            card: egui::Color32::from_rgb(18, 26, 44),
            card2: egui::Color32::from_rgb(24, 34, 56),
            line: egui::Color32::from_rgb(37, 50, 79),
            text: egui::Color32::from_rgb(231, 237, 250),
            soft: egui::Color32::from_rgb(167, 180, 206),
            faint: egui::Color32::from_rgb(124, 139, 170),
            accent: egui::Color32::from_rgb(77, 141, 255),
            accent_hi: egui::Color32::from_rgb(126, 179, 255),
            accent2: egui::Color32::from_rgb(45, 217, 184),
            accent_soft: egui::Color32::from_rgba_unmultiplied(77, 141, 255, 42),
            danger: egui::Color32::from_rgb(248, 113, 113),
            ok_text: egui::Color32::from_rgb(53, 223, 184),
            field_bg: egui::Color32::from_rgb(13, 20, 36),
            field_line: egui::Color32::from_rgb(61, 83, 120),
            sidebar_top: egui::Color32::from_rgb(6, 11, 24),
            sidebar_bot: egui::Color32::from_rgb(13, 26, 51),
            nav_fill: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 36),
            nav_hover: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 18),
            on_sidebar_soft: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 205),
            shadow: egui::Shadow::NONE,
        }
    } else {
        // Tema claro azul (v2.4.0): contenido luminoso con tinte azul muy suave,
        // textos firmes y un azul brillante (2563EB) como color de marca. La barra
        // lateral es azul marino en los DOS temas (look de app moderna).
        Palette {
            bg: egui::Color32::from_rgb(241, 245, 252),
            card: egui::Color32::from_rgb(255, 255, 255),
            card2: egui::Color32::from_rgb(235, 241, 250),
            line: egui::Color32::from_rgb(219, 228, 242),
            text: egui::Color32::from_rgb(17, 26, 44),
            soft: egui::Color32::from_rgb(69, 81, 107),
            faint: egui::Color32::from_rgb(124, 137, 161),
            accent: egui::Color32::from_rgb(37, 99, 235),
            accent_hi: egui::Color32::from_rgb(96, 165, 250),
            accent2: egui::Color32::from_rgb(15, 168, 152),
            accent_soft: egui::Color32::from_rgba_unmultiplied(37, 99, 235, 32),
            danger: egui::Color32::from_rgb(220, 38, 38),
            ok_text: egui::Color32::from_rgb(11, 141, 132),
            field_bg: egui::Color32::from_rgb(237, 243, 251),
            field_line: egui::Color32::from_rgb(168, 187, 218),
            sidebar_top: egui::Color32::from_rgb(10, 17, 36),
            sidebar_bot: egui::Color32::from_rgb(20, 39, 81),
            nav_fill: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 34),
            nav_hover: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 16),
            on_sidebar_soft: egui::Color32::from_rgba_unmultiplied(255, 255, 255, 205),
            shadow: egui::Shadow {
                offset: egui::vec2(0.0, 2.0),
                blur: 14.0,
                spread: 0.0,
                color: egui::Color32::from_rgba_unmultiplied(30, 64, 160, 26),
            },
        }
    }
}

/* ------------------------- Estado de la UI ------------------------- */

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Estado,
    Ajustes,
    Proxies,
    Historial,
    Informe,
    Acerca,
}

impl Page {
    pub fn title(&self) -> &'static str {
        match self {
            Page::Estado => crate::i18n::tr("Estado"),
            Page::Ajustes => crate::i18n::tr("Ajustes del proxy"),
            Page::Proxies => crate::i18n::tr("Mis proxies"),
            Page::Historial => crate::i18n::tr("Historial"),
            Page::Informe => crate::i18n::tr("Informe y diagnostico"),
            Page::Acerca => crate::i18n::tr("Acerca de ProxySend"),
        }
    }
    pub fn from_str(s: &str) -> Page {
        match s {
            "ajustes" => Page::Ajustes,
            "proxies" => Page::Proxies,
            "historial" => Page::Historial,
            "informe" => Page::Informe,
            "acerca" => Page::Acerca,
            _ => Page::Estado,
        }
    }
}

#[derive(Clone, Default)]
pub struct FormState {
    pub protocol: String,
    pub same_for_all: bool,
    pub host: String,
    pub port: String,
    pub http_host: String,
    pub http_port: String,
    pub https_host: String,
    pub https_port: String,
    pub auth: bool,
    pub username: String,
    pub password: String,
    pub ignore_text: String,
}

impl FormState {
    pub fn new() -> Self {
        Self {
            protocol: "http".to_string(),
            same_for_all: true,
            ignore_text: "localhost\n127.0.0.0/8\n::1".to_string(),
            ..Default::default()
        }
    }

    pub fn build_config(&self) -> ProxyConfig {
        ProxyConfig {
            protocol: self.protocol.clone(),
            same_for_all: if self.protocol == "socks5" { true } else { self.same_for_all },
            host: self.host.trim().to_string(),
            port: self.port.trim().parse().unwrap_or(0),
            http: crate::store::Endpoint {
                host: self.http_host.trim().to_string(),
                port: self.http_port.trim().parse().unwrap_or(0),
            },
            https: crate::store::Endpoint {
                host: self.https_host.trim().to_string(),
                port: self.https_port.trim().parse().unwrap_or(0),
            },
            auth: self.auth,
            username: self.username.clone(),
            password: self.password.clone(),
            ignore_hosts: core::parse_ignore_hosts(&self.ignore_text),
        }
    }

    pub fn fill_from_config(&mut self, c: &ProxyConfig) {
        self.protocol = if core::PROTOCOLS.contains(&c.protocol.as_str()) {
            c.protocol.clone()
        } else {
            "http".to_string()
        };
        self.same_for_all = c.same_for_all;
        self.host = c.host.clone();
        self.port = if c.port > 0 { c.port.to_string() } else { String::new() };
        self.http_host = c.http.host.clone();
        self.http_port = if c.http.port > 0 { c.http.port.to_string() } else { String::new() };
        self.https_host = c.https.host.clone();
        self.https_port = if c.https.port > 0 { c.https.port.to_string() } else { String::new() };
        self.auth = c.auth;
        self.username = c.username.clone();
        self.password = c.password.clone();
        self.ignore_text = if c.ignore_hosts.is_empty() {
            "localhost\n127.0.0.0/8\n::1".to_string()
        } else {
            c.ignore_hosts.join("\n")
        };
    }
}

#[derive(Clone)]
pub struct StatusScope {
    pub enabled: bool,
    pub supported: bool,
    pub active: bool,
    pub value: Option<String>,
    pub detail: String,
}

#[derive(Clone)]
pub struct StatusSnapshot {
    pub active: bool,
    pub config: Option<ProxyConfig>,
    pub active_proxy_name: Option<String>,
    pub desktop_name: String,
    pub desktop: StatusScope,
    pub system: StatusScope,
    pub terminal: StatusScope,
}

pub fn compute_status(settings: &Settings) -> StatusSnapshot {
    let (supported, backend, detected, active, value, detail) = sysops::desktop_status();
    let desktop = StatusScope { enabled: settings.scopes.desktop, supported, active, value, detail };
    let (s_supported, s_active, s_value) = sysops::env_status();
    let system = StatusScope {
        enabled: settings.scopes.system,
        supported: s_supported,
        active: s_active,
        value: s_value.clone(),
        detail: s_value.unwrap_or_default(),
    };
    let (t_supported, t_active, t_value) = sysops::shell_status();
    let terminal = StatusScope {
        enabled: settings.scopes.terminal,
        supported: t_supported,
        active: t_active,
        value: t_value.clone(),
        detail: t_value.unwrap_or_default(),
    };
    let active_proxy_name = settings
        .active_proxy_id
        .as_ref()
        .and_then(|id| settings.proxies.iter().find(|p| &p.id == id))
        .map(|p| p.name.clone());
    StatusSnapshot {
        active: settings.active,
        config: settings.config.clone(),
        active_proxy_name,
        desktop_name: sysops::desktop_name(detected, backend).to_string(),
        desktop,
        system,
        terminal,
    }
}

pub enum ToastKind {
    Success,
    Error,
    Neutral,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub start: Instant,
}

#[derive(Clone, Copy, PartialEq)]
pub enum EventFilter {
    Todos,
    Problemas,
}

#[derive(Clone)]
pub enum Modal {
    SaveProxy { title: String, hint: String, show_save_as_new: bool },
    DeleteProxy { id: String, name: String },
}

pub enum UiMsg {
    OpDone {
        ok: bool,
        toast_ok: String,
        failures: Vec<String>,
        reload_settings: bool,
        refresh_proxies: bool,
        go_estado: bool,
    },
    TestDone(TestResult),
    Imported(Result<ProxyConfig, String>),
    Exported(Result<String, String>),
    ReportDone(Result<String, String>),
    LibraryExported(Result<String, String>),
    LibraryImported(Result<String, String>),
    VerifyDone(bool, String),
}

// el resultado de prueba es el mismo tipo que produce el motor de red
pub use crate::nettest::TestResult;

pub struct ProxySendApp {
    pub settings: Settings,
    pub page: Page,
    pub dark: bool,
    pub form: FormState,
    pub editing_proxy_id: Option<String>,
    pub proxies: Vec<SavedProxy>,
    pub status: Option<StatusSnapshot>,
    pub test: Option<TestResult>,
    pub test_busy: bool,
    pub test_proxy_name: Option<String>,
    pub busy: Option<String>,
    pub busy_since: Instant,
    pub modal_focus_done: bool,
    pub toast: Option<Toast>,
    pub modal: Option<Modal>,
    pub modal_name: String,
    pub save_as_new: bool,
    pub history_cache: Vec<HistoryEntry>,
    pub history_loaded: bool,
    pub shared: Arc<TrayShared>,
    pub tx: Sender<UiMsg>,
    pub rx: Receiver<UiMsg>,
    pub last_refresh: Instant,
    pub initialized: bool,
    pub ctx_handle: Option<egui::Context>,
    // informe y diagnostico
    pub events_cache: Vec<crate::eventlog::Event>,
    pub events_loaded: bool,
    pub events_filter: EventFilter,
    // reconexion automatica
    pub reconnect_state: crate::watcher::ReconnectState,
}

impl ProxySendApp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        dark: bool,
        page: String,
        shared: Arc<TrayShared>,
        tx: Sender<UiMsg>,
        rx: Receiver<UiMsg>,
        import_cfg: Option<Result<ProxyConfig, String>>,
    ) -> Self {
        apply_theme(&cc.egui_ctx, dark);
        // publicar el contexto para el hilo puente de la bandeja (v2.3.2:
        // reemplazable, la ventana nueva de la reapertura trae contexto nuevo)
        set_bridge_ctx(cc.egui_ctx.clone());
        let mut app = Self {
            settings,
            page: Page::from_str(&page),
            dark,
            form: FormState::new(),
            editing_proxy_id: None,
            proxies: Vec::new(),
            status: None,
            test: None,
            test_busy: false,
            test_proxy_name: None,
            busy: None,
            busy_since: Instant::now(),
            modal_focus_done: false,
            toast: None,
            modal: None,
            modal_name: String::new(),
            save_as_new: false,
            history_cache: Vec::new(),
            history_loaded: false,
            shared,
            tx,
            rx,
            last_refresh: Instant::now(), // refresh_status() lo re-fija en el primer frame
            // (la resta de antes, now() - 10 s, podia hacer panic si la app
            // arrancaba en los primeros 10 s tras encender el equipo)
            initialized: false,
            ctx_handle: None,
            events_cache: Vec::new(),
            events_loaded: false,
            events_filter: EventFilter::Todos,
            reconnect_state: crate::watcher::read_state(),
        };
        if let Some(cfg) = app.settings.config.clone() {
            app.form.fill_from_config(&cfg);
        }
        app.proxies = app.settings.proxies.clone();
        if let Some(Ok(cfg)) = import_cfg {
            app.form.fill_from_config(&cfg);
            app.set_toast(crate::i18n::tr("Configuracion importada. Revisala y guarda."), ToastKind::Success);
        }
        app
    }

    pub fn pal(&self) -> Palette {
        palette(self.dark)
    }

    pub fn refresh_status(&mut self) {
        self.settings = store::load_settings();
        self.status = Some(compute_status(&self.settings));
        self.proxies = self.settings.proxies.clone();
        self.reconnect_state = crate::watcher::read_state();
        if self.events_loaded {
            self.events_cache = crate::eventlog::read_all();
        }
        self.last_refresh = Instant::now();
        // v2.3.1: el icono y el menu de la bandeja reflejan el estado del
        // sistema justo despues de cada operacion (activar, desactivar,
        // restablecer, activar desde la biblioteca). Sin esto el icono
        // quedaba con el tinte viejo hasta que la bandeja decidia refrescar.
        crate::tray::notify_state_changed();
    }

    pub fn set_toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast { text: text.into(), kind, start: Instant::now() });
    }

    /// Marca una operacion como en curso (indicador con reloj de seguridad:
    /// si el trabajo se olvida, el indicador se quita solo).
    pub fn set_busy(&mut self, text: impl Into<String>) {
        self.busy = Some(text.into());
        self.busy_since = Instant::now();
    }

    pub fn spawn_job(&self, f: impl FnOnce() -> UiMsg + Send + 'static) {
        let tx = self.tx.clone();
        let ctx = self.ctx_handle.clone();
        std::thread::spawn(move || {
            let msg = f();
            let _ = tx.send(msg);
            if let Some(c) = ctx {
                c.request_repaint();
            }
        });
    }

    pub fn handle_msg(&mut self, msg: UiMsg) {
        match msg {
            UiMsg::OpDone { ok, toast_ok, failures, reload_settings, refresh_proxies, go_estado } => {
                self.busy = None;
                if ok {
                    // con mensaje vacio no se muestra pill vacia (por ejemplo al mover un nivel)
                    if !toast_ok.is_empty() {
                        self.set_toast(toast_ok, ToastKind::Success);
                    }
                } else if failures.is_empty() {
                    self.set_toast(crate::i18n::tr("No se pudo completar la operacion."), ToastKind::Error);
                } else {
                    self.set_toast(failures.join(" | "), ToastKind::Error);
                }
                if reload_settings {
                    self.refresh_status();
                }
                if refresh_proxies {
                    self.proxies = store::load_settings().proxies.clone();
                }
                if go_estado {
                    self.page = Page::Estado;
                }
            }
            UiMsg::TestDone(r) => {
                self.test_busy = false;
                self.busy = None;
                if let Some(name) = self.test_proxy_name.clone() {
                    if r.ok {
                        self.set_toast(
                            crate::i18n::tf(
                                "\"{0}\" funciona. IP de salida: {1} - {2} s",
                                &[&name, &r.ip, &format!("{:.2}", r.seconds)],
                            ),
                            ToastKind::Success,
                        );
                    } else {
                        let detail = if r.detail.is_empty() {
                            crate::i18n::tr("sin conexion a traves del proxy.").to_string()
                        } else {
                            r.detail.clone()
                        };
                        self.set_toast(crate::i18n::tf("\"{0}\" fallo: {1}", &[&name, &detail]), ToastKind::Error);
                    }
                    self.test_proxy_name = None;
                    self.test = None;
                } else {
                    self.test = Some(r);
                }
            }
            UiMsg::Imported(Ok(cfg)) => {
                self.busy = None;
                self.form.fill_from_config(&cfg);
                self.editing_proxy_id = None;
                self.set_toast(crate::i18n::tr("Configuracion importada. Revisala y guarda."), ToastKind::Success);
            }
            UiMsg::Imported(Err(e)) => {
                self.busy = None;
                self.set_toast(e, ToastKind::Error);
            }
            UiMsg::Exported(Ok(path)) => {
                self.busy = None;
                store::log_history("exportar", &path);
                self.set_toast(crate::i18n::tf("Configuracion exportada a {0}", &[&path]), ToastKind::Success);
            }
            UiMsg::Exported(Err(e)) => {
                self.busy = None;
                self.set_toast(e, ToastKind::Error);
            }
            UiMsg::ReportDone(Ok(path)) => {
                self.busy = None;
                store::log_history("informe", &path);
                crate::eventlog::info("informe", crate::i18n::tf("Informe diagnostico generado: {0}", &[&path]));
                self.set_toast(crate::i18n::tf("Informe completo generado en {0}", &[&path]), ToastKind::Success);
                self.events_cache = crate::eventlog::read_all();
            }
            UiMsg::ReportDone(Err(e)) => {
                self.busy = None;
                self.set_toast(e, ToastKind::Error);
            }
            UiMsg::LibraryExported(Ok(path)) => {
                self.busy = None;
                store::log_history("exportar-biblioteca", &path);
                crate::eventlog::info("biblioteca", crate::i18n::tf("Biblioteca completa exportada: {0}", &[&path]));
                self.set_toast(crate::i18n::tf("Biblioteca completa exportada a {0}", &[&path]), ToastKind::Success);
            }
            UiMsg::LibraryExported(Err(e)) => {
                self.busy = None;
                self.set_toast(e, ToastKind::Error);
            }
            UiMsg::LibraryImported(Ok(resumen)) => {
                self.busy = None;
                self.set_toast(resumen, ToastKind::Success);
                self.refresh_status();
                // v2.2.4: los ajustes importados pueden activar/desactivar la
                // reconexion automatica: sincronizar el temporizador con ellos
                crate::watcher::sync_timer();
                // v2.3.0: la biblioteca importada puede traer otro idioma:
                // sincronizar el idioma global con el ajuste restaurado
                crate::i18n::sync_from_settings();
                if let Some(cfg) = self.settings.config.clone() {
                    self.form.fill_from_config(&cfg);
                }
                self.events_cache = crate::eventlog::read_all();
            }
            UiMsg::LibraryImported(Err(e)) => {
                self.busy = None;
                self.set_toast(e, ToastKind::Error);
            }
            UiMsg::VerifyDone(ok, detalle) => {
                self.busy = None;
                self.reconnect_state = crate::watcher::read_state();
                // la verificacion deja constancia en el registro: mostrarla al momento
                self.events_cache = crate::eventlog::read_all();
                if ok {
                    self.set_toast(crate::i18n::tf("El proxy responde: {0}", &[&detalle]), ToastKind::Success);
                } else {
                    self.set_toast(crate::i18n::tf("Verificacion: {0}", &[&detalle]), ToastKind::Error);
                }
            }
        }
    }
}

/* ------------------------- Tema ------------------------- */

pub fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style()).clone();
    let p = palette(dark);
    style.visuals.dark_mode = dark;
    style.visuals.override_text_color = Some(p.text);
    style.visuals.window_fill = p.card;
    style.visuals.panel_fill = p.bg;
    style.visuals.extreme_bg_color = p.field_bg;
    style.visuals.faint_bg_color = p.card2;
    // barras de escritura siempre con borde visible (antes no se veian en tema claro)
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, p.field_line);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.3_f32, p.accent);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.5_f32, p.accent);
    // mismas esquinas redondeadas en las barras de escritura que en el resto
    // de la interfaz (el marco de TextEdit sale de estos valores)
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(9.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(9.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(9.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(9.0);
    style.visuals.widgets.noninteractive.bg_fill = p.card;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, p.text);
    style.visuals.widgets.inactive.bg_fill = p.card2;
    style.visuals.widgets.inactive.weak_bg_fill = p.card2;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, p.text);
    style.visuals.widgets.hovered.weak_bg_fill = p.accent_soft;
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, p.text);
    style.visuals.widgets.active.weak_bg_fill = p.accent_soft;
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, p.text);
    style.visuals.selection.bg_fill = p.accent;
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(15.0, 8.0);
    style.spacing.menu_margin = egui::Margin::same(8.0);
    style.visuals.window_rounding = egui::Rounding::same(16.0);
    style.visuals.menu_rounding = egui::Rounding::same(12.0);
    // tamaños de texto globales: cuerpo claro y titulos con peso propio
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(11.5));
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(13.5));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(13.5));
    style.text_styles.insert(egui::TextStyle::Heading, egui::FontId::proportional(19.0));
    style.text_styles.insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
    ctx.set_style(style);
}

/* ------------------------- eframe::App ------------------------- */

impl eframe::App for ProxySendApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ctx_handle = Some(ctx.clone());

        if !self.initialized {
            self.initialized = true;
            self.refresh_status();
        }

        // banderas de la bandeja del sistema (v2.3.4: decision unificada con
        // el hilo puente de main.rs — Raise o Reopen segun el servidor
        // grafico; en Wayland mostrar una ventana MINIMIZADA = reabrirla en
        // primer plano porque el protocolo no permite des-minimizar)
        match crate::tray::take_show_action(&self.shared) {
            crate::tray::ShowAction::None => {}
            crate::tray::ShowAction::Raise => {
                // v2.3.4: ademas de Visible(true) se envia Minimized(false):
                // en X11 la ventana minimizada (iconificada) sigue "visible"
                // para winit y sin Minimized(false) el comando Visible no
                // hacia nada — por eso "Mostrar ProxySend" parecia no hacer
                // nada despues de minimizar la ventana.
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                // constancia honesta en el registro: la ventana volvio a verse
                crate::eventlog::info("bandeja", crate::i18n::tr("Ventana mostrada en pantalla"));
            }
            crate::tray::ShowAction::Reopen => {
                // Wayland: cerrar esta ventana; main() abre una nueva en
                // primer plano cuando run_native devuelva el control.
                self.shared.hidden.store(false, Ordering::SeqCst);
                self.shared.restore.store(true, Ordering::SeqCst);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        // v2.3.4: estado REAL de la ventana en pantalla. Si esta minimizada
        // (por el usuario, con el boton de la barra de titulo) se anota en la
        // bandeja para que "Mostrar ProxySend" sepa que toca des-minimizar
        // (X11) o reabrir (Wayland). Ya NO se oculta nunca la ventana por
        // decision de la app: las opciones de bandeja al cerrar o minimizar
        // se retiraron en esta version.
        let minimized_now = ctx.input(|i| i.viewport().minimized == Some(true));
        self.shared.hidden.store(minimized_now, Ordering::SeqCst);
        if self.shared.dirty.swap(false, Ordering::SeqCst) {
            self.refresh_status();
        }
        // aviso pendiente generado por la bandeja: resultado de Activar o
        // Desactivar proxy desde el menu (antes la accion no decia nada)
        if let Some(n) = self.shared.take_note() {
            self.set_toast(n.msg, if n.ok { ToastKind::Success } else { ToastKind::Error });
            self.refresh_status();
        }

        // mensajes de los trabajos en segundo plano
        while let Ok(msg) = self.rx.try_recv() {
            self.handle_msg(msg);
        }

        // refresco periodico del estado (como el setInterval de la v1.x)
        if self.last_refresh.elapsed() > Duration::from_secs(8) {
            self.refresh_status();
        }

        // reloj de seguridad: si un trabajo se queda colgado mas de 3 minutos,
        // el indicador de "en curso" se quita solo (pkexec puede tardar si la
        // ventana de contraseña esta abierta, por eso el margen es amplio)
        if self.busy.is_some() && self.busy_since.elapsed() > Duration::from_secs(180) {
            self.busy = None;
        }

        self.render_sidebar(ctx);
        self.render_content(ctx);
        self.render_toast(ctx);
        self.render_modal(ctx);

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
