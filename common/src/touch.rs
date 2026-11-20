//! Creatures that hurt what touches them (the storm spirits' zap, the jellyfish's sting): each tick the mod asks which
//! bodies touch its creature and are due a touch again (`due`), at most once every so many ticks each, and does its
//! own harm to them.
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{LevelRef, ServerHost};

/// Ticks until each (creature, body) pair may touch again.
static WAITS: Mutex<BTreeMap<(u32, u32), i32>> = Mutex::new(BTreeMap::new());

/// From the mod's `entity_tick_post`: the mobs and players touching creature `entity` of `kind` (its middle at `middle`,
/// within `reach` blocks) that are due a touch, each at most once every `every` ticks while it stays in touch.
pub fn due(
    host: &dyn ServerHost,
    level: LevelRef,
    middle: [f64; 3],
    kind: &str,
    entity: u32,
    reach: f64,
    every: i32,
) -> Vec<u32> {
    let touching = crate::projectile::hits(host, level, middle, kind, entity, reach);
    let mut waits = WAITS.lock().unwrap();
    // A body that moved away may be touched again at once when it comes back.
    waits.retain(|(e, b), _| *e != entity || touching.contains(b));
    touching
        .into_iter()
        .filter(|&body| {
            let wait = waits.entry((entity, body)).or_insert(0);
            *wait -= 1;
            if *wait <= 0 {
                *wait = every;
                true
            } else {
                false
            }
        })
        .collect()
}

/// `entity` is gone (or a body it touched).
pub fn forget(entity: u32) {
    WAITS.lock().unwrap().retain(|(e, b), _| *e != entity && *b != entity);
}
