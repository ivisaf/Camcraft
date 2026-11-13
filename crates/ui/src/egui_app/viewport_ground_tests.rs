//! #698 pinning: the viewport ground follows the live theme, the dark
//! column keeps the house inks byte for byte, the light ground's overlay,
//! move, wire and axis inks clear WCAG 1.4.11's 3:1 against it, and the
//! hand-tuned light set (paths, cube, hover) is pinned as chosen.
//! Provenance: our own tests over our own palettes (`ground_ink`).

use egui::Color32;

use super::colorvision::{MIN_DE00, Semantic, ink, palette_for};
use super::ground_ink::{Overlay, cube_ink, dimmed, light_ground, overlay_ink, wire_ink};
use super::{contrast, cvd, theme::Tokens};
use camcraft_app::schema::ColorVision;

const OVERLAYS: [Overlay; 5] = [Overlay::Region, Overlay::Retract, Overlay::Cut, Overlay::Pick, Overlay::Hover];

/// WCAG 1.4.11: graphical objects need 3:1 against their ground.
const GRAPHIC: f64 = 3.0;

#[test]
fn the_light_viewport_is_light_and_the_dark_one_stays_dark() {
    assert!(light_ground(Tokens::LIGHT.viewport), "light column's viewport must read light");
    assert!(!light_ground(Tokens::DARK.viewport), "dark column's viewport must stay dark");
    assert_ne!(Tokens::LIGHT.viewport, Tokens::DARK.viewport);
    // The dark ground is the clear colour the viewport always drew
    // (0.09, 0.10, 0.12 on an 8-bit target), so the dark look is unchanged.
    assert_eq!(Tokens::DARK.viewport, Color32::from_rgb(23, 26, 31));
}

#[test]
fn every_overlay_ink_clears_3_to_1_on_the_light_viewport() {
    let ground = Tokens::LIGHT.viewport;
    for o in OVERLAYS {
        let r = contrast::ratio(ink(overlay_ink(o, true)), ground);
        assert!(r >= GRAPHIC, "{o:?} on the light viewport reads {r:.2}:1, under {GRAPHIC}:1");
    }
}

#[test]
fn the_light_palette_move_inks_clear_3_to_1() {
    let p = &Semantic::OFF_LIGHT_GROUND;
    let ground = Tokens::LIGHT.viewport;
    for (name, c) in [("feed", p.feed), ("rapid", p.rapid), ("plunge", p.plunge)] {
        let r = contrast::ratio(ink(c), ground);
        assert!(r >= GRAPHIC, "{name} on the light viewport reads {r:.2}:1, under {GRAPHIC}:1");
    }
}

#[test]
fn the_light_palette_is_the_hand_tuned_set() {
    // Chosen on screen with the reporter (#698): a deep feed green, the
    // rapid blue and plunge red measured off their hand-drawn strokes,
    // and saturated boundary roles (these trade 3:1 for hue; see the
    // decision note).
    let p = &Semantic::OFF_LIGHT_GROUND;
    assert_eq!(p.feed, [0.08, 0.36, 0.12]);
    assert_eq!(p.rapid, [0.01, 0.31, 0.90]);
    assert_eq!(p.plunge, [0.90, 0.13, 0.13]);
    assert_eq!(p.bound_include, [0.10, 0.80, 1.00]);
    assert_eq!(p.bound_exclude, [1.00, 0.20, 0.70]);
    assert_eq!(p.bound_open, [1.00, 0.65, 0.05]);
}

#[test]
fn a_dimmed_path_fades_toward_a_light_ground_and_darkens_on_a_dark_one() {
    let red = [0.90, 0.13, 0.13];
    assert_eq!(dimmed(red, 1.0, Tokens::LIGHT.viewport), red, "a selected path is untouched");
    assert_eq!(dimmed(red, 0.45, Tokens::DARK.viewport), [0.90 * 0.45, 0.13 * 0.45, 0.13 * 0.45]);
    let faded = dimmed(red, 0.45, Tokens::LIGHT.viewport);
    let g = Tokens::LIGHT.viewport;
    let ground = [g.r(), g.g(), g.b()].map(|c| f32::from(c) / 255.0);
    for i in 0..3 {
        let (lo, hi) = (red[i].min(ground[i]), red[i].max(ground[i]));
        assert!(faded[i] >= lo && faded[i] <= hi, "channel {i} lies between the ink and the ground");
    }
    assert!(faded[0] > faded[1] + 0.2, "the faded red is still red: {faded:?}");
}

#[test]
fn the_light_cube_sits_a_step_under_the_viewport_with_darker_edges() {
    let k = cube_ink(true);
    let ground = Tokens::LIGHT.viewport;
    let brightest = k.face_base + k.face_span;
    assert!(brightest < f32::from(ground.r().min(ground.g()).min(ground.b())), "faces darker than the ground");
    assert!(f32::from(k.edge.r()) < k.face_base, "edges darker than the faces");
    for (what, c) in [("label", k.label), ("hint", k.hint), ("glyph", k.glyph_off)] {
        let r = contrast::ratio(c, Color32::from_gray(k.button));
        assert!(r >= GRAPHIC, "{what} on the light cube button reads {r:.2}:1");
    }
    let r = contrast::ratio(k.hint, ground);
    assert!(r >= contrast::BODY, "the corner hint reads {r:.2}:1 on the light viewport");
    for m in k.marker {
        assert!(contrast::ratio(m, ground) >= GRAPHIC, "the tool marker reads on the light viewport");
    }
}

