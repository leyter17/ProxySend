//! ProxySend - paginas de la interfaz.

use super::widgets::*;
use super::{ToastKind, UiMsg};
use crate::core;
use crate::ops;
use crate::store::{self, ProxyConfig, Scopes};
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

impl super::ProxySendApp {
    /* ------------------------- Barra lateral azul ------------------------- */

    pub fn render_sidebar(&mut self, ctx: &egui::Context) {
        let pal = self.pal();
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(248.0)
            .frame(egui::Frame::none().fill(pal.sidebar_top))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                // degradado vertical azul marino (top oscuro -> bottom azul)
                paint_gradient(ui.painter(), rect, pal.sidebar_top, pal.sidebar_bot);

                // --- marca de la app: logo dibujado + nombre ---
                ui.add_space(26.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    let (lrect, _) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                    let lc = lrect.center();
                    let lp = ui.painter_at(lrect);
                    // marca cuadrada redondeada con el simbolo de encendido
                    lp.rect_filled(lrect, egui::Rounding::same(10.0), Color32::from_rgba_unmultiplied(255, 255, 255, 30));
                    arc_stroke(&lp, lc, 8.5, -60.0, 240.0, 2.6, pal.accent_hi);
                    lp.line_segment(
                        [Pos2::new(lc.x, lc.y - 11.5), Pos2::new(lc.x, lc.y - 1.0)],
                        Stroke::new(2.6_f32, pal.accent_hi),
                    );
                    ui.add_space(10.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("ProxySend").size(22.0).strong().color(Color32::WHITE));
                        ui.label(
                            egui::RichText::new(crate::i18n::tr("Gestor de proxy para Linux"))
                                .size(11.5)
                                .color(pal.on_sidebar_soft),
                        );
                    });
                });
                ui.add_space(20.0);

                // --- menu de navegacion con iconos dibujados ---
                let items = [
                    (super::Page::Estado, crate::i18n::tr("Estado")),
                    (super::Page::Ajustes, crate::i18n::tr("Ajustes del proxy")),
                    (super::Page::Proxies, crate::i18n::tr("Mis proxies")),
                    (super::Page::Historial, crate::i18n::tr("Historial")),
                    (super::Page::Informe, crate::i18n::tr("Informe")),
                    (super::Page::Acerca, crate::i18n::tr("Acerca de")),
                ];
                for (page, label) in items {
                    self.nav_row(ui, &pal, page, label);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    ui.add_space(18.0);
                    let theme_label = if self.dark { crate::i18n::tr("Tema claro") } else { crate::i18n::tr("Tema oscuro") };
                    // pastilla de cambio de tema (fill suave, texto blanco)
                    let avail = ui.available_width() - 36.0;
                    let (trect, tresp) = ui.allocate_exact_size(Vec2::new(avail, 36.0), egui::Sense::click());
                    let tp = ui.painter_at(trect);
                    let hovered = tresp.hovered();
                    let fill_alpha = if hovered { 58 } else { 40 };
                    tp.rect_filled(trect, egui::Rounding::same(999.0), Color32::from_rgba_unmultiplied(255, 255, 255, fill_alpha));
                    // punto indicador: azul cuando el tema activo es oscuro, teal en claro
                    let dot_c = Pos2::new(trect.left() + 18.0, trect.center().y);
                    let dot_color = if self.dark { pal.accent_hi } else { pal.accent2 };
                    tp.circle_filled(dot_c, 4.5, dot_color);
                    tp.text(
                        Pos2::new(trect.left() + 32.0, trect.center().y),
                        Align2::LEFT_CENTER,
                        theme_label,
                        FontId::proportional(13.5),
                        Color32::from_rgba_unmultiplied(255, 255, 255, 245),
                    );
                    if tresp.clicked() {
                        self.dark = !self.dark;
                        super::apply_theme(ctx, self.dark);
                        self.settings.theme = Some(if self.dark { "dark".into() } else { "light".into() });
                        store::save_settings(&self.settings);
                    }
                    ui.add_space(10.0);
                    // indicador vivo del proxy vigente, visible en todas las paginas
                    self.sidebar_status_pill(ui);
                });
            });
    }

    /// Fila del menu lateral: icono dibujado + etiqueta, con relleno al pasar
    /// el puntero, barra indicadora azul y pastilla blanca en el activo.
    fn nav_row(&mut self, ui: &mut egui::Ui, pal: &super::Palette, page: super::Page, label: &str) {
        let selected = self.page == page;
        let avail = ui.available_width() - 28.0;
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail, 42.0), egui::Sense::click());
        let p = ui.painter_at(rect);
        if selected {
            p.rect_filled(rect, egui::Rounding::same(12.0), pal.nav_fill);
            // barra indicadora azul a la izquierda del elemento activo
            let bar = Rect::from_min_size(
                Pos2::new(rect.left() + 6.0, rect.center().y - 11.0),
                Vec2::new(3.0, 22.0),
            );
            p.rect_filled(bar, egui::Rounding::same(2.0), pal.accent_hi);
        } else if resp.hovered() {
            p.rect_filled(rect, egui::Rounding::same(12.0), pal.nav_hover);
        }
        let icon_color = if selected { Color32::WHITE } else { pal.on_sidebar_soft };
        nav_icon(ui, page, Pos2::new(rect.left() + 26.0, rect.center().y), icon_color, 15.0);
        let text_color = if selected { Color32::WHITE } else { Color32::from_rgba_unmultiplied(255, 255, 255, 222) };
        p.text(
            Pos2::new(rect.left() + 48.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(15.0),
            text_color,
        );
        if resp.clicked() {
            self.page = page;
            if page == super::Page::Historial {
                self.history_cache = store::read_history();
                self.history_loaded = true;
            }
            if page == super::Page::Informe {
                self.events_cache = crate::eventlog::read_all();
                self.events_loaded = true;
            }
        }
        ui.add_space(2.0);
    }

    /// Pastilla de estado fija en la barra lateral: muestra que proxy esta
    /// vigente ahora mismo, o si no hay ninguno (conexion directa).
    fn sidebar_status_pill(&self, ui: &mut egui::Ui) {
        let pal = self.pal();
        let s = &self.settings;
        let (dot, title, sub) = if s.active {
            let name = s
                .active_proxy_id
                .as_ref()
                .and_then(|id| s.proxies.iter().find(|p| &p.id == id))
                .map(|p| p.name.clone());
            let title = name.unwrap_or_else(|| crate::i18n::tr("Proxy activo").to_string());
            let sub = s
                .config
                .as_ref()
                .map(summary_text)
                .unwrap_or_default();
            (Color32::from_rgb(64, 255, 214), title, sub)
        } else {
            (
                Color32::from_rgba_unmultiplied(255, 255, 255, 130),
                crate::i18n::tr("Sin proxy").to_string(),
                crate::i18n::tr("conexion directa").to_string(),
            )
        };
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(0, 0, 0, 90))
            .rounding(egui::Rounding::same(14.0))
            .inner_margin(egui::Margin::symmetric(12.0, 9.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::hover());
                    let c = rect.center();
                    ui.painter().circle_filled(c, 5.0, dot);
                    if s.active {
                        ui.painter().circle_stroke(c, 8.0, egui::Stroke::new(1.5_f32, dot.linear_multiply(0.45)));
                    }
                    ui.vertical(|ui| {
                        ui.set_min_width(ui.available_width() - 10.0);
                        ui.label(
                            egui::RichText::new(title).size(13.5).strong().color(Color32::WHITE),
                        );
                        let subtxt = if sub.is_empty() {
                            if s.active { crate::i18n::tr("proxy en uso").to_string() } else { crate::i18n::tr("conexion directa").to_string() }
                        } else {
                            sub
                        };
                        ui.label(
                            egui::RichText::new(subtxt)
                                .size(11.0)
                                .color(pal.on_sidebar_soft),
                        );
                    });
                });
            });
        ui.add_space(8.0);
    }

    /* ------------------------- Contenido ------------------------- */

    pub fn render_content(&mut self, ctx: &egui::Context) {
        let pal = self.pal();
        let title = self.page.title().to_string();
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(pal.bg).inner_margin(egui::Margin::symmetric(24.0, 18.0)))
            .show(ctx, |ui| {
                // encabezado: titulo con subrayado azul de marca (v2.4.0)
                ui.label(egui::RichText::new(title).size(22.0).strong().color(pal.text));
                let (brect, _) = ui.allocate_exact_size(Vec2::new(38.0, 3.0), egui::Sense::hover());
                ui.painter().rect_filled(brect, egui::Rounding::same(2.0), pal.accent);
                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        super::Page::Estado => self.p_estado(ui),
                        super::Page::Ajustes => self.p_ajustes(ui),
                        super::Page::Proxies => self.p_proxies(ui),
                        super::Page::Historial => self.p_historial(ui),
                        super::Page::Informe => self.p_informe(ui),
                        super::Page::Acerca => self.p_acerca(ui),
                    });
            });

        // indicador de operacion en curso
        if let Some(text) = self.busy.clone() {
            egui::Area::new(egui::Id::new("busy"))
                .order(egui::Order::Foreground)
                .anchor(Align2::RIGHT_TOP, [-18.0, 14.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(pal.card)
                        .stroke(egui::Stroke::new(1.0_f32, pal.line))
                        .rounding(egui::Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(egui::Spinner::new().size(16.0));
                                ui.label(egui::RichText::new(text).size(13.0).color(pal.soft));
                            });
                        });
                });
        }
    }

    /* ------------------------- Estado ------------------------- */

    fn p_estado(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal();
        let Some(status) = self.status.clone() else {
            ui.label(egui::RichText::new(crate::i18n::tr("Cargando...")).color(pal.soft));
            return;
        };

        // tarjeta del proxy vigente: que proxy se esta usando ahora, o si no hay ninguno
        card(ui, &pal, |ui| {
            ui.horizontal(|ui| {
                let dot_col = if status.active {
                    pal.accent2
                } else {
                    Color32::from_rgb(148, 156, 170)
                };
                let (rect, _) = ui.allocate_exact_size(Vec2::new(36.0, 36.0), egui::Sense::hover());
                let c = rect.center();
                ui.painter().circle_filled(c, 11.0, dot_col);
                if status.active {
                    ui.painter().circle_stroke(c, 14.5, egui::Stroke::new(2.5_f32, dot_col.linear_multiply(0.35)));
                }
                ui.add_space(6.0);
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width() - 130.0);
                    ui.label(
                        egui::RichText::new(crate::i18n::tr("PROXY VIGENTE EN ESTE MOMENTO"))
                            .size(11.5)
                            .strong()
                            .color(pal.faint),
                    );
                    if status.active {
                        let name = status
                            .active_proxy_name
                            .clone()
                            .unwrap_or_else(|| crate::i18n::tr("Configuracion actual").to_string());
                        ui.label(egui::RichText::new(name).size(17.0).strong().color(pal.text));
                        if let Some(cfg) = &status.config {
                            ui.label(
                                egui::RichText::new(summary_text(cfg)).size(13.0).color(pal.soft),
                            );
                        }
                    } else {
                        ui.label(
                            egui::RichText::new(crate::i18n::tr("Ninguno - la conexion es directa"))
                                .size(17.0)
                                .strong()
                                .color(pal.soft),
                        );
                        ui.label(
                            egui::RichText::new(crate::i18n::tr("No hay ningun proxy aplicado: el trafico sale directo a internet."))
                                .size(13.0)
                                .color(pal.faint),
                        );
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    chip(
                        ui,
                        &pal,
                        if status.active { ChipKind::Ok } else { ChipKind::Off },
                        if status.active { crate::i18n::tr("VIGENTE") } else { crate::i18n::tr("SIN PROXY") },
                    );
                });
            });

            // linea de la reconexion automatica (si esta activada)
            if self.settings.reconnect_enabled {
                ui.separator();
                ui.horizontal(|ui| {
                    let st = &self.reconnect_state;
                    let (color, texto) = if st.last_check == 0 {
                        (pal.faint, crate::i18n::tf("Reconexion automatica activada: primera verificacion en menos de {0} min", &[&self.settings.reconnect_min.to_string()]))
                    } else {
                        let cuanto = crate::watcher::hace_cuanto(st.last_check);
                        match st.ok {
                            Some(true) => (pal.ok_text, crate::i18n::tf("Reconexion automatica: el proxy responde ({0}, {1})", &[&st.detail, &cuanto])),
                            Some(false) => (pal.danger, crate::i18n::tf("Reconexion automatica: {0}, {1}", &[&st.detail, &cuanto])),
                            None => (pal.faint, crate::i18n::tr("Reconexion automatica: verificacion en curso...").to_string()),
                        }
                    };
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 9.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 3.5, color);
                    let texto_color = if texto.contains("FALLO") || texto.contains("FAILURE") { pal.danger } else { pal.soft };
                    ui.label(egui::RichText::new(texto).size(12.0).color(texto_color));
                });
            }
        });

        ui.add_space(6.0);
        // tarjeta hero con el boton grande
        card(ui, &pal, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(6.0);
                let hero = if status.active { crate::i18n::tr("Proxy activado") } else { crate::i18n::tr("Proxy desactivado") };
                ui.label(egui::RichText::new(hero).size(21.0).strong().color(pal.text));
                ui.add_space(4.0);
                let mut sub = if let Some(cfg) = &status.config {
                    crate::i18n::tf("Configurado: {0}", &[&summary_text(cfg)])
                } else {
                    crate::i18n::tr("Todavia no hay un proxy configurado. Ve a Ajustes para agregar uno.").to_string()
                };
                if status.active {
                    if let Some(name) = &status.active_proxy_name {
                        sub += &crate::i18n::tf(" - Perfil: {0}", &[name]);
                    }
                }
                ui.label(egui::RichText::new(sub).size(13.5).color(pal.soft));
                ui.add_space(10.0);

                let resp = big_toggle(ui, &pal, status.active);
                ui.add_space(4.0);
                let hint = if status.active {
                    crate::i18n::tr("Pulsa para desactivar")
                } else if status.config.is_some() {
                    crate::i18n::tr("Pulsa para activar")
                } else {
                    crate::i18n::tr("Configura un proxy primero")
                };
                ui.label(egui::RichText::new(hint).size(12.0).color(pal.faint));

                if resp.clicked() {
                    self.on_big_toggle();
                }
            });
        });

        // niveles de aplicacion
        ui.add_space(6.0);
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Donde se aplica")).size(16.0).strong().color(pal.text));
            ui.add_space(6.0);

            self.scope_row(ui, "desktop", &status.desktop_name, &status.desktop);
            ui.separator();
            self.scope_row(ui, "system", crate::i18n::tr("Sistema (/etc/environment)"), &status.system);
            ui.separator();
            self.scope_row(ui, "terminal", crate::i18n::tr("Terminales (~/.bashrc)"), &status.terminal);
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if btn_ghost(ui, &pal, crate::i18n::tr("Editar configuracion")).clicked() {
                self.page = super::Page::Ajustes;
            }
            if btn_danger(ui, &pal, crate::i18n::tr("Restablecer valores por defecto")).clicked() {
                self.job_reset();
            }
        });
    }

    fn scope_row(&mut self, ui: &mut egui::Ui, scope: &str, name: &str, s: &super::StatusScope) {
        let pal = self.pal();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_min_width(ui.available_width() - 150.0);
                ui.label(egui::RichText::new(name).size(14.5).strong().color(pal.text));
                let text = if !s.supported {
                    s.detail.clone()
                } else {
                    s.value.clone().unwrap_or_default()
                };
                if !text.is_empty() {
                    ui.label(egui::RichText::new(text).size(12.5).color(pal.soft));
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if s.supported {
                    let mut checked = s.enabled;
                    let resp = switch(ui, &pal, &mut checked);
                    if resp.changed() {
                        let scope = scope.to_string();
                        let enabled = checked;
                        if enabled {
                            self.settings.scopes = Scopes {
                                desktop: if scope == "desktop" { true } else { self.settings.scopes.desktop },
                                system: if scope == "system" { true } else { self.settings.scopes.system },
                                terminal: if scope == "terminal" { true } else { self.settings.scopes.terminal },
                            };
                        } else {
                            self.settings.scopes = Scopes {
                                desktop: if scope == "desktop" { false } else { self.settings.scopes.desktop },
                                system: if scope == "system" { false } else { self.settings.scopes.system },
                                terminal: if scope == "terminal" { false } else { self.settings.scopes.terminal },
                            };
                        }
                        self.spawn_job(move || {
                            let mut s = store::load_settings();
                            let r = ops::set_scope(&mut s, &scope, enabled);
                            UiMsg::OpDone {
                                ok: r.ok,
                                toast_ok: if r.ok { String::new() } else { r.detail },
                                failures: Vec::new(),
                                reload_settings: true,
                                refresh_proxies: false,
                                go_estado: false,
                            }
                        });
                    }
                    chip(
                        ui,
                        &pal,
                        if s.active { ChipKind::Ok } else { ChipKind::Off },
                        if s.active { crate::i18n::tr("activo") } else { crate::i18n::tr("inactivo") },
                    );
                } else {
                    chip(ui, &pal, ChipKind::Na, crate::i18n::tr("no disponible"));
                }
            });
        });
        ui.add_space(4.0);
    }

    fn on_big_toggle(&mut self) {
        let Some(status) = self.status.clone() else { return };
        if status.active {
            self.set_busy(crate::i18n::tr("Desactivando proxy..."));
            self.job_deactivate();
        } else if let Some(cfg) = status.config.clone() {
            let scopes = self.settings.scopes;
            self.set_busy(crate::i18n::tr("Activando proxy..."));
            self.job_activate(cfg, scopes, None, crate::i18n::tr("Proxy activado en los niveles seleccionados."));
        } else {
            self.page = super::Page::Ajustes;
            self.set_toast(crate::i18n::tr("Primero configura un proxy en Ajustes."), ToastKind::Neutral);
        }
    }

    /* ------------------------- Ajustes ------------------------- */

    fn p_ajustes(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal();

        // protocolo + campos
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Tipo de proxy")).size(16.0).strong().color(pal.text));
            ui.add_space(4.0);
            let idx = match self.form.protocol.as_str() {
                "https" => 1,
                "socks5" => 2,
                _ => 0,
            };
            if let Some(i) = segmented(ui, &pal, &["HTTP", "HTTPS", "SOCKS5"], idx) {
                self.form.protocol = ["http", "https", "socks5"][i].to_string();
                if self.form.protocol == "socks5" {
                    self.form.same_for_all = true;
                }
            }
            ui.add_space(6.0);

            let single = self.form.protocol == "socks5" || self.form.same_for_all;
            if self.form.protocol != "socks5" {
                if check(ui, &pal, &mut self.form.same_for_all, crate::i18n::tr("Usar el mismo proxy para todos los protocolos")).changed() {
                    // nada extra
                }
                ui.add_space(4.0);
            }

            if single {
                field_row(ui, &pal, crate::i18n::tr("Direccion del servidor"), &mut self.form.host, "192.168.1.10", 90.0, &mut self.form.port, crate::i18n::tr("Puerto"));
            } else {
                field_row(ui, &pal, crate::i18n::tr("Servidor HTTP"), &mut self.form.http_host, "192.168.1.10", 90.0, &mut self.form.http_port, crate::i18n::tr("Puerto"));
                field_row(ui, &pal, crate::i18n::tr("Servidor HTTPS"), &mut self.form.https_host, "192.168.1.10", 90.0, &mut self.form.https_port, crate::i18n::tr("Puerto"));
            }
        });

        ui.add_space(4.0);
        card(ui, &pal, |ui| {
            check(ui, &pal, &mut self.form.auth, crate::i18n::tr("El proxy necesita usuario y contraseña"));
            if self.form.auth {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        field_label(ui, &pal, crate::i18n::tr("Usuario"));
                        let w = ui.available_width() * 0.5 - 8.0;
                        text_field(ui, &pal, &mut self.form.username, w, false);
                    });
                    ui.vertical(|ui| {
                        field_label(ui, &pal, crate::i18n::tr("Contraseña"));
                        let w = ui.available_width();
                        text_field(ui, &pal, &mut self.form.password, w, true);
                    });
                });
            }
        });

        ui.add_space(4.0);
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Excepciones (no pasan por el proxy)")).size(16.0).strong().color(pal.text));
            ui.label(
                egui::RichText::new(crate::i18n::tr("Una por linea o separadas por coma. Por defecto: localhost, 127.0.0.0/8 y ::1."))
                    .size(12.0)
                    .color(pal.faint),
            );
            text_area(ui, &pal, &mut self.form.ignore_text, 4, ui.available_width());
        });

        ui.add_space(8.0);
        // botones de accion
        ui.horizontal_wrapped(|ui| {
            if btn_primary(ui, &pal, crate::i18n::tr("Guardar configuracion")).clicked() {
                let cfg = self.form.build_config();
                let scopes = self.settings.scopes;
                self.job_save_config(cfg, scopes);
            }
            if btn_primary(ui, &pal, crate::i18n::tr("Guardar y activar")).clicked() {
                let cfg = self.form.build_config();
                let scopes = self.settings.scopes;
                self.set_busy(crate::i18n::tr("Activando proxy..."));
                self.job_save_and_activate(cfg, scopes);
            }
            let lib_label = if self.editing_proxy_id.is_some() {
                crate::i18n::tr("Actualizar en la biblioteca")
            } else {
                crate::i18n::tr("Guardar en la biblioteca")
            };
            if btn_secondary(ui, &pal, lib_label).clicked() {
                self.open_save_modal();
            }
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if btn_ghost(ui, &pal, crate::i18n::tr("Probar proxy")).clicked() {
                let cfg = self.form.build_config();
                self.test_busy = true;
                self.test = None;
                self.test_proxy_name = None;
                self.job_test(Some(cfg), true);
            }
            if btn_ghost(ui, &pal, crate::i18n::tr("Probar sin proxy")).clicked() {
                self.test_busy = true;
                self.test = None;
                self.test_proxy_name = None;
                self.job_test(None, false);
            }
            if btn_ghost(ui, &pal, crate::i18n::tr("Exportar configuracion")).clicked() {
                let cfg = self.form.build_config();
                self.job_export(cfg);
            }
            if btn_ghost(ui, &pal, crate::i18n::tr("Importar configuracion")).clicked() {
                self.job_import();
            }
        });
        if self.editing_proxy_id.is_some() {
            ui.label(
                egui::RichText::new(crate::i18n::tr("Estas editando un proxy guardado: usa \"Actualizar en la biblioteca\" para conservar los cambios, o desmarca la casilla al guardar para crear una copia nueva."))
                    .size(12.0)
                    .color(pal.faint),
            );
        }

        ui.add_space(8.0);
        if self.test_busy {
            test_box(ui, &pal, "wait", crate::i18n::tr("Probando la conexion..."));
        } else if let Some(r) = &self.test {
            if r.ok {
                test_box(
                    ui,
                    &pal,
                    "ok",
                    &crate::i18n::tf(
                        "Funciona. IP publica de salida: {0} - HTTP {1} - {2} s",
                        &[&r.ip, &r.code, &format!("{:.2}", r.seconds)],
                    ),
                );
            } else {
                let detail = if r.detail.is_empty() {
                    crate::i18n::tr("No se pudo conectar a traves del proxy.").to_string()
                } else {
                    r.detail.clone()
                };
                test_box(ui, &pal, "fail", &detail);
            }
        }

        ui.add_space(6.0);
        // Preferencias
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Preferencias")).size(16.0).strong().color(pal.text));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width() - 70.0);
                    ui.label(egui::RichText::new(crate::i18n::tr("Abrir ProxySend al iniciar sesion")).size(13.5).color(pal.text));
                    ui.label(
                        egui::RichText::new(crate::i18n::tr("Al iniciar sesion se vuelve a aplicar el proxy guardado en escritorio y terminales."))
                            .size(11.5)
                            .color(pal.faint),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut v = self.settings.autostart;
                    if switch(ui, &pal, &mut v).changed() {
                        self.job_autostart(v);
                    }
                });
            });
            ui.separator();
            // v2.3.4: las opciones de bandeja al cerrar o minimizar se
            // retiraron. Cerrar la ventana cierra la app; minimizar solo
            // minimiza. La bandeja sigue en el icono del sistema con sus tres
            // acciones: Mostrar ProxySend, Activar/Desactivar proxy y Salir.
            // v2.3.0: selector de idioma (espanol por defecto, ingles opcional)
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width() - 70.0);
                    ui.label(egui::RichText::new(crate::i18n::tr("Idioma de la aplicacion")).size(13.5).color(pal.text));
                    ui.label(
                        egui::RichText::new(crate::i18n::tr("Cambia entre espanol e ingles; se guarda y se aplica en el momento a toda la app."))
                            .size(11.5)
                            .color(pal.faint),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // los nombres de idioma se muestran en su idioma (sin traducir)
                    let idx = if crate::i18n::is_en() { 1 } else { 0 };
                    if let Some(i) = segmented(ui, &pal, &["Español", "English"], idx) {
                        self.job_set_language(if i == 0 { "es" } else { "en" });
                    }
                });
            });
        });
        ui.add_space(6.0);
        // Reconexion automatica (temporizador)
        card(ui, &pal, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_min_width(ui.available_width() - 70.0);
                    ui.label(egui::RichText::new(crate::i18n::tr("Reconexion automatica")).size(16.0).strong().color(pal.text));
                    ui.label(egui::RichText::new(crate::i18n::tr("Un temporizador comprueba cada cierto tiempo que el proxy activo siga funcionando. Si deja de responder, ProxySend lo vuelve a aplicar solo en escritorio y terminales y lo deja anotado en el Informe. El temporizador solo esta en marcha mientras esta opcion este activada: al abrir la app con la opcion apagada no arranca ningun temporizador."))
                            .size(11.5)
                            .color(pal.faint),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut v = self.settings.reconnect_enabled;
                    if switch(ui, &pal, &mut v).changed() {
                        self.job_set_reconnect(v, self.settings.reconnect_min);
                    }
                });
            });

            if self.settings.reconnect_enabled {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(crate::i18n::tr("Frecuencia de la verificacion")).size(12.5).color(pal.soft));
                ui.add_space(2.0);
                let opciones = store::reconnect_min_options();
                let idx = opciones.iter().position(|(m, _)| *m == self.settings.reconnect_min).unwrap_or(1);
                let etiquetas: Vec<&str> = opciones
                    .iter()
                    .map(|(_, txt)| crate::i18n::tr(txt))
                    .collect();
                if let Some(i) = segmented(ui, &pal, &etiquetas, idx) {
                    self.job_set_reconnect(true, opciones[i].0);
                }
                ui.add_space(6.0);

                // estado de la ultima verificacion
                let st = self.reconnect_state.clone();
                if self.busy.as_deref() == Some(crate::i18n::tr("Verificando el proxy...")) {
                    test_box(ui, &pal, "wait", crate::i18n::tr("Verificando la conexion a traves del proxy..."));
                } else if st.last_check == 0 {
                    test_box(ui, &pal, "wait", crate::i18n::tr("Todavia no se ha hecho ninguna verificacion automatica. Pulsa \"Verificar ahora\" para comprobarlo ya."));
                } else {
                    let cuando = crate::watcher::fecha(st.last_check);
                    match st.ok {
                        Some(true) => test_box(ui, &pal, "ok", &crate::i18n::tf("Ultima verificacion ({0}): {1}", &[&cuando, &st.detail])),
                        Some(false) => test_box(ui, &pal, "fail", &crate::i18n::tf("Ultima verificacion ({0}): {1}", &[&cuando, &st.detail])),
                        None => test_box(ui, &pal, "wait", crate::i18n::tr("Verificacion en curso...")),
                    }
                }
                if btn_ghost(ui, &pal, crate::i18n::tr("Verificar ahora")).clicked() {
                    self.set_busy(crate::i18n::tr("Verificando el proxy..."));
                    self.job_verify_now();
                }
            }
        });
    }

    /* ------------------------- Mis proxies ------------------------- */

    fn p_proxies(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal();
        ui.horizontal(|ui| {
            if btn_primary(ui, &pal, crate::i18n::tr("Nuevo proxy")).clicked() {
                self.editing_proxy_id = None;
                self.form = super::FormState::new();
                self.page = super::Page::Ajustes;
                self.set_toast(crate::i18n::tr("Configura el proxy nuevo y pulsa \"Guardar en la biblioteca\"."), ToastKind::Neutral);
            }
            if btn_ghost(ui, &pal, crate::i18n::tr("Actualizar")).clicked() {
                self.proxies = store::load_settings().proxies.clone();
            }
        });
        ui.add_space(4.0);
        // copia de seguridad de la biblioteca completa
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(crate::i18n::tr("Biblioteca completa (perfiles + ajustes + historial):"))
                    .size(12.5)
                    .color(pal.faint),
            );
            if btn_ghost(ui, &pal, crate::i18n::tr("Exportar biblioteca")).clicked() {
                self.set_busy(crate::i18n::tr("Exportando la biblioteca..."));
                self.job_export_library();
            }
            if btn_ghost(ui, &pal, crate::i18n::tr("Importar biblioteca")).clicked() {
                self.set_busy(crate::i18n::tr("Importando la biblioteca..."));
                self.job_import_library();
            }
        });
        ui.add_space(2.0);

        if self.proxies.is_empty() {
            card(ui, &pal, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(crate::i18n::tr("Todavia no hay proxies guardados."))
                            .size(14.5)
                            .color(pal.soft),
                    );
                    ui.label(
                        egui::RichText::new(crate::i18n::tr("Configura uno en Ajustes y pulsa \"Guardar en la biblioteca\", o crea uno con el boton \"Nuevo proxy\"."))
                            .size(12.5)
                            .color(pal.faint),
                    );
                });
            });
            return;
        }

        let status = self.status.clone();
        for p in self.proxies.clone() {
            let in_use = status
                .as_ref()
                .map(|s| {
                    s.active
                        && self.editing_proxy_id.as_deref() != Some(p.id.as_str())
                        && (s.config.as_ref() == Some(&p.config) || s.active_proxy_name.as_deref() == Some(p.name.as_str()))
                })
                .unwrap_or(false);

            card(ui, &pal, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_min_width(ui.available_width() - 300.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&p.name).size(15.0).strong().color(pal.text));
                            if in_use {
                                egui::Frame::none()
                                    .fill(pal.accent2.gamma_multiply(0.16))
                                    .stroke(Stroke::new(1.0_f32, pal.accent2.gamma_multiply(0.55)))
                                    .rounding(egui::Rounding::same(999.0))
                                    .inner_margin(egui::Margin::symmetric(9.0, 2.0))
                                    .show(ui, |ui| {
                                        ui.label(egui::RichText::new(crate::i18n::tr("En uso")).size(11.5).color(pal.ok_text).strong());
                                    });
                            }
                        });
                        ui.label(egui::RichText::new(summary_text(&p.config)).size(13.0).color(pal.soft));
                        ui.label(egui::RichText::new(proxy_meta_text(&p.config)).size(11.5).color(pal.faint));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if btn_small_danger(ui, &pal, crate::i18n::tr("Eliminar")).clicked() {
                            self.modal = Some(super::Modal::DeleteProxy { id: p.id.clone(), name: p.name.clone() });
                            self.modal_name.clear();
                        }
                        if btn_small_ghost(ui, &pal, crate::i18n::tr("Probar")).clicked() {
                            self.set_toast(crate::i18n::tf("Probando \"{0}\" a traves del proxy...", &[&p.name]), ToastKind::Neutral);
                            self.test_proxy_name = Some(p.name.clone());
                            self.job_test(Some(p.config.clone()), true);
                        }
                        if btn_small_ghost(ui, &pal, crate::i18n::tr("Editar")).clicked() {
                            self.form.fill_from_config(&p.config);
                            self.editing_proxy_id = Some(p.id.clone());
                            self.page = super::Page::Ajustes;
                            self.set_toast(
                                crate::i18n::tf("Editando \"{0}\". Cambia lo que quieras y pulsa \"Actualizar en la biblioteca\".", &[&p.name]),
                                ToastKind::Neutral,
                            );
                        }
                        if btn_small_primary(ui, &pal, crate::i18n::tr("Activar")).clicked() {
                            self.set_busy(crate::i18n::tf("Activando \"{0}\"...", &[&p.name]));
                            self.job_activate_proxy_id(p.id.clone(), p.name.clone());
                        }
                    });
                });
            });
            ui.add_space(4.0);
        }
    }

    /* ------------------------- Historial ------------------------- */

    fn p_historial(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal();
        if !self.history_loaded {
            self.history_cache = store::read_history();
            self.history_loaded = true;
        }
        ui.horizontal(|ui| {
            if btn_ghost(ui, &pal, crate::i18n::tr("Actualizar")).clicked() {
                self.history_cache = store::read_history();
            }
            if btn_danger(ui, &pal, crate::i18n::tr("Limpiar historial")).clicked() {
                store::clear_history();
                store::log_history("limpiar", "");
                self.history_cache = store::read_history();
                self.set_toast(crate::i18n::tr("Historial limpiado."), ToastKind::Success);
            }
        });
        ui.add_space(4.0);

        if self.history_cache.is_empty() {
            card(ui, &pal, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(crate::i18n::tr("Todavia no hay actividad registrada.")).size(14.0).color(pal.soft));
                });
            });
            return;
        }

        card(ui, &pal, |ui| {
            for item in self.history_cache.clone() {
                ui.horizontal(|ui| {
                    let color = super::action_color(&item.action);
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, color);
                    ui.vertical(|ui| {
                        ui.set_min_width(ui.available_width() - 170.0);
                        ui.label(
                            egui::RichText::new(super::action_label(&item.action))
                                .size(13.5)
                                .strong()
                                .color(pal.text),
                        );
                        if !item.detail.is_empty() {
                            ui.label(egui::RichText::new(&item.detail).size(12.0).color(pal.soft));
                        }
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.label(
                            egui::RichText::new(super::format_time(item.ts))
                                .size(11.5)
                                .color(pal.faint),
                        );
                    });
                });
                ui.add_space(6.0);
            }
        });
    }

    /* ------------------------- Informe y diagnostico ------------------------- */

    fn p_informe(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal();
        if !self.events_loaded {
            self.events_cache = crate::eventlog::read_all();
            self.events_loaded = true;
        }

        // resumen diagnostico
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Resumen del diagnostico")).size(16.0).strong().color(pal.text));
            ui.add_space(4.0);
            let (i, a, e) = crate::eventlog::counts();
            ui.horizontal(|ui| {
                chip(ui, &pal, ChipKind::Off, &crate::i18n::tf("{0} informaciones", &[&i.to_string()]));
                let (bg, fg) = if a > 0 {
                    (Color32::from_rgba_unmultiplied(255, 152, 0, 40), Color32::from_rgb(255, 152, 0))
                } else {
                    (pal.card2, pal.soft)
                };
                egui::Frame::none().fill(bg).rounding(egui::Rounding::same(999.0)).inner_margin(egui::Margin::symmetric(10.0, 3.0)).show(ui, |ui| {
                    ui.label(egui::RichText::new(crate::i18n::tf("{0} avisos", &[&a.to_string()])).size(12.5).color(fg).strong());
                });
                let (bg, fg) = if e > 0 {
                    (Color32::from_rgba_unmultiplied(211, 47, 47, 34), pal.danger)
                } else {
                    (pal.card2, pal.soft)
                };
                egui::Frame::none().fill(bg).rounding(egui::Rounding::same(999.0)).inner_margin(egui::Margin::symmetric(10.0, 3.0)).show(ui, |ui| {
                    ui.label(egui::RichText::new(crate::i18n::tf("{0} errores", &[&e.to_string()])).size(12.5).color(fg).strong());
                });
            });
            ui.add_space(4.0);
            match crate::eventlog::last_error() {
                None => ui.label(
                    egui::RichText::new(crate::i18n::tr("No hay ningun error registrado: todo ha ido bien."))
                        .size(12.5)
                        .color(pal.ok_text),
                ),
                Some(ev) => ui.label(
                    egui::RichText::new(crate::i18n::tf("Ultimo error [{0}]: {1}", &[&ev.cat, &ev.msg]))
                        .size(12.5)
                        .color(pal.danger),
                ),
            };
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(crate::i18n::tf(
                    "Escritorio detectado: {0} - version {1} - eventos guardados en ~/.config/ProxySend/eventos.log",
                    &[&sysops_desktop_label(), crate::APP_VERSION],
                ))
                .size(11.5)
                .color(pal.faint),
            );
        });

        ui.add_space(6.0);
        // boton grande de informe
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Informe completo para detectar errores")).size(16.0).strong().color(pal.text));
            ui.label(
                egui::RichText::new(crate::i18n::tr("Genera un archivo de texto con TODO lo que sucede en la app: sistema y escritorio detectado, herramientas disponibles, estado de los tres niveles, biblioteca de proxies, reconexion automatica, historial y el registro completo de eventos con avisos y errores. No incluye contrasenas (se muestran como ****)."))
                    .size(12.0)
                    .color(pal.soft),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if btn_primary(ui, &pal, crate::i18n::tr("Generar informe completo")).clicked() {
                    self.set_busy(crate::i18n::tr("Generando el informe..."));
                    self.job_generate_report();
                }
                if btn_danger(ui, &pal, crate::i18n::tr("Limpiar registro")).clicked() {
                    crate::eventlog::clear();
                    store::clear_history();
                    store::log_history("limpiar", "");
                    self.events_cache = crate::eventlog::read_all();
                    self.set_toast(crate::i18n::tr("Registro de eventos limpiado."), ToastKind::Success);
                }
            });
        });

        ui.add_space(6.0);
        // registro de eventos en vivo
        card(ui, &pal, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(crate::i18n::tr("Registro de eventos (log completo)")).size(16.0).strong().color(pal.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if btn_small_ghost(ui, &pal, crate::i18n::tr("Actualizar")).clicked() {
                        self.events_cache = crate::eventlog::read_all();
                    }
                });
            });
            ui.add_space(4.0);
            let idx = match self.events_filter {
                super::EventFilter::Todos => 0,
                super::EventFilter::Problemas => 1,
            };
            if let Some(i) = segmented(ui, &pal, &[crate::i18n::tr("Todos"), crate::i18n::tr("Avisos y errores")], idx) {
                self.events_filter = if i == 0 { super::EventFilter::Todos } else { super::EventFilter::Problemas };
            }
            ui.add_space(4.0);

            let eventos = self.events_cache.clone();
            let filtrados: Vec<&crate::eventlog::Event> = eventos
                .iter()
                .filter(|ev| match self.events_filter {
                    super::EventFilter::Todos => true,
                    super::EventFilter::Problemas => ev.level != crate::eventlog::Level::Info,
                })
                .collect();

            if filtrados.is_empty() {
                ui.label(
                    egui::RichText::new(if self.events_filter == super::EventFilter::Problemas {
                        crate::i18n::tr("Ningun aviso ni error registrado hasta ahora.")
                    } else {
                        crate::i18n::tr("Todavia no hay eventos registrados.")
                    })
                    .size(13.0)
                    .color(pal.soft),
                );
            } else {
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for ev in &filtrados {
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(Vec2::new(11.0, 11.0), egui::Sense::hover());
                                let color = match ev.level {
                                    crate::eventlog::Level::Info => pal.accent,
                                    crate::eventlog::Level::Aviso => Color32::from_rgb(255, 152, 0),
                                    crate::eventlog::Level::Error => pal.danger,
                                };
                                ui.painter().circle_filled(rect.center(), 4.0, color);
                                ui.vertical(|ui| {
                                    ui.set_min_width(ui.available_width() - 160.0);
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&ev.cat).size(11.5).strong().color(pal.soft));
                                        ui.label(
                                            egui::RichText::new(super::format_time(ev.ts))
                                                .size(11.0)
                                                .color(pal.faint),
                                        );
                                    });
                                    ui.label(egui::RichText::new(&ev.msg).size(12.5).color(pal.text));
                                });
                            });
                            ui.add_space(5.0);
                        }
                    });
            }
        });

        ui.add_space(6.0);
        // verificaciones de calidad de esta version: constancia de la revision
        // completa (campos resaltados, colores, botones Actualizar, idioma y el
        // comportamiento sin contrasena de administrador)
        card(ui, &pal, |ui| {
            ui.label(
                egui::RichText::new(crate::i18n::tr("Verificaciones de calidad de la version"))
                    .size(16.0)
                    .strong()
                    .color(pal.text),
            );
            ui.label(
                egui::RichText::new(crate::i18n::tf(
                    "Comprobaciones hechas sobre la version {0} antes de publicarla. El mismo listado sale en el informe completo (seccion 11).",
                    &[crate::APP_VERSION],
                ))
                .size(12.0)
                .color(pal.soft),
            );
            ui.add_space(4.0);
            for item in [
                crate::i18n::tr("Todos los campos de escritura (servidor, puerto, usuario, contrasena, excepciones y nombre al guardar) tienen fondo y borde resaltado."),
                crate::i18n::tr("Tamano e intensidad de los textos afinados: se lee todo bien en tema claro, incluidos los menus, manteniendo el tema azul."),
                crate::i18n::tr("Botones Actualizar de Mis proxies, Historial e Informe verificados: recargan los datos al pulsarlos."),
                crate::i18n::tr("Activar un proxy SIN contrasena de administrador: se explica el fallo en pantalla y queda como aviso en el registro (no pasa en silencio)."),
                crate::i18n::tr("Activar sin ningun nivel marcado: rechazado con mensaje claro."),
                crate::i18n::tr("Licencia cambiada de MIT a GNU GPL v3 en Acerca de, en Cargo.toml y en el copyright del paquete .deb (texto completo en COPYING.txt)."),
                crate::i18n::tr("Temporizador de reconexion automatica reparado: ya no arranca al ejecutar la app; solo corre al activar la opcion y se detiene de verdad al desactivarla."),
                crate::i18n::tr("Idioma espanol/ingles incorporado: la app arranca en espanol y se cambia en Preferencias; se guarda y aplica a menus, bandeja, informe y consola."),
                crate::i18n::tr("Menus de la bandeja del sistema reparados y probados por D-Bus: Mostrar ProxySend, Activar/Desactivar proxy y Salir responden siempre, con la ventana visible o minimizada."),
                crate::i18n::tr("Activar/Desactivar proxy desde la bandeja muestra el resultado: aviso en pantalla con el detalle de cada nivel y constancia en el registro."),
                crate::i18n::tr("Las opciones de bandeja al cerrar o minimizar se quitaron de Ajustes del proxy (v2.3.4): cerrar la ventana cierra ProxySend y minimizar solo minimiza, sin ocultar nada."),
                crate::i18n::tr("Sin rastros de la funcionalidad retirada: casillas, ajustes guardados, registros, botones y traducciones fuera; los ajustes viejos con esas claves se leen sin error y se descartan."),
                crate::i18n::tr("Abrir ProxySend cuando ya esta corriendo trae la ventana existente al frente (la peticion de la segunda instancia ahora se atiende)."),
                crate::i18n::tr("Revision a fondo del codigo completo: compilacion limpia sin ninguna advertencia del compilador (antes 38), codigo muerto eliminado y una clave de traduccion duplicada corregida."),
                crate::i18n::tr("Icono de la bandeja sincronizado: al activar o desactivar el proxy desde la ventana, el icono y su tooltip se refrescan al momento (antes quedaba con el estado viejo)."),
                crate::i18n::tr("Mostrar ProxySend reparado a fondo: en X11 la ventana minimizada vuelve a pantalla porque ahora se des-minimiza de verdad (antes solo se pedia foco y la ventana seguia abajo); en Wayland se reabre en primer plano con una ventana nueva."),
                crate::i18n::tr("Registro de la bandeja sincero: el informe cuenta la accion real de cada momento (mostrar, activar, desactivar, salir) y ya no anuncia ocultaciones que la app ya no hace."),
                crate::i18n::tr("Simulacion real en X11: al minimizar la ventana por el gestor de ventanas queda abajo de forma normal y Mostrar ProxySend la devuelve a pantalla con foco; no hay ocultacion a la bandeja."),
                crate::i18n::tr("Mostrar ProxySend verificado por D-Bus (dbusmenu) con la ventana minimizada: la ventana vuelve a aparecer; tambien responde la segunda instancia."),
                crate::i18n::tr("Salir desde la bandeja termina el proceso AL INSTANTE (medido en simulacion con la ventana visible y minimizada): sin esperas ni cierre garantizado de respaldo."),
                crate::i18n::tr("Activar y Desactivar proxy desde la bandeja verificados por D-Bus: el nivel de escritorio se aplica de verdad (gsettings) y el resultado queda en historial, registro e informe."),
                crate::i18n::tr("Portabilidad verificada en los 4 entregables (28 comprobaciones): AppImage se extrae y corre sin libfuse2, el .deb solo declara libc6, el tar.gz instala en el HOME de prueba y el .run se autoextrae e instala."),
                crate::i18n::tr("Cero dependencias externas: el binario solo enlaza glibc (libc, libm, libgcc) y las 2 librerias XKB que usa van embebidas en los 4 formatos; el resto es base de cualquier escritorio Linux."),
                crate::i18n::tr("Revision a fondo de la 2.3.5: cero funciones muertas, cero dependencias sin usar y clippy en silencio total."),
                crate::i18n::tr("Regresion re-verificada en simulacion real: menus de la bandeja al instante y las 28 comprobaciones de portabilidad pasadas de nuevo."),
                crate::i18n::tr("Nueva interfaz azul: barra lateral azul marino con iconos dibujados, boton de encendido rediseñado y tarjetas con sombra suave; tema claro y oscuro reafinados en azul."),
                crate::i18n::tr("Revision de codigo muerto con escaneo automatizado de funciones, constantes, dependencias y claves de traduccion: cero restos en toda la base de codigo."),
                crate::i18n::tr("Simulacion E2E real en X11 con bandeja por D-Bus y capturas de pantalla: arranque, cambio de tema, navegacion, activacion del proxy y salida verificados."),
                crate::i18n::tr("Revision a fondo post-entrega: timeout que elimina al proceso externo colgado, handle de bandeja reemplazable, escritura atomica de ajustes e historial y aritmetica de Instant sin panics."),
            ] {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(12.0, 12.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.5, pal.ok_text);
                    ui.label(egui::RichText::new(item).size(12.5).color(pal.text));
                });
                ui.add_space(2.0);
            }
        });
    }

    /* ------------------------- Acerca de ------------------------- */

    fn p_acerca(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal();
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new("ProxySend").size(26.0).strong().color(pal.text));
            ui.label(
                egui::RichText::new(crate::i18n::tf("Version {0} - gestor de proxy para Linux", &[crate::APP_VERSION]))
                    .size(13.0)
                    .color(pal.soft),
            );
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    crate::i18n::tr("Cambia el proxy de tu PC con un clic: agrega, guarda varios proxies en una biblioteca y activa el que quieras a gusto personal, sin tocar archivos a mano. La interfaz se inspira en LocalSend."),
                )
                .size(13.0)
                .color(pal.soft),
            );
        });

        ui.add_space(4.0);
        // tarjeta del autor
        egui::Frame::none()
            .fill(pal.accent_soft)
            .stroke(egui::Stroke::new(1.2_f32, pal.accent))
            .rounding(egui::Rounding::same(14.0))
            .inner_margin(egui::Margin::same(16.0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(crate::i18n::tr("Creado por: Leyter Vázquez . V")).size(16.0).strong().color(pal.accent));
                ui.label(egui::RichText::new(crate::i18n::tr("Autor y desarrollador de ProxySend")).size(12.5).color(pal.soft));
            });

        ui.add_space(4.0);
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Detalles tecnicos")).size(16.0).strong().color(pal.text));
            ui.add_space(2.0);
            for line in [
                crate::i18n::tr("Motor grafico propio en Rust (egui): sin Electron ni Chromium, la app pesa poco y arranca rapido."),
                crate::i18n::tr("Ejecutable en cualquier distribucion Linux moderna, en X11 y Wayland (incluido COSMIC)."),
                crate::i18n::tr("Escritorios soportados: GNOME, KDE (kioslaverc), COSMIC y XFCE, con deteccion automatica del resto."),
                crate::i18n::tr("Indicador del proxy vigente siempre visible: la barra lateral y la pagina Estado muestran que proxy se esta usando, o si no hay ninguno."),
                crate::i18n::tr("Reconexion automatica: un temporizador comprueba el proxy activo cada 1/5/10/30/60 minutos y lo re-aplica si deja de responder."),
                crate::i18n::tr("Biblioteca completa exportable e importable: perfiles, ajustes e historial en un archivo JSON (copia de seguridad)."),
                crate::i18n::tr("Informe diagnostico con el log completo de la app: sistema, escritorio, herramientas, niveles, errores y eventos."),
                crate::i18n::tr("Bandeja del sistema via StatusNotifierItem (D-Bus), sin libappindicator."),
                crate::i18n::tr("Prueba de conexion con motor de red interno TLS, con curl como respaldo."),
                crate::i18n::tr("Configuracion e historial en ~/.config/ProxySend; todo funciona en local, sin telemetria."),
                crate::i18n::tr("Idioma espanol/ingles incorporado: la app arranca en espanol y se cambia en Preferencias; se guarda y aplica a menus, bandeja, informe y consola."),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("-").color(pal.accent).strong());
                    ui.label(egui::RichText::new(line).size(12.5).color(pal.soft));
                });
            }
        });

        ui.add_space(4.0);
        card(ui, &pal, |ui| {
            ui.label(egui::RichText::new(crate::i18n::tr("Licencia y privacidad")).size(16.0).strong().color(pal.text));
            ui.label(
                egui::RichText::new(crate::i18n::tr("Licencia GNU GPL v3. La app no envia datos a ningun servidor: solo cambia la configuracion de tu equipo (las pruebas de IP que tu pulsas consultan api.ipify.org)."))
                    .size(12.5)
                    .color(pal.soft),
            );
        });
    }

    /* ------------------------- Avisos y modales ------------------------- */

    pub fn render_toast(&mut self, ctx: &egui::Context) {
        let pal = self.pal();
        if let Some(t) = &self.toast {
            if t.start.elapsed() > std::time::Duration::from_millis(4200) {
                self.toast = None;
            }
        }
        if let Some(t) = &self.toast {
            let (border, text_color) = match t.kind {
                ToastKind::Success => (pal.accent2, pal.text),
                ToastKind::Error => (pal.danger, pal.text),
                ToastKind::Neutral => (pal.accent, pal.text),
            };
            let text = t.text.clone();
            egui::Area::new(egui::Id::new("toast"))
                .order(egui::Order::Foreground)
                .anchor(Align2::CENTER_BOTTOM, [0.0, -22.0])
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(pal.card)
                        .stroke(egui::Stroke::new(1.4_f32, border))
                        .rounding(egui::Rounding::same(12.0))
                        .inner_margin(egui::Margin::symmetric(16.0, 10.0))
                        .show(ui, |ui| {
                            ui.set_max_width(560.0);
                            ui.label(egui::RichText::new(text).size(13.5).color(text_color));
                        });
                });
        }
    }

    pub fn render_modal(&mut self, ctx: &egui::Context) {
        let pal = self.pal();
        let Some(modal) = self.modal.clone() else { return };
        let screen = ctx.screen_rect();
        let title = match &modal {
            super::Modal::SaveProxy { title, .. } => title.clone(),
            super::Modal::DeleteProxy { .. } => crate::i18n::tr("Eliminar proxy").to_string(),
        };
        let mut do_close = false;
        let mut do_ok = false;
        let mut dim_click: Option<egui::Pos2> = None;
        let mut tarjeta_rect = egui::Rect::NOTHING;

        // Un solo area en primer plano: primero se dibuja el fondo oscurecido y
        // despues la tarjeta. Al estar las dos en la misma capa, la tarjeta
        // queda ENCIMA del fondo y recibe los clics (con la version anterior la
        // capa oscura tapaba la ventana y se comia los clics del boton Guardar).
        egui::Area::new(egui::Id::new("modal_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.left_top())
            .interactable(true)
            .show(ctx, |ui| {
                // 1) capa oscura sobre toda la pantalla
                ui.painter_at(screen).rect_filled(screen, 0.0, Color32::from_black_alpha(110));
                // el clic que no se coman la tarjeta o sus botones llega aqui
                let dim = ui.interact(screen, egui::Id::new("modal_dim"), egui::Sense::click());
                if dim.clicked() {
                    dim_click = dim.interact_pointer_pos();
                }

                // 2) tarjeta centrada (se dibuja despues, queda por encima)
                let ancho = 480.0;
                let mut card_ui = ui.new_child(
                    egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                        egui::pos2(screen.center().x - ancho / 2.0, screen.center().y - 190.0),
                        egui::vec2(ancho, 380.0),
                    )),
                );
                let frame = egui::Frame::none()
                    .fill(pal.card)
                    .stroke(egui::Stroke::new(1.2_f32, pal.line))
                    .rounding(egui::Rounding::same(16.0))
                    .inner_margin(egui::Margin::same(18.0))
                    .show(&mut card_ui, |ui| {
                        ui.set_min_width(ancho - 36.0);
                        ui.label(egui::RichText::new(&title).size(17.0).strong().color(pal.text));
                        ui.add_space(6.0);
                        match &modal {
                            super::Modal::SaveProxy { hint, show_save_as_new, .. } => {
                                ui.label(egui::RichText::new(hint).size(13.0).color(pal.soft));
                                ui.add_space(8.0);
                                field_label(ui, &pal, crate::i18n::tr("Nombre del proxy"));
                                ui.label(
                                    egui::RichText::new(crate::i18n::tr("Ejemplos: Casa - 3128, Trabajo - Squid, VPN del club"))
                                        .size(11.0)
                                        .color(pal.faint),
                                );
                                let resp = text_field(ui, &pal, &mut self.modal_name, ancho - 36.0, false);
                                // foco automatico la primera vez que se abre
                                if !self.modal_focus_done {
                                    resp.request_focus();
                                    self.modal_focus_done = true;
                                }
                                // Enter guarda: la barra de una sola linea ENTREGA el
                                // foco en el mismo frame en que llega Enter (egui 0.30
                                // hace surrender_focus con la tecla Enter), por eso la
                                // comprobacion correcta es lost_focus + Enter. Con el
                                // has_focus de antes Enter no guardaba nunca.
                                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                    do_ok = true;
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    do_close = true;
                                }
                                if *show_save_as_new {
                                    ui.add_space(2.0);
                                    let mut v = self.save_as_new;
                                    if check(ui, &pal, &mut v, crate::i18n::tr("Guardar como proxy nuevo (no actualizar el actual)")).changed() {
                                        self.save_as_new = v;
                                    }
                                }
                            }
                            super::Modal::DeleteProxy { name, .. } => {
                                ui.label(
                                    egui::RichText::new(crate::i18n::tf(
                                        "Se eliminara \"{0}\" de la biblioteca. El proxy que este aplicado en el sistema no se desactiva (para eso usa Estado).",
                                        &[name],
                                    ))
                                    .size(13.0)
                                    .color(pal.soft),
                                );
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    do_close = true;
                                }
                            }
                        }
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if btn_ghost(ui, &pal, crate::i18n::tr("Cancelar")).clicked() {
                                do_close = true;
                            }
                            match &modal {
                                super::Modal::SaveProxy { .. } => {
                                    if btn_primary(ui, &pal, crate::i18n::tr("Guardar")).clicked() {
                                        do_ok = true;
                                    }
                                }
                                super::Modal::DeleteProxy { .. } => {
                                    if btn_danger(ui, &pal, crate::i18n::tr("Eliminar")).clicked() {
                                        do_ok = true;
                                    }
                                }
                            }
                        });
                    });
                // rectangulo REAL de la tarjeta pintada (para saber si un clic
                // en el fondo oscuro cayo fuera de ella)
                tarjeta_rect = frame.response.rect;
            });

        // clic en el fondo oscuro: cerrar SOLO si fue fuera de la tarjeta
        if let Some(pos) = dim_click {
            if !tarjeta_rect.contains(pos) {
                do_close = true;
            }
        }
        if do_close {
            self.modal = None;
        } else if do_ok {
            self.modal_confirm();
        }
    }

    fn modal_confirm(&mut self) {
        let Some(m) = self.modal.clone() else { return };
        match m {
            super::Modal::DeleteProxy { id, name } => {
                self.modal = None;
                self.job_delete_proxy(id, name);
            }
            super::Modal::SaveProxy { .. } => {
                let name = self.modal_name.trim().to_string();
                if name.is_empty() {
                    self.set_toast(crate::i18n::tr("Escribe un nombre para el proxy."), ToastKind::Error);
                    return;
                }
                let cfg = self.form.build_config();
                let as_new = self.save_as_new;
                let id = if as_new { None } else { self.editing_proxy_id.clone() };
                self.modal = None;
                self.job_save_proxy(id, name, cfg);
            }
        }
    }

    fn open_save_modal(&mut self) {
        let editing = self
            .editing_proxy_id
            .as_ref()
            .and_then(|id| self.proxies.iter().find(|p| &p.id == id))
            .cloned();
        self.modal_focus_done = false;
        match editing {
            Some(e) => {
                self.modal_name = e.name.clone();
                self.modal = Some(super::Modal::SaveProxy {
                    title: crate::i18n::tr("Actualizar el proxy de la biblioteca").to_string(),
                    hint: crate::i18n::tf(
                        "Actualizaras los datos de \"{0}\" con la configuracion actual del formulario.",
                        &[&e.name],
                    ),
                    show_save_as_new: true,
                });
            }
            None => {
                self.modal_name.clear();
                self.save_as_new = false;
                self.modal = Some(super::Modal::SaveProxy {
                    title: crate::i18n::tr("Guardar en la biblioteca").to_string(),
                    hint: crate::i18n::tr("La configuracion actual del formulario se guardara con este nombre.").to_string(),
                    show_save_as_new: false,
                });
            }
        }
    }
}

