//! A vulture's own state (SPECS §5, §6): its owner, whether it sits, its harness and chests.
//!
//! It lives on the mob, in `Mob.extra` (`tame`, `owner`, `sitting`, `variant2` = chests, `age`: negative for a
//! baby, positive for a breeding pause, aged by Horizon as any `AgeableMob`'s), where the goals, the right-click
//! (`interaction.rs`) and the drawing read it. A wandering trader saves none of those but the age, so it is also
//! kept in a server attachment (`vulture:state`, saved with the entity), and the two meet here:
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

use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, Host, Mob, RandomSource, Tag,
};

/// Each vulture's flight speed share, once rolled or loaded (`Mob` has no field for it).
static PACES: Mutex<BTreeMap<u32, f32>> = Mutex::new(BTreeMap::new());

fn set_stats(m: &mut Mob, health: f32, pace: f32) {
    m.attrs.max_health = health;
    m.health = m.health.min(health);
    PACES.lock().unwrap().insert(m.id, pace);
}

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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Saved {
    /// The owner's UUID as four ints (`UUIDUtil.uuidToIntArray`), when tamed.
    pub owner: Option<[i32; 4]>,
    pub sitting: bool,
    pub harness: bool,
    /// 0, 1 or 2 chests (SPECS §6.3).
    pub chests: u8,
    /// Ticks: negative while a baby (grows up at 0), positive while it may not breed again (SPECS §8).
    pub age: i32,
    /// Its stats (SPECS §2): max health, and its flight speed as a share of the base speed; 0 until rolled.
    pub health: f32,
    pub pace: f32,
}

/// The stats' ranges (SPECS §2).
pub const HEALTH: (f32, f32) = (16.0, 30.0);
pub const PACE: (f32, f32) = (0.6, 1.2);
/// A baby grows up in 20 minutes; parents wait 5 minutes before breeding again (SPECS §8).
pub const BABY_AGE: i32 = -24_000;
pub const BREED_PAUSE: i32 = 6_000;

fn float(data: &Compound, key: &str) -> f32 {
    match data.get(&key.into()) {
        Some(Tag::Float(bits)) => f32::from_bits(*bits),
        _ => 0.0,
    }
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
            data.insert("Age".into(), Tag::Int(s.age));
            data.insert("Health".into(), Tag::Float(s.health.to_bits()));
            data.insert("Pace".into(), Tag::Float(s.pace.to_bits()));
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
                age: match data.get(&"Age".into()) {
                    Some(Tag::Int(n)) => *n,
                    _ => 0,
                },
                health: float(data, "Health"),
                pace: float(data, "Pace"),
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
    /// Whether it may breed now, for a partner's goal.
    in_love: bool,
    look: Look,
    /// Its harness, kept here rather than on the mob (`of`).
    harness: bool,
    /// Below its max health.
    hurt: bool,
}

/// A baby to bring into the world (from the breeding goal, for the tick hook to summon).
#[derive(Clone, Debug)]
pub struct Birth {
    pub at: [f64; 3],
    pub baby: Saved,
}

static BIRTHS: Mutex<Vec<Birth>> = Mutex::new(Vec::new());

/// Vultures that have had their baby this time, until they are out of love: one baby per pair, whichever of the two
/// goals gets there first.
static BRED: Mutex<std::collections::BTreeSet<u32>> = Mutex::new(std::collections::BTreeSet::new());

/// Whether vulture `id` is in love (another vulture's breeding goal asks).
pub fn in_love(id: u32) -> bool {
    SHARED.lock().unwrap().get(&id).is_some_and(|s| s.in_love)
}

/// Rolls a stat between `range`'s ends from `roll` (0 to 1).
pub fn stat(range: (f32, f32), roll: f32) -> f32 {
    range.0 + (range.1 - range.0) * roll
}

/// From the breeding goal: a baby of `a` and `b` (SPECS §8): tamed to `a`'s owner; each stat the average of both
/// parents' and a fresh roll's, like a horse's.
pub fn breed(a: &Mob, b: u32, rolls: [f32; 2]) {
    {
        let mut bred = BRED.lock().unwrap();
        if bred.contains(&a.id) || bred.contains(&b) {
            return;
        }
        bred.insert(a.id);
        bred.insert(b);
    }
    let other = saved(b);
    let mine = of(a);
    let mix = |x: f32, y: f32, range: (f32, f32), roll: f32| (x + y + stat(range, roll)) / 3.0;
    let baby = Saved {
        owner: mine.owner,
        age: BABY_AGE,
        health: mix(mine.health, other.health, HEALTH, rolls[0]),
        pace: mix(mine.pace, other.pace, PACE, rolls[1]),
        ..Saved::default()
    };
    BIRTHS.lock().unwrap().push(Birth {
        at: [a.position.x, a.position.y, a.position.z],
        baby,
    });
}

