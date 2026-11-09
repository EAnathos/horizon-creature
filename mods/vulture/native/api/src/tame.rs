//! Taming and the tamed vulture (SPECS §5, §6).
//!
//! - `Eat`: it walks to the meat the hook found on the ground (`state::food`) and eats it. A wild vulture eating a raw
//!   chicken is tamed 1 time in 4 (hearts; smoke otherwise), by the player it sees (in single player, the one who
//!   threw it). A tamed one eats only when hurt, and heals.
//! - `Sit`: it stays put when told to (right-click, `interact.rs`).
//! - `FollowOwner`: flies to its owner when more than 10 blocks away and lands near them; teleports past 32 blocks.
//! - `OwnerHurtBy` / `OwnerHurt`: it attacks what hurts its owner, and what its owner attacks (as vanilla's wolf).
use horizon_client::mobs::goals::{self, Cx, Goal};
use horizon_client::mobs::{MobEvent, Who};
use horizon_client::mod_api::{Mob, RandomSource};

use crate::flight::{Controls, Flight};
use crate::{sounds, state};

/// One raw chicken in this many tames it.
const TAME_CHANCE: i32 = 4;
/// What a meat eaten heals a tamed vulture.
const MEAT_HEAL: f32 = 4.0;

fn particles(m: &Mob, cx: &mut Cx, kind: &'static str) {
    cx.events.push(MobEvent::LevelParticles {
        kind,
        at: [m.position.x, m.position.y + 1.0, m.position.z],
        spread: [0.4, 0.4, 0.4],
        speed: 0.02,
        count: 7,
    });
}

fn free(m: &Mob) -> bool {
    m.rider.is_none() && m.ai.target.is_none()
}

#[derive(Clone, Debug, Default)]
pub struct Eat {
    ticks: i32,
}