/* ------------------------- Trabajos en segundo plano ------------------------- */

impl super::ProxySendApp {
    pub fn job_activate(&mut self, config: ProxyConfig, scopes: Scopes, proxy_id: Option<String>, ok_msg: &'static str) {
        self.spawn_job(move || {
            let mut s = store::load_settings();
            let r = ops::apply_proxy_config(&mut s, &config, Some(&scopes), proxy_id.as_deref());
            if r.results.contains_key("validacion") {
                return UiMsg::OpDone {
                    ok: false,
                    toast_ok: String::new(),
                    failures: r.failures(),
                    reload_settings: false,
                    refresh_proxies: false,
                    go_estado: false,
                };
            }
            store::log_history("activar", &r.summary);
            UiMsg::OpDone {
                ok: r.ok(),
                toast_ok: ok_msg.to_string(),
                failures: r.failures(),
                reload_settings: true,
                refresh_proxies: false,
                go_estado: false,
            }
        });
    }

    pub fn job_deactivate(&mut self) {
        self.spawn_job(move || {
            let mut s = store::load_settings();
            let r = ops::remove_proxy_everywhere(&mut s);
            store::log_history("desactivar", crate::i18n::tr("Proxy desactivado en todos los niveles"));
            UiMsg::OpDone {
                ok: r.ok(),
                toast_ok: crate::i18n::tr("Proxy desactivado: el sistema vuelve a la conexion directa.").to_string(),
                failures: r.failures(),
                reload_settings: true,
                refresh_proxies: false,
                go_estado: false,
            }
        });
    }

