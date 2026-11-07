//! The vulture (docs/SPECS.md).
//!
//! - `vulture:vulture` copies the wandering trader (content part, `content/`): we clear its goals and give it ours,
//!   `NoFall` and `Fly`.
//! - Mounting: a right-click on it (`mount_click.rs` injection), or `/ride`. Horizon then copies the rider's keys into
//!   `Mob.rider` every tick (`game/mounts.rs`, `tick_mount_riding`): forward/back, left/right, jump (Space), sprint
//!   (Ctrl). Sneaking gets off.
//! - Flight: `Fly` reads `Mob.rider` and sets the vulture's speed directly, without gravity.
//! - Sounds: its own, not the trader's (`sounds.rs`).
//! - Drawing: the Blockbench model (`assets/models/vulture.bbmodel`, converted into `model.rs`), posed by `pose`, in
//!   place of the trader's model; its hearts, once ridden, without the empty ones (`mount_hud.rs` injection).
mod model;
mod sounds;

use horizon_client::mobs::goals;
use horizon_client::mod_api::{
    BlockEntityDraw, CubeFace, Cx, EntityTickListenedEvent, EntityTickPostEvent, Flow, Goal, Hooks, Host, Mob,
    MobSubmitEvent, NativeMobRender, RandomSource, RegisterGoalsEvent,
};

const KIND: &str = "vulture:vulture";

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Vulture)
}

struct Vulture;

/// Speeds in blocks per tick, before air drag (×0.91 per tick).
const FLY_SPEED: f64 = 0.6;
const DIVE_SPEED: f64 = 1.0;
const CLIMB: f64 = 0.35;
const GLIDE_SINK: f64 = -0.08;
const DIVE_SINK: f64 = -0.7;

/// Ridden flight: while a player rides it, their keys give the vulture its speed.
#[derive(Clone, Debug)]
struct Fly;

