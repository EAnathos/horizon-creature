//! The vulture's sounds (SPECS §7).
//!
//! The wandering trader it copies has its own, and Horizon always prefixes mob sounds with `minecraft:`: we mute it
//! (`Mob.silent`) and play ours, `vulture:entity.vulture.*` (`content/assets/vulture/sounds.json`), with
//! `Host::play_sound`, which accepts a mod's sounds.
//!
//! - Goals, which have no host, queue the call and the wing flaps (`queue`).
//! - `entity_tick_post` empties the queue of the vulture that just ticked, and hears its injuries and its death by
//!   comparing its health from one tick to the next (goals no longer run once it dies).
use std::collections::HashMap;
use std::sync::Mutex;

use horizon_client::mod_api::{EntityTickPostEvent, Host, Tag};

pub const AMBIENT: &str = "vulture:entity.vulture.ambient";
pub const HURT: &str = "vulture:entity.vulture.hurt";
pub const DEATH: &str = "vulture:entity.vulture.death";
pub const FLAP: &str = "vulture:entity.vulture.flap";

/// `SoundSource.NEUTRAL`, like animals.
const NEUTRAL: usize = 6;

/// A sound a goal asked for: (vulture, sound, volume, pitch).
static QUEUE: Mutex<Vec<(u32, &'static str, f32, f32)>> = Mutex::new(Vec::new());

/// Each vulture's health at the previous tick.
static HEALTH: Mutex<Option<HashMap<u32, f32>>> = Mutex::new(None);

pub fn queue(mob: u32, sound: &'static str, volume: f32, pitch: f32) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push((mob, sound, volume, pitch));
    }
}

/// `Health` in the entity's data (a float, kept as IEEE bits).
fn health(host: &mut dyn Host, id: u32) -> Option<f32> {
    let entity = host.entity(id)?;
    let nbt = host.entity_nbt(entity)?;
    nbt.iter().find_map(|(k, v)| match v {
        Tag::Float(bits) if k.to_string_lossy() == "Health" => Some(f32::from_bits(*bits)),
        _ => None,
    })
}

pub fn tick_post(host: &mut dyn Host, event: &EntityTickPostEvent<'_>) {
    let id = event.entity;
    let Some(at) = host.entity(id).and_then(|e| host.entity_position(e)) else {
        return;
    };
    let mut play = Vec::new();
    if let Ok(mut q) = QUEUE.lock() {
        q.retain(|&(mob, sound, volume, pitch)| {
            if mob == id {
                play.push((sound, volume, pitch));
            }
            mob != id
        });
    }
    if let Some(now) = health(host, id) {
        let before = HEALTH
            .lock()
            .ok()
            .and_then(|mut h| {
                let h = h.get_or_insert_with(HashMap::new);
                if now <= 0.0 { h.remove(&id) } else { h.insert(id, now) }
            });
        if let Some(before) = before.filter(|&b| now < b && b > 0.0) {
            // A slightly different voice per vulture, as `LivingEntity.getVoicePitch` varies; harder hits sound higher.
            let pitch = 0.9 + (id % 5) as f32 * 0.05;
            play.push(if now <= 0.0 {
                (DEATH, 1.0, pitch)
            } else {
                (HURT, 1.0, pitch + (before - now).min(4.0) * 0.01)
            });
        }
    }
    for (sound, volume, pitch) in play {
        host.play_sound(sound, at, NEUTRAL, volume, pitch);
    }
}
