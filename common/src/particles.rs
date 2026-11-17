//! The vanilla particles the series' mods send. Sunrise hands out a particle's key only at registration
//! (`ParticlesRegistrar::particle_key`): the library asks for every one of these in its `register`, and the mods take
//! them from here (`key`).
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{ParticleKey, Registrar};

/// Every particle a mod of the series sends.
const USED: &[&str] = &[
    "minecraft:happy_villager",
    "minecraft:dust",
    "minecraft:wax_on",
    "minecraft:firework",
    "minecraft:block",
    "minecraft:end_rod",
    "minecraft:glow",
    "minecraft:electric_spark",
    "minecraft:poof",
    "minecraft:crit",
    "minecraft:smoke",
    "minecraft:heart",
    "minecraft:effect",
    "minecraft:entity_effect",
    "minecraft:item",
];

static KEYS: Mutex<BTreeMap<&'static str, ParticleKey>> = Mutex::new(BTreeMap::new());

/// From the library's `register`.
pub fn declare(r: &mut dyn Registrar) {
    let mut keys = KEYS.lock().unwrap();
    for id in USED {
        keys.insert(id, r.particle_key(id));
    }
}

/// The key of particle `id` (`minecraft:dust`), if the library asked for it.
pub fn key(id: &str) -> Option<ParticleKey> {
    KEYS.lock().unwrap().get(id).copied()
}