impl Goal for Eat {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        free(m) && !m.extra.sitting && state::food(m.id).is_some()
    }
    fn start(&mut self, _: &mut Mob, _: &mut Cx) {
        self.ticks = 0;
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        m.nav_stop();
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(food) = state::food(m.id) else {
            return;
        };
        self.ticks += 1;
        m.look_at_speed(food.at, 30.0, 30.0);
        let d2 = (m.position.x - food.at[0]).powi(2) + (m.position.z - food.at[2]).powi(2);
        if d2 > 1.5 * 1.5 || (m.position.y - food.at[1]).abs() > 2.0 {
            if self.ticks % 10 == 1 {
                m.nav_move_to(cx.w, food.at, 1.0);
            }
            return;
        }
        state::eaten(m.id, food);
        let pitch = 0.9 + m.random.next_float() * 0.2;
        sounds::queue(m.id, sounds::EAT, 1.0, pitch);
        if m.extra.tame {
            m.heal(MEAT_HEAL);
        } else if food.taming
            && let Some(p) = cx.w.player.as_ref()
        {
            if m.random.next_int_bound(TAME_CHANCE).unwrap_or(1) == 0 {
                m.extra.tame = true;
                m.extra.owner = Some(p.uuid);
                m.extra.sitting = false;
                particles(m, cx, "minecraft:heart");
            } else {
                particles(m, cx, "minecraft:smoke");
            }
        }
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Sit;

impl Goal for Sit {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        m.extra.tame && m.extra.sitting && m.rider.is_none()
    }
    fn start(&mut self, m: &mut Mob, _: &mut Cx) {
        m.nav_stop();
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Starts following past this distance, lands within `LAND`, teleports past `TELEPORT` (blocks).
const FOLLOW: f64 = 10.0;
const LAND: f64 = 5.0;
const TELEPORT: f64 = 32.0;
/// How high above its owner it flies when following.
const ABOVE: f64 = 5.0;

#[derive(Clone, Debug, Default)]
pub struct FollowOwner {
    flight: Flight,
    landing: bool,
}

/// Where its owner is, when it is tamed by the player it sees.
fn owner_at(m: &Mob, cx: &Cx) -> Option<[f64; 3]> {
    let p = cx.w.player.as_ref()?;
    (m.extra.tame && m.extra.owner == Some(p.uuid) && p.alive).then_some([p.position.x, p.position.y, p.position.z])
}

fn distance(m: &Mob, at: [f64; 3]) -> f64 {
    ((m.position.x - at[0]).powi(2) + (m.position.z - at[2]).powi(2)).sqrt()
}

impl Goal for FollowOwner {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        free(m) && !m.extra.sitting && owner_at(m, cx).is_some_and(|at| distance(m, at) > FOLLOW)
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        free(m) && !m.extra.sitting && owner_at(m, cx).is_some() && !(self.landing && m.on_ground)
    }
    fn start(&mut self, _: &mut Mob, _: &mut Cx) {
        self.flight = Flight::default();
        self.landing = false;
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.flight.land(m);
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(at) = owner_at(m, cx) else {
            return;
        };
        let d = distance(m, at);
        if d > TELEPORT {
            m.position.x = at[0] + 2.0;
            m.position.y = at[1] + 2.0;
            m.position.z = at[2] + 2.0;
            m.previous = m.position;
            self.flight.speed = 0.0;
        }
        self.landing = d < LAND;
        let yaw = (-(at[0] - m.position.x)).atan2(at[2] - m.position.z).to_degrees() as f32;
        let below = at[1] + ABOVE - m.position.y;
        let controls = if self.landing {
            Controls {
                yaw,
                pitch: 15.0,
                flap: false,
                brake: true,
                boost: false,
            }
        } else {
            Controls {
                yaw,
                pitch: (-below * 3.0).clamp(-35.0, 35.0) as f32,
                flap: true,
                brake: false,
                boost: m.on_ground,
            }
        };
        self.flight.tick(m, &controls);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Never its owner, nor itself.
fn may_attack(m: &Mob, who: Who) -> bool {
    matches!(who, Who::Mob(id) if id != m.id)
}

/// `OwnerHurtByTargetGoal`: attack what hurt the owner.
#[derive(Clone, Debug, Default)]
pub struct OwnerHurtBy {
    timestamp: i64,
}

impl Goal for OwnerHurtBy {
    fn flags(&self) -> u8 {
        goals::TARGET
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        if owner_at(m, cx).is_none() || m.extra.sitting || m.rider.is_some() {
            return false;
        }
        let Some((who, at)) = cx.w.player.as_ref().and_then(|p| p.hurt_by) else {
            return false;
        };
        at != self.timestamp && may_attack(m, who)
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        m.ai.target.and_then(|t| cx.w.target_info(t)).is_some_and(|t| t.alive)
    }
    fn start(&mut self, m: &mut Mob, cx: &mut Cx) {
        if let Some((who, at)) = cx.w.player.as_ref().and_then(|p| p.hurt_by) {
            m.ai.target = Some(who);
            self.timestamp = at;
        }
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        m.ai.target = None;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// `OwnerHurtTargetGoal`: attack what the owner attacks.
#[derive(Clone, Debug, Default)]
pub struct OwnerHurt {
    timestamp: i64,
}

impl Goal for OwnerHurt {
    fn flags(&self) -> u8 {
        goals::TARGET
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        if owner_at(m, cx).is_none() || m.extra.sitting || m.rider.is_some() {
            return false;
        }
        let Some((who, at)) = cx.w.player.as_ref().and_then(|p| p.last_hurt) else {
            return false;
        };
        at != self.timestamp && may_attack(m, who)
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        m.ai.target.and_then(|t| cx.w.target_info(t)).is_some_and(|t| t.alive)
    }
    fn start(&mut self, m: &mut Mob, cx: &mut Cx) {
        if let Some((who, at)) = cx.w.player.as_ref().and_then(|p| p.last_hurt) {
            m.ai.target = Some(who);
            self.timestamp = at;
        }
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        m.ai.target = None;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}
