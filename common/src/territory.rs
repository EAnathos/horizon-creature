//! Territories (SPECS §4.2): a creature released from a jar settles in an area it won't leave.
//!
//! Three rules (`Rule`). The **garden** (Worms): released on farmland (or a crop on it), the farmland connected to it (4
//! ways, the same level or one up or down) is its area, which must count at least `GARDEN_MIN` blocks, or it won't
//! settle; if the garden shrinks below that, it is free again. The **flower patch** (Butterflies, Fireflies): released on
//! a flower, that flower is its anchor; its area is
//! the flowers within `PATCH` blocks of the anchor, plus `AROUND` blocks around them and `ABOVE` blocks up. If the anchor
//! flower is broken, the anchor moves to the nearest flower within `PATCH` blocks, or the creature is free again. The
//! **sand** (Scorpions): released on sand, red sand, sandstone or terracotta, the sand-like blocks connected to it
//! within `SAND_RANGE` blocks are its area, at least `SAND_MIN` of them, or it won't settle; a patch grown too small
//! frees it.
//!
//! The anchor is saved on the entity (`creature:territory`). The mod's goals read it through a static map (`anchor`),
//! filled by `tick` from the mod's `entity_tick_post` (single player, as the colours), and tell a moved or lost anchor
//! back (`moved`), which `tick` saves. Blocks are read through a `Ground`: the host and the level the creature is in.
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, LevelRef, Registrar, ServerHost, Tag,
};

/// The flowers within this many blocks of the anchor make the area; it reaches this far around them and up.
pub const PATCH: i32 = 8;
/// A garden counts at least this many farmland blocks; it is looked for up to `BLOCKS_MAX`.
pub const GARDEN_MIN: usize = 10;
const BLOCKS_MAX: usize = 256;
const FARMLAND: &str = "minecraft:farmland";
/// A sand patch reaches this far from the anchor and counts at least this many blocks.
pub const SAND_RANGE: i32 = 8;
pub const SAND_MIN: usize = 10;

/// Where a released creature settles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// The flowers around the one it was released on.
    FlowerPatch,
    /// The farmland connected to the block it was released on, at least `GARDEN_MIN` blocks.
    Garden,
    /// The sand-like blocks connected to the one it was released on, within `SAND_RANGE`, at least `SAND_MIN`.
    Sand,
}
pub const AROUND: i32 = 2;
pub const ABOVE: i32 = 4;
const FLOWERS: &str = "minecraft:flowers";

static ANCHOR: AttachmentType<Option<[i32; 3]>> = AttachmentType {
    namespace: "creature",
    name: "territory",
    default: || None,
    codec: Some(AttachmentCodec { encode, decode }),
    copy_on_death: false,
    sync: false,
};

/// From the library's `register`: the anchor attachment is declared.
pub fn declare(r: &mut dyn Registrar) {
    let _ = r.register_attachment_type("creature:territory", false);
}

/// The blocks of a level, read through a host: what a goal (its tick's read view) or a hook sees.
#[derive(Clone, Copy)]
pub struct Ground<'a> {
    pub host: &'a dyn Host,
    pub level: LevelRef,
}

impl<'a> Ground<'a> {
    pub fn new(host: &'a dyn Host, level: LevelRef) -> Self {
        Ground { host, level }
    }

    /// The level entity `entity` is in.
    pub fn of_entity(host: &'a dyn Host, entity: u32) -> Option<Self> {
        let level = host.entity(entity)?.level(host)?;
        Some(Ground { host, level })
    }

    /// The block at `p` (`minecraft:farmland`), empty where it can't be read (not loaded).
    pub fn block(&self, p: [i32; 3]) -> String {
        self.level
            .block(self.host, p)
            .map_or(String::new(), |b| b.id(self.host))
    }

    /// Whether the block at `p` is a flower.
    pub fn flower_at(&self, p: [i32; 3]) -> bool {
        self.level
            .block(self.host, p)
            .and_then(|b| self.host.block_in_tag(b, FLOWERS))
            .unwrap_or(false)
    }
}

