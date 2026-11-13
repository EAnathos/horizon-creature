//! What its blows break (SPECS §2): the club and the boulder tear up the blocks around where they land, without
//! drops. Left alone: blocks that Horizon's players can't break either (bedrock, obsidian, portals, command blocks...),
//! liquids, containers (their contents would be lost with no drops) and the library's placed jars (their creature).
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use horizon_client::mod_api::{Host, ParticleOptions, ParticleType};

/// How far from the club's blow it breaks (blocks, a ball) and from the boulder's burst.
pub const CLUB_RADIUS: i32 = 1;
pub const BOULDER_RADIUS: i32 = 2;

/// What is never broken: any block whose id holds one of these...
const KEPT: &[&str] = &[
    "bedrock",
    "obsidian",
    "command_block",
    "structure_",
    "jigsaw",
    "portal",
    "end_gateway",
    "reinforced_deepslate",
    "spawner",
    "vault",
    // Containers.
    "chest",
    "barrel",
    "shulker_box",
    "furnace",
    "smoker",
    "hopper",
    "dispenser",
    "dropper",
    "brewing_stand",
    "crafter",
    "decorated_pot",
    "chiseled_bookshelf",
    "jukebox",
    "lectern",
];
/// ...and these exactly.
const KEPT_EXACTLY: &[&str] = &[
    "minecraft:air",
    "minecraft:cave_air",
    "minecraft:void_air",
    "minecraft:barrier",
    "minecraft:light",
    "minecraft:water",
    "minecraft:lava",
    "minecraft:bubble_column",
];

/// Club blows to make, from the goals' ticks (which have no host): where each cyclops's club came down.
static BLOWS: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());

/// From the club goal: `cyclops`'s club came down at `at`.
pub fn blow(cyclops: u32, at: [f64; 3]) {
    BLOWS.lock().unwrap().insert(cyclops, at);
}

/// From the cyclops's tick: its club's last blow breaks its blocks.
pub fn tick(host: &mut dyn Host, dimension: &str, cyclops: u32) {
    let at = BLOWS.lock().unwrap().remove(&cyclops);
    if let Some(at) = at {
        tear(host, dimension, at, CLUB_RADIUS);
    }
}

/// From its death: a blow it had no tick left to make is dropped.
pub fn forget(cyclops: u32) {
    BLOWS.lock().unwrap().remove(&cyclops);
}

/// Whether `name` is a block a blow breaks.
fn breakable(name: &str) -> bool {
    !KEPT_EXACTLY.contains(&name) && !KEPT.iter().any(|kept| name.contains(kept))
}

/// Breaks the blocks within `radius` of `at`, with a puff of dust where each one was.
pub fn tear(host: &mut dyn Host, dimension: &str, at: [f64; 3], radius: i32) {
    let centre = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    let mut blocks = Vec::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            for dz in -radius..=radius {
                if dx * dx + dy * dy + dz * dz <= radius * radius + 1 {
                    blocks.push([centre[0] + dx, centre[1] + dy, centre[2] + dz]);
                }
            }
        }
    }
    creature::jar::spare_from_blast(host, &mut blocks);
    let blocks: Vec<([i32; 3], String)> = blocks
        .into_iter()
        .map(|pos| (pos, host.block(pos)))
        .filter(|(_, name)| breakable(name))
        .collect();
    if blocks.is_empty() {
        return;
    }
    // One pass over the block states for every name needed (the dust's look) and air.
    let wanted: BTreeSet<&str> = blocks.iter().map(|(_, name)| name.as_str()).collect();
    let mut states = BTreeMap::new();
    let mut air = None;
    for (id, state) in host.block_states().iter() {
        if &*state.name == "minecraft:air" {
            air.get_or_insert(id);
        } else if wanted.contains(&*state.name) {
            states.entry(state.name.to_string()).or_insert(id);
        }
    }
    let Some(air) = air else {
        return;
    };
    let dust = ParticleOptions::simple(ParticleType::Block);
    for (pos, name) in blocks {
        if host.set_block(dimension, pos, air, 3) {
            let middle = [
                f64::from(pos[0]) + 0.5,
                f64::from(pos[1]) + 0.5,
                f64::from(pos[2]) + 0.5,
            ];
            host.send_particles(&dust, states.get(&name).copied(), middle, [0.3, 0.3, 0.3], 0.05, 6);
        }
    }
}
