//! Hunters eat small creatures (SPECS §3.7): a mod registers its kind as the prey of hunters, vanilla mobs or another
//! mod's (by entity id: a hunter whose mod isn't installed simply never shows up, so neither mod requires the other).
//! From the prey's tick (`tick`), the nearest hunter within its range is sent to it (`MobRef::move_to`, as a goal
//! would), and eats it once it reaches it: the creature is gone, with no drop, and the players near learn the mod's
//! fact. A creature in a jar or a bucket isn't in the world, so it is safe.
//!
//! Single player: the hooks share the mob's id (as the rest of the series).
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{ParticleOptions, ServerHost, SoundSource};

/// A hunter of a prey: its entity type, how far it sees the prey (blocks) and its speed going for it (its navigation's
/// multiplier).
pub struct Hunter {
    pub kind: &'static str,
    pub range: f64,
    pub speed: f64,
}

/// A prey kind, its hunters, and the fact learnt by the players who see one eaten.
pub struct Prey {
    pub kind: &'static str,
    pub hunters: &'static [Hunter],
    pub fact: Option<&'static str>,
}

/// What a mod does to the hunter of its prey after a catch (`when_caught`), given the hunter's id.
pub type Caught = fn(&mut dyn ServerHost, u32);

static PREY: Mutex<Vec<&'static Prey>> = Mutex::new(Vec::new());
static CAUGHT: Mutex<Vec<(&'static str, Caught)>> = Mutex::new(Vec::new());
/// The hunter going for each prey.
static CHASED: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());
/// How often a prey looks out for a new hunter (ticks), and how near a hunter eats it (blocks).
const LOOK_EVERY: i32 = 20;
const REACH: f64 = 1.3;

/// From a mod's `register`.
pub fn register(prey: &'static Prey) {
    let mut all = PREY.lock().unwrap();
    if !all.iter().any(|p| p.kind == prey.kind) {
        all.push(prey);
    }
}

/// From a mod's `register`: after a catch of `kind`, `caught` runs on its hunter (the flying fish's dolphin is fed).
pub fn when_caught(kind: &'static str, caught: Caught) {
    let mut all = CAUGHT.lock().unwrap();
    if !all.iter().any(|(k, _)| *k == kind) {
        all.push((kind, caught));
    }
}

fn of(kind: &str) -> Option<&'static Prey> {
    PREY.lock().unwrap().iter().copied().find(|p| p.kind == kind)
}

/// From the mod's `entity_tick_post`, for each creature of a prey kind: its hunter goes for it, and eats it once there.
/// Whether it was eaten (it is gone: the mod forgets it).
pub fn tick(host: &mut dyn ServerHost, dimension: &str, entity: u32, kind: &str) -> bool {
    let Some(prey) = of(kind) else {
        return false;
    };
    let Some(e) = host.entity(entity) else {
        return false;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return false;
    };
    let kept = CHASED.lock().unwrap().get(&entity).copied().and_then(|h| {
        let hunter = host.entity(h)?;
        let kind = hunter.kind(&*host)?.id(&*host);
        let range = prey.hunters.iter().find(|x| x.kind == kind)?.range;
        let there = hunter.position(&*host)?;
        (crate::body::distance(there, at) <= range * 1.5).then_some((h, there))
    });
    let chase = kept.or_else(|| {
        if e.tick_count(&*host).unwrap_or(0) % LOOK_EVERY != 0 {
            return None;
        }
        let r = prey.hunters.iter().map(|h| h.range).fold(0.0, f64::max);
        let from = [at[0] - r, at[1] - r, at[2] - r];
        let to = [at[0] + r, at[1] + r, at[2] + r];
        level
            .entities_in(&*host, from, to)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|h| {
                let kind = h.kind(&*host)?.id(&*host);
                let range = prey.hunters.iter().find(|x| x.kind == kind)?.range;
                let there = h.position(&*host)?;
                let d = crate::body::distance(there, at);
                (d <= range).then_some((h.id(), there, d))
            })
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(h, there, _)| (h, there))
    });
    let Some((hunter, there)) = chase else {
        CHASED.lock().unwrap().remove(&entity);
        return false;
    };
    CHASED.lock().unwrap().insert(entity, hunter);
    if crate::body::distance(there, at) <= REACH {
        eaten(host, dimension, prey, entity, at);
        let caught = CAUGHT
            .lock()
            .unwrap()
            .iter()
            .find(|(k, _)| *k == prey.kind)
            .map(|(_, f)| *f);
        if let Some(caught) = caught {
            caught(host, hunter);
        }
        return true;
    }
    let speed = host
        .entity(hunter)
        .and_then(|h| h.kind(&*host))
        .map(|k| k.id(&*host))
        .and_then(|k| prey.hunters.iter().find(|x| x.kind == k))
        .map_or(1.0, |h| h.speed);
    if let Some(m) = host.entity(hunter).and_then(|h| host.mob(h)) {
        m.move_to(host, at, speed);
    }
    false
}

/// Prey `entity` at `at` eaten: a crunch and crumbs, the players near learn the fact, it is gone with no drop.
fn eaten(host: &mut dyn ServerHost, dimension: &str, prey: &Prey, entity: u32, at: [f64; 3]) {
    if let Some(level) = host.level(dimension) {
        level.play_sound(
            host,
            "minecraft:entity.generic.eat",
            at,
            SoundSource::Neutral,
            0.8,
            1.0,
            None,
        );
        if let Some(k) = crate::particles::key("minecraft:poof") {
            level.send_particles(host, &ParticleOptions::simple(k), None, at, [0.2, 0.2, 0.2], 0.02, 6);
        }
    }
    if let Some(fact) = prey.fact {
        for player in crate::bestiary::players_near(&*host, dimension, at) {
            crate::bestiary::learn(host, player, prey.kind, fact);
        }
    }
    if let Some(e) = host.entity(entity) {
        e.discard(host);
    }
    forget(entity);
}

/// From the library when a creature is gone.
pub fn forget(entity: u32) {
    CHASED.lock().unwrap().retain(|p, h| *p != entity && *h != entity);
}
