//! The vulture's flight (SPECS §6.2), shared by the ridden flight (`Fly`, steered by the rider) and the wild one
//! (`wild::Soar`, steered by its own AI).
//!
//! Flown like a plane: it always moves along its heading, turns towards the wanted heading (banking, drawn by `pose`
//! from the gap between its body and its head, which looks where it wants to go), noses up or down towards the wanted
//! pitch, and trades speed for height like a glider.
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::mod_api::{Mob, RandomSource};

use crate::sounds;

/// Speeds in blocks per tick, before air drag (×0.91 per tick).
pub const CRUISE: f64 = 0.55;
const DIVE_MAX: f64 = 1.0;
pub const STALL: f64 = 0.18;
/// What a nose-down tick adds to the speed (× the sine of the pitch), a nose-up one takes away.
const GRAVITY: f64 = 0.03;
/// Wing beats close this share of the gap to the cruising speed every tick.
const FLAP: f64 = 0.06;
const GLIDE_DRAG: f64 = 0.004;
const BRAKE: f64 = 0.04;
const GLIDE_SINK: f64 = 0.04;
const TAKE_OFF: f64 = 0.4;
const MIN_DESCENT: f64 = 0.2;
/// From this pitch down a flight counts as a descent (smaller ones are a wild vulture holding its height).
const DESCENT_PITCH: f32 = 10.0;
/// The steepest climb and dive it takes, and how fast its pitch follows the wanted one, in degrees.
pub const MAX_CLIMB: f32 = 50.0;
const MAX_DIVE: f32 = 75.0;
const PITCH_RATE: f32 = 4.0;
/// Its turn: this share of the gap between its heading and the wanted one, at most `MAX_TURN` a tick.
const TURN: f32 = 0.08;
const MAX_TURN: f32 = 5.0;

/// What steers the flight for one tick.
pub struct Controls {
    /// Where it should head and how steeply (positive nose down), in degrees.
    pub yaw: f32,
    pub pitch: f32,
    /// Wing beats up to the cruising speed (W).
    pub flap: bool,
    /// Air brake (S).
    pub brake: bool,
    /// A strong flap that climbs, and takes off from the ground (Space).
    pub boost: bool,
    /// Ridden (SPECS §6.2): twice its own cruising and diving speeds, to travel far.
    pub ridden: bool,
}

/// What a rider's flight multiplies its cruising and diving speeds by.
const RIDDEN_SPEED: f64 = 2.0;

/// Hovering: the share of the distance to its spot it wants to cover a tick, its fastest, and how quickly its velocity
/// follows.
const HOVER_PULL: f64 = 0.2;
const HOVER_SPEED: f64 = 0.3;
const HOVER_EASE: f64 = 0.25;

/// A flying vulture's momentum.
#[derive(Clone, Debug, Default)]
pub struct Flight {
    /// Speed along the heading, blocks per tick.
    pub speed: f64,
    /// Its pitch, positive nose down (as `head_pitch`).
    pub pitch: f32,
}

impl Flight {
    /// Back on the ground: no momentum, level.
    pub fn land(&mut self, m: &mut Mob) {
        self.speed = 0.0;
        self.pitch = 0.0;
        m.head_pitch = 0.0;
        m.no_gravity = false;
        set_flapping(m.id, false);
    }

    /// Hovering on beating wings towards `at` (its feet's place), facing `yaw`: no momentum like a plane's, it eases its
    /// velocity towards the spot, at most `HOVER_SPEED` a tick. For a strike from just above its target.
    pub fn hover(&mut self, m: &mut Mob, at: [f64; 3], yaw: f32) {
        m.no_gravity = true;
        let want = [at[0] - m.position.x, at[1] - m.position.y, at[2] - m.position.z];
        let len = (want[0] * want[0] + want[1] * want[1] + want[2] * want[2]).sqrt();
        let scale = if len > 0.0 {
            (len * HOVER_PULL).min(HOVER_SPEED) / len
        } else {
            0.0
        };
        m.velocity.x += (want[0] * scale - m.velocity.x) * HOVER_EASE;
        m.velocity.y += (want[1] * scale - m.velocity.y) * HOVER_EASE;
        m.velocity.z += (want[2] * scale - m.velocity.z) * HOVER_EASE;
        self.speed = m.velocity.x.hypot(m.velocity.z);
        // Body up, wings beating: the pitch eases back to level.
        self.pitch += (0.0 - self.pitch).clamp(-PITCH_RATE, PITCH_RATE);
        m.head_pitch = self.pitch;
        let gap = wrap(yaw - m.yaw);
        m.yaw += gap.clamp(-MAX_TURN * 2.0, MAX_TURN * 2.0);
        m.body_yaw = m.yaw;
        m.head_yaw = yaw;
        if m.age.is_multiple_of(6) {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            sounds::queue(m.id, sounds::FLAP, 0.6, pitch);
        }
        set_flapping(m.id, true);
    }

