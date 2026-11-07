//! Les sons du vautour (SPECS §7).
//!
//! Le marchand ambulant qu'il copie a les siens, et Horizon préfixe d'office les sons des mobs par
//! `minecraft:` : on le rend muet (`Mob.silent`) et on joue les nôtres, `vulture:entity.vulture.*`
//! (`content/assets/vulture/sounds.json`), avec `Host::play_sound`, qui accepte les sons d'un mod.
//!
//! - Les goals, qui n'ont pas d'hôte, mettent en file le cri et les battements d'ailes (`queue`).
//! - `entity_tick_post` vide la file du vautour qui vient de tiquer, et entend ses blessures et sa
//!   mort en comparant sa vie d'un tick à l'autre (les goals ne tournent plus quand il meurt).
use std::collections::HashMap;
use std::sync::Mutex;

use horizon_client::mod_api::{EntityTickPostEvent, Host, Tag};

pub const AMBIENT: &str = "vulture:entity.vulture.ambient";
pub const HURT: &str = "vulture:entity.vulture.hurt";
pub const DEATH: &str = "vulture:entity.vulture.death";
pub const FLAP: &str = "vulture:entity.vulture.flap";

/// `SoundSource.NEUTRAL`, comme les animaux.
const NEUTRAL: usize = 6;

/// Un son demandé par un goal : (vautour, son, volume, hauteur).
static QUEUE: Mutex<Vec<(u32, &'static str, f32, f32)>> = Mutex::new(Vec::new());

/// La vie de chaque vautour au tick précédent.
static HEALTH: Mutex<Option<HashMap<u32, f32>>> = Mutex::new(None);

pub fn queue(mob: u32, sound: &'static str, volume: f32, pitch: f32) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push((mob, sound, volume, pitch));
    }
}

/// `Health` dans les données de l'entité (un float, gardé en bits IEEE).
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
            // Plus grave et plus fort pour la mort, comme `LivingEntity.getVoicePitch` varie.
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
