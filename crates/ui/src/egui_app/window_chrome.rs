//! A floating window's own title row (#698): a collapse arrow, the title
//! and a close button whose hover reads like the OS window's caption
//! buttons - a theme-grey fill behind the arrow, a red fill and a white
//! cross behind close. egui's built-in title bar paints those glyphs bare
//! (no hover fill), so on the light theme a hovered glyph only changed
//! ink; a window that wants the caption look turns `title_bar(false)`
//! and draws this row as its first content.
//!
//! Provenance: our own; the caption-button hover convention (neutral fill,
//! red close) is the common desktop one. No vendor material consulted.

use egui::{Align, Color32, Id, Layout, Rect, Response, RichText, Sense, Stroke, Ui, WidgetInfo, WidgetType, vec2};

use super::theme;

/// The close button's hover fill (the desktop caption-close red).
pub const CLOSE_HOVER: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);

/// Whether the window `id` is collapsed (kept in egui memory, like the
/// built-in title bar's own collapse state).
#[must_use]
pub fn collapsed(ui: &Ui, id: Id) -> bool {
    ui.ctx().data(|d| d.get_temp::<bool>(id.with("collapsed"))).unwrap_or(false)
}

/// One caption button: the hover fill first, then the glyph on top.
fn caption(ui: &mut Ui, close: bool, label: &str, glyph: impl FnOnce(&Ui, Rect, Stroke)) -> Response {
    let h = ui.spacing().interact_size.y;
    let (rect, resp) = ui.allocate_exact_size(vec2(h * 1.3, h), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), label));
    let mut ink = ui.visuals().text_color();
    if resp.hovered() {
        let fill = if close { CLOSE_HOVER } else { theme::current().field_hover };
        ui.painter().rect_filled(rect, 2.0, fill);
        if close {
            ink = Color32::WHITE;
        }
    }
    let side = h * 0.42;
    glyph(ui, Rect::from_center_size(rect.center(), vec2(side, side)), Stroke::new(1.3, ink));
    resp
}

/// The title row: arrow (toggles collapse), title, close (clears `open`).
/// Returns whether the window is collapsed after this frame's clicks.
pub fn title_row(ui: &mut Ui, id: Id, title: &str, open: &mut bool) -> bool {
    let mut folded = collapsed(ui, id);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        let arrow = caption(ui, false, if folded { "Expand window" } else { "Collapse window" }, |ui, r, s| {
            let pts = if folded {
                vec![r.left_top(), r.right_center(), r.left_bottom()]
            } else {
                vec![r.left_top(), r.right_top(), r.center_bottom()]
            };
            ui.painter().add(egui::Shape::convex_polygon(pts, s.color, Stroke::NONE));
        });
        if arrow.clicked() {
            folded = !folded;
            ui.ctx().data_mut(|d| d.insert_temp(id.with("collapsed"), folded));
        }
        ui.label(RichText::new(title).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let close = caption(ui, true, "Close window", |ui, r, s| {
                ui.painter().line_segment([r.left_top(), r.right_bottom()], s);
                ui.painter().line_segment([r.right_top(), r.left_bottom()], s);
            });
            if close.clicked() {
                *open = false;
            }
        });
    });
    folded
}
