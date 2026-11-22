//! ProxySend - prueba de conexion con el motor de red interno (ureq/rustls),
//! con respaldo a curl si el motor interno no puede completar la prueba.

use crate::core;
use crate::store::ProxyConfig;
use crate::sysops;
use crate::i18n::{tf, tr};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct TestResult {
    pub ok: bool,
    pub ip: String,
    pub code: String,
    pub seconds: f64,
    pub detail: String,
}

pub fn test_with_ureq(proxy_url: Option<&str>) -> Result<TestResult, String> {
    let started = Instant::now();
    let result = match proxy_url {
        Some(url) => {
            let proxy = ureq::Proxy::new(url).map_err(|e| e.to_string())?;
            let agent = ureq::AgentBuilder::new()
                .timeout_connect(Duration::from_secs(8))
                .timeout(Duration::from_secs(25))
                .proxy(proxy)
                .build();
            agent.get("https://api.ipify.org").call()
        }
        None => ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout(Duration::from_secs(25))
            .build()
            .get("https://api.ipify.org")
            .call(),
    };
    let resp = match result {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            // respuesta HTTP con codigo inesperado: igualmente es informativa
            let body = r.into_string().unwrap_or_default().trim().to_string();
            let seconds = started.elapsed().as_secs_f64();
            return Ok(TestResult {
                ok: false,
                ip: body,
                code: code.to_string(),
                seconds,
                detail: tf("respuesta HTTP {0}", &[&code.to_string()]),
            });
        }
        Err(e) => return Err(e.to_string()),
    };
    let code = resp.status().to_string();
    let body = resp.into_string().unwrap_or_default().trim().to_string();
    let seconds = started.elapsed().as_secs_f64();
    if body.is_empty() {
        return Err(tr("respuesta vacia").to_string());
    }
    Ok(TestResult { ok: code == "200", ip: body, code, seconds, detail: String::new() })
}

fn proxy_url_for_test(config: &ProxyConfig) -> String {
    // la prueba va a un destino HTTPS: si el proxy es separado se usa el de HTTPS
    let v = core::env_values(config);
    v.https_url
}

/// Prueba de conexion: motor interno primero, curl como respaldo.
/// SOCKS5 con credenciales va directo a curl (igual que en la v1.x).
pub fn test_connection(config: Option<&ProxyConfig>, use_proxy: bool) -> TestResult {
    let socks_auth = use_proxy
        && config
            .map(|c| c.auth && c.protocol == "socks5")
            .unwrap_or(false);

    if !socks_auth {
        let url = if use_proxy {
            config.map(proxy_url_for_test)
        } else {
            None
        };
        match test_with_ureq(url.as_deref()) {
            Ok(r) if r.ok => {
                log_test(&r, use_proxy);
                return r;
            }
            Ok(r) => {
                // HTTP != 200 pero hubo respuesta: devolver tal cual
                log_test(&r, use_proxy);
                return r;
            }
            Err(_) => { /* cae al respaldo con curl */ }
        }
    }

    match sysops::test_proxy_curl(config, use_proxy) {
        Ok(r) => {
            log_test(&r, use_proxy);
            r
        }
        Err(detail) => {
            crate::eventlog::error("prueba", tf("Fallo de conexion: {0}", &[&detail]));
            TestResult {
                ok: false,
                ip: String::new(),
                code: String::new(),
                seconds: 0.0,
                detail,
            }
        }
    }
}

fn log_test(r: &TestResult, use_proxy: bool) {
    if r.ok {
        crate::eventlog::info(
            "prueba",
            tf(
                "{0}: IP {1} - HTTP {2} - {3} s",
                &[if use_proxy { tr("Prueba con proxy") } else { tr("Prueba directa") }, &r.ip, &r.code, &format!("{:.2}", r.seconds)],
            ),
        );
    } else {
        let detalle = if r.detail.is_empty() { tf("respuesta HTTP {0}", &[&r.code]) } else { r.detail.clone() };
        crate::eventlog::error("prueba", tf("Fallo de conexion: {0}", &[&detalle]));
    }
}
