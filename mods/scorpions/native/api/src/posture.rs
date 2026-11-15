//! Its defensive posture (SPECS §3), the scorpion's signature: a player coming within `POSTURE` blocks (`POSTURE_SNEAK`
//! for a sneaking one) makes it stop, turn to face them, rear up (claws open, tail arched: `set_reared`), hiss now and
//! then, and back away slowly. It lowers its tail and goes on once they are `STAND_DOWN` blocks away. A player whose box
//! steps onto it (within `STEP_ON`) is stung once, at most every `STEP_EVERY` ticks, without a chase.
//!
//! It never starts a fight: this goal gives way to `Fight` (a higher priority) once the scorpion has a target.
use horizon_client::mobs::goals;
use horizon_client::mobs::{Who, ai::do_hurt_target};
use horizon_client::mod_api::{Cx, Goal, Mob, RandomSource};

use crate::{HISS, kind, queue_sting, set_reared};

const POSTURE: f64 = 3.0;
const POSTURE_SNEAK: f64 = 1.5;
const STAND_DOWN: f64 = 5.0;
const STEP_ON: f64 = 0.6;
const STEP_EVERY: i32 = 40;
/// How fast it backs away (blocks a tick) and how often it hisses while reared (one tick in this many).
const BACK_SPEED: f64 = 0.03;
const HISS_CHANCE: i32 = 50;

/// How close a player is to the scorpion, flat (blocks), and whether they are a threat (alive, not creative).
fn threat(m: &Mob, cx: &Cx) -> Option<(f64, bool)> {
    let p = cx.w.player.as_ref()?;
    if !p.alive || p.creative {
        return None;
    }
    let d = (p.position.x - m.position.x).hypot(p.position.z - m.position.z);
    Some((d, p.sneaking))
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Posture {
    sting_cd: i32,
}

impl Goal for Posture {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        // Not already fighting (Fight has the target), and a player is close enough to threaten.
        if m.ai.target.is_some() {
            return false;
        }
        threat(m, cx).is_some_and(|(d, sneaking)| d < if sneaking { POSTURE_SNEAK } else { POSTURE })
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        if m.ai.target.is_some() {
            set_reared(m.id, false);
            return false;
        }
        match threat(m, cx) {
            Some((d, _)) if d < STAND_DOWN => true,
            _ => {
                set_reared(m.id, false);
                false
            }
        }
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, m: &mut Mob, _: &mut Cx) {
        set_reared(m.id, true);
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        set_reared(m.id, false);
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(p) = cx.w.player.as_ref() else {
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
                m.swing();
                do_hurt_target(m, Who::Player, cx);
                queue_sting(kind(m.id));
                creature::sounds::queue(m.id, HISS, 0.8, 0.85);
                self.sting_cd = STEP_EVERY;
            }
        } else {
            // Back away slowly, keeping its distance.
            m.velocity.x = dx / dist.max(0.01) * BACK_SPEED;
            m.velocity.z = dz / dist.max(0.01) * BACK_SPEED;
            if m.random.next_int_bound(HISS_CHANCE).unwrap_or(1) == 0 {
                creature::sounds::queue(m.id, HISS, 0.6, 0.9 + m.random.next_float() * 0.2);
            }
        }
        // Turn to face the player.
        let yaw = (-(px - m.position.x)).atan2(pz - m.position.z).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
