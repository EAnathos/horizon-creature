//! It fights back (SPECS §3): once something hits it (the mod's `living_incoming_damage` notes who: `provoke`), the
//! scorpion runs at them and strikes about once a second, a pinch or, one strike in two, a poisoned sting. It lets a
//! target go out of `GIVE_UP` blocks, dead, or a creative player.
use std::collections::BTreeMap;
use std::sync::Mutex;

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

/// Each scorpion's target: who hit it last (an entity id).
static TARGETS: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

/// From the mod's damage hook: `scorpion` was hit by `by`.
pub(crate) fn provoke(scorpion: u32, by: u32) {
    TARGETS.lock().unwrap().insert(scorpion, by);
}

/// Whom `scorpion` fights, if anyone.
pub(crate) fn target(scorpion: u32) -> Option<u32> {
    TARGETS.lock().unwrap().get(&scorpion).copied()
}

pub(crate) fn forget(scorpion: u32) {
    TARGETS.lock().unwrap().remove(&scorpion);
}

#[derive(Debug, Default)]
pub(crate) struct Fight {
    cooldown: i32,
    strikes: u32,
}

/// The target's position, if it is still one to fight (alive, not creative, near enough).
fn chased(m: &Body, cx: &World<'_>, target: u32) -> Option<[f64; 3]> {
    let host = cx.host();
    let e = host.entity(target)?;
    let at = e.position(host)?;
    let alive = host.living(e).and_then(|l| l.alive(host)).unwrap_or(false);
    // The nearest player is the one a behaviour knows: a creative one isn't fought.
    let creative = cx.player.as_ref().is_some_and(|p| p.entity_id == target && p.creative);
    let far = (at[0] - m.position.x).hypot(at[2] - m.position.z) > GIVE_UP;
    (alive && !creative && !far && e.level(host) == Some(cx.ground.level)).then_some(at)
}

impl Behaviour for Fight {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let Some(t) = target(m.id) else {
            return false;
        };
        if chased(m, cx, t).is_none() {
            forget(m.id);
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
        let Some(who) = target(m.id) else {
            return;
        };
        let Some(t) = chased(m, cx, who) else {
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
