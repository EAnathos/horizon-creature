//! Its flock (SPECS §2): the sheep that came with it follow it, and it turns on whoever hurts one of them.
use super::*;

/// Whom each cyclops is to turn on, for hurting one of its sheep (from `living_incoming_damage`, which has no host): a
/// mob, or none for the player (Horizon gives a player's hit no attacker id yet).
pub(crate) static ANGERED: Mutex<BTreeMap<u32, Option<u32>>> = Mutex::new(BTreeMap::new());

/// Takes as its target whoever hurt one of its sheep.
#[derive(Clone, Debug)]
pub(crate) struct Shepherd;

impl Goal for Shepherd {
    fn flags(&self) -> u8 {
        goals::TARGET
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        if let Some(attacker) = ANGERED.lock().unwrap().remove(&m.id) {
            m.ai.target = Some(match attacker {
                Some(id) if cx.w.seen(id).is_some() => Who::Mob(id),
                _ => Who::Player,
            });
        }
        false
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Where each cyclops is (from its tick): for the flock and for keeping cyclopes apart.
pub(crate) static CYCLOPS_AT: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());
/// Each sheep seen near a cyclops: whether it is of a flock (its tag, read once).
pub(crate) static FLOCK: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());
/// Each flock sheep's cyclops (the nearest).
pub(crate) static SHEPHERD: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());
/// The players who have noted the flock fact this session (`bestiary::learn` keeps it).
pub(crate) static NOTED: Mutex<BTreeSet<u128>> = Mutex::new(BTreeSet::new());

/// The flock follows: every `FLOCK_EVERY` ticks, a flock sheep within `FLOCK_RANGE` of a cyclops and more than
/// `FOLLOW_FROM` blocks away walks to it. A player within `SEE_FLOCK` of a cyclops with its flock notes it.
pub(crate) const FLOCK_EVERY: u32 = 20;
pub(crate) const FLOCK_RANGE: f64 = 48.0;
pub(crate) const FOLLOW_FROM: f64 = 6.0;
pub(crate) const SEE_FLOCK: f64 = 16.0;
pub(crate) static FLOCK_TICKS: Mutex<u32> = Mutex::new(0);

pub(crate) fn flock(host: &mut dyn Host, dimension: &str) {
    {
        let mut t = FLOCK_TICKS.lock().unwrap();
        *t += 1;
        if *t < FLOCK_EVERY {
            return;
        }
        *t = 0;
    }
    let entities = host.level_entities(dimension);
    let cyclopes: Vec<(u32, [f64; 3])> = entities
        .iter()
        .filter(|(_, kind, _)| kind == CYCLOPS)
        .map(|&(id, _, at)| (id, at))
        .collect();
    if cyclopes.is_empty() {
        return;
    }
    let mut with_flock = BTreeSet::new();
    for (sheep, _, at) in entities.iter().filter(|(_, kind, _)| kind == "minecraft:sheep") {
        let Some((shepherd, distance)) = cyclopes
            .iter()
            .map(|&(id, c)| (id, (c[0] - at[0]).hypot(c[2] - at[2])))
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            continue;
        };
        if distance > FLOCK_RANGE {
            continue;
        }
        let known = FLOCK.lock().unwrap().get(sheep).copied();
        let of_flock = known.unwrap_or_else(|| {
            let tagged = host
                .entity(*sheep)
                .and_then(|e| host.entity_tags(e))
                .is_some_and(|tags| tags.iter().any(|t| t == FLOCK_TAG));
            FLOCK.lock().unwrap().insert(*sheep, tagged);
            tagged
        });
        // Not while a player's crook herds it.
        if !of_flock || crook::HERDED.lock().unwrap().contains(sheep) {
            continue;
        }
        SHEPHERD.lock().unwrap().insert(*sheep, shepherd);
        with_flock.insert(shepherd);
        if distance > FOLLOW_FROM {
            host.mob_follow(dimension, *sheep, shepherd, 1.0);
        }
    }
    for player in host.server_players().into_iter().filter(|p| p.dimension == dimension) {
        let sees = cyclopes.iter().any(|(id, c)| {
            with_flock.contains(id)
                && (0..3).map(|k| (c[k] - player.position[k]).powi(2)).sum::<f64>() < SEE_FLOCK * SEE_FLOCK
        });
        if sees && NOTED.lock().unwrap().insert(player.uuid) {
            creature::bestiary::learn(host, player.uuid, CYCLOPS, FLOCK_FACT);
        }
    }
}
