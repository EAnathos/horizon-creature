//! The boulder (SPECS §2): what a cyclops throws at a target out of its club's reach. A mob of its own
//! (`cyclops:boulder`, a copy of the wandering trader that can't be hurt), launched on an arc toward where the target
//! stood and flown by its goal (`Flight`, its own gravity), tumbling as it goes. It hits the first time it comes within
//! reach of its target (`DAMAGE`, a push) and shatters there, on the ground, against a wall, or after `LIFE` ticks:
//! stone dust and a crash, and it is gone. It tears up the blocks around where it lands (`smash`).
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use horizon_client::mobs::Who;
use horizon_client::mobs::goals;
use horizon_client::mod_api::{Compound, Cx, Goal, Host, Mob, MobSubmitEvent, ParticleOptions, ParticleType, Tag};

use crate::{model_boulder, smash};

pub const BOULDER: &str = "cyclops:boulder";
const TEXTURE: &str = "cyclops:entity/cyclops/boulder";
/// Its box (blocks: width, height, eye height).
pub const SIZE: (f64, f64, f64) = (0.9, 0.9, 0.45);
/// What it deals and the push it adds.
pub const DAMAGE: f32 = 10.0;
pub const KNOCKBACK: f64 = 1.0;
/// Its gravity (blocks per tick, each tick) and its speed along the ground (blocks per tick).
const GRAVITY: f64 = 0.05;
const SPEED: f64 = 0.9;
/// How close it must come to its target's box to hit it (blocks), and how long it may fly (ticks).
const REACH: f64 = 0.7;
const LIFE: i32 = 120;

/// A throw asked by a cyclops's goal (which has no host): where from, its velocity and its target.
pub struct Throw {
    pub from: [f64; 3],
    pub velocity: [f64; 3],
    pub target: Who,
}

/// Throws to make, from the cyclopes' ticks (`launch`).
pub static THROWS: Mutex<Vec<Throw>> = Mutex::new(Vec::new());
/// Each new boulder's velocity and target, read by its goal on its first tick.
static FLIGHTS: Mutex<BTreeMap<u32, ([f64; 3], Who)>> = Mutex::new(BTreeMap::new());
/// Boulders to shatter at their next tick (the goal has no host).
static SHATTERED: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// The velocity that carries a boulder from `from` to `to` on an arc, at `SPEED` along the ground, with `GRAVITY`.
pub fn aim(from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    let (dx, dz) = (to[0] - from[0], to[2] - from[2]);
    let flat = dx.hypot(dz).max(0.1);
    let ticks = (flat / SPEED).ceil().max(1.0);
    // y after n ticks of v - g k: n v - g n (n - 1) / 2.
    let vy = (to[1] - from[1] + GRAVITY * ticks * (ticks - 1.0) / 2.0) / ticks;
    [dx / ticks, vy, dz / ticks]
}

/// From a cyclops's tick: the throws its goal asked for, as boulders in the world.
pub fn launch(host: &mut dyn Host, dimension: &str) {
    let throws = std::mem::take(&mut *THROWS.lock().unwrap());
    for t in throws {
        let mut nbt = Compound::new();
        nbt.insert("Invulnerable".into(), Tag::Byte(1));
        if let Some(id) = host.summon(dimension, BOULDER, t.from, nbt) {
            FLIGHTS.lock().unwrap().insert(id, (t.velocity, t.target));
        }
    }
}

/// From the boulder's tick: shattered, it bursts into stone dust with a crash and is gone.
pub fn tick(host: &mut dyn Host, dimension: &str, entity: u32) {
    if !SHATTERED.lock().unwrap().remove(&entity) {
        return;
    }
    if let Some(at) = host.entity(entity).and_then(|e| host.entity_position(e)) {
        let stone = host
            .block_states()
            .iter()
            .find(|(_, f)| &*f.name == "minecraft:stone")
            .map(|(id, _)| id);
        let dust = ParticleOptions::simple(ParticleType::Block);
        host.send_particles(&dust, stone, [at[0], at[1] + 0.4, at[2]], [0.4, 0.3, 0.4], 0.1, 40);
        smash::tear(host, dimension, at, smash::BOULDER_RADIUS);
        host.play_sound("minecraft:block.stone.break", at, creature::sounds::HOSTILE, 1.5, 0.6);
        host.play_sound(
            "minecraft:entity.generic.explode",
            at,
            creature::sounds::HOSTILE,
            0.4,
            1.6,
        );
    }
    host.remove_mob(dimension, entity);
}

/// From the mod's `living_death` (or a removal): forgets it.
pub fn forget(entity: u32) {
    FLIGHTS.lock().unwrap().remove(&entity);
    SHATTERED.lock().unwrap().remove(&entity);
}

/// The boulder's flight: its own velocity and gravity every tick, until it hits or lands.
#[derive(Clone, Debug, Default)]
pub struct Flight {
    velocity: Option<[f64; 3]>,
    target: Option<Who>,
    age: i32,
    done: bool,
}

impl Flight {
    fn shatter(&mut self, m: &Mob) {
        self.done = true;
        SHATTERED.lock().unwrap().insert(m.id);
    }
}

impl Goal for Flight {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn can_continue(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        !self.done
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        if self.done {
            return;
        }
        self.age += 1;
        m.no_gravity = true;
        if self.velocity.is_none()
            && let Some((velocity, target)) = FLIGHTS.lock().unwrap().remove(&m.id)
        {
            self.velocity = Some(velocity);
            self.target = Some(target);
        }
        // A boulder without a throw (one saved mid-flight) shatters.
        let (Some(mut v), Some(who)) = (self.velocity, self.target) else {
            if self.age > 2 {
                self.shatter(m);
            }
            return;
        };
        if let Some(t) = cx.w.target_info(who) {
            let mut centre = m.position;
            centre.y += SIZE.1 / 2.0;
            if t.alive && t.bounds.distance_squared_to_point(centre) < REACH * REACH {
                horizon_client::mobs::ai::do_hurt_target(m, who, cx);
                self.shatter(m);
                return;
            }
        }
        if self.age > 1 && (m.on_ground || m.horizontal_collision) || self.age > LIFE {
            self.shatter(m);
            return;
        }
        v[1] -= GRAVITY;
        self.velocity = Some(v);
        m.velocity.x = v[0];
        m.velocity.y = v[1];
        m.velocity.z = v[2];
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Draws a boulder: tumbling end over end along its flight.
pub fn submit(host: &dyn Host, event: &mut MobSubmitEvent<'_>) {
    let mob = event.mob;
    let spin = mob.age * 18.0;
    creature::model::draw(host, &mut *event.draw, &model_boulder::BOULDER, TEXTURE, &|part| {
        if part == "boulder" {
            [spin, 0.0, spin * 0.4]
        } else {
            [0.0; 3]
        }
    });
}

/// Draws the boulder a cyclops holds over its head before it throws (no tumbling), at `[x, y, z]` blocks from its feet.
pub fn held(host: &dyn Host, draw: &mut horizon_client::mod_api::BlockEntityDraw, at: [f32; 3]) {
    draw.push();
    draw.translate(at);
    creature::model::draw(host, draw, &model_boulder::BOULDER, TEXTURE, &|_| [0.0; 3]);
    draw.pop();
}
