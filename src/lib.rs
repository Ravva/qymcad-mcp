//! `qymcad-mcp`: a headless recipe engine for AI agents.
//!
//! The window stays out of the loop: a recipe of plain structs (`RecipeOp`) builds on
//! `qymcad_core::model::Project` through the same `add_*` calls the interface delegates to,
//! and `qymcad_testkit::regenerate` rebuilds with the real OCCT kernel. Live B-rep shapes are
//! rebuilt on demand and never cross the API boundary, so the protocol carries only RON/JSON.

use qymcad_core::feature::Purpose;
use qymcad_core::model::{Id, Project};
use serde::{Deserialize, Serialize};

/// A size along three axes, in mm. One struct rather than three loose `f64`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BoxSize {
    pub dx: f64,
    pub dy: f64,
    pub dz: f64,
}

/// A rectangle on the XY plane, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RectSize {
    pub width: f64,
    pub height: f64,
}

/// An extrusion height, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExtrudeHeight {
    pub height: f64,
}

/// A fillet radius, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FilletRadius {
    pub radius: f64,
}

/// One step of a recipe. A closed enum rather than a stringly-typed call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RecipeOp {
    /// A box primitive at the origin.
    BoxOp(BoxSize),
    /// A rectangle sketch on XY, centred at the origin.
    RectSketch(RectSize),
    /// Extrude the first closed contour of the last sketch.
    Extrude(ExtrudeHeight),
    /// Fillet every edge of the last body.
    Fillet(FilletRadius),
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
    last_sketch: Option<Id>,
    last_body: Option<Id>,
}

impl McpSession {
    /// An empty document with one active part, ready for a recipe.
    pub fn new_empty() -> Self {
        let mut project = Project::default();
        project.new_document();
        Self { project, last_sketch: None, last_body: None }
    }

    /// Open a `.qcad` file exactly as the window sees it: imports restored, full rebuild.
    pub fn open(path: &str) -> Result<Self, String> {
        let mut project = qymcad_io::load_project(path)?;
        qymcad_testkit::open_like_the_app(&mut project);
        let last_body = project.bodies.last().map(|b| b.id);
        Ok(Self { project, last_sketch: None, last_body })
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

    /// Export the display meshes as binary STL.
    pub fn export_stl(&self, path: &str) -> Result<(), String> {
        let meshes: Vec<qymcad_core::geom::Mesh> = self.project.bodies.iter().map(|b| b.mesh.clone()).collect();
        qymcad_io::export_stl(&meshes, path)
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
            RecipeOp::RectSketch(size) => {
                reject_non_positive("mcp-rect-size", size.width)?;
                reject_non_positive("mcp-rect-size", size.height)?;
                let si = self.project.new_sketch("mcp-sketch");
                let sid = self.project.sketches[si].id;
                self.project.add_sketch_node(sid, "mcp-sketch");
                let (hw, hh) = (size.width / 2.0, size.height / 2.0);
                self.project.add_rect_entity(si, -hw, -hh, hw, hh, Purpose::Real);
                self.project.regen_sketch(si);
                self.last_sketch = Some(sid);
                Ok(())
            }
            RecipeOp::Extrude(spec) => {
                reject_non_positive("mcp-extrude-height", spec.height)?;
                let sketch = self.last_sketch.ok_or_else(|| "mcp-extrude-without-sketch".to_string())?;
                let body = self.project.add_extrude(sketch, spec.height);
                self.last_body = Some(self.project.finish_base_body(body, 1));
                Ok(())
            }
            RecipeOp::Fillet(spec) => {
                reject_non_positive("mcp-fillet-radius", spec.radius)?;
                let src = self.last_body.ok_or_else(|| "mcp-fillet-without-body".to_string())?;
                self.last_body = Some(self.project.add_fillet(src, spec.radius, Vec::new()));
                Ok(())
            }
        }
    }

    fn rebuild(&mut self) -> Result<RecipeResult, String> {
        let (report, _shapes) = qymcad_testkit::regenerate(&mut self.project);
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
        Err(format!("{field}#{}", value))
    }
}