/// From `entity_tick_post`: summons the babies waiting to be born, with their state.
pub fn births(host: &mut dyn Host, dimension: &str) {
    let births = std::mem::take(&mut *BIRTHS.lock().unwrap());
    for birth in births {
        if let Some(id) = host.summon(dimension, "vulture:vulture", birth.at, Compound::new()) {
            let _ = STATE.set(host, Holder::Entity(id), birth.baby);
        }
    }
}

/// Its flight speed as a share of the base speed.
pub fn pace(id: u32) -> f64 {
    SHARED.lock().unwrap().get(&id).map_or(1.0, |s| {
        if s.current.pace > 0.0 {
            f64::from(s.current.pace)
        } else {
            1.0
        }
    })
}

static SHARED: Mutex<BTreeMap<u32, Shared>> = Mutex::new(BTreeMap::new());

/// A change the hooks (its screen) ask of the mob, applied by the goal on its next tick.
#[derive(Clone, Copy, Debug)]
pub enum Change {
    ToggleSit,
    PutHarness,
    RemoveHarness,
    AddChest,
    RemoveChest,
    /// Fed rotten flesh: in love for 30 s (SPECS §8).
    InLove,
    /// A baby fed raw meat: a minute older.
    Grow,
    /// Fed raw meat while hurt: +4 health.
    Heal,
}

/// What raw meat heals, and how much older it makes a baby (ticks); how long love lasts.
const HEAL: f32 = 4.0;
const GROW: i32 = 1200;
const LOVE: i32 = 600;

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
    // Saved at once: `current` is changed here too, so the goal's next hand-over won't see a difference.
    shared.dirty = true;
    // Seen at once by a screen opened again before the goal's next tick.
    match change {
        Change::ToggleSit => shared.current.sitting = !shared.current.sitting,
        Change::PutHarness => shared.current.harness = true,
        Change::RemoveHarness => shared.current.harness = false,
        Change::AddChest => shared.current.chests = (shared.current.chests + 1).min(2),
        Change::RemoveChest => shared.current.chests = shared.current.chests.saturating_sub(1),
        Change::InLove => shared.in_love = true,
        Change::Grow | Change::Heal => {}
    }
}

/// Whether it is hurt (for raw meat to heal it).
pub fn hurt(id: u32) -> bool {
    SHARED.lock().unwrap().get(&id).is_some_and(|s| s.hurt)
}

/// What the drawing needs of a vulture, which its render state doesn't carry. Shared through a static, single player
/// only: `mob_submit` can't read an attachment (workaround for arismoko/horizon#2740).
#[derive(Clone, Copy, Debug, Default)]
pub struct Look {
    pub harness: bool,
    pub chests: u8,
    /// How far a baby has grown, from 0 at birth to 1 grown up (SPECS §8): drawn as a chick, then a juvenile.
    pub growth: f32,
    pub sitting: bool,
    /// Striking with its beak (`MeleeAttack`'s swing).
    pub striking: bool,
    /// Its age when it last ate, for the peck.
    pub ate_at: u32,
}

pub fn look(id: u32) -> Look {
    SHARED.lock().unwrap().get(&id).map_or(Look::default(), |s| s.look)
}

/// From the `Eat` goal: it is eating now.
pub fn pecked(m: &Mob) {
    if let Some(s) = SHARED.lock().unwrap().get_mut(&m.id) {
        s.look.ate_at = m.age;
    }
}

/// The mob's state as saved.
pub fn of(m: &Mob) -> Saved {
    Saved {
        owner: if m.extra.tame { m.extra.owner } else { None },
        sitting: m.extra.sitting,
        // Not on the mob: Horizon drives a mob whose `saddled` is set as its own saddled mount
        // (workaround for horizon-rust/horizon#13973).
        harness: false,
        chests: m.extra.variant2.min(2),
        age: m.extra.age,
        health: m.attrs.max_health,
        pace: PACES.lock().unwrap().get(&m.id).copied().unwrap_or(0.0),
    }
}

