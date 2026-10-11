//! Recipe checks: every op builds what it says and says its numbers.

use qymcad_mcp::{
    ArrayStep, BossHeight, BoxSize, ChamferDist, CircleSpec, ConeSize, CutHeight, CylinderSize, ExtrudeHeight, FilletRadius, HolePoint, HoleSize, McpSession, MirrorPlane, MoveBy, PrismSize, RecipeOp,
    RectSize, RevolveSpec, SphereRadius, TorusSize,
};

/// The measured volume is within `rel` of the exact one (tessellation is approximate).
fn within(volume: f64, exact: f64, rel: f64) -> bool {
    (volume - exact).abs() <= exact * rel
}

/// The last body of a recipe that built cleanly.
fn last_built(session: &mut McpSession, ops: &[RecipeOp]) -> qymcad_mcp::BodySummary {
    let result = session.apply(ops).expect("the recipe builds");
    assert!(result.errors.is_empty(), "a clean build reports no errors: {:?}", result.errors);
    result.bodies.last().cloned().expect("a recipe leaves a body")
}

/// A 40 x 30 rectangle extruded 10 up holds 12000 mm^3.
#[test]
fn a_rect_extruded_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let result = session.apply(&[RecipeOp::RectSketch(RectSize { width: 40.0, height: 30.0 }), RecipeOp::Extrude(ExtrudeHeight { height: 10.0 })]).expect("a rectangle extruded 10 builds");
    assert_eq!(result.bodies.len(), 1, "one sketch and one extrude make one body");
    let volume = result.bodies[0].volume_mm3;
    assert!((volume - 12000.0).abs() < 1.0, "the block holds 12000 mm^3, measured {volume}");
    assert!(result.errors.is_empty(), "a clean build reports no errors: {:?}", result.errors);
}

/// A 10 x 20 x 30 box holds 6000 mm^3.
#[test]
fn a_box_primitive_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let result = session.apply(&[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 20.0, dz: 30.0 })]).expect("a box builds");
    let volume = result.bodies[0].volume_mm3;
    assert!((volume - 6000.0).abs() < 1.0, "the box holds 6000 mm^3, measured {volume}");
}

/// A cylinder of r 5 and h 10 holds pi r^2 h.
#[test]
fn a_cylinder_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::CylinderOp(CylinderSize { r: 5.0, h: 10.0 })]);
    assert!(within(body.volume_mm3, 785.398, 0.02), "a cylinder holds 785 mm^3, measured {}", body.volume_mm3);
}

/// A sphere of r 5 holds 4/3 pi r^3.
#[test]
fn a_sphere_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::SphereOp(SphereRadius { r: 5.0 })]);
    assert!(within(body.volume_mm3, 523.599, 0.03), "a sphere holds 524 mm^3, measured {}", body.volume_mm3);
}

/// A cone of base 6 to a point over h 12 holds a third of pi r^2 h.
#[test]
fn a_cone_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::ConeOp(ConeSize { r1: 6.0, r2: 0.0, h: 12.0 })]);
    assert!(within(body.volume_mm3, 452.389, 0.03), "a cone holds 452 mm^3, measured {}", body.volume_mm3);
}

/// A torus of major 10 and minor 2 holds 2 pi^2 R r^2.
#[test]
fn a_torus_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::TorusOp(TorusSize { major: 10.0, minor: 2.0 })]);
    assert!(within(body.volume_mm3, 789.568, 0.03), "a torus holds 790 mm^3, measured {}", body.volume_mm3);
}

/// A hexagonal prism of circumradius 5 and h 10 holds its exact volume.
#[test]
fn a_prism_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::PrismOp(PrismSize { r: 5.0, sides: 6, h: 10.0 })]);
    assert!(within(body.volume_mm3, 649.519, 0.02), "a prism holds 650 mm^3, measured {}", body.volume_mm3);
}

/// A prism of two sides is refused with its field.
#[test]
fn a_digon_prism_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session.apply(&[RecipeOp::PrismOp(PrismSize { r: 5.0, sides: 2, h: 10.0 })]).expect_err("a two-sided prism builds nothing");
    assert!(message.contains("mcp-prism-sides"), "the refusal names its field: {message}");
}

/// A circle of r 5 extruded 10 up holds pi r^2 h.
#[test]
fn a_circle_extruded_holds_its_volume() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::CircleSketch(CircleSpec { x: 0.0, y: 0.0, r: 5.0 }), RecipeOp::Extrude(ExtrudeHeight { height: 10.0 })]);
    assert!(within(body.volume_mm3, 785.398, 0.02), "an extruded circle holds 785 mm^3, measured {}", body.volume_mm3);
}

