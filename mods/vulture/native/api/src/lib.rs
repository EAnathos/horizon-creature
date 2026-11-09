//! The vulture (mods/vulture/SPECS.md).
//!
//! - `vulture:vulture` copies the wandering trader (content part, `content/`): we clear its goals and give it ours,
//!   `NoFall` and `Fly`.
//! - Mounting: a right-click on it (`mount_click.rs` injection), or `/ride`. Horizon then copies the rider's keys into
//!   `Mob.rider` every tick (`game/mounts.rs`, `tick_mount_riding`): forward/back, left/right, jump (Space), sprint
//!   (Ctrl). Sneaking gets off.
//! - Flight: `Fly` reads `Mob.rider` and sets the vulture's speed directly, without gravity: steered by the rider's
//!   look, banking into arc turns like a plane (SPECS §6.2).
//! - Sounds: its own, not the trader's (`sounds.rs`, played by the library's `creature::sounds`).
//! - Drawing: the Blockbench model (`assets/models/vulture.bbmodel`, converted into `model.rs`), posed by `pose` and
//!   drawn by the library's `creature::model`, in place of the trader's model; its hearts, once ridden, without the
//!   empty ones (`mount_hud.rs` injection).
//!
//! It depends on the series' library, Horizon Creature (`creature`, common/SPECS.md).
mod model;
mod sounds;

use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::mobs::goals;
use horizon_client::mod_api::{
    Cx, EntityTickListenedEvent, EntityTickPostEvent, Flow, Goal, Hooks, Host, Mob, MobSubmitEvent, NativeMobRender,
    RandomSource, RegisterGoalsEvent,
};

const KIND: &str = "vulture:vulture";

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Vulture)
}

struct Vulture;

/// Flight numbers (SPECS §6.2), in blocks per tick before air drag (×0.91 per tick) and degrees.
const CRUISE: f64 = 0.6;
const DIVE_MAX: f64 = 1.2;
const STALL: f64 = 0.25;
/// What a nose-down tick adds to the speed (× the sine of the pitch), a nose-up one takes away.
const GRAVITY: f64 = 0.03;
/// Wing beats close this share of the gap to the cruising speed every tick.
const FLAP: f64 = 0.06;
const GLIDE_DRAG: f64 = 0.004;
const BRAKE: f64 = 0.04;
const GLIDE_SINK: f64 = 0.04;
const TAKE_OFF: f64 = 0.4;
const WALK: f64 = 0.12;
/// The steepest climb and dive it takes, and how fast its pitch follows the look.
const MAX_CLIMB: f32 = 50.0;
const MAX_DIVE: f32 = 75.0;
const PITCH_RATE: f32 = 4.0;
/// Its turn: this share of the gap between its heading and the look, at most `MAX_TURN` a tick.
const TURN: f32 = 0.08;
const MAX_TURN: f32 = 5.0;

/// `a - b` in degrees, wrapped to -180..180.
fn wrap(degrees: f32) -> f32 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