    pub fn job_reset(&mut self) {
        self.spawn_job(move || {
            let mut s = store::load_settings();
            let r = ops::reset_defaults(&mut s);
            store::log_history(
                "restablecer",
                crate::i18n::tr("Valores por defecto restaurados y proxy quitado (la biblioteca se conserva)"),
            );
            UiMsg::OpDone {
                ok: r.ok(),
                toast_ok: crate::i18n::tr("Proxy quitado: todo restablecido a los valores por defecto.").to_string(),
                failures: r.failures(),
                reload_settings: true,
                refresh_proxies: true,
                go_estado: false,
            }
        });
    }

    pub fn job_save_config(&mut self, cfg: ProxyConfig, scopes: Scopes) {
        self.spawn_job(move || {
            let errors = core::validate_config_errors(&cfg);
            if !errors.is_empty() {
                return UiMsg::OpDone {
                    ok: false,
                    toast_ok: String::new(),
                    failures: errors,
                    reload_settings: false,
                    refresh_proxies: false,
                    go_estado: false,
                };
            }
            let mut s = store::load_settings();
            s.config = Some(cfg.clone());
            s.scopes = scopes;
            store::save_settings(&s);
            store::log_history("guardar", &core::summary(&cfg));
            UiMsg::OpDone {
                ok: true,
                toast_ok: crate::i18n::tr("Configuracion guardada. Usa el boton grande de Estado para activarla.").to_string(),
                failures: Vec::new(),
                reload_settings: true,
                refresh_proxies: false,
                go_estado: false,
            }
        });
    }

