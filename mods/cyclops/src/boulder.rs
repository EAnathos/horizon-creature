//! The boulder (SPECS §2): what a cyclops throws at a target out of its club's reach. A projectile of the library's
//! (`creature::projectile`: `cyclops:boulder`, a copy of the wandering trader that can't be hurt), launched on an arc
//! toward where the target stood, tumbling as it goes. It hits the first time it comes within reach of its target (its
//! `minecraft:attack_damage`, `DAMAGE`) and shatters there, on the ground, against a wall, or after `LIFE` ticks: stone
//! dust and a crash, and it is gone. It tears up the blocks around where it lands (`smash`).
use std::sync::Mutex;

use creature::projectile::{self, Launch};
use mod_api::{BlockEntityDrawContext, EntityRenderState, EntityRenderer, ParticleOptions, ServerHost, SoundSource};

use crate::{model_boulder, smash};

pub const BOULDER: &str = "cyclops:boulder";
const TEXTURE: &str = "cyclops:entity/cyclops/boulder";
/// Its box.
pub const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.9,
    height: 0.9,
    eye_height: 0.45,
};
/// What it deals (its `minecraft:attack_damage`) and the push it adds (`minecraft:attack_knockback`).
pub const DAMAGE: f64 = 10.0;
pub const KNOCKBACK: f64 = 1.0;
/// Its gravity (blocks per tick, each tick) and its speed along the ground (blocks per tick).
const GRAVITY: f64 = 0.05;
const SPEED: f64 = 0.9;
/// How close it must come to its target to hit it (blocks), and how long it may fly (ticks).
const REACH: f64 = 0.7;
const LIFE: i32 = 120;

/// A throw asked by a cyclops's fight (from its behaviour, which can't summon): where from, its velocity and its
/// target (an entity id).
pub struct Throw {
    pub from: [f64; 3],
    pub velocity: [f64; 3],
    pub target: u32,
}

/// Throws to make, from the cyclopes' ticks (`launch`).
pub static THROWS: Mutex<Vec<Throw>> = Mutex::new(Vec::new());

/// The velocity that carries a boulder from `from` to `to` on an arc, at `SPEED` along the ground, with `GRAVITY`.
pub fn aim(from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    projectile::aim(from, to, SPEED, GRAVITY)
}

/// From a cyclops's tick: the throws its fight asked for, as boulders in the world.
pub fn launch(host: &mut dyn ServerHost, dimension: &str) {
    let throws = std::mem::take(&mut *THROWS.lock().unwrap());
    for t in throws {
        let launch = Launch {
            velocity: t.velocity,
            gravity: GRAVITY,
            life: LIFE,
            target: Some((t.target, REACH)),
        };
        projectile::launch(host, dimension, BOULDER, t.from, launch);
    }
}

/// From the boulder's tick: its flight, then, over (it hit, landed or hit a wall), it bursts into stone dust with a
/// crash and is gone, tearing up the blocks around it.
pub fn tick(host: &mut dyn ServerHost, entity: u32) {
    projectile::tick_post(host, entity);
    if projectile::ended(entity).is_none() {
        return;
    }
    let Some(e) = host.entity(entity) else {
        return;
    };
    if let (Some(at), Some(level)) = (e.position(host), e.level(host)) {
        if let (Some(dust), Some(stone)) = (
            creature::particles::key("minecraft:block"),
            host.block_state_id("minecraft:stone"),
        ) {
            let options = ParticleOptions::simple(dust);
            level.send_particles(
                host,
                &options,
                Some(stone),
                [at[0], at[1] + 0.4, at[2]],
                [0.4, 0.3, 0.4],
                0.1,
                40,
            );
        }
        smash::tear(host, level, at, smash::BOULDER_RADIUS);
        level.play_sound(
            host,
            "cyclops:entity.boulder.shatter",
            at,
            SoundSource::Hostile,
            1.5,
            1.0,
            None,
        );
        level.play_sound(
            host,
            "minecraft:entity.generic.explode",
            at,
            SoundSource::Hostile,
            0.4,
            1.6,
            None,
        );
    }
    e.discard(host);
    projectile::forget(entity);
}

/// From the mod's `living_death` (or a removal): forgets it.
pub fn forget(entity: u32) {
    projectile::forget(entity);
}

/// Draws a boulder: tumbling end over end along its flight.
pub struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let spin = entity.age * 18.0;
        creature::model::draw(draw, &model_boulder::BOULDER, TEXTURE, &|part| {
            if part == "boulder" {
                [spin, 0.0, spin * 0.4]
            } else {
                [0.0; 3]
            }
        });
        Ok(())
    }
}

/// Draws the boulder a cyclops holds over its head before it throws (no tumbling), at `[x, y, z]` blocks from its feet.
pub fn held(draw: &mut dyn BlockEntityDrawContext, at: [f32; 3]) {
    draw.push();
    draw.translate(at);
    creature::model::draw(draw, &model_boulder::BOULDER, TEXTURE, &|_| [0.0; 3]);
    draw.pop();
}
