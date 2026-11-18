//! Taming and the tamed vulture (SPECS §5, §6).
//!
//! - `Eat`: it walks to the meat the hook found on the ground (`state::food`) and eats it. A wild vulture eating a raw
//!   chicken is tamed 1 time in 4 (hearts; smoke otherwise), by the player it sees (in single player, the one who
//!   threw it). A tamed one eats only when hurt, and heals.
//! - `Sit`: it stays put when told to (right-click, `interaction.rs`).
//! - `FollowOwner`: flies to its owner when more than 10 blocks away and lands near them; teleports past 32 blocks.
//! - What hurts its owner, and what its owner attacks, becomes its target (as vanilla's wolf: the mod's damage hook,
//!   `crate::defend`).
//! - `Mate` (SPECS §8): fed rotten flesh by its owner (`interaction.rs`), a grown vulture is in love for 30 s; it walks to
//!   another tamed vulture in love and, once close, they have a baby (born tamed, `state::breed`) and wait 5 minutes.
use creature::body::{Behaviour, Body, World};

use crate::flight::{Controls, Flight};
use crate::{ride, sounds, state};

/// One raw chicken in this many tames it.
const TAME_CHANCE: i32 = 4;
/// How fast it walks to food (a share of its walking speed, `minecraft:movement_speed` 0.3 in `vulture.json`): a
/// vulture hurries to a carcass, about twice as fast as it strolls (0.6 felt too slow, the trader's 0.5 speed rushed).
const FOOD_PACE: f64 = 1.2;
/// What a meat eaten heals a tamed vulture.
const MEAT_HEAL: f32 = 4.0;

fn particles(m: &Body, kind: &'static str) {
    crate::particles(m.id, kind, [m.position.x, m.position.y + 1.0, m.position.z], 7);
}

/// Whether it has a target (fighting), as its mob says.
pub fn fighting(m: &Body, cx: &World<'_>) -> bool {
    crate::target(cx, m.id).is_some()
}

fn free(m: &Body, cx: &World<'_>) -> bool {
    ride::of(m.id).is_none() && !fighting(m, cx)
}

/// Following its owner: free, not sitting, not on a lead (the lead leads it), and not a chick (a juvenile does).
fn may_follow(m: &Body, cx: &World<'_>) -> bool {
    let s = state::saved(m.id);
    free(m, cx) && !s.sitting && !state::leashed(m.id) && !state::chick(s.age)
}

#[derive(Debug, Default)]
pub struct Eat {
    ticks: i32,
}

impl Behaviour for Eat {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = free(m, cx) && !state::saved(m.id).sitting && state::food(m.id).is_some();
        if !on && self.ticks > 0 {
            self.ticks = 0;
        }
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.ticks = 0;
        m.stop_walking = true;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(food) = state::food(m.id) else {
            return;
        };
        self.ticks += 1;
        m.look_at = Some(food.at);
        let d2 = (m.position.x - food.at[0]).powi(2) + (m.position.z - food.at[2]).powi(2);
        if d2 > 1.5 * 1.5 || (m.position.y - food.at[1]).abs() > 2.0 {
            if self.ticks % 10 == 1 {
                m.walk_to = Some((food.at, FOOD_PACE));
            }
            return;
        }
        state::eaten(m.id, food);
        state::pecked(m);
        let pitch = 0.9 + m.random.next_float() * 0.2;
        sounds::queue(m.id, sounds::EAT, 1.0, pitch);
        if state::saved(m.id).owner.is_some() {
            m.health = (m.health + MEAT_HEAL).min(m.max_health);
        } else if food.taming
            && let Some(p) = cx
                .player
                .as_ref()
                .filter(|p| food.thrower.is_none_or(|t| t == p.entity_id))
        {
            // The player who threw it now knows a raw chicken tames a vulture (the bestiary).
            state::taming_seen(p.uuid);
            if m.random.next_int_bound(TAME_CHANCE).unwrap_or(1) == 0 {
                let owner = creature::friend::ints(p.uuid);
                state::with(m.id, |s| {
                    s.owner = Some(owner);
                    s.sitting = false;
                });
                state::touch(m.id);
                particles(m, "minecraft:heart");
            } else {
                particles(m, "minecraft:smoke");
            }
        }
    }
}

#[derive(Debug)]
pub struct Sit;

impl Behaviour for Sit {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        let s = state::saved(m.id);
        s.owner.is_some() && s.sitting && ride::of(m.id).is_none()
    }
    fn tick(&mut self, m: &mut Body, _: &World<'_>) {
        m.stop_walking = true;
        m.velocity.x = 0.0;
        m.velocity.z = 0.0;
    }
}

