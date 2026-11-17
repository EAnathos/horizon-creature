//! The shepherd's crook (SPECS §3): crafted with a cyclops eye. While a player holds it, in either hand, the farm
//! animals within `REACH` blocks follow them as a cyclops's flock follows it (its sheep too, taken from it). Every
//! `EVERY` ticks, one more than `CLOSE` blocks away walks to the nearest such player; one newly herded shows a sparkle.
use std::collections::BTreeSet;
use std::sync::Mutex;

use horizon_client::mod_api::{EquipmentSlot, Host, ParticleOptions, ParticleType};

pub const CROOK: &str = "cyclops:shepherds_crook";

/// The animals it herds.
const ANIMALS: [&str; 6] = [
    "minecraft:sheep",
    "minecraft:cow",
    "minecraft:mooshroom",
    "minecraft:pig",
    "minecraft:goat",
    "minecraft:chicken",
];
const REACH: f64 = 12.0;
const CLOSE: f64 = 3.0;
const EVERY: u32 = 10;
const SPEED: f64 = 1.1;

static TICKS: Mutex<u32> = Mutex::new(0);
/// The animals following a player's crook now: a cyclops's flock (`flock`) leaves them be.
pub static HERDED: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());

/// From `level_tick_post`.
pub fn tick(host: &mut dyn Host, dimension: &str) {
    {
        let mut t = TICKS.lock().unwrap();
        *t += 1;
        if *t < EVERY {
            return;
        }
        *t = 0;
    }
    let holds = |host: &dyn Host, uuid: u128| {
        [EquipmentSlot::MainHand, EquipmentSlot::OffHand]
            .into_iter()
            .any(|slot| host.player_equipment(uuid, slot).is_some_and(|s| &*s.item == CROOK))
    };
    let shepherds: Vec<([f64; 3], u32)> = host
        .server_players()
        .into_iter()
        .filter(|p| p.dimension == dimension && holds(host, p.uuid))
        .filter_map(|p| u32::try_from(p.entity_id).ok().map(|id| (p.position, id)))
        .collect();
    if shepherds.is_empty() {
        HERDED.lock().unwrap().clear();
        return;
    }
    let mut herded = BTreeSet::new();
    let was = HERDED.lock().unwrap().clone();
    for (animal, kind, at) in host.level_entities(dimension) {
        if !ANIMALS.contains(&kind.as_str()) {
            continue;
        }
        let Some((player, distance)) = shepherds
            .iter()
            .map(|&(p, id)| (id, (0..3).map(|k| (p[k] - at[k]).powi(2)).sum::<f64>().sqrt()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            continue;
        };
        if distance > REACH {
            continue;
        }
        herded.insert(animal);
        if !was.contains(&animal) {
            let sparkle = ParticleOptions::simple(ParticleType::HappyVillager);
            host.send_particles(&sparkle, None, [at[0], at[1] + 1.2, at[2]], [0.3, 0.3, 0.3], 0.0, 5);
        }
        if distance > CLOSE {
            host.mob_follow(dimension, animal, player, SPEED);
        }
    }
    *HERDED.lock().unwrap() = herded;
}
