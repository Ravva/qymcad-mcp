//! Recipe checks: a sketch becomes a solid and says its numbers.

use qymcad_mcp::{BoxSize, ExtrudeHeight, FilletRadius, McpSession, RecipeOp, RectSize};

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