#[test]
fn the_dark_cube_keeps_its_greys() {
    let k = cube_ink(false);
    assert_eq!((k.face_base, k.face_span, k.hover_lift, k.button), (46.0, 46.0, 52, 46));
    assert_eq!(k.edge, Color32::from_white_alpha(170));
    assert_eq!(k.label, Color32::from_white_alpha(230));
    assert_eq!(k.hint, Color32::from_white_alpha(200));
    assert_eq!(k.marker, [Color32::from_gray(225), Color32::from_gray(240), Color32::WHITE]);
    assert!(!k.outline_buttons);
}

#[test]
fn light_hover_is_the_viewport_grey_with_dark_ink_and_dark_keeps_its_accent() {
    let light = Tokens::LIGHT.visuals();
    assert_eq!(Tokens::LIGHT.field_hover, Tokens::LIGHT.viewport);
    assert_eq!(light.widgets.hovered.weak_bg_fill, Tokens::LIGHT.field_hover);
    let r = contrast::ratio(light.widgets.hovered.fg_stroke.color, light.widgets.hovered.weak_bg_fill);
    assert!(r >= contrast::BODY, "light hover ink reads {r:.2}:1");
    // Title bars wear the window fill, never egui's dark default grey.
    assert_eq!(light.widgets.open.weak_bg_fill, Tokens::LIGHT.ink_alt);
    let dark = Tokens::DARK.visuals();
    assert_eq!(dark.widgets.hovered.weak_bg_fill, Tokens::DARK.accent);
    assert_eq!(dark.widgets.hovered.fg_stroke.color, Color32::WHITE);
}

#[test]
fn the_light_wireframe_and_axis_inks_clear_3_to_1() {
    let ground = Tokens::LIGHT.viewport;
    let r = contrast::ratio(ink(wire_ink(true)), ground);
    assert!(r >= GRAPHIC, "light wire reads {r:.2}:1");
    for (rgb, _) in crate::interact::AXES_LIGHT {
        let r = contrast::ratio(ink(rgb), ground);
        assert!(r >= GRAPHIC, "light axis reads {r:.2}:1");
    }
}

#[test]
fn the_dark_ground_keeps_the_house_inks() {
    assert_eq!(overlay_ink(Overlay::Region, false), crate::interact::REGION_RGB);
    assert_eq!(overlay_ink(Overlay::Retract, false), crate::interact::RETRACT_RGB);
    assert_eq!(overlay_ink(Overlay::Cut, false), crate::interact::CUT_RGB);
    assert_eq!(overlay_ink(Overlay::Pick, false), crate::interact::PICK_RGB);
    assert_eq!(overlay_ink(Overlay::Hover, false), crate::boundary_viz::HOVER_RGB);
    assert_eq!(wire_ink(false), [0.62, 0.66, 0.72]);
    assert_eq!(palette_for(ColorVision::Off, false), &Semantic::OFF);
}

#[test]
fn every_mode_wears_its_light_set_on_a_light_ground() {
    assert_eq!(palette_for(ColorVision::Off, true), &Semantic::OFF_LIGHT_GROUND);
    assert_eq!(palette_for(ColorVision::Protan, true), &Semantic::RED_GREEN_LIGHT_GROUND);
    assert_eq!(palette_for(ColorVision::Deutan, true), &Semantic::RED_GREEN_LIGHT_GROUND);
    assert_eq!(palette_for(ColorVision::Tritan, true), &Semantic::TRITAN_LIGHT_GROUND);
    for mode in ColorVision::ALL {
        assert_eq!(palette_for(mode, false), super::colorvision::palette(mode), "{mode:?} dark is unchanged");
    }
}

/// The reviewer's case on PR #838: a colour-vision user on the light
/// theme must not get unreadable lines. Each remap's light set keeps
/// every must-distinguish pair apart for its own eye(s) (the M13.4
/// CIEDE2000 floor) and every line ink clears 3:1 on the ground.
#[test]
fn the_light_colour_vision_sets_pass_the_cvd_gate_and_clear_3_to_1() {
    let wide = |c: [f32; 3]| [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])];
    let ground = Tokens::LIGHT.viewport;
    for mode in [ColorVision::Protan, ColorVision::Deutan, ColorVision::Tritan] {
        let p = palette_for(mode, true);
        for (name, a, b) in p.must_distinguish() {
            let d = cvd::seen_apart(wide(a), wide(b), mode);
            assert!(d >= MIN_DE00, "{mode:?} light {name}: CIEDE2000 {d:.1} < {MIN_DE00}");
        }
        for (name, c) in [
            ("feed", p.feed),
            ("rapid", p.rapid),
            ("plunge", p.plunge),
            ("bound include", p.bound_include),
            ("bound exclude", p.bound_exclude),
            ("bound open", p.bound_open),
        ] {
            let r = contrast::ratio(ink(c), ground);
            assert!(r >= GRAPHIC, "{mode:?} light {name} reads {r:.2}:1 on the light viewport");
        }
    }
}

#[test]
fn the_dark_title_bar_keeps_eguis_own_open_fill() {
    let dark = Tokens::DARK.visuals();
    let egui_dark = egui::Visuals::dark();
    assert_eq!(dark.widgets.open.weak_bg_fill, egui_dark.widgets.open.weak_bg_fill);
    assert_eq!(dark.widgets.open.fg_stroke, egui_dark.widgets.open.fg_stroke);
}
