//! ProxySend - widgets personalizados de la interfaz.
//! v2.4.0 (paleta azul): tarjetas con sombra suave, boton de encendido
//! rediseñado (arco real con hueco, glow al activar) e iconos de navegacion
//! dibujados con el painter (sin fuentes de iconos).

use super::Palette;
use egui::{Color32, Pos2, Rect, Response, Rounding, Sense, Stroke, Ui, Vec2};

/// Rectangulo con gradiente vertical (c1 arriba, c2 abajo).
pub fn paint_gradient(painter: &egui::Painter, rect: Rect, c1: Color32, c2: Color32) {
    let mut mesh = egui::Mesh::default();
    let lt = rect.left_top();
    let rt = rect.right_top();
    let rb = rect.right_bottom();
    let lb = rect.left_bottom();
    let v0 = mesh.vertices.len() as u32;
    mesh.colored_vertex(lt, c1);
    mesh.colored_vertex(rt, c1);
    mesh.colored_vertex(rb, c2);
    mesh.colored_vertex(lb, c2);
    mesh.add_triangle(v0, v0 + 1, v0 + 2);
    mesh.add_triangle(v0, v0 + 2, v0 + 3);
    painter.add(egui::Shape::mesh(mesh));
}

/// Tarjeta redondeada con sombra suave en tema claro (bordes finos en oscuro).
pub fn card<R>(ui: &mut Ui, pal: &Palette, add: impl FnOnce(&mut Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::none()
        .fill(pal.card)
        .stroke(Stroke::new(1.0_f32, pal.line))
        .rounding(Rounding::same(16.0))
        .shadow(pal.shadow)
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
}

/* ------------------------- Arco (para el simbolo de encendido) ------------------------- */

/// Dibuja un arco de circunferencia (en grados; 0 = derecha, -90 = arriba,
/// sentido horario porque el eje Y de egui apunta hacia abajo).
pub fn arc_stroke(painter: &egui::Painter, center: Pos2, radius: f32, start_deg: f32, end_deg: f32, width: f32, color: Color32) {
    let mut prev: Option<Pos2> = None;
    let steps = 48;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let ang = (start_deg + (end_deg - start_deg) * t).to_radians();
        let p = Pos2::new(center.x + radius * ang.cos(), center.y + radius * ang.sin());
        if let Some(a) = prev {
            painter.line_segment([a, p], Stroke::new(width, color));
        }
        prev = Some(p);
    }
}

/* ------------------------- Botones ------------------------- */

fn button_base(text: &str, fill: Color32, text_color: Color32, stroke: Stroke) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).color(text_color))
        .fill(fill)
        .stroke(stroke)
        .rounding(Rounding::same(10.0))
        .min_size(Vec2::new(0.0, 36.0))
}

pub fn btn_primary(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    ui.add(button_base(text, pal.accent, Color32::WHITE, Stroke::NONE))
}

pub fn btn_secondary(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    ui.add(button_base(text, pal.accent_soft, pal.accent, Stroke::NONE))
}

pub fn btn_ghost(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    ui.add(button_base(text, pal.card, pal.soft, Stroke::new(1.0_f32, pal.line)))
}

pub fn btn_danger(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    ui.add(button_base(
        text,
        Color32::TRANSPARENT,
        pal.danger,
        Stroke::new(1.2_f32, pal.danger),
    ))
}

pub fn btn_small_primary(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(Color32::WHITE).size(13.0))
            .fill(pal.accent)
            .rounding(Rounding::same(9.0))
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

pub fn btn_small_ghost(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(pal.soft).size(13.0))
            .fill(pal.card)
            .stroke(Stroke::new(1.0_f32, pal.line))
            .rounding(Rounding::same(9.0))
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

pub fn btn_small_danger(ui: &mut Ui, pal: &Palette, text: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(text).color(pal.danger).size(13.0))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0_f32, pal.danger))
            .rounding(Rounding::same(9.0))
            .min_size(Vec2::new(0.0, 28.0)),
    )
}

/* ------------------------- Chips de estado ------------------------- */

pub enum ChipKind {
    Ok,
    Off,
    Na,
}

pub fn chip(ui: &mut Ui, pal: &Palette, kind: ChipKind, text: &str) {
    let (bg, fg, stroke) = match kind {
        ChipKind::Ok => (
            pal.accent2.gamma_multiply(0.16),
            if pal.bg.r() < 100 { pal.accent2 } else { pal.ok_text },
            Stroke::new(1.0_f32, pal.accent2.gamma_multiply(0.55)),
        ),
        ChipKind::Off => (pal.card2, pal.soft, Stroke::NONE),
        ChipKind::Na => (pal.card2, pal.faint, Stroke::NONE),
    };
    egui::Frame::none()
        .fill(bg)
        .stroke(stroke)
        .rounding(Rounding::same(999.0))
        .inner_margin(egui::Margin::symmetric(10.0, 3.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(400.0));
            ui.label(egui::RichText::new(text).size(12.5).color(fg).strong());
        });
}