/// A circle of r 3 at x 10 revolved full about Y is a torus.
#[test]
fn an_offset_circle_revolved_is_a_torus() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::CircleSketch(CircleSpec { x: 10.0, y: 0.0, r: 3.0 }), RecipeOp::Revolve(RevolveSpec { axis: 1, angle_deg: 360.0 })]);
    assert!(within(body.volume_mm3, 1776.53, 0.04), "a revolved circle holds 1777 mm^3, measured {}", body.volume_mm3);
}

/// A revolve about no world axis is refused with its field.
#[test]
fn a_revolve_about_nowhere_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session
        .apply(&[RecipeOp::CircleSketch(CircleSpec { x: 0.0, y: 0.0, r: 5.0 }), RecipeOp::Revolve(RevolveSpec { axis: 7, angle_deg: 360.0 })])
        .expect_err("a revolve about axis 7 builds nothing");
    assert!(message.contains("mcp-revolve-axis"), "the refusal names its field: {message}");
}

/// A 4 x 4 cut through a 20 mm box leaves 8000 minus the hole.
#[test]
fn a_cut_leaves_a_hole() {
    let mut session = McpSession::new_empty();
    let body =
        last_built(&mut session, &[RecipeOp::BoxOp(BoxSize { dx: 20.0, dy: 20.0, dz: 20.0 }), RecipeOp::RectSketch(RectSize { width: 4.0, height: 4.0 }), RecipeOp::Cut(CutHeight { height: 30.0 })]);
    assert!(within(body.volume_mm3, 7680.0, 0.015), "a cut box holds 7680 mm^3, measured {}", body.volume_mm3);
}

/// A boss on a 10 mm box adds only what stands above it.
#[test]
fn a_boss_adds_what_stands_above() {
    let mut session = McpSession::new_empty();
    let body = last_built(
        &mut session,
        &[
            RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 }),
            RecipeOp::CircleSketch(CircleSpec { x: 0.0, y: 0.0, r: 2.0 }),
            RecipeOp::Boss(BossHeight { height: 15.0 }),
        ],
    );
    assert!(within(body.volume_mm3, 1062.83, 0.02), "a bossed box holds 1063 mm^3, measured {}", body.volume_mm3);
}

/// A chamfer takes material off every edge and nothing else.
#[test]
fn a_chamfer_takes_a_little_off() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 }), RecipeOp::Chamfer(ChamferDist { dist: 1.0 })]);
    assert!(body.volume_mm3 < 1000.0 && body.volume_mm3 > 900.0, "a chamfered box holds just under 1000 mm^3, measured {}", body.volume_mm3);
}

/// A hole of dia 4 through 10 mm stock leaves the bore out.
#[test]
fn a_hole_leaves_a_bore() {
    let mut session = McpSession::new_empty();
    let body = last_built(
        &mut session,
        &[
            RecipeOp::BoxOp(BoxSize { dx: 20.0, dy: 10.0, dz: 10.0 }),
            RecipeOp::HoleSketch(HolePoint { x: 3.0, y: 2.0 }),
            RecipeOp::Hole(HoleSize { diameter: 4.0, depth: 15.0 }),
        ],
    );
    assert!(within(body.volume_mm3, 1874.34, 0.03), "a drilled box holds 1874 mm^3, measured {}", body.volume_mm3);
}

/// A hole on a profile sketch is refused: holes need points.
#[test]
fn a_hole_on_a_profile_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session
        .apply(&[
            RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 }),
            RecipeOp::RectSketch(RectSize { width: 4.0, height: 4.0 }),
            RecipeOp::Hole(HoleSize { diameter: 2.0, depth: 5.0 }),
        ])
        .expect_err("a hole on a profile builds nothing");
    assert!(message.contains("mcp-hole-needs-points"), "the refusal names its cause: {message}");
}

/// An extrude on a hole sketch is refused: tools need a profile.
#[test]
fn an_extrude_on_points_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session.apply(&[RecipeOp::HoleSketch(HolePoint { x: 1.0, y: 1.0 }), RecipeOp::Extrude(ExtrudeHeight { height: 5.0 })]).expect_err("an extrude on points builds nothing");
    assert!(message.contains("mcp-needs-profile-sketch"), "the refusal names its cause: {message}");
}

/// Three copies on a 20 mm step hold three boxes.
#[test]
fn an_array_holds_its_copies() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 }), RecipeOp::LinearArray(ArrayStep { dx: 20.0, dy: 0.0, dz: 0.0, count: 3 })]);
    assert!(within(body.volume_mm3, 3000.0, 0.02), "three boxes hold 3000 mm^3, measured {}", body.volume_mm3);
}

/// More than a thousand copies are refused before the kernel hangs on them.
#[test]
fn a_huge_array_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session
        .apply(&[RecipeOp::BoxOp(BoxSize { dx: 1.0, dy: 1.0, dz: 1.0 }), RecipeOp::LinearArray(ArrayStep { dx: 2.0, dy: 0.0, dz: 0.0, count: 1001 })])
        .expect_err("a thousand copies build nothing fast");
    assert!(message.contains("mcp-array-count"), "the refusal names its field: {message}");
}