    /// One tick in the air (or taking off): sets its heading, pitch and velocity.
    pub fn tick(&mut self, m: &mut Mob, c: &Controls) {
        m.no_gravity = true;
        if m.on_ground {
            self.speed = self.speed.max(STALL);
            self.pitch = -MAX_CLIMB / 2.0;
            sounds::queue(m.id, sounds::TAKEOFF, 0.8, 1.0);
        }
        // Turn: a share of the gap, so it banks more and turns faster the further it has to go. The head looks
        // where it is going, which `pose` reads as the bank.
        let gap = wrap(c.yaw - m.yaw);
        let heading = m.yaw + (gap * TURN).clamp(-MAX_TURN, MAX_TURN);
        m.yaw = heading;
        m.body_yaw = heading;
        m.head_yaw = c.yaw;
        // Pitch: follows the wanted one, smoothly; a stall noses it down whatever is asked.
        let mut target = c.pitch.clamp(-MAX_CLIMB, MAX_DIVE);
        if self.speed < STALL && target < 0.0 {
            target = 20.0;
        }
        self.pitch += (target - self.pitch).clamp(-PITCH_RATE, PITCH_RATE);
        m.head_pitch = self.pitch;
        // Speed: gravity along the slope, then the wings.
        let (sin_pitch, cos_pitch) = f64::from(self.pitch).to_radians().sin_cos();
        self.speed += GRAVITY * sin_pitch;
        // Its own speed (SPECS §2); what it carries doesn't weigh on it (SPECS §6.4).
        let pace = crate::state::pace(m.id) * if c.ridden { RIDDEN_SPEED } else { 1.0 };
        let cruise = CRUISE * pace;
        if c.flap && self.speed < cruise {
            self.speed += (cruise - self.speed) * FLAP;
        } else if c.brake {
            self.speed -= BRAKE;
        } else {
            self.speed -= GLIDE_DRAG;
        }
        self.speed = self.speed.clamp(0.0, DIVE_MAX * pace);
        let (s, c_yaw) = f64::from(heading).to_radians().sin_cos();
        let level = self.speed * cos_pitch;
        m.velocity.x = -s * level;
        m.velocity.z = c_yaw * level;
        m.velocity.y = -self.speed * sin_pitch;
        let flapping = c.flap || c.boost;
        // Bumping into something in the air (a tree, a cliff): it climbs over it rather than staying stuck.
        let blocked = m.horizontal_collision && !m.on_ground;
        // Descending (pitch down), it sinks at least this fast even slow, so it comes down to land.
        if c.pitch >= DESCENT_PITCH {
            m.velocity.y = m.velocity.y.min(-MIN_DESCENT);
        }
        if c.boost || blocked {
            m.velocity.y = m.velocity.y.max(TAKE_OFF);
        } else if !flapping {
            // Gliding sinks gently, less the faster it goes.
            m.velocity.y -= GLIDE_SINK * (1.0 - (self.speed / (CRUISE * pace)).min(1.0)) + GLIDE_SINK / 2.0;
        }
        // One wing beat every 8 ticks while it flaps.
        if flapping && m.age.is_multiple_of(8) {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            sounds::queue(m.id, sounds::FLAP, 0.6, pitch);
        }
        set_flapping(m.id, flapping);
    }
}

/// `a` in degrees, wrapped to -180..180.
pub fn wrap(degrees: f32) -> f32 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

/// Whether each flying vulture is beating its wings, for `pose` (the render state doesn't carry it). Single player:
/// the goals and the drawing share the mob's id (multiplayer, v2, will need it sent; workaround for
/// arismoko/horizon#2740).
static FLAPPING: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());

fn set_flapping(id: u32, flapping: bool) {
    let mut map = FLAPPING.lock().unwrap();
    if flapping {
        map.insert(id, true);
    } else {
        map.remove(&id);
    }
}

pub fn flapping(id: u32) -> bool {
    FLAPPING.lock().unwrap().contains_key(&id)
}