/* ------------------------- Interruptor personalizado ------------------------- */

pub fn switch(ui: &mut Ui, pal: &Palette, checked: &mut bool) -> Response {
    let size = Vec2::new(40.0, 22.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let track = if *checked { pal.accent } else { pal.line };
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(999.0), track);
    let knob_r = 8.0;
    let knob_x = if *checked { rect.right() - knob_r - 3.0 } else { rect.left() + knob_r + 3.0 };
    let knob_c = Pos2::new(knob_x, rect.center().y);
    if *checked {
        // aro luminoso suave alrededor del knob cuando esta encendido
        painter.circle_filled(knob_c, knob_r + 3.5, pal.accent.gamma_multiply(0.28));
    }
    painter.circle_filled(knob_c, knob_r, Color32::WHITE);
    resp
}

/* ------------------------- Control segmentado ------------------------- */

pub fn segmented(ui: &mut Ui, pal: &Palette, options: &[&str], selected: usize) -> Option<usize> {
    let mut clicked = None;
    egui::Frame::none()
        .fill(pal.card2)
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(3.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                for (i, opt) in options.iter().enumerate() {
                    let is_sel = i == selected;
                    let btn = egui::Button::new(egui::RichText::new(*opt).size(13.5).color(if is_sel {
                        Color32::WHITE
                    } else {
                        pal.soft
                    }))
                    .fill(if is_sel { pal.accent } else { Color32::TRANSPARENT })
                    .rounding(Rounding::same(9.0))
                    .min_size(Vec2::new(92.0, 30.0));
                    if ui.add(btn).clicked() {
                        clicked = Some(i);
                    }
                }
            });
        });
    clicked
}

/* ------------------------- Boton grande de encendido ------------------------- */