    pub fn job_save_and_activate(&mut self, cfg: ProxyConfig, scopes: Scopes) {
        self.spawn_job(move || {
            let errors = core::validate_config_errors(&cfg);
            if !errors.is_empty() {
                return UiMsg::OpDone {
                    ok: false,
                    toast_ok: String::new(),
                    failures: errors,
                    reload_settings: false,
                    refresh_proxies: false,
                    go_estado: false,
                };
            }
            let mut s = store::load_settings();
            let r = ops::apply_proxy_config(&mut s, &cfg, Some(&scopes), None);
            if r.results.contains_key("validacion") {
                return UiMsg::OpDone {
                    ok: false,
                    toast_ok: String::new(),
                    failures: r.failures(),
                    reload_settings: false,
                    refresh_proxies: false,
                    go_estado: false,
                };
            }
            store::log_history("activar", &r.summary);
            UiMsg::OpDone {
                ok: r.ok(),
                toast_ok: crate::i18n::tr("Proxy guardado y activado.").to_string(),
                failures: r.failures(),
                reload_settings: true,
                refresh_proxies: false,
                go_estado: true,
            }
        });
    }

    pub fn job_save_proxy(&mut self, id: Option<String>, name: String, cfg: ProxyConfig) {
        self.spawn_job(move || {
            let errors = core::validate_config_errors(&cfg);
            if !errors.is_empty() {
                return UiMsg::OpDone {
                    ok: false,
                    toast_ok: String::new(),
                    failures: errors,
                    reload_settings: false,
                    refresh_proxies: false,
                    go_estado: false,
                };
            }
            let mut s = store::load_settings();
            let mut found = false;
            if let Some(id) = &id {
                for p in s.proxies.iter_mut() {
                    if &p.id == id {
                        p.name = name.clone();
                        p.config = cfg.clone();
                        found = true;
                        store::log_history(
                            "guardar-biblioteca",
                            &crate::i18n::tf("{0} (actualizado) - {1}", &[&name, &core::summary(&cfg)]),
                        );
                        break;
                    }
                }
            }
            if !found {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                s.proxies.push(store::SavedProxy {
                    id: store::new_proxy_id(),
                    name: name.clone(),
                    config: cfg.clone(),
                    created_at: now,
                });
                store::log_history("guardar-biblioteca", &format!("{} - {}", name, core::summary(&cfg)));
            }
            store::save_settings(&s);
            UiMsg::OpDone {
                ok: true,
                toast_ok: crate::i18n::tf("Proxy \"{0}\" guardado en la biblioteca.", &[&name]),
                failures: Vec::new(),
                reload_settings: true,
                refresh_proxies: true,
                go_estado: false,
            }
        });
    }