/// Starts following past this distance, lands within `LAND`, teleports past `TELEPORT` (blocks).
const FOLLOW: f64 = 10.0;
const LAND: f64 = 5.0;
const TELEPORT: f64 = 32.0;
/// How high above its owner it flies when following.
const ABOVE: f64 = 5.0;

#[derive(Debug, Default)]
pub struct FollowOwner {
    flight: Flight,
    landing: bool,
    on: bool,
}

/// Where its owner is, when it is tamed by the nearest player.
fn owner_at(m: &Body, cx: &World<'_>) -> Option<[f64; 3]> {
    let p = cx.player.as_ref()?;
    let owner = state::saved(m.id).owner?;
    (owner == creature::friend::ints(p.uuid) && p.alive).then_some([p.position.x, p.position.y, p.position.z])
}

fn distance(m: &Body, at: [f64; 3]) -> f64 {
    ((m.position.x - at[0]).powi(2) + (m.position.z - at[2]).powi(2)).sqrt()
}

impl Behaviour for FollowOwner {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let on = if self.on {
            may_follow(m, cx) && owner_at(m, cx).is_some() && !(self.landing && m.on_ground)
        } else {
            may_follow(m, cx) && owner_at(m, cx).is_some_and(|at| distance(m, at) > FOLLOW)
        };
        if on && !self.on {
            self.flight = Flight::default();
            self.landing = false;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(at) = owner_at(m, cx) else {
            return;
        };
        let d = distance(m, at);
        if d > TELEPORT {
            m.position.x = at[0] + 2.0;
            m.position.y = at[1] + 2.0;
            m.position.z = at[2] + 2.0;
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
                ridden: false,
            }
        } else {
            Controls {
                yaw,
                pitch: (-below * 3.0).clamp(-35.0, 35.0) as f32,
                flap: true,
                brake: false,
                boost: m.on_ground,
                ridden: false,
            }
        };
        self.flight.tick(m, &controls);
    }
}

/// How close two vultures in love must come to breed, in blocks.
const MATE_RANGE: f64 = 2.5;
/// How far it looks for a partner.
const MATE_SEARCH: f64 = 8.0;

#[derive(Debug, Default)]
pub struct Mate {
    partner: Option<u32>,
    /// Bred already this time (a baby once per pair, whatever the ticks left of it).
    bred: bool,
}

impl Mate {
    /// The nearest other tamed vulture in love.
    fn find(m: &Body, cx: &World<'_>) -> Option<u32> {
        let host = cx.host();
        let r = MATE_SEARCH;
        let (x, y, z) = (m.position.x, m.position.y, m.position.z);
        cx.ground
            .level
            .entities_in(host, [x - r, y - r, z - r], [x + r, y + r, z + r])
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.id() != m.id && e.kind(host).is_some_and(|k| k.id(host) == crate::KIND))
            .filter(|e| state::in_love(e.id()) && state::saved(e.id()).owner.is_some())
            .filter_map(|e| Some((e.id(), e.position(host)?)))
            .map(|(id, at)| (id, (at[0] - x).powi(2) + (at[2] - z).powi(2)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }
}

impl Behaviour for Mate {
    fn can_use(&mut self, m: &mut Body, cx: &World<'_>) -> bool {
        let s = state::saved(m.id);
        if !(s.owner.is_some() && state::love(m.id) > 0 && s.age == 0 && free(m, cx)) {
            if self.partner.take().is_some() {
                self.bred = false;
            }
            return false;
        }
        if self.partner.is_none() {
            self.partner = Self::find(m, cx);
            self.bred = false;
        }
        self.partner.is_some()
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.partner = None;
        m.stop_walking = true;
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(partner) = self.partner else {
            return;
        };
        let Some(at) = cx.host().entity(partner).and_then(|e| e.position(cx.host())) else {
            self.partner = None;
            return;
        };
        m.look_at = Some(at);
        let d2 = (at[0] - m.position.x).powi(2) + (at[2] - m.position.z).powi(2);
        if d2 > MATE_RANGE * MATE_RANGE {
            m.walk_to = Some((at, FOOD_PACE));
            return;
        }
        // Both see each other close; the one with the smaller id brings the baby, both wait.
        if self.bred {
            return;
        }
        self.bred = true;
        if m.id < partner {
            let rolls = [m.random.next_float(), m.random.next_float()];
            state::breed(m, partner, rolls);
            particles(m, "minecraft:heart");
        }
        state::end_love(m.id);
        state::with(m.id, |s| s.age = state::BREED_PAUSE);
        state::touch(m.id);
    }
}
