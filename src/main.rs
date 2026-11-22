//! ProxySend 2.4.0 - punto de entrada.
//! Reescrito en Rust: binario pequeno, sin Electron ni dependencias de ejecucion.

mod core;
mod eventlog;
mod i18n;
mod libexport;
mod nettest;
mod ops;
mod report;
mod store;
mod sysops;
#[cfg(test)]
mod tests;
mod tray;
mod ui;
mod watcher;

use std::sync::atomic::Ordering;

pub const APP_VERSION: &str = "2.4.0";

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned()
}

fn print_help() {
    // v2.3.0: la ayuda de consola sale en el idioma guardado (espanol por defecto)
    println!(
        "{}",
        crate::i18n::tf(
            "ProxySend {0} - gestor de proxy para Linux\n\nUso:\n  proxysend                 abre la aplicacion\n  proxysend --autostart     aplica el proxy guardado al iniciar sesion (sin ventana)\n  proxysend --aplicar       aplica el proxy guardado y sale\n  proxysend --quitar        quita el proxy configurado en todos los niveles y sale\n  proxysend --informe       genera el informe diagnostico completo y sale\n  proxysend --verificar     verifica el proxy activo ahora mismo (reconexion) y sale\n  proxysend --pagina X      abre en una pagina (estado|ajustes|proxies|historial|informe|acerca)\n  proxysend --oscuro        inicia con el tema oscuro (no se guarda)\n  proxysend --importar F    importa una configuracion JSON al formulario\n  proxysend --probar URL    prueba una conexion a traves de un proxy y sale (URL como\n                            http://host:puerto, socks5://host:puerto o \"directo\" para\n                            probar sin proxy)\n  proxysend --version       muestra la version\n",
            &[APP_VERSION]
        )
    );
}

/// Bloqueo de instancia unica: si ya hay una copia corriendo, le pide mostrarse y sale.
fn acquire_instance_lock() -> Option<std::fs::File> {
    let dir = store::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("instance.lock");
    let file = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(path).ok()?;
    let locked = file.try_lock().is_ok();
    if !locked {
        // pedir a la instancia activa que muestre su ventana
        let _ = std::fs::write(store::config_dir().join("show.request"), "1");
        return None;
    }
    let _ = write_lock_pid(&file);
    Some(file)
}

fn write_lock_pid(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.write_all_at(format!("{}\n", std::process::id()).as_bytes(), 0)
}

fn load_window_icon() -> Option<egui::IconData> {
    let png = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(png).ok()?;
    let rgba = img.to_rgba8();
    Some(egui::IconData {
        width: rgba.width(),
        height: rgba.height(),
        rgba: rgba.into_raw(),
    })
}