    pub fn job_delete_proxy(&mut self, id: String, name: String) {
        self.spawn_job(move || {
            let mut s = store::load_settings();
            let before = s.proxies.len();
            s.proxies.retain(|p| p.id != id);
            let removed = s.proxies.len() < before;
            if s.active_proxy_id.as_deref() == Some(id.as_str()) {
                s.active_proxy_id = None;
            }
            store::save_settings(&s);
            if removed {
                store::log_history("eliminar-biblioteca", &crate::i18n::tf("Proxy \"{0}\" eliminado de la biblioteca", &[&name]));
            }
            UiMsg::OpDone {
                ok: removed,
                toast_ok: crate::i18n::tf("Proxy \"{0}\" eliminado de la biblioteca.", &[&name]),
                failures: if removed { Vec::new() } else { vec![crate::i18n::tr("Ese proxy ya no esta en la biblioteca.").to_string()] },
                reload_settings: true,
                refresh_proxies: true,
                go_estado: false,
            }
        });
    }

    pub fn job_activate_proxy_id(&mut self, id: String, name: String) {
        self.spawn_job(move || {
            let mut s = store::load_settings();
            let entry = s.proxies.iter().find(|p| p.id == id).cloned();
            match entry {
                None => UiMsg::OpDone {
                    ok: false,
                    toast_ok: String::new(),
                    failures: vec![crate::i18n::tr("Ese proxy ya no esta en la biblioteca.").to_string()],
                    reload_settings: true,
                    refresh_proxies: true,
                    go_estado: false,
                },
                Some(p) => {
                    let r = ops::apply_proxy_config(&mut s, &p.config, None, Some(&p.id));
                    if r.results.contains_key("validacion") {
                        return UiMsg::OpDone {
                            ok: false,
                            toast_ok: String::new(),
                            failures: r.failures(),
                            reload_settings: false,
                            refresh_proxies: false,
                            go_estado: false,
                        };
                    }
                    store::log_history("activar-biblioteca", &format!("{} - {}", p.name, core::summary(&p.config)));
                    UiMsg::OpDone {
                        ok: r.ok(),
                        toast_ok: crate::i18n::tf("Proxy \"{0}\" activado en los niveles seleccionados.", &[&name]),
                        failures: r.failures(),
                        reload_settings: true,
                        refresh_proxies: true,
                        go_estado: false,
                    }
                }
            }
        });
    }

