//! Territories (SPECS §4.2): a creature released from a jar settles in an area it won't leave.
//!
//! Two rules (`Rule`). The **garden** (Worms): released on farmland (or a crop on it), the farmland connected to it (4
//! ways, the same level or one up or down) is its area, which must count at least `GARDEN_MIN` blocks, or it won't
//! settle; if the garden shrinks below that, it is free again. The **flower patch** (Butterflies, Fireflies): released on
//! a flower, that flower is its anchor; its area is
//! the flowers within `PATCH` blocks of the anchor, plus `AROUND` blocks around them and `ABOVE` blocks up. If the anchor
//! flower is broken, the anchor moves to the nearest flower within `PATCH` blocks, or the creature is free again.
//!
//! The anchor is saved on the entity (`creature:territory`). The mod's goals read it through a static map (`anchor`),
//! filled by `tick` from the mod's `entity_tick_post` (single player, as the colours; arismoko/horizon#2740), and tell a
//! moved or lost anchor back (`moved`), which `tick` saves.
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Cx, Holder, Host, Tag};

/// The flowers within this many blocks of the anchor make the area; it reaches this far around them and up.
pub const PATCH: i32 = 8;
/// A garden counts at least this many farmland blocks; it is looked for up to `GARDEN_MAX`.
pub const GARDEN_MIN: usize = 10;
const GARDEN_MAX: usize = 256;
const FARMLAND: &str = "minecraft:farmland";

/// Where a released creature settles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// The flowers around the one it was released on.
    FlowerPatch,
    /// The farmland connected to the block it was released on, at least `GARDEN_MIN` blocks.
    Garden,
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

/// Whether the block named `name` is a flower (from a hook).
pub fn is_flower(host: &dyn Host, name: &str) -> bool {
    host.block_name_in_tag(name, FLOWERS).unwrap_or(false)
}

/// From the jar's release (or a mod): `entity` settles with `anchor`.
pub fn settle(host: &mut dyn Host, entity: u32, anchor: [i32; 3]) {
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
pub fn tick(host: &mut dyn Host, entity: u32) {
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

fn block_name<'a>(cx: &'a Cx, p: [i32; 3]) -> &'a str {
    cx.w.registry
        .state(cx.w.world.block(p[0], p[1], p[2]))
        .map_or("", |s| s.name.as_str())
}

/// From a goal: whether the block at `p` is a flower.
pub fn flower_at(cx: &Cx, p: [i32; 3]) -> bool {
    cx.w.has_tag(FLOWERS, block_name(cx, p))
}

/// From a goal: the flowers within `radius` blocks (and 3 up or down) of `center`.
pub fn flowers_near(cx: &Cx, center: [i32; 3], radius: i32) -> Vec<[i32; 3]> {
    let mut found = Vec::new();
    for dy in -3..=3 {
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                let p = [center[0] + dx, center[1] + dy, center[2] + dz];
                if flower_at(cx, p) {
                    found.push(p);
                }
            }
        }
    }
    found
}

/// From a goal, every few seconds: the area still holds. An anchor flower that is gone moves to the nearest flower
/// within `PATCH` blocks, or the creature is free again. The flowers of its area.
pub fn check(cx: &Cx, entity: u32) -> Vec<[i32; 3]> {
    let Some(anchor) = anchor(entity) else {
        return Vec::new();
    };
    let flowers = flowers_near(cx, anchor, PATCH);
    if flower_at(cx, anchor) {
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

/// The farmland connected to `start` (4 ways, the same level or one up or down), at most `GARDEN_MAX` blocks;
/// `farmland` says whether a block is.
fn garden_from(start: [i32; 3], farmland: impl Fn([i32; 3]) -> bool) -> Vec<[i32; 3]> {
    let mut seen = std::collections::BTreeSet::from([start]);
    let mut queue = std::collections::VecDeque::from([start]);
    let mut found = Vec::new();
    while let Some(p) = queue.pop_front() {
        found.push(p);
        if found.len() >= GARDEN_MAX {
            break;
        }
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for dy in [0, 1, -1] {
                let q = [p[0] + dx, p[1] + dy, p[2] + dz];
                if !seen.contains(&q) && farmland(q) {
                    seen.insert(q);
                    queue.push_back(q);
                    break;
                }
            }
        }
    }
    found
}

/// From the jar's release: the farmland a creature released on `clicked` (farmland, or a crop on farmland) would
/// settle on, if its garden is big enough.
pub fn garden_at(host: &dyn Host, clicked: [i32; 3]) -> Option<[i32; 3]> {
    let below = [clicked[0], clicked[1] - 1, clicked[2]];
    let start = if host.block(clicked) == FARMLAND {
        clicked
    } else if host.block(below) == FARMLAND {
        below
    } else {
        return None;
    };
    (garden_from(start, |p| host.block(p) == FARMLAND).len() >= GARDEN_MIN).then_some(start)
}

/// From a hook: the farmland of creature `entity`'s garden (empty if it isn't settled).
pub fn garden_of(host: &dyn Host, entity: u32) -> Vec<[i32; 3]> {
    match anchor(entity) {
        Some(a) => garden_from(a, |p| host.block(p) == FARMLAND),
        None => Vec::new(),
    }
}

/// From a goal, every few seconds: the farmland of its garden; a garden grown too small (or its anchor no longer
/// farmland) lets it go free.
pub fn check_garden(cx: &Cx, entity: u32) -> Vec<[i32; 3]> {
    let Some(anchor) = anchor(entity) else {
        return Vec::new();
    };
    let is_farmland = |p: [i32; 3]| block_name(cx, p) == FARMLAND;
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
