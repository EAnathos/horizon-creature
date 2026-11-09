//! A vulture's own state (SPECS §5, §6): its owner, whether it sits, its harness and chests.
//!
//! It lives on the mob, in `Mob.extra` (`tame`, `owner`, `sitting`, `saddled`, `variant2` = chests), where the goals,
//! the right-click (`interact.rs` injection) and the drawing read it. A wandering trader saves none of those, so it is
//! also kept in a server attachment (`vulture:state`, saved with the entity), and the two meet here:
//!
//! - every tick, the `NoFall` goal hands the mob's state over (`from_mob`), and applies a state loaded from the save
//!   once (`to_mob`);
//! - `entity_tick_post`, with the host, loads the attachment the first time it sees a vulture and writes it back when
//!   the state changed (`save`).
//!
//! The same bridge carries food (SPECS §4, §5): the hook, which can see the items on the ground, finds the nearest
//! raw meat for a hungry vulture (`food`); the `Eat` goal walks to it and eats it (`eaten`); the hook then takes one
//! from the stack.
//!
//! Single player: the goals and the hooks share the mob's id (multiplayer is v2).
use std::collections::BTreeMap;
use std::sync::Mutex;

use horizon_client::mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, Mob, Tag};

/// The raw meats it eats from the ground (SPECS §4); a thrown raw chicken may tame it (§5). Cooked meat is ignored.
pub const MEATS: [&str; 5] = [
    TAMING_FOOD,
    "minecraft:beef",
    "minecraft:porkchop",
    "minecraft:mutton",
    "minecraft:rabbit",
];
pub const TAMING_FOOD: &str = "minecraft:chicken";
/// How far it sees food on the ground, in blocks.
const FOOD_RANGE: f64 = 16.0;

/// Meat on the ground: the item entity, where it is, and whether it can tame.
#[derive(Clone, Copy, Debug)]
pub struct Food {
    pub item: u32,
    pub at: [f64; 3],
    pub taming: bool,
}

/// What is saved of a vulture.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Saved {
    /// The owner's UUID as four ints (`UUIDUtil.uuidToIntArray`), when tamed.
    pub owner: Option<[i32; 4]>,
    pub sitting: bool,
    pub harness: bool,
    /// 0, 1 or 2 chests (SPECS §6.3).
    pub chests: u8,
}

fn flag(data: &Compound, key: &str) -> bool {
    matches!(data.get(&key.into()), Some(Tag::Byte(1)))
}

static STATE: AttachmentType<Saved> = AttachmentType {
    namespace: "vulture",
    name: "state",
    default: Saved::default,
    codec: Some(AttachmentCodec {
        encode: |s| {
            let mut data = Compound::new();
            if let Some(owner) = s.owner {
                data.insert("Owner".into(), Tag::IntArray(owner.to_vec()));
            }
            data.insert("Sitting".into(), Tag::Byte(i8::from(s.sitting)));
            data.insert("Harness".into(), Tag::Byte(i8::from(s.harness)));
            data.insert("Chests".into(), Tag::Byte(s.chests as i8));
            Ok(data)
        },
        decode: |data: &Compound| {
            let owner = match data.get(&"Owner".into()) {
                Some(Tag::IntArray(v)) => {
                    Some(<[i32; 4]>::try_from(v.as_slice()).map_err(|_| AttachmentError::Decode)?)
                }
                _ => None,
            };
            let chests = match data.get(&"Chests".into()) {
                Some(Tag::Byte(n)) => (*n).clamp(0, 2) as u8,
                _ => 0,
            };
            Ok(Saved {
                owner,
                sitting: flag(data, "Sitting"),
                harness: flag(data, "Harness"),
                chests,
            })
        },
    }),
    copy_on_death: false,
    sync: false,
};

#[derive(Default)]
struct Shared {
    /// The mob's state, as the goal last handed it over.
    current: Saved,
    /// Loaded from the save, for the goal to apply.
    loaded: Option<Saved>,
    /// The attachment has been read.
    read: bool,
    /// `current` differs from what is saved.
    dirty: bool,
    /// Where it is and whether it wants food, from the goal.
    pos: [f64; 3],
    hungry: bool,
    /// The meat the hook found for it, and the one it ate.
    food: Option<Food>,
    ate: Option<u32>,
    /// Ticks since it last looked for food.
    scan: u8,
    /// Changes asked by the hooks.
    pending: Vec<Change>,
    /// Its dimension, from the tick hook.
    dimension: String,
}

static SHARED: Mutex<BTreeMap<u32, Shared>> = Mutex::new(BTreeMap::new());

/// A change the hooks (its screen) ask of the mob, applied by the goal on its next tick.
#[derive(Clone, Copy, Debug)]
pub enum Change {
    ToggleSit,
    RemoveChest,
    RemoveHarness,
}

/// Its state as last handed over by the goal.
pub fn saved(id: u32) -> Saved {
    SHARED
        .lock()
        .unwrap()
        .get(&id)
        .map(|s| s.current.clone())
        .unwrap_or_default()
}