/// Boton grande redondo de encendido (rediseño v2.4.0): aro con arco real
/// (hueco arriba para el simbolo), relleno suave y glow al estar activo.
pub fn big_toggle(ui: &mut Ui, pal: &Palette, active: bool) -> Response {
    let size = Vec2::new(168.0, 168.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let center = rect.center();
    let painter = ui.painter_at(rect);
    let ring_color = if active { pal.accent } else { pal.line };
    let width = 10.0;
    let r = 56.0;

    if active {
        // doble glow: halo externo suave + relleno interno tenue
        arc_stroke(&painter, center, r + 10.0, -55.0, 235.0, 7.0, pal.accent.gamma_multiply(0.16));
        painter.circle_filled(center, r - width / 2.0 - 3.0, pal.accent.gamma_multiply(0.10));
    }
    // aro principal con hueco arriba (arco de 300 grados)
    arc_stroke(&painter, center, r, -60.0, 240.0, width, ring_color);
    // linea vertical del simbolo de encendido, atravesando el hueco
    let line_top = Pos2::new(center.x, center.y - r - 5.0);
    let line_bot = Pos2::new(center.x, center.y - r * 0.30);
    painter.line_segment([line_top, line_bot], Stroke::new(width, ring_color));
    // puntas redondeadas del simbolo
    painter.circle_filled(line_top, width / 2.0, ring_color);
    painter.circle_filled(line_bot, width / 2.0, ring_color);
    resp
}

/* ------------------------- Iconos de navegacion ------------------------- */

/// Iconos geometricos de la barra lateral, dibujados con el painter.
/// Todos caben en un cuadrado `size` y usan `color` (blanco sobre navy).
pub fn nav_icon(ui: &mut Ui, page: super::Page, center: Pos2, color: Color32, size: f32) {
    let p = ui.painter_at(Rect::from_center_size(center, Vec2::splat(size * 2.0)));
    let s = size / 2.0; // medio lado del area de dibujo (iconos ~14 px)
    let c = center;
    let w = Stroke::new(1.7_f32, color);
    let (l, r, t) = (c.x - s, c.x + s, c.y - s);
    match page {
        super::Page::Estado => {
            // simbolo de encendido en miniatura
            arc_stroke(&p, c, s * 0.72, -60.0, 240.0, 1.7, color);
            p.line_segment(
                [Pos2::new(c.x, c.y - s * 0.85), Pos2::new(c.x, c.y - s * 0.05)],
                Stroke::new(1.7_f32, color),
            );
        }
        super::Page::Ajustes => {
            // deslizadores: 3 lineas con perilla
            for (i, xk) in [0.32_f32, 0.62, 0.45].iter().enumerate() {
                let y = t + s * (0.35 + 1.3 * i as f32);
                p.line_segment([Pos2::new(l, y), Pos2::new(r, y)], w);
                let kx = l + s * 2.0 * xk;
                p.circle_filled(Pos2::new(kx, y), 2.6, color);
            }
        }
        super::Page::Proxies => {
            // capas: dos tarjetas apiladas
            let off = s * 0.42;
            p.rect_filled(
                Rect::from_min_size(Pos2::new(l + off * 0.9, t + off), Vec2::new(s * 2.0 - off * 1.9, s * 2.0 - off * 1.9)),
                Rounding::same(3.0),
                color.gamma_multiply(0.28),
            );
            p.rect(
                Rect::from_min_size(Pos2::new(l, t), Vec2::new(s * 2.0 - off * 1.9, s * 2.0 - off * 1.9)),
                Rounding::same(3.0),
                Color32::TRANSPARENT,
                w,
            );
        }
        super::Page::Historial => {
            // reloj
            p.circle_stroke(c, s * 0.9, w);
            p.line_segment([c, Pos2::new(c.x, c.y - s * 0.55)], Stroke::new(1.6_f32, color));
            p.line_segment([c, Pos2::new(c.x + s * 0.45, c.y + s * 0.12)], Stroke::new(1.6_f32, color));
        }
        super::Page::Informe => {
            // documento con lineas
            p.rect(
                Rect::from_min_size(Pos2::new(l + s * 0.15, t), Vec2::new(s * 1.7, s * 2.0)),
                Rounding::same(3.0),
                Color32::TRANSPARENT,
                w,
            );
            for i in 0..3 {
                let y = t + s * (0.55 + 0.45 * i as f32);
                p.line_segment([Pos2::new(l + s * 0.5, y), Pos2::new(r - s * 0.35, y)], w);
            }
        }
        super::Page::Acerca => {
            // info
            p.circle_stroke(c, s * 0.9, w);
            p.circle_filled(Pos2::new(c.x, c.y - s * 0.45), 1.5, color);
            p.line_segment([Pos2::new(c.x, c.y - s * 0.05), Pos2::new(c.x, c.y + s * 0.5)], Stroke::new(1.8_f32, color));
        }
    }
}

/* ------------------------- Caja de resultado de prueba ------------------------- */

pub fn test_box(ui: &mut Ui, pal: &Palette, kind: &str, text: &str) {
    let (bg, fg, stroke) = match kind {
        "ok" => (
            pal.accent2.gamma_multiply(0.14),
            if pal.bg.r() < 100 { pal.accent2 } else { pal.ok_text },
            Stroke::new(1.0_f32, pal.accent2.gamma_multiply(0.5)),
        ),
        "fail" => (
            pal.danger.gamma_multiply(0.10),
            pal.danger,
            Stroke::new(1.0_f32, pal.danger.gamma_multiply(0.5)),
        ),
        _ => (pal.card2, pal.soft, Stroke::NONE),
    };
    egui::Frame::none()
        .fill(bg)
        .stroke(stroke)
        .rounding(Rounding::same(11.0))
        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new(text).size(13.5).color(fg));
        });
}

/// Etiqueta de campo de formulario
pub fn field_label(ui: &mut Ui, pal: &Palette, text: &str) {
    ui.label(egui::RichText::new(text).size(12.5).color(pal.soft));
}

/// Barra de escritura con resalte suave: fondo levemente tintado y borde
/// visible para que los campos (servidor, puerto, usuario, contrasena) se
/// distingan claramente del fondo de la tarjeta, en tema claro y oscuro.
/// El borde visible lo pone el tema global (apply_theme: inactive.bg_stroke
/// = field_line, con resalte azul al enfocar); aqui se fija el fondo tintado.
pub fn text_field(ui: &mut Ui, pal: &Palette, text: &mut String, width: f32, password: bool) -> egui::Response {
    egui::TextEdit::singleline(text)
        .desired_width(width)
        .font(egui::TextStyle::Body)
        .password(password)
        .background_color(pal.field_bg)
        .show(ui)
        .response
}

/// Area de texto multilínea con el mismo resalte suave (excepciones, etc.).
pub fn text_area(ui: &mut Ui, pal: &Palette, text: &mut String, rows: usize, width: f32) -> egui::Response {
    egui::TextEdit::multiline(text)
        .desired_rows(rows)
        .desired_width(width)
        .font(egui::TextStyle::Monospace)
        .background_color(pal.field_bg)
        .show(ui)
        .response
}

/// Casilla de verificacion con texto (estilo v1.x)
pub fn check(ui: &mut Ui, pal: &Palette, checked: &mut bool, text: &str) -> Response {
    ui.horizontal(|ui| {
        let resp = switch(ui, pal, checked);
        ui.label(egui::RichText::new(text).size(13.5).color(pal.text));
        resp
    })
    .inner
}
