//! The shepherd's crook (SPECS §3): crafted with a cyclops eye. While a player holds it, in either hand, the farm
//! animals within `REACH` blocks follow them as a cyclops's flock follows it (its sheep too, taken from it). Every
//! `EVERY` ticks, one more than `CLOSE` blocks away walks to the nearest such player; one newly herded shows a sparkle.
use std::collections::BTreeSet;
use std::sync::Mutex;

use mod_api::{Hand, LevelRef, ParticleOptions, ServerHost};

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
pub fn tick(host: &mut dyn ServerHost, level: LevelRef) {
    {
        let mut t = TICKS.lock().unwrap();
        *t += 1;
        if *t < EVERY {
            return;
        }
        *t = 0;
    }
    let players = host.players().server_players();
    let shepherds: Vec<([f64; 3], u32)> = players
        .into_iter()
        .filter(|p| p.level(&*host) == Some(level))
        .filter(|p| {
            [Hand::Main, Hand::Off]
                .into_iter()
                .any(|h| creature::hand::held(&*host, *p, h).is_some_and(|s| creature::hand::is(&*host, &s, CROOK)))
        })
        .filter_map(|p| Some((p.entity().position(&*host)?, p.entity().id())))
        .collect();
    if shepherds.is_empty() {
        HERDED.lock().unwrap().clear();
        return;
    }
    let mut herded = BTreeSet::new();
    let was = HERDED.lock().unwrap().clone();
    let mut animals = Vec::new();
    for (p, _) in &shepherds {
        let from = [p[0] - REACH, p[1] - REACH, p[2] - REACH];
        let to = [p[0] + REACH, p[1] + REACH, p[2] + REACH];
        for e in level.entities_in(&*host, from, to).unwrap_or_default() {
            let kind = e.kind(&*host).map(|k| k.id(&*host)).unwrap_or_default();
            if let (true, Some(at)) = (ANIMALS.contains(&kind.as_str()), e.position(&*host)) {
                animals.push((e.id(), at));
            }
        }
    }
    animals.sort_by_key(|a| a.0);
    animals.dedup_by_key(|a| a.0);
    for (animal, at) in animals {
        let Some((player_at, distance)) = shepherds
            .iter()
            .map(|&(p, _)| (p, (0..3).map(|k| (p[k] - at[k]).powi(2)).sum::<f64>().sqrt()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            continue;
        };
        if distance > REACH {
            continue;
        }
        herded.insert(animal);
        if !was.contains(&animal)
            && let Some(k) = creature::particles::key("minecraft:happy_villager")
        {
            let sparkle = ParticleOptions::simple(k);
            level.send_particles(
                host,
                &sparkle,
                None,
                [at[0], at[1] + 1.2, at[2]],
                [0.3, 0.3, 0.3],
                0.0,
                5,
            );
        }
        if distance > CLOSE {
            crate::flock::follow(host, animal, player_at, SPEED);
        }
    }
    *HERDED.lock().unwrap() = herded;
}
