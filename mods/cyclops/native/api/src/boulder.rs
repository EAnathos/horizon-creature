//! The boulder (SPECS §2): what a cyclops throws at a target out of its club's reach. A projectile of the library's
//! (`creature::projectile`: `cyclops:boulder`, a copy of the wandering trader that can't be hurt), launched on an arc
//! toward where the target stood, tumbling as it goes. It hits the first time it comes within
//! reach of its target (`DAMAGE`, a push) and shatters there, on the ground, against a wall, or after `LIFE` ticks:
//! stone dust and a crash, and it is gone. It tears up the blocks around where it lands (`smash`).
use std::sync::Mutex;

use creature::projectile::{self, Launch};
use horizon_client::mobs::Who;
use horizon_client::mod_api::{Host, MobSubmitEvent, ParticleOptions, ParticleType};

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

/// The velocity that carries a boulder from `from` to `to` on an arc, at `SPEED` along the ground, with `GRAVITY`.
pub fn aim(from: [f64; 3], to: [f64; 3]) -> [f64; 3] {
    projectile::aim(from, to, SPEED, GRAVITY)
}

/// From a cyclops's tick: the throws its goal asked for, as boulders in the world.
pub fn launch(host: &mut dyn Host, dimension: &str) {
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

/// From the boulder's tick: its flight over (it hit, landed or hit a wall), it bursts into stone dust with a crash and is
/// gone, tearing up the blocks around it.
pub fn tick(host: &mut dyn Host, dimension: &str, entity: u32) {
    if projectile::ended(entity).is_none() {
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
        host.play_sound(
            "cyclops:entity.boulder.shatter",
            at,
            creature::sounds::HOSTILE,
            1.5,
            1.0,
        );
        host.play_sound(
            "minecraft:entity.generic.explode",
            at,
            creature::sounds::HOSTILE,
            0.4,
            1.6,
        );
    }
    host.remove_mob(dimension, entity);
    projectile::forget(entity);
}

/// From the mod's `living_death` (or a removal): forgets it.
pub fn forget(entity: u32) {
    projectile::forget(entity);
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
