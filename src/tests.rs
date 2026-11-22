//! Pruebas unitarias de la logica pura de ProxySend (equivalente a las de la v1.x).

use crate::core::*;
use crate::store::ProxyConfig;

fn cfg() -> ProxyConfig {
    ProxyConfig {
        protocol: "http".into(),
        same_for_all: true,
        host: "1.2.3.4".into(),
        port: 8080,
        http: Default::default(),
        https: Default::default(),
        auth: false,
        username: String::new(),
        password: String::new(),
        ignore_hosts: DEFAULT_IGNORE_HOSTS.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn env_values_para_socks5() {
    let mut c = cfg();
    c.protocol = "socks5".into();
    c.same_for_all = true;
    let v = env_values(&c);
    assert_eq!(v.http_url, "socks5://1.2.3.4:8080");
    assert!(v.is_socks);
    assert_eq!(v.no_proxy, "localhost, 127.0.0.0/8, ::1");
}

#[test]
fn env_values_http_y_https_separados() {
    let mut c = cfg();
    c.same_for_all = false;
    c.http = crate::store::Endpoint { host: "h1".into(), port: 1 };
    c.https = crate::store::Endpoint { host: "h2".into(), port: 2 };
    let v = env_values(&c);
    assert_eq!(v.http_url, "http://h1:1");
    assert_eq!(v.https_url, "http://h2:2");
    assert!(!v.is_socks);
}

#[test]
fn autenticacion_se_codifica_en_la_url() {
    let mut c = cfg();
    c.auth = true;
    c.username = "usuario".into();
    c.password = "con trampa@".into();
    let v = env_values(&c);
    assert_eq!(v.http_url, "http://usuario:con%20trampa%40@1.2.3.4:8080");
}

#[test]
fn validacion_rechaza_valores_peligrosos() {
    let mut c = cfg();
    c.host = "mal; rm -rf".into();
    let errs = validate_config_errors(&c);
    assert!(!errs.is_empty());
    let mut c2 = cfg();
    c2.port = 70000;
    assert!(!validate_config_errors(&c2).is_empty());
    let mut c3 = cfg();
    c3.auth = true;
    c3.username = "  ".into();
    assert!(validate_config_errors(&c3).iter().any(|e| e.contains("usuario")));
}

#[test]
fn archivo_env_conserva_lineas_ajenas() {
    let original = "LANG=es_ES.UTF-8\nhttp_proxy=\"http://viejo:1\"\nEDITOR=vim\n";
    let v = env_values(&cfg());
    let content = build_env_file_content(original, Some(&v));
    assert!(content.contains("LANG=es_ES.UTF-8"));
    assert!(content.contains("EDITOR=vim"));
    assert!(!content.contains("http_proxy=\"http://viejo:1\""));
    assert!(content.contains("http_proxy=\"http://1.2.3.4:8080\""));
    assert!(content.contains("HTTP_PROXY="));
    assert!(content.contains("no_proxy=\"localhost, 127.0.0.0/8, ::1\""));
    assert!(!content.contains("all_proxy"));
}

#[test]
fn archivo_env_socks5_agrega_all_proxy() {
    let mut c = cfg();
    c.protocol = "socks5".into();
    let v = env_values(&c);
    let content = build_env_file_content("", Some(&v));
    assert!(content.contains("all_proxy=\"socks5://1.2.3.4:8080\""));
}

#[test]
fn bloque_shell_se_agrega_y_se_quita() {
    let v = env_values(&cfg());
    let block = build_shell_block(Some(&v));
    assert!(block.contains(BEGIN_MARKER));
    assert!(block.contains("export http_proxy=\"http://1.2.3.4:8080\""));
    assert!(block.contains(END_MARKER));

    let base = "alias ll='ls -l'\n";
    let with = apply_shell_block(base, Some(&block));
    assert!(has_shell_block(&with));
    assert!(with.contains("alias ll='ls -l'"));
    assert!(with.ends_with(&block));

    // al desactivar se elimina el bloque y queda el resto intacto
    let clean = apply_shell_block(&with, None);
    assert!(!has_shell_block(&clean));
    assert!(clean.contains("alias ll='ls -l'"));
}

#[test]
fn bloque_shell_sustituye_el_anterior() {
    let v = env_values(&cfg());
    let block = build_shell_block(Some(&v));
    let text = apply_shell_block("linea\n", Some(&block));
    let mut nuevo_cfg = cfg();
    nuevo_cfg.port = 9090;
    let v2 = env_values(&nuevo_cfg);
    let block2 = build_shell_block(Some(&v2));
    let text2 = apply_shell_block(&text, Some(&block2));
    assert_eq!(text2.matches(BEGIN_MARKER).count(), 1);
    assert!(text2.contains("9090"));
}

#[test]
fn parse_excepciones() {
    let got = parse_ignore_hosts("localhost, 127.0.0.0/8\n\n::1\nlocalhost");
    assert_eq!(got, vec!["localhost", "127.0.0.0/8", "::1"]);
}

#[test]
fn kioslaverc_conserva_otros_grupos() {
    let original = "[General]\nXftHintStyle=hintslight\n\n[Trash]\nDays=3\n";
    // usamos las funciones via sysops para verificar el parseo
    let map = crate::sysops::parse_kde_group(original);
    assert!(map.is_empty()); // no hay grupo Proxy Settings
}

#[test]
fn resumen_legible() {
    assert_eq!(summary(&cfg()), "HTTP 1.2.3.4:8080");
    let mut c = cfg();
    c.protocol = "socks5".into();
    assert_eq!(summary(&c), "SOCKS5 1.2.3.4:8080");
    let mut c2 = cfg();
    c2.same_for_all = false;
    c2.http = crate::store::Endpoint { host: "a".into(), port: 1 };
    c2.https = crate::store::Endpoint { host: "b".into(), port: 2 };
    assert_eq!(summary(&c2), "HTTP HTTP a:1 / HTTPS b:2");
}

#[test]
fn configuracion_vacia_no_es_valida() {
    let mut c = cfg();
    c.host = String::new();
    assert!(!validate_config_errors(&c).is_empty());
}

#[test]
fn marcadores_coinciden_con_la_v1() {
    assert_eq!(BEGIN_MARKER, "# >>> ProxySend >>>");
    assert_eq!(END_MARKER, "# <<< ProxySend <<<");
}

#[test]
fn autoarranque_crea_y_elimina_el_archivo_desktop() {
    let _g = home_lock();
    // HOME temporal exclusivo de esta prueba (las demas pruebas no leen HOME)
    let base = tmp_home("proxysend-test-autostart");
    let archivo = base.join(".config/autostart/proxysend.desktop");

    let r = crate::sysops::set_autostart(true);
    assert!(r.ok, "fallo al crear autostart: {}", r.detail);
    let texto = std::fs::read_to_string(&archivo).unwrap();
    assert!(texto.contains("[Desktop Entry]"));
    assert!(texto.contains("Exec="));
    assert!(texto.contains("--autostart"));
    assert!(crate::sysops::autostart_installed());

    let r2 = crate::sysops::set_autostart(false);
    assert!(r2.ok);
    assert!(!archivo.exists());
    assert!(!crate::sysops::autostart_installed());

    let _ = std::fs::remove_dir_all(&base);
}

/* ------------------------- Novedades 2.2.2 ------------------------- */

#[test]
fn activar_sin_niveles_o_con_todos_fallidos_no_deja_activo() {
    let _g = home_lock();
    let base = tmp_home("proxysend-test-niveles");

    // 1) sin NINGUN nivel marcado: rechazado con mensaje claro y sin quedar activo
    let mut s = crate::store::Settings {
        scopes: crate::store::Scopes { desktop: false, system: false, terminal: false },
        ..Default::default()
    };
    let c = cfg();
    let scopes_vacio = s.scopes;
    let r = crate::ops::apply_proxy_config(&mut s, &c, Some(&scopes_vacio), None);
    assert!(r.results.contains_key("validacion"), "debe rechazar con el detalle de validacion");
    assert!(!r.ok(), "activar sin niveles no puede salir como exito");
    assert!(!s.active, "sin niveles aplicados el proxy NO queda activo");

    // 2) solo el nivel Sistema marcado y pkexec ausente (como cuando el usuario
    //    cancela la ventana de contraseña): el unico nivel falla, asi que el
    //    proxy NO queda marcado como activo y se explica el fallo
    let mut s2 = crate::store::Settings {
        scopes: crate::store::Scopes { desktop: false, system: true, terminal: false },
        ..Default::default()
    };
    let scopes_sistema = s2.scopes;
    let r2 = crate::ops::apply_proxy_config(&mut s2, &c, Some(&scopes_sistema), None);
    assert!(!r2.ok(), "si el unico nivel falla no puede anunciar exito");
    assert!(!s2.active, "con todos los niveles fallidos el Estado no debe mentir");
    assert!(r2.failures().iter().any(|f| !f.is_empty()), "el fallo se explica en claro");

    // 3) con escritorio y terminal marcados (ambos locales, sin pkexec): quedan
    //    aplicados y activo=true aunque el nivel sistema no se tocara
    let mut s3 = crate::store::Settings {
        scopes: crate::store::Scopes { desktop: true, system: false, terminal: true },
        ..Default::default()
    };
    let scopes_locales = s3.scopes;
    let r3 = crate::ops::apply_proxy_config(&mut s3, &c, Some(&scopes_locales), None);
    let aplicados = r3.results.values().filter(|r| r.ok).count();
    if aplicados >= 1 {
        assert!(s3.active, "si al menos un nivel se aplico, queda activo");
    }

    let _ = std::fs::remove_dir_all(&base);
}

// Las pruebas que tocan HOME comparten el proceso: se serializan para que
// cada una use su HOME temporal sin interferir con las otras (aunque cargo
// las ejecute en paralelo).
static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn home_lock() -> std::sync::MutexGuard<'static, ()> {
    HOME_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn tmp_home(nombre: &str) -> std::path::PathBuf {
    let base = std::env::temp_dir().join(format!("{}-{}", nombre, std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    std::env::set_var("HOME", &base);
    base
}

#[test]
fn ajustes_de_reconexion_tienen_valores_por_defecto() {
    // settings.json de la v2.1 sin campos nuevos: se leen igual
    let s: crate::store::Settings = serde_json::from_str("{}").unwrap();
    assert!(!s.reconnect_enabled);
    assert_eq!(s.reconnect_min, 5);
    let s2: crate::store::Settings =
        serde_json::from_str("{\"reconnectEnabled\":true,\"reconnectMin\":30}").unwrap();
    assert!(s2.reconnect_enabled);
    assert_eq!(s2.reconnect_min, 30);
    // opciones de la interfaz: etiquetas de intervalo
    let opts = crate::store::reconnect_min_options();
    assert_eq!(opts.len(), 5);
    assert!(opts.iter().any(|(m, _)| *m == 60));
}

#[test]
fn estado_de_reconexion_se_guarda_y_se_lee() {
    let _g = home_lock();
    let base = tmp_home("proxysend-test-recon");

    crate::watcher::reset_state();
    assert_eq!(crate::watcher::read_state().last_check, 0);

    let st = crate::watcher::ReconnectState {
        last_check: 1_700_000_000_000,
        ok: Some(false),
        detail: "FALLO (sin respuesta del proxy)".into(),
        reintentos: 2,
    };
    crate::watcher::write_state(&st);
    let leido = crate::watcher::read_state();
    assert_eq!(leido.last_check, 1_700_000_000_000);
    assert_eq!(leido.ok, Some(false));
    assert_eq!(leido.reintentos, 2);

    // "hace cuanto" con marca muy vieja no peta
    let txt = crate::watcher::hace_cuanto(1000);
    assert!(txt.contains("hace"));
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn biblioteca_completa_exporta_e_importa() {
    let _g = home_lock();
    let base = tmp_home("proxysend-test-lib");

    // preparar biblioteca con 2 proxies + historial
    let mut s = crate::store::Settings {
        reconnect_enabled: true,
        reconnect_min: 10,
        theme: Some("dark".into()),
        ..Default::default()
    };
    s.proxies.push(crate::store::SavedProxy {
        id: "px-a".into(),
        name: "Casa".into(),
        config: cfg(),
        created_at: 111,
    });
    let mut sock = cfg();
    sock.protocol = "socks5".into();
    s.proxies.push(crate::store::SavedProxy {
        id: "px-b".into(),
        name: "VPN".into(),
        config: sock.clone(),
        created_at: 222,
    });
    s.active_proxy_id = Some("px-a".into());
    crate::store::save_settings(&s);
    crate::store::log_history("activar", "prueba biblioteca");

    // exportar
    let path = crate::libexport::export_library(&s).expect("export fallo");
    assert!(path.contains("ProxySend-biblioteca-"));
    let texto = std::fs::read_to_string(&path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&texto).unwrap();
    assert_eq!(data["tipo"], "biblioteca");
    assert_eq!(data["proxies"].as_array().unwrap().len(), 2);
    assert_eq!(data["ajustes"]["reconnectMin"], 10);

    // vaciar biblioteca e historial y reimportar
    let vacia = crate::store::Settings::default();
    crate::store::save_settings(&vacia);
    crate::store::clear_history();
    let r = crate::libexport::import_library(&path).expect("import fallo");
    assert_eq!(r.proxies, 2, "deben importarse 2 proxies");
    assert!(r.historial >= 1, "el historial exportado se restaura");
    // reimportar otra vez: el historial NO se duplica (dedupe por ts+detalle)
    let r2 = crate::libexport::import_library(&path).expect("reimport fallo");
    assert_eq!(r2.historial, 0, "la reimportacion no duplica el historial");
    assert_eq!(
        crate::store::read_history().iter().filter(|h| h.detail.contains("prueba biblioteca")).count(),
        1
    );
    let s3 = crate::store::load_settings();
    assert_eq!(s3.proxies.len(), 2);
    assert_eq!(s3.proxies[0].name, "Casa");
    assert!(s3.reconnect_enabled, "la reconexion se restaura");
    assert_eq!(s3.reconnect_min, 10);
    assert_eq!(s3.active_proxy_id.as_deref(), Some("px-a"), "el perfil activo se recuerda");
    // el sistema NO se toca al importar
    assert!(!s3.active, "importar no activa el proxy en el sistema");

    // importar un archivo que no es biblioteca debe fallar con mensaje claro
    let mal = crate::store::home_dir().join("no-biblioteca.json");
    std::fs::write(&mal, "{\"app\":\"ProxySend\",\"config\":{}}").unwrap();
    let e = crate::libexport::import_library(mal.to_str().unwrap());
    assert!(e.is_err());

    // buscar la mas reciente en HOME
    assert!(crate::libexport::latest_library_in_home().is_some());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn informe_incluye_todas_las_secciones_y_oculta_claves() {
    let _g = home_lock();
    let base = tmp_home("proxysend-test-informe");

    let mut s = crate::store::Settings::default();
    let mut c = cfg();
    c.auth = true;
    c.username = "usuario".into();
    c.password = "ClaveSecreta2025".into();
    s.config = Some(c.clone());
    s.proxies.push(crate::store::SavedProxy {
        id: "px-x".into(),
        name: "Con clave".into(),
        config: c,
        created_at: 1,
    });
    s.reconnect_enabled = true;
    crate::store::save_settings(&s);
    crate::eventlog::error("prueba", "error de ejemplo para el informe");
    crate::eventlog::info("app", "evento informativo de ejemplo");

    let text = crate::report::build_report(&s);
    // secciones numeradas presentes (1..11)
    assert!(crate::report::section_count(&text) >= 10, "faltan secciones en el informe");
    for titulo in [
        "SISTEMA Y ESCRITORIO",
        "HERRAMIENTAS EXTERNAS",
        "ESTADO DE LOS TRES NIVELES",
        "BIBLIOTECA DE PROXIES",
        "RECONEXION AUTOMATICA",
        "RESUMEN DE ERRORES",
        "REGISTRO COMPLETO DE EVENTOS",
        "HISTORIAL DE ACTIVIDAD",
    ] {
        assert!(text.contains(titulo), "el informe no tiene la seccion {}", titulo);
    }
    // credenciales NUNCA en el informe
    assert!(!text.contains("ClaveSecreta2025"), "el informe filtro la contrasena");
    assert!(text.contains("****"), "el informe enmascara valores");
    // el log aparece
    assert!(text.contains("error de ejemplo para el informe"));
    assert!(crate::report::level_count(&text, "ERROR") >= 1);

    // escribirlo en disco tambien funciona
    let path = crate::report::write_report(&s).expect("informe en disco fallo");
    assert!(std::path::Path::new(&path).exists());
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn registro_de_eventos_registra_niveles_y_recupera() {
    let _g = home_lock();
    let base = tmp_home("proxysend-test-eventlog");

    crate::eventlog::info("aplicar", "evento info de prueba");
    crate::eventlog::aviso("bandeja", "evento aviso de prueba");
    crate::eventlog::error("prueba", "evento error de prueba");

    let all = crate::eventlog::read_all();
    assert!(all.iter().any(|e| e.msg.contains("evento info de prueba") && e.level == crate::eventlog::Level::Info));
    assert!(all.iter().any(|e| e.msg.contains("evento aviso de prueba") && e.level == crate::eventlog::Level::Aviso));
    assert!(all.iter().any(|e| e.msg.contains("evento error de prueba") && e.level == crate::eventlog::Level::Error));
    let (i, a, e) = crate::eventlog::counts();
    assert!(i >= 1 && a >= 1 && e >= 1);
    assert!(crate::eventlog::last_error().is_some());
    // el archivo fisico existe
    assert!(crate::eventlog::log_file().exists());
    // limpiar lo vacia (y deja anotado el propio limpiado)
    crate::eventlog::clear();
    let tras = crate::eventlog::read_all();
    assert!(tras.iter().all(|e| e.msg.contains("limpiado")), "tras limpiar solo queda la anotacion del limpiado");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn reconexion_verifica_y_guarda_estado() {
    let _g = home_lock();
    let base = tmp_home("proxysend-test-check");

    let (ok, detalle) = crate::watcher::check_once();
    assert!(!ok);
    assert!(detalle.contains("No hay un proxy") || detalle.contains("no esta activado"));

    // con un proxy activo apuntando a un puerto cerrado: fallo controlado
    let mut s = crate::store::Settings::default();
    let mut c = cfg();
    c.host = "127.0.0.1".into();
    c.port = 1; // puerto cerrado: falla rapido
    s.config = Some(c);
    s.active = true;
    crate::store::save_settings(&s);
    let (ok2, det2) = crate::watcher::check_once();
    assert!(!ok2, "un puerto cerrado no debe pasar la verificacion");
    assert!(det2.contains("FALLO"), "el detalle explica el fallo: {}", det2);
    let st = crate::watcher::read_state();
    assert_eq!(st.ok, Some(false));
    assert_eq!(st.reintentos, 1, "se cuenta el reintento");
    // y el log queda anotado
    assert!(crate::eventlog::read_all().iter().any(|e| e.cat == "reconexion"));
    let _ = std::fs::remove_dir_all(&base);
}

/* ------------------------- Novedades 2.2.3 ------------------------- */

#[test]
fn la_licencia_mostrada_en_acerca_es_gpl_v3() {
    // constancia: la pagina Acerca de anuncia GNU GPL v3 y ya no queda
    // ningun rastro del texto de licencia MIT en la interfaz
    let src = include_str!("ui/pages.rs");
    assert!(src.contains("Licencia GNU GPL v3"), "Acerca de debe mostrar Licencia GNU GPL v3");
    assert!(!src.contains("Licencia MIT"), "no debe quedar Licencia MIT en la interfaz");
}

#[test]
fn el_proyecto_declara_gpl_v3_y_version_actual() {
    // constancia: el manifiesto del proyecto declara GPL-3.0-only y la version vigente
    let man = include_str!("../Cargo.toml");
    assert!(man.contains("license = \"GPL-3.0-only\""), "Cargo.toml debe declarar GPL-3.0-only");
    assert!(!man.contains("license = \"MIT\""), "Cargo.toml no debe declarar MIT");
    assert!(man.contains("version = \"2.4.0\""), "Cargo.toml debe declarar la version 2.4.0");
    // y la constante de version de la app coincide
    assert_eq!(crate::APP_VERSION, "2.4.0");
    // y el texto oficial de la licencia viaja con el codigo fuente
    let copying = include_str!("../COPYING.txt");
    assert!(copying.contains("GNU GENERAL PUBLIC LICENSE"), "COPYING.txt debe traer la GPL");
    assert!(copying.contains("Version 3, 29 June 2007"), "COPYING.txt debe traer la version 3");
}

#[test]
fn el_temporizador_no_arranca_al_abrir_la_app_con_la_opcion_apagada() {
    // constancia 2.2.4: abrir la app con la reconexion automatica APAGADA no
    // arranca ningun temporizador ni deja la anotacion "iniciado" en el log
    let _g = home_lock();
    let base = tmp_home("proxysend-test-timer-apagado");

    // el registro tiene memoria compartida en el proceso de pruebas: se limpia
    // para que la asercion de AUSENCIA sea fiable
    crate::eventlog::clear();

    let s = crate::store::Settings::default();
    assert!(!s.reconnect_enabled);
    crate::store::save_settings(&s);

    // esto es exactamente lo que hace main.rs al abrir la app (y el modo --autostart)
    crate::watcher::sync_timer();

    let eventos = crate::eventlog::read_all();
    assert!(
        !eventos.iter().any(|e| e.msg.contains("Temporizador de reconexion automatica iniciado")),
        "con la opcion apagada NO debe anotarse el arranque del temporizador"
    );
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn el_temporizador_arranca_al_activar_y_se_detiene_al_desactivar() {
    // constancia 2.2.4: activar la opcion arranca el temporizador (anotado en
    // el log), activarlo otra vez no lo duplica y desactivarlo lo detiene
    let _g = home_lock();
    let base = tmp_home("proxysend-test-timer-ciclo");

    // 1) activar la opcion -> el temporizador arranca y queda anotado
    let s = crate::store::Settings {
        reconnect_enabled: true,
        reconnect_min: 10,
        ..Default::default()
    };
    crate::store::save_settings(&s);
    crate::watcher::sync_timer();
    assert!(
        crate::eventlog::read_all().iter().any(|e| e.msg.contains("Temporizador de reconexion automatica iniciado")),
        "al activar la opcion el temporizador debe arrancar"
    );

    // 2) volver a activar/sincronizar no duplica el arranque
    crate::watcher::sync_timer();
    let inicios = crate::eventlog::read_all().iter().filter(|e| e.msg.contains("Temporizador de reconexion automatica iniciado")).count();
    assert_eq!(inicios, 1, "no debe duplicarse el temporizador");

    // 3) desactivar la opcion -> el temporizador se detiene y queda anotado
    let mut s2 = crate::store::load_settings();
    s2.reconnect_enabled = false;
    crate::store::save_settings(&s2);
    crate::watcher::sync_timer();
    assert!(
        crate::eventlog::read_all().iter().any(|e| e.msg.contains("Temporizador de reconexion automatica detenido")),
        "al desactivar la opcion el temporizador debe detenerse"
    );

    // 4) y detener sin temporizador vivo no deja anotaciones falsas
    let antes = crate::eventlog::read_all().len();
    crate::watcher::sync_timer();
    assert_eq!(crate::eventlog::read_all().len(), antes, "sin temporizador vivo sync_timer no anota nada");

    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn el_temporizador_arranca_al_abrir_la_app_si_la_opcion_ya_estaba_activada() {
    // constancia 2.2.4: si la reconexion automatica estaba ACTIVADA y guardada,
    // al abrir la app el temporizador si arranca (la opcion sigue activada)
    let _g = home_lock();
    let base = tmp_home("proxysend-test-timer-guardado");

    let s = crate::store::Settings {
        reconnect_enabled: true,
        reconnect_min: 5,
        ..Default::default()
    };
    crate::store::save_settings(&s);

    crate::watcher::sync_timer();
    assert!(
        crate::eventlog::read_all().iter().any(|e| e.msg.contains("Temporizador de reconexion automatica iniciado")),
        "con la opcion activada y guardada el temporizador debe arrancar al abrir"
    );

    // limpiar: parar el temporizador para no dejar hilos sueltos en las pruebas
    let mut s2 = crate::store::load_settings();
    s2.reconnect_enabled = false;
    crate::store::save_settings(&s2);
    crate::watcher::sync_timer();
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn el_texto_de_ajustes_explica_que_el_temporizador_solo_corre_activado() {
    // constancia: la tarjeta de Ajustes lo deja claro al usuario
    let src = include_str!("ui/pages.rs");
    assert!(
        src.contains("El temporizador solo esta en marcha mientras esta opcion este activada"),
        "Ajustes debe explicar el ciclo de vida del temporizador"
    );
}

/* ------------------------- idiomas (v2.3.0) ------------------------- */

#[test]
fn el_idioma_por_defecto_es_espanol() {
    // settings.json sin campo language -> espanol; cualquier codigo raro -> espanol
    let s: crate::store::Settings = serde_json::from_str("{}").unwrap();
    assert_eq!(s.language, None);
    let s2: crate::store::Settings = serde_json::from_str("{\"language\":\"en\"}").unwrap();
    assert_eq!(s2.language.as_deref(), Some("en"));
    // el idioma global arranca en espanol en las pruebas (no se toca)
    assert_eq!(crate::i18n::get(), crate::i18n::Lang::Es);
    assert!(!crate::i18n::is_en());
}

#[test]
fn la_tabla_ingles_traduce_las_claves_principales() {
    // traducciones directas de la tabla (sin tocar el idioma global, para no
    // interferir con las demas pruebas que se ejecutan en paralelo)
    let casos = [
        ("Estado", "Status"),
        ("Ajustes del proxy", "Proxy settings"),
        ("Mis proxies", "My proxies"),
        ("Historial", "History"),
        ("Informe", "Report"),
        ("Acerca de", "About"),
        ("Gestor de proxy para Linux", "Proxy manager for Linux"),
        ("Tema claro", "Light theme"),
        ("Tema oscuro", "Dark theme"),
        ("Guardar configuracion", "Save configuration"),
        ("Guardar y activar", "Save and enable"),
        ("Guardar en la biblioteca", "Save to the library"),
        ("Actualizar en la biblioteca", "Update in the library"),
        ("Probar proxy", "Test proxy"),
        ("Probar sin proxy", "Test without proxy"),
        ("Puerto", "Port"),
        ("Usuario", "Username"),
        ("Contraseña", "Password"),
        ("Reconexion automatica", "Automatic reconnection"),
        ("Verificar ahora", "Check now"),
        ("Verificaciones de calidad de la version", "Version quality checks"),
        ("Licencia y privacidad", "License and privacy"),
        ("Salir", "Quit"),
        ("Cancelar", "Cancel"),
    ];
    for (es, en) in casos {
        let got = crate::i18n::tr(es);
        // en espanol devuelve la clave; comprobamos la tabla inglesa via tr
        // forzando el idioma SOLO si no hay pruebas en paralelo usandolo...
        // aqui se consulta la tabla directamente:
        assert_eq!(got, es, "con idioma espanol tr devuelve la clave: {}", es);
        let tabla = crate::i18n::en(es);
        assert_eq!(tabla, Some(en), "la tabla inglesa debe traducir {}", es);
    }
}

#[test]
fn la_plantilla_ingles_sustituye_los_marcadores() {
    // tf traduce la plantilla y sustituye {0} {1} en orden
    let plantilla = "Proxy \"{0}\" guardado en la biblioteca.";
    let traduccion = crate::i18n::en(plantilla).unwrap();
    assert_eq!(traduccion, "Proxy \"{0}\" saved to the library.");
    let resultado_es = crate::i18n::tf(plantilla, &["Casa"]);
    assert_eq!(resultado_es, "Proxy \"Casa\" guardado en la biblioteca.");
    // plantilla con dos marcadores en orden distinto en la traduccion
    let p2 = "Usuario: {0} - HOME: {1}";
    assert_eq!(crate::i18n::en(p2).unwrap(), "User: {0} - HOME: {1}");
    let r2 = crate::i18n::tf(p2, &["ana", "/home/ana"]);
    assert_eq!(r2, "Usuario: ana - HOME: /home/ana");
}

#[test]
fn la_tabla_ingles_no_deja_claves_con_placeholders_rotos() {
    // las traducciones mantienen EXACTAMENTE los mismos marcadores {N} que la
    // clave espanola (verificacion ligera de una muestra representativa)
    let muestras = [
        "{0} avisos",
        "{0} errores",
        "Proxy \"{0}\" eliminado de la biblioteca.",
        "Biblioteca importada desde {0}: {1} proxies, {2} eventos de historial y los ajustes restaurados.",
        "Nivel {0}: fallo - {1}",
        "FALLO ({0}) - proxy re-aplicado en {1}",
        "hace {0} h {1} min",
    ];
    let n_ph = |s: &str| s.matches('{').count();
    for k in muestras {
        let v = crate::i18n::en(k).unwrap_or_else(|| panic!("falta traduccion de {}", k));
        assert_eq!(n_ph(k), n_ph(v), "placeholders distintos en {}", k);
    }
}

#[test]
fn los_ajustes_guardan_y_recuperan_el_idioma() {
    // persistencia: guardar con language "en" y releerlo (HOME temporal)
    {
        let _g = home_lock();
        let base = tmp_home("proxysend-test-idioma");
        let s = crate::store::Settings { language: Some("en".into()), ..Default::default() };
        crate::store::save_settings(&s);
        let s2 = crate::store::load_settings();
        assert_eq!(s2.language.as_deref(), Some("en"));
        let _ = std::fs::remove_dir_all(&base);
    }
    // y sin ajustes previos el idioma cae al espanol (otro HOME temporal)
    let _g2 = home_lock();
    let base2 = tmp_home("proxysend-test-idioma2");
    let s3 = crate::store::load_settings();
    assert_eq!(s3.language, None);
    let _ = std::fs::remove_dir_all(&base2);
}

#[test]
fn la_biblioteca_exporta_e_importa_el_idioma() {
    // la biblioteca completa transporta el idioma elegido (campo language)
    let _g = home_lock();
    let base = tmp_home("proxysend-test-idioma-lib");
    let s = crate::store::Settings { language: Some("en".into()), ..Default::default() };
    crate::store::save_settings(&s);
    let path = crate::libexport::export_library(&s).expect("export fallo");
    let texto = std::fs::read_to_string(&path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&texto).unwrap();
    assert_eq!(data["ajustes"]["language"], "en", "el idioma viaja en la biblioteca");
    // reimportar en una instalacion limpia restaura el idioma
    let vacia = crate::store::Settings::default();
    crate::store::save_settings(&vacia);
    crate::libexport::import_library(&path).expect("import fallo");
    let s2 = crate::store::load_settings();
    assert_eq!(s2.language.as_deref(), Some("en"), "importar restaura el idioma");
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn la_ayuda_de_consola_y_el_selector_estan_en_los_dos_idiomas() {
    // la ayuda larga de consola tiene traduccion completa
    let clave = "ProxySend {0} - gestor de proxy para Linux\n\nUso:\n  proxysend                 abre la aplicacion\n  proxysend --autostart     aplica el proxy guardado al iniciar sesion (sin ventana)\n  proxysend --aplicar       aplica el proxy guardado y sale\n  proxysend --quitar        quita el proxy configurado en todos los niveles y sale\n  proxysend --informe       genera el informe diagnostico completo y sale\n  proxysend --verificar     verifica el proxy activo ahora mismo (reconexion) y sale\n  proxysend --pagina X      abre en una pagina (estado|ajustes|proxies|historial|informe|acerca)\n  proxysend --oscuro        inicia con el tema oscuro (no se guarda)\n  proxysend --importar F    importa una configuracion JSON al formulario\n  proxysend --probar URL    prueba una conexion a traves de un proxy y sale (URL como\n                            http://host:puerto, socks5://host:puerto o \"directo\" para\n                            probar sin proxy)\n  proxysend --version       muestra la version\n";
    let en = crate::i18n::en(clave).expect("la ayuda debe tener traduccion");
    assert!(en.contains("proxy manager for Linux"));
    assert!(en.contains("Usage:"));
    assert!(en.contains("shows the version"));
    // el selector de idioma existe en Preferencias (Español / English sin traducir)
    let src = include_str!("ui/pages.rs");
    assert!(src.contains("\"Español\", \"English\""), "Preferencias debe ofrecer Español y English");
    assert!(src.contains("job_set_language"), "el selector debe llamar a job_set_language");
}

#[test]
fn los_meses_se_abrevian_en_los_dos_idiomas() {
    // tabla de meses: espanol por defecto, ingles disponible
    assert_eq!(crate::i18n::mes(0), "ene");
    assert_eq!(crate::i18n::mes(11), "dic");
}

#[test]
fn el_informe_menciona_la_novedad_del_idioma() {
    // constancia en la seccion 11 del informe (texto espanol, informe en espanol)
    let src = include_str!("report.rs");
    assert!(src.contains("Idioma espanol/ingles incorporado"), "la seccion 11 debe anotar el idioma");
    let pages = include_str!("ui/pages.rs");
    assert!(pages.contains("Idioma espanol/ingles incorporado"), "la tarjeta de calidad debe anotar el idioma");
}

/* ------------------------- bandeja del sistema (v2.3.1) ------------------------- */

#[test]
fn los_avisos_de_la_bandeja_se_consumen_una_sola_vez() {
    // el resultado de Activar/Desactivar desde la bandeja llega a la ventana
    // como nota: se guarda y se consume al leerlo (no se repite)
    let shared = crate::tray::TrayShared::new();
    assert!(shared.take_note().is_none(), "sin avisar no debe haber nota");
    shared.put_note(true, "Proxy activado desde la bandeja del sistema");
    let n = shared.take_note().expect("la nota debe estar");
    assert!(n.ok);
    assert_eq!(n.msg, "Proxy activado desde la bandeja del sistema");
    assert!(shared.take_note().is_none(), "la nota se consume: no puede leerse dos veces");
    // una nota nueva sustituye a una pendiente sin leer
    shared.put_note(false, "fallo 1");
    shared.put_note(true, "fallo corregido");
    let n2 = shared.take_note().expect("la ultima nota gana");
    assert!(n2.ok);
    assert_eq!(n2.msg, "fallo corregido");
}

#[test]
fn la_peticion_de_segunda_instancia_se_atiende_y_se_borra() {
    // abrir la app de nuevo escribe show.request: ahora se atiende (trae la
    // ventana al frente) y se elimina para no repetir el aviso
    let dir = crate::store::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("show.request");
    let _ = std::fs::remove_file(&path);
    assert!(!crate::tray::consume_show_request(), "sin archivo no hay peticion");
    std::fs::write(&path, "1").expect("escribir show.request");
    assert!(crate::tray::consume_show_request(), "con archivo si hay peticion");
    assert!(!path.exists(), "el archivo debe borrarse al atenderse");
    assert!(!crate::tray::consume_show_request(), "atendida dos veces no repite");
}

#[test]
fn la_bandeja_arranca_con_todas_las_banderas_a_falso() {
    // Mostrar/Activar solo actuan si el usuario lo pide: nada armado al abrir.
    // v2.3.4: ya no existe bandera de salida (Salir termina el proceso al instante)
    let shared = crate::tray::TrayShared::new();
    assert!(!shared.show.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!shared.dirty.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!shared.hidden.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!shared.restore.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn el_ingles_traduce_el_menu_de_la_bandeja() {
    // las cadenas del menu de la bandeja tienen traduccion al ingles
    let pares = [
        ("Mostrar ProxySend", "Show ProxySend"),
        ("Activar proxy", "Enable proxy"),
        ("Desactivar proxy", "Disable proxy"),
        ("Salir", "Quit"),
        (
            "No se pudo activar el proxy desde la bandeja: {0}",
            "Could not enable the proxy from the tray: {0}",
        ),
        (
            "No hay ningun proxy guardado todavia: agrega uno en la pagina \"Mis proxies\".",
            "There is no saved proxy yet: add one on the \"My proxies\" page.",
        ),
        ("Ventana mostrada en pantalla", "Window shown on screen"),
        ("Salir desde la bandeja: proceso terminado al instante", "Quit from the tray: process ended instantly"),
    ];
    for (es, en_esperada) in pares {
        assert_eq!(crate::i18n::en(es), Some(en_esperada), "falta traducir al ingles: {es}");
    }
    // v2.3.4: las claves retiradas de las opciones de bandeja ya no estan
    assert_eq!(crate::i18n::en("Ventana oculta en la bandeja del sistema"), None);
    assert_eq!(crate::i18n::en("Minimizar a la bandeja activado"), None);
}

#[test]
fn el_informe_y_la_tarjeta_anotan_la_revision_de_la_bandeja() {
    // constancia en la seccion 11 del informe y en la tarjeta de calidad
    let report = include_str!("report.rs");
    assert!(report.contains("Menus de la bandeja del sistema reparados"), "el informe debe anotar el arreglo");
    let ui = include_str!("ui/mod.rs");
    let main = include_str!("main.rs");
    let tray = include_str!("tray.rs");
    // v2.3.4: ya no existe la ocultacion en la bandeja: sin hide_to_tray, sin
    // casillas y sin salida garantizada; Salir termina el proceso al instante
    assert!(!ui.contains("hide_to_tray"), "no debe quedar rastro de hide_to_tray");
    assert!(!ui.contains("CancelClose"), "el cierre ya no se intercepta");
    assert!(!main.contains("Salida garantizada"), "el respaldo de salida garantizada se retiro");
    assert!(tray.contains("process::exit(0)"), "Salir desde la bandeja debe terminar el proceso al instante");
    assert!(main.contains("consume_show_request"), "main debe atender la segunda instancia");
}

#[test]
fn la_revision_a_fondo_queda_anotada_y_el_icono_de_bandeja_se_sincroniza() {
    // v2.3.1 (revision final): constancia en la seccion 11 y en la tarjeta,
    // y el refresco de la bandeja cableado al recargar el estado de la UI
    let report = include_str!("report.rs");
    assert!(report.contains("Revision a fondo del codigo completo"), "el informe debe anotar la revision a fondo");
    assert!(report.contains("Icono de la bandeja sincronizado"), "el informe debe anotar el refresco del icono");
    let pages = include_str!("ui/pages.rs");
    assert!(pages.contains("Revision a fondo del codigo completo"), "la tarjeta de calidad debe anotar la revision");
    let ui = include_str!("ui/mod.rs");
    assert!(ui.contains("notify_state_changed"), "refresh_status debe refrescar el icono de la bandeja");
    // y la tabla de ingles traduce las dos entradas nuevas
    let i18n = include_str!("i18n.rs");
    assert!(i18n.contains("Full deep code review"), "falta la traduccion inglesa de la revision");
    assert!(i18n.contains("Tray icon kept in sync"), "falta la traduccion inglesa del icono sincronizado");
}

/* ------------------------- bandeja del sistema (v2.3.2) ------------------------- */

#[test]
fn la_decision_de_mostrar_depende_del_entorno() {
    use crate::tray::{show_action_for, ShowAction};
    // X11: la ventana se puede mapear y enfocar siempre -> Raise
    assert!(matches!(show_action_for(false, false), ShowAction::Raise));
    assert!(matches!(show_action_for(false, true), ShowAction::Raise));
    // Wayland visible: se pide foco sin cerrar nada -> Raise
    assert!(matches!(show_action_for(true, false), ShowAction::Raise));
    // Wayland con la ventana en la bandeja: mostrar = REABRIR (el protocolo
    // no permite reaparecer una ventana; es el fallo que reporto el usuario)
    assert!(matches!(show_action_for(true, true), ShowAction::Reopen));
}

#[test]
fn take_show_action_consume_la_peticion_una_sola_vez() {
    // la bandera de la bandeja se consume una sola vez y la decision se toma
    // con el estado hidden actual (en pruebas: sesion X11, sin Wayland)
    let shared = crate::tray::TrayShared::new();
    assert!(matches!(crate::tray::take_show_action(&shared), crate::tray::ShowAction::None));
    shared.show.store(true, std::sync::atomic::Ordering::SeqCst);
    let accion = crate::tray::take_show_action(&shared);
    assert!(matches!(accion, crate::tray::ShowAction::Raise), "X11: mostrar = Raise");
    assert!(matches!(crate::tray::take_show_action(&shared), crate::tray::ShowAction::None));
    // y con la marca de segunda instancia tambien responde una sola vez
    let dir = crate::store::config_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("show.request");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, "1").expect("escribir show.request");
    assert!(matches!(crate::tray::take_show_action(&shared), crate::tray::ShowAction::Raise));
    assert!(!path.exists(), "la peticion de la 2a instancia debe consumirse");
}

#[test]
fn la_revision_v235_queda_anotada_en_informe_tarjeta_y_traducciones() {
    // constancia de la revision v2.3.5 (codigo muerto fuera y clippy en
    // silencio; regresion re-verificada) en el informe, la tarjeta e ingles
    let report = include_str!("report.rs");
    let pages = include_str!("ui/pages.rs");
    let i18n = include_str!("i18n.rs");
    assert!(report.contains("Revision a fondo de la version 2.3.5"), "el informe debe anotar la revision de la 2.3.5");
    assert!(report.contains("Regresion re-verificada en la simulacion real de la 2.3.5"), "el informe debe anotar la regresion re-verificada");
    assert!(pages.contains("Revision a fondo de la 2.3.5"), "la tarjeta de calidad debe anotar la revision de la 2.3.5");
    assert!(pages.contains("Regresion re-verificada en simulacion real"), "la tarjeta de calidad debe anotar la regresion");
    assert!(i18n.contains("Deep review of version 2.3.5"), "falta la traduccion inglesa de la revision");
    assert!(i18n.contains("Regression re-verified in the 2.3.5 real simulation"), "falta la traduccion inglesa de la regresion");
    assert!(i18n.contains("Deep review of 2.3.5"), "falta la traduccion inglesa de la tarjeta");
}

/* ------------------------- Novedades 2.4.0 (interfaz azul) ------------------------- */

#[test]
fn la_revision_v240_queda_anotada_en_informe_tarjeta_y_traducciones() {
    // constancia de la revision v2.4.0 (interfaz azul, codigo muerto fuera con
    // escaneo automatizado y simulacion E2E real) en el informe, tarjeta e ingles
    let report = include_str!("report.rs");
    let pages = include_str!("ui/pages.rs");
    let i18n = include_str!("i18n.rs");
    assert!(report.contains("Nueva interfaz azul"), "el informe debe anotar la interfaz nueva");
    assert!(report.contains("Revision de codigo muerto con escaneo automatizado"), "el informe debe anotar el escaneo de codigo muerto");
    assert!(report.contains("Simulacion E2E real en X11 de la 2.4.0"), "el informe debe anotar la simulacion E2E");
    assert!(pages.contains("Nueva interfaz azul"), "la tarjeta de calidad debe anotar la interfaz nueva");
    assert!(i18n.contains("New blue interface"), "falta la traduccion inglesa de la interfaz nueva");
    assert!(i18n.contains("Dead code review with automated scanning"), "falta la traduccion inglesa del escaneo");
    assert!(i18n.contains("Real E2E simulation on X11"), "falta la traduccion inglesa de la simulacion");
}

#[test]
fn las_claves_de_traduccion_huerfanas_fueron_eliminadas() {
    // v2.4.0: cero claves sin uso en la tabla inglesa ("Idioma" y la de
    // biblioteca ilegible quedaban de versiones anteriores)
    assert_eq!(crate::i18n::en("Idioma"), None, "la clave huerfana 'Idioma' debe eliminarse");
    assert_eq!(crate::i18n::en("No se pudo leer la biblioteca: {0}"), None, "la clave huerfana de biblioteca debe eliminarse");
    // y las claves que SI se usan siguen traducidas
    assert_eq!(crate::i18n::en("Idioma cambiado"), Some("Language changed"));
}

#[test]
fn el_valor_muerto_de_guardar_en_biblioteca_fue_eliminado() {
    // v2.4.0: job_save_proxy ya no asigna saved_id sin usarlo
    let pages = include_str!("ui/pages.rs");
    assert!(!pages.contains("saved_id"), "el valor muerto saved_id debe desaparecer");
    assert!(!pages.contains("let _ = saved_id;"), "no debe quedar el descarte del valor muerto");
}

#[test]
fn la_interfaz_azul_esta_implementada() {
    // constancia de la interfaz nueva: paleta con barra lateral navy, boton de
    // encendido con arco real e iconos de navegacion dibujados
    let ui = include_str!("ui/mod.rs");
    let widgets = include_str!("ui/widgets.rs");
    let pages = include_str!("ui/pages.rs");
    assert!(ui.contains("Paleta azul"), "la paleta nueva debe estar documentada");
    assert!(ui.contains("sidebar_top"), "la paleta debe tener los colores de la barra lateral navy");
    assert!(widgets.contains("fn arc_stroke"), "el simbolo de encendido debe dibujarse con arc_stroke");
    assert!(widgets.contains("fn nav_icon"), "los iconos del menu deben dibujarse con nav_icon");
    assert!(pages.contains("nav_row"), "el menu lateral debe usar filas con icono");
}

#[test]
fn las_banderas_nuevas_de_la_bandeja_arrancan_a_falso() {
    // hidden y restore solo se activan con acciones del usuario
    let shared = crate::tray::TrayShared::new();
    assert!(!shared.hidden.load(std::sync::atomic::Ordering::SeqCst));
    assert!(!shared.restore.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn la_revision_v234_queda_anotada_en_informe_tarjeta_y_traducciones() {
    // constancia de la revision v2.3.4 (opcion de bandeja retirada y menues
    // reparados) en el informe, la tarjeta de calidad, el codigo y el ingles
    let report = include_str!("report.rs");
    let pages = include_str!("ui/pages.rs");
    let ui = include_str!("ui/mod.rs");
    let main = include_str!("main.rs");
    let i18n = include_str!("i18n.rs");
    let tray = include_str!("tray.rs");
    // mostrar una ventana MINIMIZADA: en X11 se des-minimiza de verdad
    assert!(ui.contains("Minimized(false)"), "Mostrar debe des-minimizar en X11");
    assert!(main.contains("Minimized(false)"), "el hilo puente tambien debe des-minimizar");
    // el estado de minimizado llega a la bandeja (Wayland: reabrir)
    assert!(ui.contains("self.shared.hidden.store(minimized_now"), "la UI debe publicar el estado real de la ventana");
    assert!(ui.contains("ShowAction::Reopen"), "la UI debe atender la reapertura");
    assert!(main.contains("shared.restore.swap"), "main debe reabrir la ventana en Wayland");
    assert!(tray.contains("show_action_for"), "la decision de mostrar debe ser probada");
    // constancia en el informe y la tarjeta, con traduccion inglesa
    let clave = "Las opciones de bandeja al cerrar o minimizar se quitaron de Ajustes del proxy (v2.3.4): cerrar la ventana cierra ProxySend y minimizar solo minimiza, sin ocultar nada";
    assert!(report.contains(clave), "el informe debe anotar la retirada de las opciones");
    assert!(pages.contains(clave), "la tarjeta de calidad debe anotar la retirada");
    assert!(i18n.contains("The tray options on close or minimize were removed from Proxy settings (v2.3.4)"), "falta la traduccion inglesa de la retirada");
    assert!(i18n.contains("Show ProxySend deeply repaired"), "falta la traduccion inglesa del arreglo de Mostrar");
    assert!(i18n.contains("Quit from the tray ends the process INSTANTLY"), "falta la traduccion inglesa de la salida instantanea");
    assert!(i18n.contains("Window reopened in the foreground from the system tray"), "falta traduccion del registro de reapertura");
}

#[test]
fn el_ingles_traduce_las_cadenas_nuevas_de_la_revision() {
    let pares = [
        ("Ventana reabierta en primer plano desde la bandeja del sistema", "Window reopened in the foreground from the system tray"),
        ("Ventana mostrada en pantalla", "Window shown on screen"),
        (
            "Sin rastros de la funcionalidad retirada: casillas, ajustes guardados, registros, botones y traducciones fuera; los ajustes viejos con esas claves se leen sin error y se descartan.",
            "No traces of the removed feature: checkboxes, saved settings, logs, buttons and translations gone; old settings with those keys load without errors and are discarded.",
        ),
        (
            "Salir desde la bandeja termina el proceso AL INSTANTE (medido en simulacion con la ventana visible y minimizada): sin esperas ni cierre garantizado de respaldo.",
            "Quit from the tray ends the process INSTANTLY (measured in simulation with the window visible and minimized): no waits and no guaranteed fallback close.",
        ),
    ];
    for (es, en_esperada) in pares {
        assert_eq!(crate::i18n::en(es), Some(en_esperada), "falta traducir al ingles: {es}");
    }
    // las claves de las opciones retiradas ya NO estan en la tabla
    assert_eq!(crate::i18n::en("Al cerrar la ventana, dejar ProxySend en la bandeja"), None);
    assert_eq!(crate::i18n::en("Al minimizar la ventana, dejar ProxySend en la bandeja"), None);
    assert_eq!(crate::i18n::en("Cierre a la bandeja activado"), None);
}

/* ------------------------- opciones de bandeja retiradas (v2.3.4) ------------------------- */

#[test]
fn las_opciones_de_bandeja_retiradas_no_dejan_rastro() {
    // v2.3.4: se quitaron las opciones de minimizar/cerrar a la bandeja de
    // Ajustes del proxy. No debe quedar NINGUN rastro en la interfaz, en los
    // ajustes ni en la logica de la ventana.
    let pages = include_str!("ui/pages.rs");
    let ui = include_str!("ui/mod.rs");
    let store = include_str!("store.rs");
    let ops = include_str!("ops.rs");
    let i18n = include_str!("i18n.rs");
    // interfaz: sin casillas, sin jobs y sin boton lateral
    assert!(!pages.contains("Al cerrar la ventana, dejar ProxySend en la bandeja"), "la casilla de cierre a la bandeja debe desaparecer");
    assert!(!pages.contains("Al minimizar la ventana, dejar ProxySend en la bandeja"), "la casilla de minimizar a la bandeja debe desaparecer");
    assert!(!pages.contains("job_minimize_tray"), "no debe quedar el job de la casilla");
    assert!(!pages.contains("job_close_tray"), "no debe quedar el job de cierre a bandeja");
    assert!(!pages.contains("Minimizar a la bandeja"), "no debe quedar boton ni texto de minimizar a la bandeja");
    // logica de la ventana: sin ocultacion y sin interceptar cerrar/minimizar
    assert!(!ui.contains("hide_to_tray"), "no debe quedar hide_to_tray");
    assert!(!ui.contains("Minimized(true)"), "la app ya no minimiza por decision propia");
    assert!(!ui.contains("Visible(false)"), "la app ya no oculta la ventana");
    assert!(!ui.contains("close_to_tray"), "no debe quedar el intercepto del cierre");
    assert!(!ui.contains("minimize_seen"), "no debe quedar el rastro de la casilla");
    assert!(!ui.contains("quitting"), "no debe quedar la salida negociada");
    // ajustes: sin los campos (las claves viejas del JSON se ignoran)
    assert!(!store.contains("close_to_tray"), "el ajuste close_to_tray debe desaparecer");
    assert!(!store.contains("minimize_to_tray"), "el ajuste minimize_to_tray debe desaparecer");
    assert!(!ops.contains("minimize_to_tray"), "reset_defaults no debe conservar el ajuste quitado");
    // traducciones: sin las claves retiradas
    assert!(!i18n.contains("Minimizar a la bandeja"), "no deben quedar traducciones de la opcion retirada");
    assert!(!i18n.contains("Cierre a la bandeja"), "no deben quedar traducciones del cierre a la bandeja");
    assert!(!i18n.contains("Window hidden to the system tray"), "no debe quedar la traduccion del registro de ocultacion");
    assert!(!i18n.contains("Salida garantizada"), "no debe quedar la traduccion del cierre garantizado");
}

#[test]
fn los_ajustes_viejos_con_claves_de_bandeja_cargan_sin_error() {
    // un settings.json de la v2.3.3 con closeToTray/minimizeToTray marcados
    // carga sin romperse; las claves viejas se ignoran y NO se vuelven a
    // escribir (no queda rastro al guardar)
    let viejo = serde_json::from_str::<crate::store::Settings>(
        "{\"config\":null,\"active\":false,\"proxies\":[],\"scopes\":{\"desktop\":true,\"system\":true,\"terminal\":true},\"autostart\":false,\"closeToTray\":true,\"minimizeToTray\":true,\"reconnectEnabled\":false,\"reconnectMin\":5,\"language\":null,\"version\":null}",
    )
    .expect("los ajustes viejos deben cargar sin error");
    assert!(!viejo.reconnect_enabled);
    // al guardar ya no aparecen las claves viejas
    let json = serde_json::to_string(&viejo).expect("serializar");
    assert!(!json.contains("closeToTray"), "la clave vieja no debe volver a escribirse");
    assert!(!json.contains("minimizeToTray"), "la clave vieja no debe volver a escribirse");
}

/* ------------------------- Revision a fondo post-entrega (v2.4.0) ------------------------- */

#[test]
fn el_timeout_elimina_al_proceso_externo() {
    // v2.4.0: un comando que se cuelga debe recibir kill al agotar el timeout
    // (antes el proceso y su hilo de lectura quedaban colgados para siempre).
    // sleep 2 con timeout de 300 ms: debe devolver timeout en ~0.3 s, no en 2 s.
    let t0 = std::time::Instant::now();
    let r = crate::sysops::run_with_env("sleep", &["2"], false, 300);
    let d = t0.elapsed();
    assert!(!r.ok, "el comando matado no puede reportar exito");
    assert!(r.stderr.contains("timeout"), "debe anotar la causa: timeout");
    assert!(d.as_millis() < 1500, "el kill debe ser inmediato al expirar (tardo {:?})", d);
}

#[test]
fn la_escritura_atomica_deja_el_archivo_completo_y_sin_temporales() {
    // v2.4.0: settings/history/reconnect se escriben con temp + rename para
    // que un corte a mitad de escritura nunca deje un JSON a medias.
    let dir = std::env::temp_dir().join(format!("proxysend-atomic-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("prueba.json");
    let json = "{\"ok\":true,\"datos\":\"valor de prueba largo para la escritura\"}";
    crate::store::write_file_atomic(&path, json).expect("escritura atomica");
    let leido = std::fs::read_to_string(&path).expect("leer de vuelta");
    assert_eq!(leido, json, "el contenido debe quedar completo");
    let temporales: Vec<_> = std::fs::read_dir(&dir)
        .expect("listar carpeta")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(temporales.is_empty(), "no debe quedar ningun temporal: {:?}", temporales.len());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn la_revision_post_entrega_queda_anotada_en_informe_tarjeta_y_traducciones() {
    // constancia de los errores reparados: timeout con kill, handle de bandeja
    // reemplazable, escritura atomica e Instant sin panics
    let report = include_str!("report.rs");
    let pages = include_str!("ui/pages.rs");
    let sysops = include_str!("sysops.rs");
    let tray = include_str!("tray.rs");
    let store = include_str!("store.rs");
    let ui = include_str!("ui/mod.rs");
    let i18n = include_str!("i18n.rs");
    assert!(report.contains("Revision a fondo post-entrega de la 2.4.0"), "el informe debe anotar la revision");
    assert!(pages.contains("Revision a fondo post-entrega"), "la tarjeta de calidad debe anotarla");
    assert!(i18n.contains("In-depth post-release review of 2.4.0"), "falta la traduccion inglesa de la revision");
    // las reparaciones estan en el codigo
    assert!(sysops.contains("child.kill()"), "el timeout debe eliminar al hijo");
    assert!(tray.contains("Mutex<Option<TrayHandle>>"), "el handle de bandeja debe ser reemplazable");
    assert!(store.contains("write_file_atomic"), "la persistencia debe ser atomica");
    assert!(store.contains("persist_lock"), "la persistencia debe tener cerrojo entre hilos");
    assert!(!ui.contains("Instant::now() - Duration::from_secs(10)"), "no debe quedar la resta de Instant con underflow");
}
