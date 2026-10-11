//! `qymcad-mcp`: a headless recipe engine for AI agents.
//!
//! The window stays out of the loop: a recipe of plain structs (`RecipeOp`) builds on
//! `qymcad_core::model::Project` through the same `add_*` calls the interface delegates to,
//! and `qymcad_testkit::regenerate` rebuilds with the real OCCT kernel. Live B-rep shapes are
//! rebuilt on demand and never cross the API boundary, so the protocol carries only RON/JSON.
//!
//! Only pick-free operations are exposed: everything an op needs travels in the op itself,
//! so no viewport click is ever required. Dress-up ops consume their source body, base ops
//! are united into the part by `finish_base_body`.

use qymcad_core::feature::{Purpose, SketchPlane};
use qymcad_core::model::{HoleTool, Id, ImportNode, PlaneDef, Project, WorkPlane};
use serde::{Deserialize, Serialize};

/// A size along three axes, in mm. One struct rather than three loose `f64`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoxSize {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
}

/// A cylinder by radius and height, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CylinderSize {
    pub r: f64,
    pub h: f64,
}

/// A sphere by radius, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SphereRadius {
    pub r: f64,
}

/// A cone frustum by two radii and height, in mm. Equal radii make a cylinder.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConeSize {
    pub r1: f64,
    pub r2: f64,
    pub h: f64,
}

/// A torus by major and minor radius, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TorusSize {
    pub major: f64,
    pub minor: f64,
}

/// A regular prism by circumradius, side count and height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrismSize {
    pub r: f64,
    pub sides: u32,
    pub h: f64,
}

/// A rectangle on the XY plane, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectSize {
    pub width: f64,
    pub height: f64,
}

/// A circle on the XY plane, in mm. Off-centre circles build bodies of revolution.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CircleSpec {
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

/// One isolated point of a hole sketch on the XY plane, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HolePoint {
    pub x: f64,
    pub y: f64,
}

/// A plain hole by diameter and depth, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct HoleSize {
    pub diameter: f64,
    pub depth: f64,
}

/// An extrusion height, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtrudeHeight {
    pub height: f64,
}

/// A revolve about a world axis in the sketch plane (0 X, 1 Y), in degrees.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RevolveSpec {
    pub axis: u8,
    pub angle_deg: f64,
}

/// A cut through the last body by the last sketch, in mm of tool height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CutHeight {
    pub height: f64,
}

/// A boss grown from the last sketch into the last body, in mm of tool height.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BossHeight {
    pub height: f64,
}

/// A symmetric chamfer distance, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChamferDist {
    pub dist: f64,
}

/// A fillet radius, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FilletRadius {
    pub radius: f64,
}

/// Copies of the last body stepped by a vector, original included.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArrayStep {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
    pub count: u32,
}

/// A mirror about a world plane (0 XY, 1 XZ, 2 YZ), merged into the part.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MirrorPlane {
    pub plane: u8,
}

/// A rigid move of the last body, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MoveBy {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
}

/// The sketch level: the height later sketches are drawn at, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LevelHeight {
    pub z: f64,
}

/// Ruled or smooth walls between loft sections: a switch, so an enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoftShape {
    Ruled,
    Smooth,
}

/// A loft through the last `sections` profile sketches.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LoftSpec {
    pub sections: u32,
    pub shape: LoftShape,
}

/// Copies of the last body about world Z, original included.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CircularSpec {
    pub count: u32,
    pub angle_deg: f64,
}

/// A split of the last body by a world plane (0 XY, 1 XZ, 2 YZ) shifted along its
/// normal. The stock must part into exactly two.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SplitSpec {
    pub plane: u8,
    pub offset: f64,
}

/// An exact file brought in as new parts: STEP or IGES by extension.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportPath {
    pub path: String,
}