/// Ridden flight (SPECS §6.2), flown like a plane: it always moves along its heading, turns towards where the rider
/// looks (banking, drawn by `pose` from the gap between its heading and its head), noses up or down with the look,
/// and trades speed for height like a glider. W flaps up to the cruising speed, S brakes, Space flaps hard (and takes
/// off from the ground).
#[derive(Clone, Debug, Default)]
struct Fly {
    /// Speed along the heading, blocks per tick.
    speed: f64,
    /// Its pitch, positive nose down (as `head_pitch`).
    pitch: f32,
}

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
    fn start(&mut self, m: &mut Mob, _: &mut Cx) {
        self.speed = 0.0;
        self.pitch = 0.0;
        m.body_yaw = m.yaw;
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        m.no_gravity = false;
        m.head_pitch = 0.0;
        FLAPPING.lock().unwrap().remove(&m.id);
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        let Some(r) = m.rider else {
            return;
        };
        // The head looks where the rider looks; the body keeps its heading and turns towards it.
        m.head_yaw = r.yaw;
        let flying = !m.on_ground || r.jumping;
        m.no_gravity = flying;
        if !flying {
            // On the ground: a slow walk where the rider looks.
            self.speed = 0.0;
            self.pitch = 0.0;
            m.head_pitch = 0.0;
            m.yaw = r.yaw;
            m.body_yaw = r.yaw;
            let (s, c) = f64::from(r.yaw).to_radians().sin_cos();
            let (fwd, strafe) = (f64::from(r.forward), f64::from(r.strafe));
            m.velocity.x = (strafe * c - fwd * s) * WALK;
            m.velocity.z = (fwd * c + strafe * s) * WALK;
            return;
        }
        if m.on_ground {
            // Taking off: a jump forward.
            self.speed = self.speed.max(STALL);
            self.pitch = -MAX_CLIMB / 2.0;
            sounds::queue(m.id, sounds::TAKEOFF, 0.8, 1.0);
        }
        // Turn: a share of the gap to the look, so it banks more and turns faster the further it has to go.
        let gap = wrap(r.yaw - m.yaw);
        let heading = m.yaw + (gap * TURN).clamp(-MAX_TURN, MAX_TURN);
        m.yaw = heading;
        m.body_yaw = heading;
        // Pitch: follows the look, smoothly; a stall noses it down whatever the rider does.
        let mut target = r.pitch.clamp(-MAX_CLIMB, MAX_DIVE);
        if self.speed < STALL && target < 0.0 {
            target = 20.0;
        }
        self.pitch += (target - self.pitch).clamp(-PITCH_RATE, PITCH_RATE);
        m.head_pitch = self.pitch;
        // Speed: gravity along the slope, then the keys.
        let (sin_pitch, cos_pitch) = f64::from(self.pitch).to_radians().sin_cos();
        self.speed += GRAVITY * sin_pitch;
        let flapping = r.forward > 0.0 || r.jumping;
        if r.forward > 0.0 && self.speed < CRUISE {
            self.speed += (CRUISE - self.speed) * FLAP;
        } else if r.forward < 0.0 {
            self.speed -= BRAKE;
        } else {
            self.speed -= GLIDE_DRAG;
        }
        self.speed = self.speed.clamp(0.0, DIVE_MAX);
        let (s, c) = f64::from(heading).to_radians().sin_cos();
        let level = self.speed * cos_pitch;
        m.velocity.x = -s * level;
        m.velocity.z = c * level;
        m.velocity.y = -self.speed * sin_pitch;
        if r.jumping {
            m.velocity.y = m.velocity.y.max(TAKE_OFF);
        } else if !flapping {
            // Gliding sinks gently, less the faster it goes.
            m.velocity.y -= GLIDE_SINK * (1.0 - (self.speed / CRUISE).min(1.0)) + GLIDE_SINK / 2.0;
        }
        // One wing beat every 8 ticks while it flaps.
        if flapping && m.age % 8 == 0 {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            sounds::queue(m.id, sounds::FLAP, 0.6, pitch);
        }
        FLAPPING.lock().unwrap().insert(m.id, flapping);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Whether each ridden vulture is beating its wings, for `pose` (the render state doesn't carry the keys). Single
/// player: the goal and the drawing share the mob's id (multiplayer, v2, will need it sent).
static FLAPPING: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());

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

/// The variant's texture (only one for now: tawny).
const TEXTURE: &str = "vulture:entity/vulture/tawny";

/// How far its back slopes down to the tail on the ground, in degrees.
const HUNCH: f32 = 18.0;

/// A group's rotation for this frame, in degrees (x, y, z), on top of its rest rotation.
///
/// In flight the body takes the flight's pitch (`pitch`, which `Fly` keeps in `head_pitch`) and banks with the turn:
/// the gap between its heading and its head (where the rider looks, `head_yaw`). On the ground the wings fold along the
/// flanks of its hunched body: the arm turned back with its coverts outward, the forearm folded forward over it, the hand back again, its
/// primaries along the tail.
fn pose(name: &str, mob: &NativeMobRender<'_>, flapping: bool) -> [f32; 3] {
    let flying = !mob.on_ground;
    let bank = (mob.head_yaw * 0.8).clamp(-50.0, 50.0);
    // Flapping: the wings go up and down, the outer segments following a little later; gliding, they are held still
    // with a slight dihedral, swept back the steeper it dives.
    let (beat, lag, late) = if flapping {
        (
            (mob.age * 0.5).sin() * 35.0,
            (mob.age * 0.5 - 0.8).sin() * 20.0,
            (mob.age * 0.5 - 1.4).sin() * 15.0,
        )
    } else {
        (6.0, 0.0, 4.0)
    };
    let sweep = if flying {
        (mob.pitch - 20.0).clamp(0.0, 40.0)
    } else {
        0.0
    };
    // Walking: like a vanilla quadruped's legs; in flight, the legs are tucked in.
    let step = (mob.walk_position * 0.6662).cos() * 1.4 * mob.walk_speed.min(1.0);
    let leg = if flying { 60.0 } else { step.to_degrees() };
    if !flying {
        return match name {
            // Hunched: the back slopes down to the tail, the head and legs straightened against it.
            "body" => [-HUNCH, 0.0, 0.0],
            // The model faces +z: looking right (positive yaw) turns it towards -x.
            "head" => [mob.pitch + HUNCH, -mob.head_yaw, 0.0],
            "wing_left" => [-90.0, 0.0, 90.0],
            "wing_right" => [-90.0, 0.0, -90.0],
            "wing_left_mid" | "wing_right_mid" => [0.0, 0.0, 180.0],
            "wing_left_tip" => [0.0, 178.0, 0.0],
            "wing_right_tip" => [0.0, -178.0, 0.0],
            "leg_left" => [HUNCH + leg, 0.0, 0.0],
            "leg_right" => [HUNCH - leg, 0.0, 0.0],
            _ => [0.0; 3],
        };
    }
    match name {
        "body" => [mob.pitch, 0.0, bank],
        // The head looks into the turn, and a little up against a dive.
        "head" => [-mob.pitch * 0.5, -mob.head_yaw * 0.5, 0.0],
        "wing_left" => [0.0, sweep, beat],
        "wing_right" => [0.0, -sweep, -beat],
        "wing_left_mid" => [0.0, sweep * 0.5, lag],
        "wing_right_mid" => [0.0, -sweep * 0.5, -lag],
        "wing_left_tip" => [0.0, 0.0, late],
        "wing_right_tip" => [0.0, 0.0, -late],
        "leg_left" | "leg_right" => [leg, 0.0, 0.0],
        "tail" => [-10.0, 0.0, -bank * 0.3],
        _ => [0.0; 3],
    }
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
        ai.goals.add(1, Fly::default());
    }

    fn entity_tick_listened(&self, _: &mut dyn Host, event: &mut EntityTickListenedEvent) {
        event.listened = true;
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind == KIND {
            creature::sounds::tick_post(host, event, &sounds::VOICE, creature::sounds::NEUTRAL);
        }
    }

    /// The Blockbench model (`model.rs`), posed by `pose` every frame.
    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != KIND {
            return Flow::Continue;
        }
        let flapping = FLAPPING.lock().unwrap().get(&mob.id).copied().unwrap_or(false);
        creature::model::draw(host, &mut *event.draw, &model::VULTURE, TEXTURE, &|part| {
            pose(part, mob, flapping)
        });
        Flow::Cancel
    }
}
