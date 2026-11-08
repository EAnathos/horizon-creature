//! A mob's model drawn from cubes (SPECS §3.1).
//!
//! `cargo creature model <mod>` converts a Blockbench model into a tree of `Part`s in the mod's own crate; the mod's
//! `mob_submit` hook draws it with `draw`, giving each group's animation for the frame.
use horizon_client::mod_api::{BlockEntityDraw, CubeFace, Host};

/// A group of the model: its pivot (texels), its rest rotation (degrees, x then y then z), its cubes and its child
/// groups.
pub struct Part {
    pub name: &'static str,
    pub pivot: [f32; 3],
    pub rotation: [f32; 3],
    pub cubes: &'static [Cube],
    pub children: &'static [Part],
}

/// A cube (texels, 16 per block) and its faces' UVs in `draw_cube`'s order.
pub struct Cube {
    /// To find your way around a generated model.
    pub name: &'static str,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub uv: [[f32; 4]; 6],
}

/// Draws `part` and its children with `texture` (`<mod>:entity/...`). `pose` gives a group's rotation for this frame,
/// in degrees (x, y, z), on top of its rest rotation.
pub fn draw(
    host: &dyn Host,
    draw: &mut BlockEntityDraw,
    part: &Part,
    texture: &'static str,
    pose: &dyn Fn(&str) -> [f32; 3],
) {
    let animated = pose(part.name);
    let pivot = part.pivot.map(|v| v / 16.0);
    draw.push();
    for (k, axis) in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].into_iter().enumerate() {
        let degrees = part.rotation[k] + animated[k];
        if degrees != 0.0 {
            draw.rotate_about(axis, degrees, pivot);
        }
    }
    for cube in part.cubes {
        let faces = cube.uv.map(|uv| Some(CubeFace { sprite: texture, uv: Some(uv) }));
        host.draw_cube(draw, cube.from, cube.to, faces, 0xFFFFFF);
    }
    for child in part.children {
        self::draw(host, draw, child, texture, pose);
    }
    draw.pop();
}