/// One step of a recipe. A closed enum rather than a stringly-typed call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecipeOp {
    /// A box primitive at the origin.
    BoxOp(BoxSize),
    /// A cylinder primitive, axis Z, base at z = 0.
    CylinderOp(CylinderSize),
    /// A sphere primitive about the origin.
    SphereOp(SphereRadius),
    /// A cone frustum primitive, axis Z, base at z = 0. Zero top radius is an apex.
    ConeOp(ConeSize),
    /// A torus primitive about Z in the XY plane.
    TorusOp(TorusSize),
    /// A regular prism primitive, axis Z, base at z = 0.
    PrismOp(PrismSize),
    /// A rectangle sketch on XY, centred at the origin.
    RectSketch(RectSize),
    /// A circle sketch on XY. Off-centre circles build bodies of revolution.
    CircleSketch(CircleSpec),
    /// A sketch of one isolated point placing a hole. Keep clear of the origin and other
    /// geometry, which weld the point into themselves so it stops placing anything.
    HoleSketch(HolePoint),
    /// Drill the last sketch's points into the stock standing above the sketch plane.
    Hole(HoleSize),
    /// Extrude the first closed contour of the last sketch.
    Extrude(ExtrudeHeight),
    /// Revolve the last sketch about a world axis.
    Revolve(RevolveSpec),
    /// Cut the last body by the last sketch's first contour.
    Cut(CutHeight),
    /// Grow a boss from the last sketch into the last body.
    Boss(BossHeight),
    /// Chamfer every edge of the last body symmetrically.
    Chamfer(ChamferDist),
    /// Fillet every edge of the last body.
    Fillet(FilletRadius),
    /// A linear pattern of the last body.
    LinearArray(ArrayStep),
    /// Mirror the last body about a world plane and merge.
    Mirror(MirrorPlane),
    /// Move the last body rigidly.
    Translate(MoveBy),
    /// Draw later sketches at this height. Sketches stand on datum planes there.
    Level(LevelHeight),
    /// Loft through the last sections profile sketches.
    Loft(LoftSpec),
    /// A circular pattern of the last body about world Z.
    CircularArray(CircularSpec),
    /// Split the last body by a world plane into two.
    SplitBody(SplitSpec),
    /// Bring a STEP or IGES file in as new parts.
    ImportStep(ImportPath),
}

/// What a sketch holds: a closed profile for tools, or points placing holes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SketchKind {
    Profile,
    Points,
}

/// A sketch the next op continues from, with what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SketchRef {
    id: Id,
    kind: SketchKind,
}

/// A body as numbers: what an agent needs to verify its own work.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodySummary {
    pub id: Id,
    pub name: String,
    pub volume_mm3: f64,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub triangles: usize,
}

/// The outcome of a recipe: the bodies with their measured numbers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecipeResult {
    pub bodies: Vec<BodySummary>,
    pub rebuilt: usize,
    pub errors: Vec<String>,
}

/// The document in a few numbers: what planning starts from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DocInfo {
    pub parts: usize,
    pub bodies: usize,
    pub sketches: usize,
    pub nodes: usize,
    pub bodies_info: Vec<BodySummary>,
}

/// A headless document: the project plus the sketch and body the next op continues from.
pub struct McpSession {
    project: Project,
    last_sketch: Option<SketchRef>,
    last_body: Option<Id>,
    /// Profile sketches in creation order, for lofts spanning several of them.
    profiles: Vec<Id>,
    /// The height later sketches are drawn at, through datum planes.
    level_z: f64,
}

/// How many copies a pattern may hold: enough for any real part, too few to hang the kernel.
const MAX_ARRAY_COUNT: u32 = 1000;

impl McpSession {
    /// An empty document with one active part, ready for a recipe.
    pub fn new_empty() -> Self {
        let mut project = Project::default();
        project.new_document();
        Self { project, last_sketch: None, last_body: None, profiles: Vec::new(), level_z: 0.0 }
    }

    /// Open a `.qcad` file exactly as the window sees it: imports restored, full rebuild.
    pub fn open(path: &str) -> Result<Self, String> {
        let mut project = qymcad_io::load_project(path)?;
        qymcad_testkit::open_like_the_app(&mut project);
        let last_body = project.bodies.last().map(|b| b.id);
        Ok(Self { project, last_sketch: None, last_body, profiles: Vec::new(), level_z: 0.0 })
    }

    /// Save the document as `.qcad`.
    pub fn save(&self, path: &str) -> Result<(), String> {
        qymcad_io::save_project(&self.project, path)
    }

