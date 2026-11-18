//! Where it appears (SPECS §2): rarely, at a cave mouth near a player, with its flock.
use super::*;

/// Where it appears (SPECS §2): rarely, at a cave mouth near a player. Every `SPAWN_EVERY` ticks, one chance in
/// `SPAWN_CHANCE` for each player with no cyclops within `ALONE` blocks, `SPAWN_TRIES` columns `SPAWN_MIN` to
/// `SPAWN_MAX` blocks away are tried (`cave_mouth`); it comes with `FLOCK_SIZE` sheep.
pub(crate) const SPAWN_EVERY: u32 = 1200;
pub(crate) const SPAWN_CHANCE: u64 = 4;
pub(crate) const ALONE: f64 = 160.0;
pub(crate) const SPAWN_TRIES: usize = 24;
pub(crate) const SPAWN_MIN: f64 = 24.0;
pub(crate) const SPAWN_MAX: f64 = 64.0;
pub(crate) const FLOCK_SIZE: (u64, u64) = (3, 5);
pub(crate) static SPAWN_TICKS: Mutex<u32> = Mutex::new(0);

pub(crate) fn spawn(host: &mut dyn ServerHost, level: LevelRef) {
    if peaceful(&*host, level) {
        return;
    }
    let dimension = level.dimension().id(&*host).unwrap_or_default();
    let dimension = dimension.as_str();
    {
        let mut t = SPAWN_TICKS.lock().unwrap();
        *t += 1;
        if *t < SPAWN_EVERY {
            return;
        }
        *t = 0;
    }
    let mut random = creature::colours::Random::new(SPAWN_EVERY);
    let players: Vec<_> = host
        .players()
        .server_players()
        .into_iter()
        .filter(|p| p.level(&*host) == Some(level))
        .collect();
    for player in players {
        if !random.next().is_multiple_of(SPAWN_CHANCE) {
            continue;
        }
        let Some(p) = player.entity().position(host) else {
            continue;
        };
        let crowded = CYCLOPS_AT
            .lock()
            .unwrap()
            .values()
            .any(|c| (c[0] - p[0]).hypot(c[2] - p[2]) < ALONE);
        if crowded {
            continue;
        }
        for _ in 0..SPAWN_TRIES {
            let angle = ((random.next() % 3600) as f64 / 10.0).to_radians();
            let distance = SPAWN_MIN + (random.next() % 1000) as f64 / 1000.0 * (SPAWN_MAX - SPAWN_MIN);
            let x = (p[0] + angle.cos() * distance).floor() as i32;
            let z = (p[2] + angle.sin() * distance).floor() as i32;
            let Some(at) = cave_mouth(&*host, level, x, z) else {
                continue;
            };
            if !level.biome_at(host, at).is_some_and(|b| BIOMES.contains(&b.as_str())) {
                continue;
            }
            let feet = [f64::from(at[0]) + 0.5, f64::from(at[1]), f64::from(at[2]) + 0.5];
            if let Some(id) = host.summon(dimension, CYCLOPS, feet, Compound::new()) {
                CYCLOPS_AT.lock().unwrap().insert(id, feet);
                let count = FLOCK_SIZE.0 + random.next() % (FLOCK_SIZE.1 - FLOCK_SIZE.0 + 1);
                for _ in 0..count {
                    sheep(host, level, at, &mut random);
                }
            }
            break;
        }
    }
}

pub(crate) const AIR: [&str; 2] = ["minecraft:air", "minecraft:cave_air"];

/// Room to stand in: air, or what a block placed there would replace (grass, ferns, snow layers), but not liquids.
pub(crate) fn is_open(ground: Ground<'_>, pos: [i32; 3]) -> bool {
    let block = ground.block(pos);
    let replaceable = || {
        ground
            .level
            .block(ground.host, pos)
            .and_then(|b| ground.host.block_in_tag(b, "minecraft:replaceable"))
            .unwrap_or(false)
    };
    AIR.contains(&block.as_str()) || (!block.contains("water") && !block.contains("lava") && replaceable())
}

/// Ground a cyclops or a sheep may stand on: not open, not a liquid or leaves.
pub(crate) fn is_ground(ground: Ground<'_>, pos: [i32; 3]) -> bool {
    let block = ground.block(pos);
    !(is_open(ground, pos) || block.contains("water") || block.contains("lava") || block.contains("leaves"))
}

/// A cave mouth at column (`x`, `z`): the open ground there (sky above, 4 blocks of room for the cyclops) at the foot of
/// a covered passage going in, in one of the 4 directions: within 2 blocks of it, at least 4 cells in a row with air
/// (or grass) for 3 blocks, ground under them and the land at least 3 blocks over them. Where the cyclops stands, or none.
pub(crate) fn cave_mouth(host: &dyn ServerHost, level: LevelRef, x: i32, z: i32) -> Option<[i32; 3]> {
    let dimension = level.dimension().id(host)?;
    let ground = Ground::new(host, level);
    const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    const DEPTH: i32 = 8;
    let mut columns = vec![(x, z)];
    for (dx, dz) in DIRS {
        columns.extend((1..=DEPTH).map(|d| (x + dx * d, z + dz * d)));
    }
    let heights = host
        .surface_heights(&dimension, &columns, Heightmap::MotionBlockingNoLeaves)
        .ok()?;
    let y = heights[0]?;
    if !is_ground(ground, [x, y - 1, z]) || !(0..4).all(|k| is_open(ground, [x, y + k, z])) {
        return None;
    }
    for (i, (dx, dz)) in DIRS.into_iter().enumerate() {
        let mut run = 0;
        for d in 1..=DEPTH {
            let (cx, cz) = (x + dx * d, z + dz * d);
            let covered = heights[1 + i * DEPTH as usize + (d - 1) as usize].is_some_and(|h| h > y + 3);
            let open = (0..3).all(|k| is_open(ground, [cx, y + k, cz])) && is_ground(ground, [cx, y - 1, cz]);
            if covered && open {
                run += 1;
                if run >= 4 {
                    return Some([x, y, z]);
                }
            } else if run > 0 || d > 2 {
                break;
            }
        }
    }
    None
}

/// A sheep of the flock, on open ground 2 to 5 blocks from `at`, at about its height, tagged as the flock's.
pub(crate) fn sheep(host: &mut dyn ServerHost, level: LevelRef, at: [i32; 3], random: &mut creature::colours::Random) {
    let Some(dimension) = level.dimension().id(&*host) else {
        return;
    };
    for _ in 0..6 {
        let angle = ((random.next() % 360) as f64).to_radians();
        let distance = 2.0 + (random.next() % 300) as f64 / 100.0;
        let x = at[0] + (angle.cos() * distance).round() as i32;
        let z = at[2] + (angle.sin() * distance).round() as i32;
        let Ok(heights) = host.surface_heights(&dimension, &[(x, z)], Heightmap::MotionBlockingNoLeaves) else {
            return;
        };
        let Some(y) = heights[0] else {
            continue;
        };
        if (y - at[1]).abs() > 2 || !is_ground(Ground::new(&*host, level), [x, y - 1, z]) {
            continue;
        }
        let feet = [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5];
        if let Some(id) = host.summon(&dimension, "minecraft:sheep", feet, Compound::new()) {
            if let Some(e) = host.entity(id) {
                e.add_tag(host, FLOCK_TAG);
            }
            FLOCK.lock().unwrap().insert(id, true);
        }
        return;
    }
}
