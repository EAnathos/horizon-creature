//! A mob's model drawn from cubes (SPECS §3.1).
//!
//! `cargo creature model <mod>` converts a Blockbench model into a tree of `Part`s in the mod's own crate; the mod's
//! `mob_submit` hook draws it with `draw`, giving each group's animation for the frame.
use horizon_client::mod_api::{BlockEntityDraw, CubeFace, Host, NativeMobRender};

/// No tint.
pub const WHITE: u32 = 0xFFFFFF;
/// Vanilla's red hurt flash (`OverlayTexture`'s red, about 30 % over the texture), as a tint.
const HURT: u32 = 0xFF6464;

/// The tint for a mob this frame: red while hurt or dying, as vanilla's `LivingEntityRenderState.hasRedOverlay`.
pub fn tint(mob: &NativeMobRender<'_>) -> u32 {
    if mob.hurt_time > 0 || mob.death_time > 0 {
        HURT
    } else {
        WHITE
    }
}

/// A dying mob tips over onto its side in its first second, as vanilla's `LivingEntityRenderer.setupRotations`: call
/// before drawing it.
pub fn death_tilt(draw: &mut BlockEntityDraw, mob: &NativeMobRender<'_>) {
    if mob.death_time > 0 {
        let f = ((mob.death_time as f32 + mob.partial - 1.0) / 20.0 * 1.6)
            .sqrt()
            .min(1.0);
        draw.rotate_about([0.0, 0.0, 1.0], f * 90.0, [0.0, 0.0, 0.0]);
    }
}

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
    draw_shown(host, draw, part, texture, pose, &|_| true, WHITE);
}

/// `draw`, leaving out the groups (and their children) `shown` refuses (gear drawn only when worn: a harness, chests),
/// tinted by `tint` (0xRRGGBB, `WHITE` for none; `tint(mob)` for the hurt flash).
pub fn draw_shown(
    host: &dyn Host,
    draw: &mut BlockEntityDraw,
    part: &Part,
    texture: &'static str,
    pose: &dyn Fn(&str) -> [f32; 3],
    shown: &dyn Fn(&str) -> bool,
    tint: u32,
) {
    if !shown(part.name) {
        return;
    }
    let animated = pose(part.name);
    let pivot = part.pivot.map(|v| v / 16.0);
    draw.push();
    for (k, axis) in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        .into_iter()
        .enumerate()
    {
        let degrees = part.rotation[k] + animated[k];
        if degrees != 0.0 {
            draw.rotate_about(axis, degrees, pivot);
        }
    }
    for cube in part.cubes {
        let faces = cube.uv.map(|uv| {
            Some(CubeFace {
                sprite: texture,
                uv: Some(uv),
            })
        });
        host.draw_cube(draw, cube.from, cube.to, faces, tint);
    }
    for child in part.children {
        draw_shown(host, draw, child, texture, pose, shown, tint);
    }
    draw.pop();
}