impl Goal for Fly {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        m.rider.is_some()
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        m.no_gravity = false;
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        let Some(r) = m.rider else {
            return;
        };
        // It looks where its rider looks.
        m.yaw = r.yaw;
        m.body_yaw = r.yaw;
        m.head_yaw = r.yaw;
        m.head_pitch = r.pitch;
        let flying = !m.on_ground || r.jumping;
        m.no_gravity = flying;
        if !flying {
            // On the ground it stands still (walking comes later).
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            return;
        }
        let dive = r.sprint;
        let speed = if dive { DIVE_SPEED } else { FLY_SPEED };
        // `Entity.moveRelative`: positive strafe is left.
        let (s, c) = f64::from(r.yaw).to_radians().sin_cos();
        let (fwd, strafe) = (f64::from(r.forward), f64::from(r.strafe));
        let len = (fwd * fwd + strafe * strafe).sqrt().max(1.0);
        m.velocity.x = (strafe * c - fwd * s) / len * speed;
        m.velocity.z = (fwd * c + strafe * s) / len * speed;
        // One wing flap every 8 ticks while it climbs.
        if r.jumping && m.age % 8 == 0 {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            sounds::queue(m.id, sounds::FLAP, 0.6, pitch);
        }
        m.velocity.y = if r.jumping {
            CLIMB
        } else if dive {
            DIVE_SINK
        } else {
            GLIDE_SINK
        };
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// No fall damage (SPECS §6), ridden or not: the fall distance is reset every tick, before the move that could land
/// it. Also keeps it muted and gives it its call.
#[derive(Clone, Debug)]
struct NoFall;

impl Goal for NoFall {
    fn flags(&self) -> u8 {
        0
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        m.fall_distance = 0.0;
        // Muted as a wandering trader, every tick: a save read after `register_goals` sets its `Silent` back.
        m.silent = true;
        // A quiet bird: one call every 20 s on average (a vanilla mob: every 4 s).
        if m.random.next_int_bound(400).unwrap_or(1) == 0 {
            let pitch = 0.8 + m.random.next_float() * 0.3;
            sounds::queue(m.id, sounds::AMBIENT, 1.0, pitch);
        }
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// A group of the model: its pivot (texels), its rest rotation (degrees, x then y then z), its cubes and its child
/// groups. The data comes from `cargo vulture model`.
pub struct Part {
    pub name: &'static str,
    pub pivot: [f32; 3],
    pub rotation: [f32; 3],
    pub cubes: &'static [Cube],
    pub children: &'static [Part],
}

/// A cube (texels, 16 per block) and its faces' UVs in `draw_cube`'s order.
#[allow(dead_code)] // `name`: to find your way around `model.rs`.
pub struct Cube {
    pub name: &'static str,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub uv: [[f32; 4]; 6],
}

/// The variant's texture (only one for now: tawny).
const TEXTURE: &str = "vulture:entity/vulture/tawny";

/// A group's rotation for this frame, in degrees (x, y, z), on top of its rest rotation.
fn pose(name: &str, mob: &NativeMobRender<'_>) -> [f32; 3] {
    let flying = !mob.on_ground;
    // Flapping: the wings go up and down, the tips follow a little later.
    let beat = (mob.age * 0.5).sin();
    let lag = (mob.age * 0.5 - 0.8).sin();
    // On the ground the wings hang along the body and the tips fold back up on the body side, under the beige
    // coverts.
    let (arm, tip) = if flying { (beat * 35.0, lag * 20.0) } else { (85.0, 165.0) };
    // Walking: like a vanilla quadruped's legs; in flight, the legs are tucked in.
    let step = (mob.walk_position * 0.6662).cos() * 1.4 * mob.walk_speed.min(1.0);
    let leg = if flying { 60.0 } else { step.to_degrees() };
    match name {
        // The model faces +z: looking right (positive yaw) turns it towards -x.
        "head" => [mob.pitch, -mob.head_yaw, 0.0],
        "wing_left" => [0.0, 0.0, -arm],
        "wing_right" => [0.0, 0.0, arm],
        "wing_left_tip" => [0.0, 0.0, -tip],
        "wing_right_tip" => [0.0, 0.0, tip],
        "leg_left" => [leg, 0.0, 0.0],
        "leg_right" => [if flying { leg } else { -leg }, 0.0, 0.0],
        "tail" if flying => [-10.0, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

fn draw_part(host: &dyn Host, draw: &mut BlockEntityDraw, part: &Part, mob: &NativeMobRender<'_>) {
    let animated = pose(part.name, mob);
    let pivot = part.pivot.map(|v| v / 16.0);
    draw.push();
    for (k, axis) in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].into_iter().enumerate() {
        let degrees = part.rotation[k] + animated[k];
        if degrees != 0.0 {
            draw.rotate_about(axis, degrees, pivot);
        }
    }
    for cube in part.cubes {
        let faces = cube.uv.map(|uv| Some(CubeFace { sprite: TEXTURE, uv: Some(uv) }));
        host.draw_cube(draw, cube.from, cube.to, faces, 0xFFFFFF);
    }
    for child in part.children {
        draw_part(host, draw, child, mob);
    }
    draw.pop();
}

impl Hooks for Vulture {
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id != KIND {
            return;
        }
        // Without the wandering trader's sounds: its own come from `sounds.rs`.
        event.mob.silent = true;
        let ai = &mut event.mob.ai;
        ai.goals.clear();
        ai.targets.clear();
        ai.goals.add(0, NoFall);
        ai.goals.add(1, Fly);
    }

    fn entity_tick_listened(&self, _: &mut dyn Host, event: &mut EntityTickListenedEvent) {
        event.listened = true;
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind == KIND {
            sounds::tick_post(host, event);
        }
    }

    /// The Blockbench model (`model.rs`), posed by `pose` every frame.
    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != KIND {
            return Flow::Continue;
        }
        draw_part(host, &mut *event.draw, &model::VULTURE, mob);
        Flow::Cancel
    }
}