fn encode(anchor: &Option<[i32; 3]>) -> Result<Compound, AttachmentError> {
    let mut c = Compound::new();
    if let Some(a) = anchor {
        c.insert("Anchor".into(), Tag::IntArray(a.to_vec()));
    }
    Ok(c)
}

fn decode(c: &Compound) -> Result<Option<[i32; 3]>, AttachmentError> {
    Ok(match c.get(&"Anchor".into()) {
        Some(Tag::IntArray(a)) if a.len() == 3 => Some([a[0], a[1], a[2]]),
        _ => None,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct Known {
    anchor: Option<[i32; 3]>,
    /// Changed by a goal, to save.
    dirty: bool,
}

static ANCHORS: Mutex<BTreeMap<u32, Known>> = Mutex::new(BTreeMap::new());

/// From the jar's release (or a mod): `entity` settles with `anchor`.
pub fn settle(host: &mut dyn ServerHost, entity: u32, anchor: [i32; 3]) {
    let _ = ANCHOR.set(host, Holder::Entity(entity), Some(anchor));
    ANCHORS.lock().unwrap().insert(
        entity,
        Known {
            anchor: Some(anchor),
            dirty: false,
        },
    );
}

/// From the mod's `entity_tick_post`: its saved anchor the first time, and a moved or lost one saved.
pub fn tick(host: &mut dyn ServerHost, entity: u32) {
    let known = ANCHORS.lock().unwrap().get(&entity).copied();
    match known {
        None => {
            // Not readable yet (its first tick after loading): next tick.
            let Ok(saved) = ANCHOR.get_existing(host, Holder::Entity(entity)) else {
                return;
            };
            ANCHORS.lock().unwrap().insert(
                entity,
                Known {
                    anchor: saved.flatten(),
                    dirty: false,
                },
            );
        }
        Some(k) if k.dirty => {
            let _ = ANCHOR.set(host, Holder::Entity(entity), k.anchor);
            if let Some(k) = ANCHORS.lock().unwrap().get_mut(&entity) {
                k.dirty = false;
            }
        }
        Some(_) => {}
    }
}

/// From the mod's `living_death`.
pub fn forget(entity: u32) {
    ANCHORS.lock().unwrap().remove(&entity);
}

/// Where creature `entity` is settled (its anchor), if it is.
pub fn anchor(entity: u32) -> Option<[i32; 3]> {
    ANCHORS.lock().unwrap().get(&entity).and_then(|k| k.anchor)
}

/// Whether it is settled (a settled creature never despawns, and doesn't count as wild for the bestiary).
pub fn settled(entity: u32) -> bool {
    anchor(entity).is_some()
}

/// The flowers within `radius` blocks (and 3 up or down) of `center`.
pub fn flowers_near(ground: Ground<'_>, center: [i32; 3], radius: i32) -> Vec<[i32; 3]> {
    let mut found = Vec::new();
    for dy in -3..=3 {
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let p = [center[0] + dx, center[1] + dy, center[2] + dz];
                if ground.flower_at(p) {
                    found.push(p);
                }
            }
        }
    }
    found
}

/// From a goal, every few seconds: the area still holds. An anchor flower that is gone moves to the nearest flower
/// within `PATCH` blocks, or the creature is free again. The flowers of its area.
pub fn check(host: &dyn Host, entity: u32) -> Vec<[i32; 3]> {
    let (Some(anchor), Some(ground)) = (anchor(entity), Ground::of_entity(host, entity)) else {
        return Vec::new();
    };
    let flowers = flowers_near(ground, anchor, PATCH);
    if ground.flower_at(anchor) {
        return flowers;
    }
    let distance = |p: &[i32; 3]| (0..3).map(|k| (p[k] - anchor[k]).pow(2)).sum::<i32>();
    let moved = flowers.iter().min_by_key(|p| distance(p)).copied();
    if let Some(k) = ANCHORS.lock().unwrap().get_mut(&entity) {
        k.anchor = moved;
        k.dirty = true;
    }
    if moved.is_some() { flowers } else { Vec::new() }
}

/// Whether `p` (a block) is inside the area made by `flowers`: within `AROUND` blocks of one of them, up to `ABOVE` up.
pub fn inside(flowers: &[[i32; 3]], p: [i32; 3]) -> bool {
    flowers.iter().any(|f| {
        (p[0] - f[0]).abs() <= AROUND && (p[2] - f[2]).abs() <= AROUND && p[1] >= f[1] - 1 && p[1] <= f[1] + ABOVE
    })
}

/// The blocks connected to `start` (4 ways, the same level or one up or down), within `within` blocks of it when given,
/// at most `BLOCKS_MAX`; `belongs` says whether a block is one.
fn connected(start: [i32; 3], within: Option<i32>, belongs: impl Fn([i32; 3]) -> bool) -> Vec<[i32; 3]> {
    let mut seen = std::collections::BTreeSet::from([start]);
    let mut queue = std::collections::VecDeque::from([start]);
    let mut found = Vec::new();
    while let Some(p) = queue.pop_front() {
        found.push(p);
        if found.len() >= BLOCKS_MAX {
            break;
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for dy in [0, 1, -1] {
                let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                let near = within.is_none_or(|w| (q[0] - start[0]).abs() <= w && (q[2] - start[2]).abs() <= w);
                if near && !seen.contains(&q) && belongs(q) {
                    seen.insert(q);
                    queue.push_back(q);
                    break;
                }
            }
        }
    }
    found
}

/// The farmland connected to `start`; `farmland` says whether a block is.
fn garden_from(start: [i32; 3], farmland: impl Fn([i32; 3]) -> bool) -> Vec<[i32; 3]> {
    connected(start, None, farmland)
}

/// Whether the block named `name` is sand-like (the sand rule's ground): sand, red sand, any sandstone or any
/// terracotta.
pub fn is_sand(name: &str) -> bool {
    matches!(name, "minecraft:sand" | "minecraft:red_sand")
        || name.ends_with("sandstone")
        || name.ends_with("terracotta")
}

/// From the jar's release: the sand-like block a creature released on `clicked` (a sand-like block, or one just below)
/// would settle on, if its patch is big enough.
pub fn sand_at(ground: Ground<'_>, clicked: [i32; 3]) -> Option<[i32; 3]> {
    let below = [clicked[0], clicked[1] - 1, clicked[2]];
    let start = if is_sand(&ground.block(clicked)) {
        clicked
    } else if is_sand(&ground.block(below)) {
        below
    } else {
        return None;
    };
    (connected(start, Some(SAND_RANGE), |p| is_sand(&ground.block(p))).len() >= SAND_MIN).then_some(start)
}

/// From a hook: the sand-like blocks of creature `entity`'s patch (empty if it isn't settled).
pub fn sand_of(host: &dyn Host, entity: u32) -> Vec<[i32; 3]> {
    match (anchor(entity), Ground::of_entity(host, entity)) {
        (Some(a), Some(ground)) => connected(a, Some(SAND_RANGE), |p| is_sand(&ground.block(p))),
        _ => Vec::new(),
    }
}

/// From a goal, every few seconds: the sand-like blocks of its patch; a patch grown too small (or its anchor no longer
/// sand) lets it go free.
pub fn check_sand(host: &dyn Host, entity: u32) -> Vec<[i32; 3]> {
    let (Some(anchor), Some(ground)) = (anchor(entity), Ground::of_entity(host, entity)) else {
        return Vec::new();
    };
    let sand = |p: [i32; 3]| is_sand(&ground.block(p));
    let patch = if sand(anchor) {
        connected(anchor, Some(SAND_RANGE), sand)
    } else {
        Vec::new()
    };
    if patch.len() >= SAND_MIN {
        return patch;
    }
    if let Some(k) = ANCHORS.lock().unwrap().get_mut(&entity) {
        k.anchor = None;
        k.dirty = true;
    }
    Vec::new()
}

/// From the jar's release: the farmland a creature released on `clicked` (farmland, or a crop on farmland) would
/// settle on, if its garden is big enough.
pub fn garden_at(ground: Ground<'_>, clicked: [i32; 3]) -> Option<[i32; 3]> {
    let below = [clicked[0], clicked[1] - 1, clicked[2]];
    let start = if ground.block(clicked) == FARMLAND {
        clicked
    } else if ground.block(below) == FARMLAND {
        below
    } else {
        return None;
    };
    (garden_from(start, |p| ground.block(p) == FARMLAND).len() >= GARDEN_MIN).then_some(start)
}

/// From a hook: the farmland of creature `entity`'s garden (empty if it isn't settled).
pub fn garden_of(host: &dyn Host, entity: u32) -> Vec<[i32; 3]> {
    match (anchor(entity), Ground::of_entity(host, entity)) {
        (Some(a), Some(ground)) => garden_from(a, |p| ground.block(p) == FARMLAND),
        _ => Vec::new(),
    }
}

/// From a goal, every few seconds: the farmland of its garden; a garden grown too small (or its anchor no longer
/// farmland) lets it go free.
pub fn check_garden(host: &dyn Host, entity: u32) -> Vec<[i32; 3]> {
    let (Some(anchor), Some(ground)) = (anchor(entity), Ground::of_entity(host, entity)) else {
        return Vec::new();
    };
    let is_farmland = |p: [i32; 3]| ground.block(p) == FARMLAND;
    let garden = if is_farmland(anchor) {
        garden_from(anchor, is_farmland)
    } else {
        Vec::new()
    };
    if garden.len() >= GARDEN_MIN {
        return garden;
    }
    if let Some(k) = ANCHORS.lock().unwrap().get_mut(&entity) {
        k.anchor = None;
        k.dirty = true;
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_garden_is_the_connected_farmland_one_step_up_or_down() {
        // A 4 by 3 field at y 0, a strip going up one step at x 4, and a lone block apart.
        let field = |p: [i32; 3]| {
            (p[1] == 0 && (0..4).contains(&p[0]) && (0..3).contains(&p[2]))
                || (p[1] == 1 && p[0] == 4 && p[2] == 0)
                || p == [10, 0, 10]
        };
        let garden = garden_from([0, 0, 0], field);
        assert_eq!(garden.len(), 13);
        assert!(garden.contains(&[4, 1, 0]));
        assert!(!garden.contains(&[10, 0, 10]));
    }

    #[test]
    fn sand_like_blocks_are_the_rule_s_ground_glazed_or_not() {
        assert!(is_sand("minecraft:sand"));
        assert!(is_sand("minecraft:red_sand"));
        assert!(is_sand("minecraft:sandstone"));
        assert!(is_sand("minecraft:smooth_red_sandstone"));
        assert!(is_sand("minecraft:terracotta"));
        assert!(is_sand("minecraft:orange_terracotta"));
        assert!(!is_sand("minecraft:grass_block"));
        assert!(!is_sand("minecraft:stone"));
    }

    #[test]
    fn a_sand_patch_stays_within_range_of_its_anchor() {
        // Sand everywhere on y 0: the patch is the blocks within SAND_RANGE of the anchor, not the whole desert.
        let patch = connected([0, 0, 0], Some(SAND_RANGE), |p| p[1] == 0);
        assert!(patch.len() >= SAND_MIN);
        assert!(
            patch
                .iter()
                .all(|p| (p[0]).abs() <= SAND_RANGE && (p[2]).abs() <= SAND_RANGE)
        );
        assert!(!patch.contains(&[SAND_RANGE + 1, 0, 0]));
    }

    #[test]
    fn a_flower_patch_reaches_around_and_above_its_flowers() {
        let flowers = [[0, 64, 0]];
        assert!(inside(&flowers, [AROUND, 64 + ABOVE, -AROUND]));
        assert!(!inside(&flowers, [AROUND + 1, 64, 0]));
        assert!(!inside(&flowers, [0, 64 + ABOVE + 1, 0]));
        assert!(!inside(&flowers, [0, 62, 0]));
    }
}