    pub fn job_test(&mut self, cfg: Option<ProxyConfig>, use_proxy: bool) {
        self.spawn_job(move || {
            let r = crate::nettest::test_connection(cfg.as_ref(), use_proxy);
            if use_proxy {
                if let Some(c) = &cfg {
                    let errors = core::validate_config_errors(c);
                    if errors.is_empty() && r.ok {
                        store::log_history("probar", &crate::i18n::tf("Salida por el proxy: IP {0} en {1} s", &[&r.ip, &format!("{:.2}", r.seconds)]));
                    }
                }
            }
            UiMsg::TestDone(r)
        });
    }

    pub fn job_autostart(&mut self, enabled: bool) {
        self.spawn_job(move || {
            let r = crate::sysops::set_autostart(enabled);
            let mut s = store::load_settings();
            s.autostart = enabled;
            store::save_settings(&s);
            store::log_history(
                "autostart",
                &format!(
                    "{}{}",
                    if enabled { crate::i18n::tr("Autoarranque activado") } else { crate::i18n::tr("Autoarranque desactivado") },
                    if r.detail.is_empty() { String::new() } else { format!(" - {}", r.detail) }
                ),
            );
            UiMsg::OpDone {
                ok: r.ok,
                toast_ok: if r.detail.is_empty() {
                    if enabled { crate::i18n::tr("Autoarranque activado.").to_string() } else { crate::i18n::tr("Autoarranque desactivado.").to_string() }
                } else {
                    r.detail
                },
                failures: Vec::new(),
                reload_settings: true,
                refresh_proxies: false,
                go_estado: false,
            }
        });
    }

