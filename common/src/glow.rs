//! Lights around glowing creatures (the fairies' night light, the fireflies' lanterns): coloured light the client
//! draws (`visuals`), seen only, lighting nothing in the world.
//!
//! The game takes at most 8 such lights from all mods together, and only within 32 blocks of the player, so the
//! library hands them out: to the creatures nearest a player. The client's `visuals` hook knows no positions: a mod
//! notes each glowing creature from its server tick (`shine`), the library's `visuals` draws them (single player, as
//! the rest of the series).
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use mod_api::{ColoredLight, ServerHost, VisualsEvent};

/// A creature's light: its colour (`0xRRGGBB`), reach (blocks) and strength (0 to 1), how high above its feet, and
/// whether it shines at this moment (a firefly's pulse).
#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub rgb: u32,
    pub radius: f32,
    pub intensity: f32,
    pub lift: f64,
    pub shown: fn(u32) -> bool,
}

/// The game's budget, for all mods.
const LIGHTS: usize = 8;
/// How far from the player the game draws one.
const REACH: f64 = 32.0;
/// A light not noted again for this long is dropped (its creature unloaded, say).
const KEPT_SECONDS: u64 = 2;

struct Shining {
    at: [f64; 3],
    light: Light,
    /// How far the nearest player was.
    near: f64,
    when: Instant,
}

static SHINING: Mutex<BTreeMap<u32, Shining>> = Mutex::new(BTreeMap::new());

/// From the mod's `entity_tick_post`: creature `entity`, in level `dimension`, shines `light` this tick.
pub fn shine(host: &dyn ServerHost, dimension: &str, entity: u32, light: Light) {
    let Some(at) = host.entity(entity).and_then(|e| e.position(host)) else {
        return;
    };
    let near = crate::mob::nearest_player(host, dimension, at);
    let at = [at[0], at[1] + light.lift, at[2]];
    let when = Instant::now();
    SHINING
        .lock()
        .unwrap()
        .insert(entity, Shining { at, light, near, when });
}

/// Creature `entity` no longer shines (it landed, it died).
pub fn dim(entity: u32) {
    SHINING.lock().unwrap().remove(&entity);
}

/// From the library's `visuals`: the lights of the creatures nearest a player.
pub fn visuals(event: &mut VisualsEvent<'_>) {
    let mut shining = SHINING.lock().unwrap();
    shining.retain(|_, s| s.when.elapsed().as_secs() < KEPT_SECONDS);
    let mut lights: Vec<(&u32, &Shining)> = shining
        .iter()
        .filter(|(id, s)| s.near <= REACH && (s.light.shown)(**id))
        .collect();
    lights.sort_by(|a, b| a.1.near.total_cmp(&b.1.near));
    for (_, s) in lights.into_iter().take(LIGHTS) {
        let l = s.light;
        event.lights.push(ColoredLight::new(s.at, l.rgb, l.radius, l.intensity));
    }
}
