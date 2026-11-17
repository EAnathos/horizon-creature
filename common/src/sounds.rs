//! A mod mob's own sounds (SPECS §3.2).
//!
//! Its ambient, hurt, death and step sounds are its entity type's (`"sounds"` in `data/<mod>/sunrise/entity_type/`),
//! played by the game. What its goals play besides (the worm's dig, a flap) goes through here: goals have no host to
//! play a sound with, so they queue it (`queue`), and the mod's `entity_tick_post` plays what its mob queued
//! (`play_queued`).
use std::sync::Mutex;

use mod_api::{ServerHost, SoundSource};

/// A sound a goal asked for: (mob, sound, volume, pitch).
static QUEUE: Mutex<Vec<(u32, &'static str, f32, f32)>> = Mutex::new(Vec::new());

pub fn queue(mob: u32, sound: &'static str, volume: f32, pitch: f32) {
    if let Ok(mut q) = QUEUE.lock() {
        q.push((mob, sound, volume, pitch));
    }
}

/// What mob `id` queued, taken out of the queue.
fn queued(id: u32) -> Vec<(&'static str, f32, f32)> {
    let mut play = Vec::new();
    if let Ok(mut q) = QUEUE.lock() {
        q.retain(|&(mob, sound, volume, pitch)| {
            if mob == id {
                play.push((sound, volume, pitch));
            }
            mob != id
        });
    }
    play
}

/// From the mod's `entity_tick_post`: plays what mob `id` queued, in `source` (`SoundSource::Neutral` for animals,
/// `Hostile` for monsters), heard by every player near it.
pub fn play_queued(host: &mut dyn ServerHost, id: u32, source: SoundSource) {
    let play = queued(id);
    if play.is_empty() {
        return;
    }
    let Some(entity) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (entity.position(host), entity.level(host)) else {
        return;
    };
    for (sound, volume, pitch) in play {
        level.play_sound(host, sound, at, source, volume, pitch, None);
    }
}