/// Asks the mob for `change` (from a hook).
pub fn request(id: u32, change: Change) {
    let mut map = SHARED.lock().unwrap();
    let shared = map.entry(id).or_default();
    shared.pending.push(change);
    // Seen at once by a screen opened again before the goal's next tick.
    match change {
        Change::ToggleSit => shared.current.sitting = !shared.current.sitting,
        Change::RemoveChest => shared.current.chests = shared.current.chests.saturating_sub(1),
        Change::RemoveHarness => shared.current.harness = false,
    }
}

/// What the drawing needs of it: its harness and chests (the render state doesn't carry them).
pub fn gear(id: u32) -> (bool, u8) {
    SHARED
        .lock()
        .unwrap()
        .get(&id)
        .map_or((false, 0), |s| (s.current.harness, s.current.chests))
}

/// The mob's state as saved.
pub fn of(m: &Mob) -> Saved {
    Saved {
        owner: if m.extra.tame { m.extra.owner } else { None },
        sitting: m.extra.sitting,
        harness: m.extra.saddled,
        chests: m.extra.variant2.min(2),
    }
}

/// From a goal, every tick: applies a state just loaded, then hands the mob's state over.
pub fn sync_mob(m: &mut Mob) {
    let mut map = SHARED.lock().unwrap();
    let shared = map.entry(m.id).or_default();
    if let Some(saved) = shared.loaded.take() {
        m.extra.tame = saved.owner.is_some();
        m.extra.owner = saved.owner;
        m.extra.sitting = saved.sitting;
        m.extra.saddled = saved.harness;
        m.extra.variant2 = saved.chests;
    }
    for change in std::mem::take(&mut shared.pending) {
        match change {
            Change::ToggleSit => m.extra.sitting = !m.extra.sitting,
            Change::RemoveChest => m.extra.variant2 = m.extra.variant2.saturating_sub(1),
            Change::RemoveHarness => m.extra.saddled = false,
        }
    }
    let now = of(m);
    if shared.read && now != shared.current {
        shared.dirty = true;
    }
    shared.current = now;
    shared.pos = [m.position.x, m.position.y, m.position.z];
    // Wild, it eats whenever it finds meat; tamed, only when hurt (SPECS §6: healing).
    shared.hungry = m.rider.is_none() && m.ai.target.is_none() && (!m.extra.tame || m.health < m.attrs.max_health);
    if !shared.hungry {
        shared.food = None;
    }
}

/// The meat found for it, if any.
pub fn food(id: u32) -> Option<Food> {
    SHARED.lock().unwrap().get(&id).and_then(|s| s.food)
}

/// It ate `food`: the hook takes one from the stack.
pub fn eaten(id: u32, food: Food) {
    if let Some(s) = SHARED.lock().unwrap().get_mut(&id) {
        s.food = None;
        s.ate = Some(food.item);
    }
}

/// From `entity_tick_post` (looking every 10 ticks): takes what it ate from the ground, and finds the nearest raw meat for a
/// hungry vulture.
pub fn look_for_food(host: &mut dyn Host, dimension: &str, id: u32) {
    let (pos, hungry, ate) = {
        let mut map = SHARED.lock().unwrap();
        let Some(s) = map.get_mut(&id) else {
            return;
        };
        s.scan = (s.scan + 1) % 10;
        if s.scan != 0 && s.ate.is_none() {
            return;
        }
        (s.pos, s.hungry, s.ate.take())
    };
    let r = FOOD_RANGE;
    let items = host.item_entities(
        dimension,
        [pos[0] - r, pos[1] - r, pos[2] - r],
        [pos[0] + r, pos[1] + r, pos[2] + r],
    );
    if let Some(item) = ate
        && let Some(e) = items.iter().find(|e| e.id == item)
    {
        let mut stack = e.stack.clone();
        stack.count = stack.count.saturating_sub(1);
        host.set_item_entity(dimension, item, stack);
    }
    if !hungry {
        return;
    }
    let distance = |at: [f64; 3]| (0..3).map(|k| (at[k] - pos[k]).powi(2)).sum::<f64>();
    let found = items
        .iter()
        .filter(|e| e.stack.count > 0 && MEATS.contains(&&*e.stack.item))
        .min_by(|a, b| distance(a.position).total_cmp(&distance(b.position)))
        .map(|e| Food {
            item: e.id,
            at: e.position,
            taming: &*e.stack.item == TAMING_FOOD,
        });
    if let Some(s) = SHARED.lock().unwrap().get_mut(&id) {
        s.food = found;
    }
}

/// Its dimension, as its last tick heard it.
pub fn dimension(id: u32) -> String {
    SHARED
        .lock()
        .unwrap()
        .get(&id)
        .map(|s| s.dimension.clone())
        .unwrap_or_default()
}

/// From `entity_tick_post`: reads the save the first time, writes it when the state changed.
pub fn save(host: &mut dyn Host, dimension: &str, id: u32) {
    let mut map = SHARED.lock().unwrap();
    let shared = map.entry(id).or_default();
    if shared.dimension != dimension {
        shared.dimension = dimension.to_owned();
    }
    let holder = Holder::Entity(id);
    if !shared.read {
        shared.read = true;
        if let Ok(Some(saved)) = STATE.get_existing(host, holder) {
            shared.current = saved.clone();
            shared.loaded = Some(saved);
        }
        return;
    }
    if shared.dirty && STATE.set(host, holder, shared.current.clone()).is_ok() {
        shared.dirty = false;
    }
}