    pub fn job_export(&mut self, cfg: ProxyConfig) {
        self.spawn_job(move || {
            let errors = core::validate_config_errors(&cfg);
            if !errors.is_empty() {
                return UiMsg::Exported(Err(errors.join(" ")));
            }
            let path = store::home_dir().join("proxysend-config.json");
            let payload = serde_json::json!({
                "app": "ProxySend",
                "version": crate::APP_VERSION,
                "config": cfg,
            });
            let json = match serde_json::to_string_pretty(&payload) {
                Ok(j) => j,
                Err(e) => return UiMsg::Exported(Err(e.to_string())),
            };
            if let Err(e) = std::fs::write(&path, json) {
                return UiMsg::Exported(Err(crate::i18n::tf("No se pudo escribir el archivo: {0}", &[&e.to_string()])));
            }
            UiMsg::Exported(Ok(path.to_string_lossy().to_string()))
        });
    }

    pub fn job_import(&mut self) {
        self.spawn_job(move || {
            // elegir archivo con zenity/kdialog si existen; si no, ruta por defecto
            let mut path: Option<String> = None;
            for (cmd, args) in [
                ("zenity", vec!["--file-selection", &format!("--title={}", crate::i18n::tr("Importar configuracion de ProxySend"))]),
                ("kdialog", vec!["--getopenfilename", ".", "*.json", "--title", crate::i18n::tr("Importar configuracion de ProxySend")]),
            ] {
                if crate::sysops::tool_exists(cmd) {
                    if let Ok(out) = std::process::Command::new(cmd).args(&args).output() {
                        if out.status.success() {
                            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            if !p.is_empty() {
                                path = Some(p);
                            }
                        }
                    }
                    break;
                }
            }
            let path = match path {
                Some(p) => p,
                None => store::home_dir().join("proxysend-config.json").to_string_lossy().to_string(),
            };
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => return UiMsg::Imported(Err(crate::i18n::tf("No se pudo leer {0}: {1}", &[&path, &e.to_string()]))),
            };
            let data: serde_json::Value = match serde_json::from_str(&text) {
                Ok(d) => d,
                Err(e) => return UiMsg::Imported(Err(crate::i18n::tf("Archivo JSON invalido: {0}", &[&e.to_string()]))),
            };
            let cfg_val = data.get("config").cloned().unwrap_or(data);
            let cfg: ProxyConfig = match serde_json::from_value(cfg_val) {
                Ok(c) => c,
                Err(e) => return UiMsg::Imported(Err(crate::i18n::tf("El archivo no contiene una configuracion valida: {0}", &[&e.to_string()]))),
            };
            let errors = core::validate_config_errors(&cfg);
            if !errors.is_empty() {
                return UiMsg::Imported(Err(crate::i18n::tf("El archivo no contiene una configuracion valida: {0}", &[&errors.join(" ")])));
            }
            store::log_history("importar", &path);
            UiMsg::Imported(Ok(cfg))
        });
    }

    pub fn job_generate_report(&mut self) {
        self.spawn_job(move || {
            let s = store::load_settings();
            match crate::report::write_report(&s) {
                Ok(path) => UiMsg::ReportDone(Ok(path)),
                Err(e) => UiMsg::ReportDone(Err(e)),
            }
        });
    }

    pub fn job_export_library(&mut self) {
        self.spawn_job(move || {
            let s = store::load_settings();
            match crate::libexport::export_library(&s) {
                Ok(path) => UiMsg::LibraryExported(Ok(path)),
                Err(e) => UiMsg::LibraryExported(Err(e)),
            }
        });
    }

    pub fn job_import_library(&mut self) {
        self.spawn_job(move || {
            // elegir archivo con zenity/kdialog si existen; si no, la biblioteca mas reciente en HOME
            let mut path: Option<String> = None;
            for (cmd, args) in [
                ("zenity", vec!["--file-selection", &format!("--title={}", crate::i18n::tr("Importar biblioteca completa de ProxySend"))]),
                ("kdialog", vec!["--getopenfilename", ".", "*.json", "--title", crate::i18n::tr("Importar biblioteca completa de ProxySend")]),
            ] {
                if crate::sysops::tool_exists(cmd) {
                    if let Ok(out) = std::process::Command::new(cmd).args(&args).output() {
                        if out.status.success() {
                            let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                            if !p.is_empty() {
                                path = Some(p);
                            }
                        }
                    }
                    break;
                }
            }
            let path = match path {
                Some(p) => p,
                None => match crate::libexport::latest_library_in_home() {
                    Some(p) => p,
                    None => {
                        return UiMsg::LibraryImported(Err(
                            crate::i18n::tr("No se encontro ninguna biblioteca (archivo ProxySend-biblioteca-*.json) en tu carpeta personal, ni zenity/kdialog para buscarla. Genera primero una con Exportar biblioteca.").into(),
                        ));
                    }
                },
            };
            match crate::libexport::import_library(&path) {
                Ok(r) => {
                    let mut msg = crate::i18n::tf(
                        "Biblioteca importada desde {0}: {1} proxies, {2} eventos de historial y los ajustes restaurados.",
                        &[&r.path, &r.proxies.to_string(), &r.historial.to_string()],
                    );
                    if !r.advertencias.is_empty() {
                        msg += &crate::i18n::tf(" Avisos: {0}", &[&r.advertencias.join("; ")]);
                    }
                    UiMsg::LibraryImported(Ok(msg))
                }
                Err(e) => UiMsg::LibraryImported(Err(e)),
            }
        });
    }

    pub fn job_set_reconnect(&mut self, enabled: bool, minutos: u64) {
        self.spawn_job(move || {
            let mut s = store::load_settings();
            s.reconnect_enabled = enabled;
            s.reconnect_min = minutos.max(1);
            store::save_settings(&s);
            // v2.2.4: arrancar/parar el temporizador de verdad segun el interruptor.
            // Al desactivar la opcion el temporizador se detiene de verdad y al
            // activarla arranca en el momento (primera verificacion en el siguiente tic).
            crate::watcher::sync_timer();
            crate::eventlog::info(
                "reconexion",
                &if enabled {
                    crate::i18n::tf("Temporizador activado: cada {0} min", &[&s.reconnect_min.to_string()])
                } else {
                    crate::i18n::tf("Temporizador desactivado: cada {0} min", &[&s.reconnect_min.to_string()])
                },
            );
            store::log_history(
                "reconexion",
                &if enabled {
                    crate::i18n::tf("Reconexion automatica activada: cada {0} minutos", &[&s.reconnect_min.to_string()])
                } else {
                    crate::i18n::tr("Reconexion automatica desactivada").to_string()
                },
            );
            UiMsg::OpDone {
                ok: true,
                toast_ok: if enabled {
                    crate::i18n::tf("Reconexion automatica activada: verificacion cada {0} min.", &[&s.reconnect_min.to_string()])
                } else {
                    crate::i18n::tr("Reconexion automatica desactivada.").to_string()
                },
                failures: Vec::new(),
                reload_settings: true,
                refresh_proxies: false,
                go_estado: false,
            }
        });
    }

    /// v2.3.0: cambia el idioma de la app ("es"/"en"), lo guarda y lo aplica
    /// al momento: el selector de Preferencias es la unica via de cambio.
    pub fn job_set_language(&mut self, code: &'static str) {
        crate::i18n::set_code(code);
        self.settings.language = Some(code.to_string());
        let mut s = store::load_settings();
        s.language = Some(code.to_string());
        store::save_settings(&s);
        store::log_history(
            "idioma",
            if code == "en" {
                crate::i18n::tr("Idioma cambiado a ingles")
            } else {
                crate::i18n::tr("Idioma cambiado a espanol")
            },
        );
        self.set_toast(
            if code == "en" {
                crate::i18n::tr("Idioma cambiado a ingles").to_string()
            } else {
                crate::i18n::tr("Idioma cambiado a espanol").to_string()
            },
            ToastKind::Success,
        );
    }

    pub fn job_verify_now(&mut self) {
        self.spawn_job(move || {
            let (ok, detalle) = crate::watcher::check_once();
            UiMsg::VerifyDone(ok, detalle)
        });
    }
}

