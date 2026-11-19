//! It fights back (SPECS §3): once something hits it (the library's `anger` notes who), the scorpion runs at them and
//! strikes about once a second, a pinch or, one strike in two, a poisoned sting. It lets a target go out of `GIVE_UP`
//! blocks, dead, or a creative player.
use creature::body::{Behaviour, Body, World};

use crate::{kind, set_reared, strike};

/// Within this it can strike (blocks); it chases up to `GIVE_UP`, striking every `EVERY` ticks, at `CHASE_SPEED`
/// (blocks a tick).
const REACH: f64 = 1.1;
const GIVE_UP: f64 = 12.0;
const EVERY: i32 = 20;
const CHASE_SPEED: f64 = 0.1;

/// The snap of its claws on a strike (`sources/sounds.toml`).
const STRIKE: &str = "scorpions:entity.scorpion.strike";

#[derive(Debug, Default)]
pub(crate) struct Fight {
    cooldown: i32,
    strikes: u32,
}

impl Behaviour for Fight {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        if creature::anger::pursued(m, cx, GIVE_UP).is_none() {
            set_reared(m.id, false);
            return false;
        }
        true
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        set_reared(m.id, false);
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some((who, t)) = creature::anger::pursued(m, cx, GIVE_UP) else {
            return;
        };
        set_reared(m.id, true);
        let (dx, dz) = (t[0] - m.position.x, t[2] - m.position.z);
        let reach = dx.hypot(dz);
        let yaw = (-dx).atan2(dz).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
        self.cooldown -= 1;
        if reach < REACH {
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            if self.cooldown <= 0 {
                self.strikes += 1;
                // One strike in two is a sting: its kind's poison.
                let sting = self.strikes.is_multiple_of(2).then(|| kind(m.id));
                strike(m.id, who, sting);
                let pitch = 0.9 + m.random.next_float() * 0.2;
                creature::sounds::queue(m.id, STRIKE, 0.8, pitch);
                self.cooldown = EVERY;
            }
            return;
        }
        m.velocity.x = dx / reach * CHASE_SPEED;
        m.velocity.z = dz / reach * CHASE_SPEED;
    }
}