    /// Apply a recipe in order and rebuild once at the end.
    pub fn apply(&mut self, ops: &[RecipeOp]) -> Result<RecipeResult, String> {
        for op in ops {
            self.apply_one(op)?;
        }
        self.rebuild()
    }

    /// What the document holds now, without rebuilding.
    pub fn inspect(&self) -> DocInfo {
        DocInfo {
            parts: self.project.components.len(),
            bodies: self.project.bodies.len(),
            sketches: self.project.sketches.len(),
            nodes: self.project.timeline.len(),
            bodies_info: summarize(&self.project),
        }
    }

    /// The display meshes of every body, for the mesh exporters below.
    fn meshes(&self) -> Vec<qymcad_core::geom::Mesh> {
        self.project.bodies.iter().map(|b| b.mesh.clone()).collect()
    }

    /// Export the display meshes as binary STL.
    pub fn export_stl(&self, path: &str) -> Result<(), String> {
        qymcad_io::export_stl(&self.meshes(), path)
    }

    /// Export the display meshes as Wavefront OBJ.
    pub fn export_obj(&self, path: &str) -> Result<(), String> {
        qymcad_io::export_obj(&self.meshes(), path)
    }

    /// Export the display meshes as PLY.
    pub fn export_ply(&self, path: &str) -> Result<(), String> {
        qymcad_io::export_ply(&self.meshes(), path)
    }

    /// Export the display meshes as binary glTF.
    pub fn export_glb(&self, path: &str) -> Result<(), String> {
        qymcad_io::export_glb(&self.meshes(), path)
    }

    /// Export the display meshes as 3MF.
    pub fn export_3mf(&self, path: &str) -> Result<(), String> {
        qymcad_io::export_3mf(&self.meshes(), path)
    }

    /// Export the display meshes as AMF.
    pub fn export_amf(&self, path: &str) -> Result<(), String> {
        qymcad_io::export_amf(&self.meshes(), path)
    }

    /// Export the live B-rep as STEP. Rebuilds first so the shapes are current.
    pub fn export_step(&mut self, path: &str) -> Result<(), String> {
        let (_report, shapes) = qymcad_testkit::regenerate(&mut self.project);
        let mut pairs: Vec<(&qymcad_kernel::Shape, [f64; 12])> = Vec::new();
        for body in &self.project.bodies {
            if let Some(shape) = shapes.get(&body.id) {
                pairs.push((shape, qymcad_core::feature::PLACE_IDENTITY));
            }
        }
        if pairs.is_empty() {
            return Err("mcp-nothing-to-export".to_string());
        }
        qymcad_kernel::write_step(&pairs, path)
    }

    /// The last sketch when it holds a closed profile, else a named refusal.
    fn profile_sketch(&self) -> Result<Id, String> {
        match self.last_sketch {
            Some(sketch) if sketch.kind == SketchKind::Profile => Ok(sketch.id),
            Some(_) => Err("mcp-needs-profile-sketch".to_string()),
            None => Err("mcp-extrude-without-sketch".to_string()),
        }
    }

    /// The last sketch when it holds hole points, else a named refusal.
    fn points_sketch(&self) -> Result<Id, String> {
        match self.last_sketch {
            Some(sketch) if sketch.kind == SketchKind::Points => Ok(sketch.id),
            Some(_) => Err("mcp-hole-needs-points".to_string()),
            None => Err("mcp-hole-without-sketch".to_string()),
        }
    }

    /// The last body, else a named refusal.
    fn stock_body(&self, missing: &str) -> Result<Id, String> {
        self.last_body.ok_or_else(|| missing.to_string())
    }

    /// A fresh sketch with a timeline node, returning its index and id. Above z = 0 the
    /// sketch stands on a datum plane at the current level, which is what lofts span.
    fn fresh_sketch(&mut self) -> (usize, Id) {
        let si = self.project.new_sketch("mcp-sketch");
        let sid = self.project.sketches[si].id;
        if self.level_z != 0.0 {
            let plane = self.project.add_plane(WorkPlane { id: 0, name: "mcp-level".into(), origin: [0.0, 0.0, self.level_z], normal: [0.0, 0.0, 1.0], rot_deg: 0.0, def: PlaneDef::Manual });
            self.project.sketches[si].plane = SketchPlane::Datum(plane);
        }
        self.project.add_sketch_node(sid, "mcp-sketch");
        (si, sid)
    }