/* ------------------------- Textos auxiliares ------------------------- */

// (los helpers de trabajos estan mas abajo, separados de las paginas)

pub fn sysops_desktop_label() -> String {
    use crate::sysops::{detect_desktop_env, desktop_backend, DetectedDesktop};
    let de = detect_desktop_env();
    let backend = desktop_backend();
    let nombre = match de {
        DetectedDesktop::Gnome => "GNOME",
        DetectedDesktop::Cosmic => "COSMIC",
        DetectedDesktop::Kde => "KDE",
        DetectedDesktop::Xfce => "XFCE",
        DetectedDesktop::Other => crate::i18n::tr("otro"),
    };
    match backend {
        Some(crate::sysops::Backend::Kde) => crate::i18n::tf("{0} (ajustes de KDE)", &[nombre]),
        Some(crate::sysops::Backend::Gnome) => crate::i18n::tf("{0} (ajustes de GNOME)", &[nombre]),
        None => crate::i18n::tf("{0} (solo niveles sistema y terminal)", &[nombre]),
    }
}

pub fn summary_text(config: &ProxyConfig) -> String {
    let proto = if config.protocol == "socks5" { "SOCKS5".to_string() } else { config.protocol.to_uppercase() };
    let user = if config.auth && !config.username.is_empty() {
        format!("{}@", config.username)
    } else {
        String::new()
    };
    if config.protocol == "socks5" || config.same_for_all {
        format!("{} - {}{}:{}", proto, user, config.host, config.port)
    } else {
        format!(
            "{} - HTTP {}:{} / HTTPS {}:{}",
            proto, config.http.host, config.http.port, config.https.host, config.https.port
        )
    }
}

pub fn proxy_meta_text(config: &ProxyConfig) -> String {
    let mut bits: Vec<String> = Vec::new();
    bits.push(if config.protocol == "socks5" { "SOCKS5".to_string() } else { config.protocol.to_uppercase() });
    if config.auth && !config.username.is_empty() {
        bits.push(crate::i18n::tf("con usuario {0}", &[&config.username]));
    }
    if config.protocol != "socks5" && !config.same_for_all {
        bits.push(crate::i18n::tr("HTTP y HTTPS separados").to_string());
    }
    let n = config.ignore_hosts.len();
    if n > 0 {
        bits.push(crate::i18n::tf(
            if n == 1 { "{0} excepcion" } else { "{0} excepciones" },
            &[&n.to_string()],
        ));
    }
    bits.join(" · ")
}

// ayuda de la interfaz para dibujar una fila servidor+puerto en una sola linea:
// los 8 parametros son campos intencionales de la fila (permitido por revision)
#[allow(clippy::too_many_arguments)]
fn field_row(
    ui: &mut egui::Ui,
    pal: &super::Palette,
    label1: &str,
    value1: &mut String,
    ejemplo: &str,
    port_width: f32,
    value2: &mut String,
    label2: &str,
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            field_label(ui, pal, label1);
            // el ejemplo va ENCIMA de la barra, la barra queda en blanco
            ui.label(
                egui::RichText::new(crate::i18n::tf("Ejemplo: {0}", &[ejemplo]))
                    .size(11.0)
                    .color(pal.faint),
            );
            text_field(ui, pal, value1, ui.available_width() - port_width - 16.0, false);
        });
        ui.vertical(|ui| {
            field_label(ui, pal, label2);
            ui.label(
                egui::RichText::new(crate::i18n::tr("Ejemplo: 3128"))
                    .size(11.0)
                    .color(pal.faint),
            );
            text_field(ui, pal, value2, port_width, false);
        });
    });
}