/// How far a baby of this `age` has grown (0 at birth, 1 grown up); a breeding pause (positive) is grown up.
fn growth(age: i32) -> f32 {
    if age < 0 {
        (1.0 - age as f32 / BABY_AGE as f32).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// From a goal, every tick: applies a state just loaded, then hands the mob's state over. Whether a baby was
/// just fed (for its hearts).
pub fn sync_mob(m: &mut Mob) -> bool {
    let mut map = SHARED.lock().unwrap();
    let shared = map.entry(m.id).or_default();
    if let Some(saved) = shared.loaded.take() {
        m.extra.tame = saved.owner.is_some();
        m.extra.owner = saved.owner;
        m.extra.sitting = saved.sitting;
        shared.harness = saved.harness;
        m.extra.variant2 = saved.chests;
        m.extra.age = saved.age;
        if saved.pace > 0.0 {
            set_stats(m, saved.health, saved.pace);
        }
    }
    // A vulture seen for the first time (a wild spawn, an egg): its stats are rolled once.
    if shared.read && shared.loaded.is_none() && PACES.lock().unwrap().get(&m.id).is_none() {
        let health = stat(HEALTH, m.random.next_float());
        let pace = stat(PACE, m.random.next_float());
        set_stats(m, health, pace);
        m.health = m.attrs.max_health;
    }
    if m.extra.in_love > 0 {
        m.extra.in_love -= 1;
    }
    shared.in_love = m.extra.in_love > 0;
    if !shared.in_love {
        BRED.lock().unwrap().remove(&m.id);
    }
    let mut fed = false;
    for change in std::mem::take(&mut shared.pending) {
        match change {
            Change::ToggleSit => m.extra.sitting = !m.extra.sitting,
            Change::PutHarness => shared.harness = true,
            Change::RemoveHarness => shared.harness = false,
            Change::AddChest => m.extra.variant2 = (m.extra.variant2 + 1).min(2),
            Change::RemoveChest => m.extra.variant2 = m.extra.variant2.saturating_sub(1),
            Change::InLove => m.extra.in_love = LOVE,
            Change::Grow => {
                m.extra.age = (m.extra.age + GROW).min(-1);
                fed = true;
            }
            Change::Heal => m.heal(HEAL),
        }
    }
    // Never a saddled mount for Horizon: it would drive it itself, ignoring the vulture's flight.
    m.extra.saddled = false;
    let mut now = of(m);
    now.harness = shared.harness;
    // The age alone changes every tick: saved once a minute, and when it grows up.
    let aged_only = Saved {
        age: shared.current.age,
        ..now.clone()
    } == shared.current;
    if shared.read && now != shared.current && (!aged_only || now.age % 1200 == 0) {
        shared.dirty = true;
    }
    shared.look = Look {
        harness: now.harness,
        chests: now.chests,
        growth: growth(now.age),
        sitting: now.sitting && m.on_ground,
        striking: m.ai.swinging,
        ate_at: shared.look.ate_at,
    };
    shared.current = now;
    shared.hurt = m.health < m.attrs.max_health;
    shared.pos = [m.position.x, m.position.y, m.position.z];
    // Wild, it eats whenever it finds meat; tamed, only when hurt (SPECS §6: healing).
    shared.hungry = m.rider.is_none() && m.ai.target.is_none() && (!m.extra.tame || m.health < m.attrs.max_health);
    if !shared.hungry {
        shared.food = None;
    }
    fed
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

/// From `entity_tick_post` (looking every 10 ticks): takes what it ate from the ground, and finds the nearest raw meat
/// for a hungry vulture.
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
        match STATE.get_existing(host, holder) {
            Ok(Some(saved)) => {
                shared.current = saved.clone();
                shared.loaded = Some(saved);
            }
            Ok(None) => {}
            // Not reachable yet (just loaded): read again next tick rather than take it for a new vulture.
            Err(_) => return,
        }
        shared.read = true;
        return;
    }
    if shared.dirty && shared.current.owner.is_none() {
        // Never untame a vulture by saving over it: a saved owner wins, and is applied to the mob.
        if let Ok(Some(saved)) = STATE.get_existing(host, holder)
            && saved.owner.is_some()
        {
            shared.current = saved.clone();
            shared.loaded = Some(saved);
            shared.dirty = false;
            return;
        }
    }
    if shared.dirty && STATE.set(host, holder, shared.current.clone()).is_ok() {
        shared.dirty = false;
    }
}
