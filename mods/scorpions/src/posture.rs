//! Its defensive posture (SPECS §3), the scorpion's signature: a player coming within `POSTURE` blocks (`POSTURE_SNEAK`
//! for a sneaking one) makes it stop, turn to face them, rear up (claws open, tail arched: `set_reared`), hiss now and
//! then, and back away slowly. It lowers its tail and goes on once they are `STAND_DOWN` blocks away. A player whose box
//! steps onto it (within `STEP_ON`) is stung once, at most every `STEP_EVERY` ticks, without a chase.
//!
//! It never starts a fight: this behaviour gives way to `Fight` (before it) once the scorpion has a target. Both run
//! from the server tick (the library's `body`); while either holds it, its crawl waits (`busy`).
use creature::body::{Behaviour, Body, World};

use crate::{HISS, kind, set_reared, strike};

const POSTURE: f64 = 3.0;
const POSTURE_SNEAK: f64 = 1.5;
const STAND_DOWN: f64 = 5.0;
const STEP_ON: f64 = 0.6;
const STEP_EVERY: i32 = 40;
/// How fast it backs away (blocks a tick) and how often it hisses while reared (one tick in this many).
const BACK_SPEED: f64 = 0.03;
const HISS_CHANCE: i32 = 50;

/// How close the nearest player is to the scorpion, flat (blocks), and whether they sneak; none for no threat (dead,
/// creative or a spectator).
fn threat(m: &Body, cx: &World<'_>) -> Option<(f64, bool)> {
    let p = cx.player.as_ref()?;
    if !p.alive || p.creative || p.spectator {
        return None;
    }
    let d = (p.position.x - m.position.x).hypot(p.position.z - m.position.z);
    Some((d, p.sneaking))
}

#[derive(Debug, Default)]
pub(crate) struct Posture {
    sting_cd: i32,
    active: bool,
}

impl Behaviour for Posture {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        // Not already fighting (Fight has the target), and a player close enough to threaten (or not yet gone far).
        let reach = |sneaking: bool| {
            if self.active {
                STAND_DOWN
            } else if sneaking {
                POSTURE_SNEAK
            } else {
                POSTURE
            }
        };
        let on = crate::fight::target(m.id).is_none() && threat(m, cx).is_some_and(|(d, s)| d < reach(s));
        if self.active && !on {
            set_reared(m.id, false);
        }
        self.active = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.active = false;
        set_reared(m.id, false);
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(p) = cx.player.as_ref() else {
            return;
        };
        let (px, pz) = (p.position.x, p.position.z);
        set_reared(m.id, true);
        self.sting_cd -= 1;
        let (dx, dz) = (m.position.x - px, m.position.z - pz);
        let dist = dx.hypot(dz);
        if dist < STEP_ON {
            // Stepped on: a single sting, no chase.
            if self.sting_cd <= 0 {
                strike(m.id, p.entity_id, Some(kind(m.id)));
                creature::sounds::queue(m.id, HISS, 0.8, 0.85);
                self.sting_cd = STEP_EVERY;
            }
        } else {
            // Back away slowly, keeping its distance.
            m.velocity.x = dx / dist.max(0.01) * BACK_SPEED;
            m.velocity.z = dz / dist.max(0.01) * BACK_SPEED;
            if m.random.next_int_bound(HISS_CHANCE).unwrap_or(1) == 0 {
                let pitch = 0.9 + m.random.next_float() * 0.2;
                creature::sounds::queue(m.id, HISS, 0.6, pitch);
            }
        }
        // Turn to face the player.
        let yaw = (-(px - m.position.x)).atan2(pz - m.position.z).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
}