/// A mirror about YZ of a centred box keeps its volume.
#[test]
fn a_mirror_keeps_a_centred_box() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 }), RecipeOp::Mirror(MirrorPlane { plane: 2 })]);
    assert!(within(body.volume_mm3, 1000.0, 0.01), "a mirrored box holds 1000 mm^3, measured {}", body.volume_mm3);
}

/// A mirror about no world plane is refused with its field.
#[test]
fn a_mirror_about_nowhere_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session.apply(&[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 }), RecipeOp::Mirror(MirrorPlane { plane: 9 })]).expect_err("a mirror about plane 9 builds nothing");
    assert!(message.contains("mcp-mirror-plane"), "the refusal names its field: {message}");
}

/// A move of 5 mm in X shifts the extent and keeps the volume.
#[test]
fn a_move_shifts_the_extent() {
    let mut session = McpSession::new_empty();
    let body = last_built(&mut session, &[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 }), RecipeOp::Translate(MoveBy { dx: 5.0, dy: 0.0, dz: 0.0 })]);
    assert!((body.volume_mm3 - 1000.0).abs() < 1.0, "a moved box holds 1000 mm^3, measured {}", body.volume_mm3);
    assert!((body.min[0] - 0.0).abs() < 0.01 && (body.max[0] - 10.0).abs() < 0.01, "a move of 5 mm spans x 0 to 10, measured {:?} to {:?}", body.min, body.max);
}

/// An extrude with no sketch says so instead of building nothing.
#[test]
fn an_extrude_without_a_sketch_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session.apply(&[RecipeOp::Extrude(ExtrudeHeight { height: 10.0 })]).expect_err("an extrude with no sketch builds nothing");
    assert!(message.contains("mcp-extrude-without-sketch"), "the refusal names its cause: {message}");
}

/// A negative width is refused with its field.
#[test]
fn a_negative_width_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session.apply(&[RecipeOp::RectSketch(RectSize { width: -40.0, height: 30.0 })]).expect_err("a negative width builds nothing");
    assert!(message.contains("mcp-rect-size"), "the refusal names its field: {message}");
}

/// A fillet with no body is refused.
#[test]
fn a_fillet_without_a_body_is_refused() {
    let mut session = McpSession::new_empty();
    let message = session.apply(&[RecipeOp::Fillet(FilletRadius { radius: 1.0 })]).expect_err("a fillet with no body builds nothing");
    assert!(message.contains("mcp-fillet-without-body"), "the refusal names its cause: {message}");
}

/// A saved document opens back with the same body.
#[test]
fn a_saved_document_opens_back() {
    let mut session = McpSession::new_empty();
    session.apply(&[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 })]).expect("a box builds");
    let path = std::env::temp_dir().join("qymcad_mcp_roundtrip.qcad");
    let path = path.to_string_lossy().to_string();
    session.save(&path).expect("the document saves");
    let reopened = McpSession::open(&path).expect("the saved document opens");
    assert_eq!(reopened.inspect().bodies, 1, "the round trip keeps the one body");
    let _ = std::fs::remove_file(&path);
}

/// One mesh exporter: the session and the path it writes.
type Exporter = fn(&McpSession, &str) -> Result<(), String>;

/// Every mesh exporter writes a non-empty file.
#[test]
fn every_exporter_writes_a_file() {
    let mut session = McpSession::new_empty();
    session.apply(&[RecipeOp::BoxOp(BoxSize { dx: 10.0, dy: 10.0, dz: 10.0 })]).expect("a box builds");
    let dir = std::env::temp_dir();
    let outputs: Vec<(&str, Exporter)> = vec![
        ("qymcad_mcp_probe.stl", McpSession::export_stl),
        ("qymcad_mcp_probe.obj", McpSession::export_obj),
        ("qymcad_mcp_probe.ply", McpSession::export_ply),
        ("qymcad_mcp_probe.glb", McpSession::export_glb),
        ("qymcad_mcp_probe.3mf", McpSession::export_3mf),
        ("qymcad_mcp_probe.amf", McpSession::export_amf),
    ];
    for (name, export) in outputs {
        let path = dir.join(name).to_string_lossy().to_string();
        export(&session, &path).unwrap_or_else(|e| panic!("{name} exports: {e}"));
        let size = std::fs::metadata(&path).expect("an export leaves a file").len();
        assert!(size > 0, "{name} is not empty");
        let _ = std::fs::remove_file(&path);
    }
    let step = dir.join("qymcad_mcp_probe.step").to_string_lossy().to_string();
    session.export_step(&step).expect("STEP exports");
    assert!(std::fs::metadata(&step).expect("STEP leaves a file").len() > 0, "STEP is not empty");
    let _ = std::fs::remove_file(&step);
}