    fn apply_one(&mut self, op: &RecipeOp) -> Result<(), String> {
        match op {
            RecipeOp::BoxOp(size) => {
                reject_non_positive("mcp-box-size", size.dx)?;
                reject_non_positive("mcp-box-size", size.dy)?;
                reject_non_positive("mcp-box-size", size.dz)?;
                let body = self.project.add_box(size.dx, size.dy, size.dz);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::CylinderOp(size) => {
                reject_non_positive("mcp-cylinder-radius", size.r)?;
                reject_non_positive("mcp-cylinder-height", size.h)?;
                let body = self.project.add_cylinder(size.r, size.h);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::SphereOp(spec) => {
                reject_non_positive("mcp-sphere-radius", spec.r)?;
                let body = self.project.add_sphere(spec.r);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::ConeOp(size) => {
                reject_non_positive("mcp-cone-radius", size.r1)?;
                reject_non_negative("mcp-cone-radius", size.r2)?;
                reject_non_positive("mcp-cone-height", size.h)?;
                let body = self.project.add_cone(size.r1, size.r2, size.h);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::TorusOp(size) => {
                reject_non_positive("mcp-torus-major", size.major)?;
                reject_non_positive("mcp-torus-minor", size.minor)?;
                let body = self.project.add_torus(size.major, size.minor);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::PrismOp(size) => {
                reject_non_positive("mcp-prism-radius", size.r)?;
                reject_non_positive("mcp-prism-height", size.h)?;
                if size.sides < 3 {
                    return Err(format!("mcp-prism-sides#{}", size.sides));
                }
                let body = self.project.add_prism(size.r, size.sides, size.h);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::RectSketch(size) => {
                reject_non_positive("mcp-rect-size", size.width)?;
                reject_non_positive("mcp-rect-size", size.height)?;
                let (si, sid) = self.fresh_sketch();
                let (hw, hh) = (size.width / 2.0, size.height / 2.0);
                self.project.add_rect_entity(si, -hw, -hh, hw, hh, Purpose::Real);
                self.project.regen_sketch(si);
                self.last_sketch = Some(SketchRef { id: sid, kind: SketchKind::Profile });
                self.profiles.push(sid);
                Ok(())
            }
            RecipeOp::CircleSketch(spec) => {
                reject_non_positive("mcp-circle-radius", spec.r)?;
                accept_finite("mcp-circle-x", spec.x)?;
                accept_finite("mcp-circle-y", spec.y)?;
                let (si, sid) = self.fresh_sketch();
                self.project.add_circle_entity(si, spec.x, spec.y, spec.r, Purpose::Real);
                self.project.regen_sketch(si);
                self.last_sketch = Some(SketchRef { id: sid, kind: SketchKind::Profile });
                self.profiles.push(sid);
                Ok(())
            }
            RecipeOp::HoleSketch(point) => {
                accept_finite("mcp-hole-point", point.x)?;
                accept_finite("mcp-hole-point", point.y)?;
                let (si, sid) = self.fresh_sketch();
                self.project.sketch_point_at(si, point.x, point.y, 1e-6);
                self.project.regen_sketch(si);
                self.last_sketch = Some(SketchRef { id: sid, kind: SketchKind::Points });
                Ok(())
            }
            RecipeOp::Hole(spec) => {
                reject_non_positive("mcp-hole-diameter", spec.diameter)?;
                reject_non_positive("mcp-hole-depth", spec.depth)?;
                let sketch = self.points_sketch()?;
                let src = self.stock_body("mcp-hole-without-body")?;
                let tool = HoleTool { kind: 0, diameter: spec.diameter, depth: spec.depth, dia2: 0.0, depth2: 0.0 };
                // Sketches sit at z = 0 with the stock above, while the tool cuts along -Z by
                // default: flipped, it drills up from the sketch plane into the stock.
                self.last_body = Some(self.project.add_hole_from_sketch(src, sketch, tool, true));
                Ok(())
            }
            RecipeOp::Extrude(spec) => {
                reject_non_positive("mcp-extrude-height", spec.height)?;
                let sketch = self.profile_sketch()?;
                let body = self.project.add_extrude(sketch, spec.height);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::Revolve(spec) => {
                if spec.axis > 1 {
                    return Err(format!("mcp-revolve-axis#{}", spec.axis));
                }
                reject_non_positive("mcp-revolve-angle", spec.angle_deg)?;
                let sketch = self.profile_sketch()?;
                let body = self.project.add_revolve(sketch, spec.axis, spec.angle_deg);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::Cut(spec) => {
                reject_non_positive("mcp-cut-height", spec.height)?;
                let sketch = self.profile_sketch()?;
                let src = self.stock_body("mcp-cut-without-body")?;
                self.last_body = Some(self.project.add_combine(src, sketch, spec.height, 0));
                Ok(())
            }
            RecipeOp::Boss(spec) => {
                reject_non_positive("mcp-boss-height", spec.height)?;
                let sketch = self.profile_sketch()?;
                let src = self.stock_body("mcp-boss-without-body")?;
                self.last_body = Some(self.project.add_combine(src, sketch, spec.height, 1));
                Ok(())
            }
            RecipeOp::Chamfer(spec) => {
                reject_non_positive("mcp-chamfer-dist", spec.dist)?;
                let src = self.stock_body("mcp-chamfer-without-body")?;
                self.last_body = Some(self.project.add_chamfer(src, spec.dist, Vec::new()));
                Ok(())
            }
            RecipeOp::Fillet(spec) => {
                reject_non_positive("mcp-fillet-radius", spec.radius)?;
                let src = self.stock_body("mcp-fillet-without-body")?;
                self.last_body = Some(self.project.add_fillet(src, spec.radius, Vec::new()));
                Ok(())
            }
            RecipeOp::LinearArray(spec) => {
                accept_finite("mcp-array-step", spec.dx)?;
                accept_finite("mcp-array-step", spec.dy)?;
                accept_finite("mcp-array-step", spec.dz)?;
                if spec.count < 1 || spec.count > MAX_ARRAY_COUNT {
                    return Err(format!("mcp-array-count#{}", spec.count));
                }
                let src = self.stock_body("mcp-array-without-body")?;
                self.last_body = Some(self.project.add_linear_array(src, spec.dx, spec.dy, spec.dz, spec.count));
                Ok(())
            }
            RecipeOp::Mirror(spec) => {
                if spec.plane > 2 {
                    return Err(format!("mcp-mirror-plane#{}", spec.plane));
                }
                let src = self.stock_body("mcp-mirror-without-body")?;
                let body = self.project.add_mirror(src, spec.plane, true, 0);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::Translate(spec) => {
                accept_finite("mcp-move-dx", spec.dx)?;
                accept_finite("mcp-move-dy", spec.dy)?;
                accept_finite("mcp-move-dz", spec.dz)?;
                let src = self.stock_body("mcp-move-without-body")?;
                let mat = [1.0, 0.0, 0.0, spec.dx, 0.0, 1.0, 0.0, spec.dy, 0.0, 0.0, 1.0, spec.dz];
                self.last_body = Some(self.project.add_move(src, mat));
                Ok(())
            }
            RecipeOp::Level(spec) => {
                accept_finite("mcp-level-z", spec.z)?;
                self.level_z = spec.z;
                Ok(())
            }
            RecipeOp::Loft(spec) => {
                if spec.sections < 2 {
                    return Err(format!("mcp-loft-sections#{}", spec.sections));
                }
                if spec.sections as usize > self.profiles.len() {
                    return Err(format!("mcp-loft-sections#{}of{}", spec.sections, self.profiles.len()));
                }
                let at = self.profiles.len() - spec.sections as usize;
                let sections: Vec<Id> = self.profiles[at..].to_vec();
                let ruled = matches!(spec.shape, LoftShape::Ruled);
                let body = self.project.add_loft(sections, Vec::new(), ruled, 0, 0, false);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::CircularArray(spec) => {
                if spec.count < 1 || spec.count > MAX_ARRAY_COUNT {
                    return Err(format!("mcp-circular-count#{}", spec.count));
                }
                reject_non_positive("mcp-circular-angle", spec.angle_deg)?;
                let src = self.stock_body("mcp-circular-without-body")?;
                self.last_body = Some(self.project.add_circular_array_axis(src, spec.count, spec.angle_deg, 0));
                Ok(())
            }
            RecipeOp::SplitBody(spec) => {
                if spec.plane > 2 {
                    return Err(format!("mcp-split-plane#{}", spec.plane));
                }
                accept_finite("mcp-split-offset", spec.offset)?;
                let src = self.stock_body("mcp-split-without-body")?;
                let pieces = self.project.add_split_body(src, spec.plane, 0, spec.offset, 2);
                self.last_body = pieces.last().copied();
                Ok(())
            }
            RecipeOp::ImportStep(spec) => {
                let data = std::fs::read(&spec.path).map_err(|e| format!("mcp-import-read#{}: {e}", spec.path))?;
                let ext = std::path::Path::new(&spec.path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                if !["step", "stp", "igs", "iges"].contains(&ext.as_str()) {
                    return Err(format!("mcp-import-format#{ext}"));
                }
                let format = qymcad_kernel::ExactFormat::of_extension(&ext);
                let (built, _shapes) = qymcad_kernel::read_exact(format, &spec.path, 0.1).map_err(|e| format!("mcp-import-parse#{e}"))?;
                if built.is_empty() {
                    return Err("mcp-import-empty".to_string());
                }
                let stem = file_stem(&spec.path);
                let source = self.project.add_source(stem.clone(), data);
                let count = built.len();
                let mut last = None;
                for (k, solid) in built.into_iter().enumerate() {
                    let bid = self.project.add_mesh(solid.mesh);
                    self.project.set_body_faces(bid, solid.faces);
                    let name = if count == 1 { stem.clone() } else { format!("{stem} {}", k + 1) };
                    self.project.import_tree_as_parts(vec![ImportNode { name, body: Some(bid), solid: k as u32, ..Default::default() }], source, &stem);
                    last = Some(bid);
                }
                self.last_body = last;
                Ok(())
            }
        }
    }

    fn rebuild(&mut self) -> Result<RecipeResult, String> {
        // Imports raise their live shapes from the embedded sources, exactly as on opening:
        // without sources this is the plain regenerate.
        let shapes = qymcad_testkit::restore_import_shapes(&self.project);
        let (report, _shapes) = qymcad_testkit::regenerate_with_shapes(&mut self.project, shapes);
        let errors: Vec<String> = report.errors.iter().map(|(id, err)| format!("node {id}: {err:?}")).collect();
        if self.project.bodies.is_empty() {
            return Err("mcp-recipe-built-nothing".to_string());
        }
        let rebuilt = report.built.len();
        let bodies = summarize(&self.project);
        if let Some(last) = bodies.last() {
            self.last_body = Some(last.id);
        }
        Ok(RecipeResult { bodies, rebuilt, errors })
    }
}

/// Measured numbers of every body of the project.
fn summarize(project: &Project) -> Vec<BodySummary> {
    project
        .bodies
        .iter()
        .map(|b| {
            let (min, max) = body_extent(b);
            BodySummary { id: b.id, name: b.name.clone(), volume_mm3: b.mesh.volume(), min, max, triangles: b.mesh.tris.len() }
        })
        .collect()
}

/// Extent of a body mesh; zeros where the mesh has no vertices yet.
fn body_extent(body: &qymcad_core::model::Body) -> ([f64; 3], [f64; 3]) {
    match body.mesh.bounds() {
        Some(b) => ([b.min.x, b.min.y, b.min.z], [b.max.x, b.max.y, b.max.z]),
        None => ([0.0; 3], [0.0; 3]),
    }
}

/// A dimension must be a finite positive number; the caller learns which field refused.
fn reject_non_positive(field: &str, value: f64) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{field}#{value}"))
    }
}

/// A radius that may close into an apex: finite and not negative.
fn reject_non_negative(field: &str, value: f64) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("{field}#{value}"))
    }
}

/// A coordinate must be finite; the caller learns which field refused.
fn accept_finite(field: &str, value: f64) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(format!("{field}#{value}"))
    }
}

/// The file name without its folder and extension, for part and source names.
fn file_stem(path: &str) -> String {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    match base.rfind('.') {
        Some(at) => base[..at].to_string(),
        None => base.to_string(),
    }
}
