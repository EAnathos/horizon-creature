//! A mob's model drawn from cubes (SPECS §3.1).
//!
//! `cargo creature model <mod>` converts a Blockbench model into a tree of `Part`s in the mod's own crate; the mod's
//! `EntityRenderer` draws it with `draw`, giving each group's animation for the frame.
use mod_api::{BlockEntityDrawContext, CubeFace, EntityRenderState, LivingRenderState};

/// No tint.
pub const WHITE: u32 = 0xFFFFFF;
/// Vanilla's red hurt flash (`OverlayTexture`'s red, about 30 % over the texture), as a tint.
const HURT: u32 = 0xFF6464;

/// Two tints applied together (0xRRGGBB, channel by channel): a creature's colour and the hurt flash.
pub fn multiply(a: u32, b: u32) -> u32 {
    let channel = |shift: u32| ((a >> shift & 0xFF) * (b >> shift & 0xFF) / 255) << shift;
    channel(16) | channel(8) | channel(0)
}

/// The tint for a mob this frame: red while hurt or dying, as vanilla's `LivingEntityRenderState.hasRedOverlay`.
pub fn tint(mob: Option<&LivingRenderState>) -> u32 {
    if mob.is_some_and(|m| m.has_red_overlay) {
        HURT
    } else {
        WHITE
    }
}

/// Turns the pose the way a living mob faces and scales it by its `minecraft:scale`, as vanilla's
/// `LivingEntityRenderer.setupRotations` does before the model: Sunrise hands a mod's renderer the pose at the mob's
/// feet, not turned. Then a dying mob tips over onto its side in its first second. Call before drawing it.
/// A mob drawn without its living state faces the viewer. In a screen (`IN_A_SCREEN`) it is mirrored along z: at our
/// pin a screen's entity keeps only the faces turned towards the viewer, and ours are wound the other way there (fixed
/// upstream by sunrise-org/sunrise `8c00340`: drop the mirror once the pin has it, docs/SUNRISE-ISSUES.md).
pub fn living_pose(draw: &mut dyn BlockEntityDrawContext, entity: &EntityRenderState) {
    if entity.id == IN_A_SCREEN {
        draw.scale([1.0, 1.0, -1.0]);
    }
    let Some(mob) = entity.living.as_ref() else {
        return;
    };
    draw.rotate_about([0.0, 1.0, 0.0], 180.0 - mob.body_rot, [0.0, 0.0, 0.0]);
    if mob.death_time > 0.0 {
        let f = ((mob.death_time - 1.0) / 20.0 * 1.6).max(0.0).sqrt().min(1.0);
        draw.rotate_about([0.0, 0.0, 1.0], f * 90.0, [0.0, 0.0, 0.0]);
    }
    if mob.scale != 1.0 {
        draw.scale([mob.scale; 3]);
    }
}

/// The id Sunrise gives a creature it draws in a screen (`DrawContext::entity`), made for drawing only.
pub const IN_A_SCREEN: u32 = u32::MAX;

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
    draw: &mut dyn BlockEntityDrawContext,
    part: &Part,
    texture: &'static str,
    pose: &dyn Fn(&str) -> [f32; 3],
) {
    draw_shown(draw, part, texture, pose, &|_| true, WHITE);
}

/// `draw`, leaving out the groups (and their children) `shown` refuses (gear drawn only when worn: a harness, chests),
/// tinted by `tint` (0xRRGGBB, `WHITE` for none; `tint(mob)` for the hurt flash).
pub fn draw_shown(
    draw: &mut dyn BlockEntityDrawContext,
    part: &Part,
    texture: &'static str,
    pose: &dyn Fn(&str) -> [f32; 3],
    shown: &dyn Fn(&str) -> bool,
    tint: u32,
) {
    draw_layer(draw, part, texture, pose, shown, tint, 0.0);
}

/// `draw_shown` with every cube grown by `inflate` texels on each side: a layer drawn over the model (a pattern, common
/// SPECS §5) is grown a little so it shows over the faces below instead of fighting them, as vanilla's
/// `CubeDeformation` for the tropical fish's pattern.
#[allow(clippy::too_many_arguments)]
pub fn draw_layer(
    draw: &mut dyn BlockEntityDrawContext,
    part: &Part,
    texture: &'static str,
    pose: &dyn Fn(&str) -> [f32; 3],
    shown: &dyn Fn(&str) -> bool,
    tint: u32,
    inflate: f32,
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
        let faces = cube.uv.map(|uv| Some(CubeFace::new(texture).with_uv(uv)));
        let from = cube.from.map(|v| v - inflate);
        let to = cube.to.map(|v| v + inflate);
        draw.cube(from, to, faces, tint);
    }
    for child in part.children {
        draw_layer(draw, child, texture, pose, shown, tint, inflate);
    }
    draw.pop();
}
