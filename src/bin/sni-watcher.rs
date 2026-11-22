//! sni-watcher: simulador de la bandeja del sistema (org.kde.StatusNotifierWatcher)
//! para las pruebas de ProxySend. Imita el rol que cumplen GNOME/KDE/XFCE/COSMIC
//! en un escritorio real: recibe el registro de items SNI y los lista.
//!
//! Uso: sni-watcher  (dentro de una sesion de D-Bus; imprime cada item registrado)

use std::sync::Mutex;
use zbus::fdo::Result;
use zbus::interface;

struct Watcher {
    items: Mutex<Vec<String>>,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    fn register_status_notifier_item(&self, service: &str) {
        println!("REGISTERED: {service}");
        self.items.lock().unwrap().push(service.to_string());
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.items.lock().unwrap().clone()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }
}

fn main() -> Result<()> {
    let watcher = Watcher { items: Mutex::new(Vec::new()) };
    let conn = zbus::blocking::connection::Builder::session()?
        .name("org.kde.StatusNotifierWatcher")?
        .serve_at("/StatusNotifierWatcher", watcher)?
        .build()?;
    println!("watcher listo en {:?}", conn.unique_name());
    // mantener vivo
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
