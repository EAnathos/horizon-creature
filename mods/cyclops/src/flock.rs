//! Its flock (SPECS §2): the sheep that came with it follow it, and it turns on whoever hurts one of them.
use super::*;

/// Whom each cyclops is to turn on, for hurting one of its sheep (from `living_incoming_damage`, which can't change a
/// target): the attacker's entity id. Its tick takes it as its target (`anger`).
pub(crate) static ANGERED: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

/// From the cyclops's tick: whoever hurt one of its sheep becomes its target.
pub(crate) fn anger(host: &mut dyn ServerHost, cyclops: u32) {
    let Some(attacker) = ANGERED.lock().unwrap().remove(&cyclops) else {
        return;
    };
    let (Some(m), Some(t)) = (
        host.entity(cyclops).and_then(|e| host.mob(e)),
        host.entity(attacker).and_then(|e| host.living(e)),
    ) else {
        return;
    };
    m.set_target(host, Some(t));
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

pub(crate) fn flock(host: &mut dyn ServerHost, level: LevelRef) {
    {
        let mut t = FLOCK_TICKS.lock().unwrap();
        *t += 1;
        if *t < FLOCK_EVERY {
            return;
        }
        *t = 0;
    }
    let entities = entities_near_cyclopes(&*host, level, FLOCK_RANGE);
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
                .and_then(|e| e.tags(&*host))
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
        if distance > FOLLOW_FROM
            && let Some(&(_, c)) = cyclopes.iter().find(|(id, _)| *id == shepherd)
        {
            follow(host, *sheep, c, 1.0);
        }
    }
    let players: Vec<_> = host
        .players()
        .server_players()
        .into_iter()
        .filter(|p| p.level(&*host) == Some(level))
        .collect();
    for player in players {
        let Some(at) = player.entity().position(host) else {
            continue;
        };
        let sees = cyclopes.iter().any(|(id, c)| {
            with_flock.contains(id) && (0..3).map(|k| (c[k] - at[k]).powi(2)).sum::<f64>() < SEE_FLOCK * SEE_FLOCK
        });
        if sees && NOTED.lock().unwrap().insert(player.uuid()) {
            creature::bestiary::learn(host, player.uuid(), CYCLOPS, FLOCK_FACT);
        }
    }
}

/// The entities within `range` blocks of a cyclops (as their tick knows them, `CYCLOPS_AT`): (id, kind, position).
pub(crate) fn entities_near_cyclopes(host: &dyn Host, level: LevelRef, range: f64) -> Vec<(u32, String, [f64; 3])> {
    let mut found = BTreeMap::new();
    let cyclopes: Vec<[f64; 3]> = CYCLOPS_AT.lock().unwrap().values().copied().collect();
    for c in cyclopes {
        let (from, to) = (
            [c[0] - range, c[1] - 16.0, c[2] - range],
            [c[0] + range, c[1] + 16.0, c[2] + range],
        );
        for e in level.entities_in(host, from, to).unwrap_or_default() {
            if let (Some(kind), Some(at)) = (e.kind(host), e.position(host)) {
                found.insert(e.id(), (kind.id(host), at));
            }
        }
    }
    found.into_iter().map(|(id, (kind, at))| (id, kind, at)).collect()
}

/// Mob `id` walks to `to` (pathfinding), at `speed` times its own.
pub(crate) fn follow(host: &mut dyn ServerHost, id: u32, to: [f64; 3], speed: f64) {
    if let Some(m) = host.entity(id).and_then(|e| host.mob(e)) {
        m.move_to(host, to, speed);
    }
}