fn main() {
    eventlog::install_panic_hook();
    // v2.3.0: idioma global (espanol por defecto) desde el ajuste guardado;
    // vale para la consola, la bandeja, la UI y todos los mensajes nuevos.
    i18n::sync_from_settings();
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("ProxySend {}", APP_VERSION);
        return;
    }
    if args.iter().any(|a| a == "--ayuda" || a == "--help" || a == "-h") {
        print_help();
        return;
    }

    // Prueba de conexion por CLI: util para diagnosticar sin abrir la ventana.
    if let Some(url) = arg_value(&args, "--probar") {
        let r = if url == "directo" {
            nettest::test_with_ureq(None)
        } else {
            nettest::test_with_ureq(Some(&url))
        };
        match r {
            Ok(t) => {
                if t.ok {
                    println!("{}", i18n::tf("OK  IP {0}  HTTP {1}  {2} s", &[&t.ip, &t.code, &format!("{:.2}", t.seconds)]));
                } else if t.detail.is_empty() {
                    println!("{}", i18n::tf("FALLO  HTTP {0}  respuesta inesperada", &[&t.code]));
                } else {
                    println!("{}", i18n::tf("FALLO  {0}", &[&t.detail]));
                }
            }
            Err(e) => println!("ERROR  {}", e),
        }
        return;
    }

    // Modo autoarranque: aplica el proxy guardado, se queda en la bandeja si existe.
    if args.iter().any(|a| a == "--autostart") {
        ops::log_app_start(i18n::tr("autoarranque"));
        ops::apply_on_autostart();
        // el temporizador SOLO se arranca si la opcion de reconexion automatica
        // esta activada en los ajustes guardados (v2.2.4)
        watcher::sync_timer();
        let shared = tray::TrayShared::new();
        if tray::spawn_tray(shared.clone()) {
            // permanece vivo para que la bandeja siga disponible. "Salir" de la
            // bandeja termina el proceso al instante (v2.3.4).
            loop {
                // v2.3.1: "Mostrar ProxySend" de la bandeja o una segunda
                // instancia abren la ventana. Antes, con la app abierta desde
                // el autoarranque, la bandeja vieja dejaba de atender Mostrar
                // y Salir mientras la GUI tenia otra bandeja propia (2 iconos).
                if shared.show.swap(false, Ordering::SeqCst) || tray::consume_show_request() {
                    tray::shutdown_tray();
                    open_gui(vec!["proxysend".to_string()]);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        }
        // sin bandeja disponible: cerramos despues de aplicar (igual que la v1.x)
        return;
    }

    // Modo aplicar sin ventana: aplica el proxy guardado y sale.
    if args.iter().any(|a| a == "--aplicar") {
        ops::apply_on_autostart();
        return;
    }

    // Modo quitar sin ventana: limpia el proxy configurado en todos los niveles y sale.
    if args.iter().any(|a| a == "--quitar") {
        let mut s = store::load_settings();
        let r = ops::remove_proxy_everywhere(&mut s);
        for (nivel, res) in &r.results {
            println!("{}: {}", nivel, if res.ok { "ok".to_string() } else { i18n::tf("fallo - {0}", &[&res.detail]) });
        }
        store::log_history("desactivar", i18n::tr("Proxy desactivado con proxysend --quitar"));
        return;
    }

    // Informe diagnostico completo sin abrir la ventana (para soporte/revision).
    if args.iter().any(|a| a == "--informe") {
        let s = store::load_settings();
        match report::write_report(&s) {
            Ok(path) => {
                println!("{}", i18n::tf("Informe generado: {0}", &[&path]));
                eventlog::info("informe", i18n::tf("Informe generado desde CLI: {0}", &[&path]));
            }
            Err(e) => println!("ERROR: {}", e),
        }
        return;
    }

    // Verificacion de reconexion inmediata (la misma que hace el temporizador).
    if args.iter().any(|a| a == "--verificar") {
        let (ok, detalle) = watcher::check_once();
        println!("{}  {}", if ok { "OK" } else { "FALLO" }, detalle);
        return;
    }

    open_gui(args);
}

fn open_gui(args: Vec<String>) {
    // instancia unica (se conserva viva durante todo el proceso)
    let _lock = match acquire_instance_lock() {
        Some(f) => f,
        None => return,
    };

    ops::log_app_start(i18n::tr("ventana"));

    let page = arg_value(&args, "--pagina");
    let forced_dark = args.iter().any(|a| a == "--oscuro");
    let import_file = arg_value(&args, "--importar");

    let settings = store::load_settings();
    let dark = forced_dark || settings.theme.as_deref() == Some("dark");

    let shared = tray::TrayShared::new();
    let tray_ready = tray::spawn_tray(shared.clone());
    eprintln!("[proxysend] bandeja lista={}", tray_ready);

    // temporizador de reconexion automatica: SOLO se arranca si la opcion
    // esta activada en los ajustes guardados (v2.2.4: antes arrancaba siempre
    // al abrir la app aunque la opcion estuviera apagada)
    watcher::sync_timer();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([880.0, 600.0])
            .with_title("ProxySend")
            .with_app_id("proxysend".to_string())
            .with_icon(load_window_icon().expect("icono embebido")),
        ..Default::default()
    };

    let mut import_cfg: Option<Result<crate::store::ProxyConfig, String>> = import_file.map(|path| {
        std::fs::read_to_string(&path)
            .map_err(|e| i18n::tf("No se pudo leer {0}: {1}", &[&path, &e.to_string()]))
            .and_then(|text| {
                let data: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| i18n::tf("Archivo JSON invalido: {0}", &[&e.to_string()]))?;
                let cfg_val = data.get("config").cloned().unwrap_or(data);
                serde_json::from_value::<crate::store::ProxyConfig>(cfg_val)
                    .map_err(|e| i18n::tf("La configuracion no es valida: {0}", &[&e.to_string()]))
            })
    });

    let page = page.unwrap_or_else(|| "estado".to_string());

    // v2.3.4: hilo puente bandeja <-> ventana. Atiende la peticion de mostrar
    // la ventana aunque el bucle de la interfaz no este pintando. "Salir" ya
    // NO pasa por aqui: el menu de la bandeja termina el proceso al instante.
    // Cada vuelta vuelve a pedir el contexto de egui porque en Wayland la
    // ventana se puede cerrar y volver a abrir (reapertura de "Mostrar
    // ProxySend" sobre una ventana minimizada): el contexto nuevo sustituye
    // al viejo.
    {
        let bshared = shared.clone();
        std::thread::spawn(move || loop {
            // Mostrar ProxySend (menu de la bandeja o clic en el icono) o
            // segunda instancia: decision unificada segun el servidor grafico.
            match tray::take_show_action(&bshared) {
                tray::ShowAction::None => {}
                tray::ShowAction::Raise => {
                    if let Some(ctx) = ui::bridge_ctx() {
                        // v2.3.4: Minimized(false) antes de Visible/Focus: en
                        // X11 una ventana minimizada sigue "visible" para winit
                        // y sin des-minimizar los otros comandos no hacian nada.
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        ctx.request_repaint();
                        // constancia honesta (si la ventana consumio la
                        // peticion antes, su propio handler ya la dejo escrita)
                        crate::eventlog::info(
                            "bandeja",
                            crate::i18n::tr("Ventana mostrada en pantalla"),
                        );
                    }
                }
                tray::ShowAction::Reopen => {
                    // Wayland: cerrar la ventana actual; el bucle de open_gui
                    // abre una nueva en primer plano al salir run_native.
                    bshared.hidden.store(false, Ordering::SeqCst);
                    bshared.restore.store(true, Ordering::SeqCst);
                    if let Some(ctx) = ui::bridge_ctx() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ctx.request_repaint();
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        });
    }

    eprintln!("[proxysend] abriendo ventana (pagina {})", page);
    // v2.3.2: bucle de ventana. En Wayland "Mostrar ProxySend" con la ventana
    // en la bandeja se hace cerrando y reabriendo (el protocolo no permite
    // ocultar/reaparecer una ventana ya creada): cada vuelta del bucle es una
    // ventana nueva en primer plano. La bandeja, el hilo puente y el bloqueo
    // de instancia sobreviven a cada vuelta.
    loop {
        let (tx, rx) = std::sync::mpsc::channel();
        let import_ahora = import_cfg.take();
        let shared_v = shared.clone();
        let page_v = page.clone();
        let settings_v = settings.clone();
        eframe::run_native(
            "ProxySend",
            options.clone(),
            Box::new(move |cc| {
                let app = ui::ProxySendApp::new(
                    cc,
                    settings_v,
                    dark,
                    page_v,
                    shared_v,
                    tx,
                    rx,
                    import_ahora,
                );
                Ok(Box::new(app))
            }),
        )
        .ok();

        // La ventana se cerro. ¿Hay que reabrirla (Wayland) o salir?
        if shared.restore.swap(false, Ordering::SeqCst) {
            shared.hidden.store(false, Ordering::SeqCst);
            crate::eventlog::info(
                "bandeja",
                crate::i18n::tr("Ventana reabierta en primer plano desde la bandeja del sistema"),
            );
            eprintln!("[proxysend] ventana reabierta (wayland)");
            continue;
        }
        break;
    }
}
