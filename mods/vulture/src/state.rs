//! A vulture's own state (SPECS §5, §6): its owner, whether it sits, its harness and chests.
//!
//! It lives here, per vulture (`Shared`), where its behaviours (run from the server tick, the library's `body`), the
//! right-click (`interaction.rs`) and the drawing read it: tamed or not, its owner, sitting, harness, chests, its age
//! (negative for a baby, positive for a breeding pause, counted by `sync`), love. It is saved in attachments with the
//! entity: its owner and whether it sits in the library's `creature:friend` (`creature::friend`, common/SPECS.md
//! §6.2), the rest in `vulture:state` (a save from before held the owner there too: it is moved over once).
//!
//! - every tick, `sync` (the first behaviour) applies a state just loaded and the changes the hooks asked for;
//! - `entity_tick_post` loads the attachment the first time it sees a vulture and writes it back when the state
//!   changed (`save`).
//!
//! Food (SPECS §4, §5): the hook, which can see the items on the ground, finds the nearest raw meat for a hungry vulture
//! (`food`); the `Eat` behaviour walks to it and eats it (`eaten`); the hook then takes one from the stack.
//!
//! Single player: the behaviours and the hooks share the mob's id.
use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::body::Body;
use mod_api::{AttachmentCodec, AttachmentError, AttachmentType, Compound, Holder, LevelRef, ServerHost, Tag};

fn set_stats(m: &mut Body, current: &mut Saved, health: f32, pace: f32) {
    m.max_health = health;
    m.health = m.health.min(health);
    current.health = health;
    current.pace = pace;
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
    /// Who threw it (its entity id), if anyone.
    pub thrower: Option<u32>,
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
            // The owner and sitting are the library's friend's (`save`).
            let mut data = Compound::new();
            data.insert("Harness".into(), Tag::Byte(i8::from(s.harness)));
            data.insert("Chests".into(), Tag::Byte(s.chests as i8));
            data.insert("Age".into(), Tag::Int(s.age));
            data.insert("Health".into(), Tag::Float(s.health.to_bits()));
            data.insert("Pace".into(), Tag::Float(s.pace.to_bits()));
            Ok(data)
        },
        // `Owner` and `Sitting`: only in a save from before the owner moved to `creature:friend` (`save` moves it).
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
    /// Ticks of love left (SPECS §8).
    love: i32,
    /// Its stats were rolled or loaded.
    rolled: bool,
    /// Its dimension, from the tick hook.
    dimension: String,
    /// Whether it may breed now, for a partner's goal.
    in_love: bool,
    /// Whether it is on a lead, for the right-click.
    leashed: bool,
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
pub fn breed(a: &Body, b: u32, rolls: [f32; 2]) {
    {
        let mut bred = BRED.lock().unwrap();
        if bred.contains(&a.id) || bred.contains(&b) {
            return;
        }
        bred.insert(a.id);
        bred.insert(b);
    }
    let other = saved(b);
    let mine = saved(a.id);
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
pub fn births(host: &mut dyn ServerHost, dimension: &str) {
    let births = std::mem::take(&mut *BIRTHS.lock().unwrap());
    for birth in births {
        if let Some(id) = host.summon(dimension, "vulture:vulture", birth.at, Compound::new()) {
            if let Some(owner) = birth.baby.owner {
                creature::friend::befriend(host, id, creature::friend::uuid(owner));
            }
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

/// A change the hooks ask of the vulture, applied on its next tick (taking the saddle or a chest off: its screen's).
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
    /// Told it is (not) on a lead now (the right-click tied or took it, natively).
    Leash,
    Unleash,
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
        Change::Leash => shared.leashed = true,
        Change::Unleash => shared.leashed = false,
        Change::Grow | Change::Heal => {}
    }
}

/// Whether it is on a lead.
pub fn leashed(id: u32) -> bool {
    SHARED.lock().unwrap().get(&id).is_some_and(|s| s.leashed)
}

/// Whether it is hurt (for raw meat to heal it).
pub fn hurt(id: u32) -> bool {
    SHARED.lock().unwrap().get(&id).is_some_and(|s| s.hurt)
}

/// What the drawing needs of a vulture, which its render state doesn't carry. Shared through a static, single player
/// only (a renderer has no host).
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
    /// Its age when its talons last snapped down (SPECS §6.4), caught or missed.
    pub grabbed_at: u32,
}

/// Its look for the drawing; a vulture no tick has seen (the book's, drawn in a screen) is grown up.
pub fn look(id: u32) -> Look {
    let grown = Look {
        growth: 1.0,
        ..Look::default()
    };
    SHARED.lock().unwrap().get(&id).map_or(grown, |s| s.look)
}

/// From the `Eat` behaviour: it is eating now.
pub fn pecked(m: &Body) {
    if let Some(s) = SHARED.lock().unwrap().get_mut(&m.id) {
        s.look.ate_at = m.age.max(0) as u32;
    }
}

/// Players who saw a wild vulture eat the raw chicken they threw, for the bestiary (`lib.rs`, `entity_tick_post`).
static TAMING_SEEN: Mutex<Vec<u128>> = Mutex::new(Vec::new());

/// From the `Eat` behaviour: `player` saw it eat a taming chicken.
pub fn taming_seen(player: u128) {
    TAMING_SEEN.lock().unwrap().push(player);
}

pub fn take_taming_seen() -> Vec<u128> {
    std::mem::take(&mut *TAMING_SEEN.lock().unwrap())
}

/// From the talons (`talons::toggle`): they snap down now.
pub fn grabbed(m: &Body) {
    if let Some(s) = SHARED.lock().unwrap().get_mut(&m.id) {
        s.look.grabbed_at = m.age.max(0) as u32;
    }
}

/// Changes vulture `id`'s state (from a behaviour: tamed, sitting, love, age).
pub fn with(id: u32, change: impl FnOnce(&mut Saved)) {
    let mut map = SHARED.lock().unwrap();
    let shared = map.entry(id).or_default();
    change(&mut shared.current);
}

/// Ticks of love it has left.
pub fn love(id: u32) -> i32 {
    SHARED.lock().unwrap().get(&id).map_or(0, |s| s.love)
}

/// Its love over (it bred).
pub fn end_love(id: u32) {
    if let Some(s) = SHARED.lock().unwrap().get_mut(&id) {
        s.love = 0;
        s.in_love = false;
    }
}

/// A baby is a chick until this share of its growth, then a juvenile (SPECS §8): only a juvenile follows its owner.
pub const JUVENILE: f32 = 0.5;

/// Whether a vulture of this `age` is still a chick.
pub fn chick(age: i32) -> bool {
    growth(age) < JUVENILE
}

/// How far a baby of this `age` has grown (0 at birth, 1 grown up); a breeding pause (positive) is grown up.
fn growth(age: i32) -> f32 {
    if age < 0 {
        (1.0 - age as f32 / BABY_AGE as f32).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// From its first behaviour, every tick: applies a state just loaded and the changes the hooks asked, counts its age
/// and love. Whether a baby was just fed (for its hearts). `ridden`, `fighting`: whether it has a rider, a target.
pub fn sync(m: &mut Body, ridden: bool, fighting: bool) -> bool {
    let mut map = SHARED.lock().unwrap();
    let shared = map.entry(m.id).or_default();
    if let Some(saved) = shared.loaded.take() {
        let (health, pace) = (saved.health, saved.pace);
        shared.harness = saved.harness;
        shared.current = saved;
        if pace > 0.0 {
            set_stats(m, &mut shared.current, health, pace);
            shared.rolled = true;
        }
    }
    // A vulture seen for the first time (a wild spawn, an egg): its stats are rolled once.
    if shared.read && !shared.rolled {
        let health = stat(HEALTH, m.random.next_float());
        let pace = stat(PACE, m.random.next_float());
        set_stats(m, &mut shared.current, health, pace);
        m.health = m.max_health;
        shared.rolled = true;
        shared.dirty = true;
    }
    // Its age: a baby grows towards 0, a breeding pause runs down to 0.
    let age = &mut shared.current.age;
    *age -= age.signum();
    if shared.love > 0 {
        shared.love -= 1;
    }
    shared.in_love = shared.love > 0;
    if !shared.in_love {
        BRED.lock().unwrap().remove(&m.id);
    }
    let mut fed = false;
    for change in std::mem::take(&mut shared.pending) {
        let c = &mut shared.current;
        match change {
            // Already in `current` (`request`).
            Change::ToggleSit | Change::AddChest | Change::RemoveChest => {}
            Change::PutHarness => shared.harness = true,
            Change::RemoveHarness => shared.harness = false,
            Change::InLove => shared.love = LOVE,
            Change::Grow => {
                c.age = (c.age + GROW).min(-1);
                fed = true;
            }
            Change::Heal => m.health = (m.health + HEAL).min(m.max_health),
            Change::Leash => shared.leashed = true,
            Change::Unleash => shared.leashed = false,
        }
    }
    shared.current.harness = shared.harness;
    let now = shared.current.clone();
    if now.age.rem_euclid(1200) == 0 {
        shared.dirty = true;
    }
    shared.look = Look {
        harness: now.harness,
        chests: now.chests,
        growth: growth(now.age),
        sitting: now.sitting && m.on_ground,
        striking: fighting,
        ate_at: shared.look.ate_at,
        grabbed_at: shared.look.grabbed_at,
    };
    shared.hurt = m.health < m.max_health;
    shared.pos = [m.position.x, m.position.y, m.position.z];
    // Wild, it eats whenever it finds meat; tamed, only when hurt (SPECS §6: healing).
    shared.hungry = !ridden && !fighting && (now.owner.is_none() || shared.hurt);
    if !shared.hungry {
        shared.food = None;
    }
    fed
}

/// Its state's change since it was saved is to be saved (a behaviour tamed it, made it sit...).
pub fn touch(id: u32) {
    if let Some(s) = SHARED.lock().unwrap().get_mut(&id) {
        s.dirty = true;
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

/// From `entity_tick_post` (looking every 10 ticks): takes what it ate from the ground, and finds the nearest raw meat
/// for a hungry vulture.
pub fn look_for_food(host: &mut dyn ServerHost, level: LevelRef, id: u32) {
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
    let items: Vec<(mod_api::ItemEntityRef, [f64; 3], mod_api::ItemStack)> = level
        .entities_in(
            &*host,
            [pos[0] - r, pos[1] - r, pos[2] - r],
            [pos[0] + r, pos[1] + r, pos[2] + r],
        )
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let item = host.item_entity(e)?;
            Some((item, e.position(&*host)?, item.stack(&*host)?))
        })
        .collect();
    if let Some(eaten) = ate
        && let Some((item, _, stack)) = items.iter().find(|(i, _, _)| i.entity().id() == eaten)
    {
        match stack.clone().shrink(1) {
            Some(rest) => {
                let _ = item.set_stack(host, rest);
            }
            None => {
                item.entity().discard(host);
            }
        }
    }
    if !hungry {
        return;
    }
    let distance = |at: [f64; 3]| (0..3).map(|k| (at[k] - pos[k]).powi(2)).sum::<f64>();
    let found = items
        .iter()
        .filter(|(_, _, stack)| MEATS.contains(&creature::hand::id(&*host, stack).as_str()))
        .min_by(|a, b| distance(a.1).total_cmp(&distance(b.1)))
        .map(|(item, at, stack)| Food {
            item: item.entity().id(),
            at: *at,
            taming: creature::hand::is(&*host, stack, TAMING_FOOD),
            thrower: item.owner(&*host).map(|o| o.id()),
        });
    if let Some(s) = SHARED.lock().unwrap().get_mut(&id) {
        s.food = found;
    }
}

/// From `entity_tick_post`: reads the saves the first time (moving an old save's owner to the friend), writes them
/// when the state changed; the owner and sitting go to the library's friend.
pub fn save(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    creature::friend::tick(host, id);
    if !creature::friend::loaded(id) {
        // Not reachable yet (just loaded): read again next tick rather than take it for a new vulture.
        return;
    }
    let mut map = SHARED.lock().unwrap();
    let shared = map.entry(id).or_default();
    if shared.dimension != dimension {
        shared.dimension = dimension.to_owned();
    }
    let holder = Holder::Entity(id);
    if !shared.read {
        let mut saved = match STATE.get_existing(host, holder) {
            Ok(saved) => saved,
            Err(_) => return,
        };
        let mut friend = creature::friend::of(id);
        // A save from before: its owner moves to the friend, and `vulture:state` is written again without it.
        if friend.is_none()
            && let Some(old) = saved.as_ref().and_then(|s| s.owner.map(|o| (o, s.sitting)))
        {
            creature::friend::befriend(host, id, creature::friend::uuid(old.0));
            creature::friend::set_staying(id, old.1);
            friend = creature::friend::of(id);
            shared.dirty = true;
        }
        if let Some(f) = friend {
            let s = saved.get_or_insert_with(Saved::default);
            s.owner = Some(f.uuid);
            s.sitting = f.staying;
        }
        if let Some(saved) = saved {
            shared.current = saved.clone();
            shared.loaded = Some(saved);
        }
        shared.read = true;
        return;
    }
    let friend = creature::friend::of(id);
    match (shared.current.owner, friend) {
        // Tamed (or born tamed) since: the friend is saved.
        (Some(owner), f) if f.is_none_or(|f| f.uuid != owner) => {
            creature::friend::befriend(host, id, creature::friend::uuid(owner));
            creature::friend::set_staying(id, shared.current.sitting);
        }
        (Some(_), Some(f)) if f.staying != shared.current.sitting => {
            creature::friend::set_staying(id, shared.current.sitting);
        }
        // Never untamed by a state that lost its owner: the friend wins, and is applied to the mob.
        (None, Some(f)) => {
            let mut saved = shared.current.clone();
            saved.owner = Some(f.uuid);
            saved.sitting = f.staying;
            shared.current = saved.clone();
            shared.loaded = Some(saved);
            return;
        }
        _ => {}
    }
    if shared.dirty && STATE.set(host, holder, shared.current.clone()).is_ok() {
        shared.dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_save_s_owner_is_read_and_a_new_save_keeps_it_out() {
        let codec = STATE.codec.as_ref().unwrap();
        let mut old = Compound::new();
        old.insert("Owner".into(), Tag::IntArray(vec![1, 2, 3, 4]));
        old.insert("Sitting".into(), Tag::Byte(1));
        old.insert("Chests".into(), Tag::Byte(5));
        let saved = (codec.decode)(&old).unwrap();
        assert_eq!(saved.owner, Some([1, 2, 3, 4]));
        assert!(saved.sitting);
        assert_eq!(saved.chests, 2, "at most two chests");
        // The owner now lives in the library's friend: `vulture:state` no longer writes it.
        let written = (codec.encode)(&saved).unwrap();
        assert!(!written.contains_key(&"Owner".into()));
        assert!(!written.contains_key(&"Sitting".into()));
        let again = (codec.decode)(&written).unwrap();
        assert_eq!(again.chests, 2);
        assert_eq!(again.owner, None);
    }

    #[test]
    fn a_baby_grows_from_chick_to_juvenile_to_grown() {
        assert!(chick(BABY_AGE));
        assert!(!chick(BABY_AGE / 4), "three quarters grown: a juvenile");
        assert_eq!(growth(0), 1.0);
        assert_eq!(growth(BREED_PAUSE), 1.0, "a breeding pause is grown up");
        assert!((growth(BABY_AGE / 2) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_stat_rolls_within_its_range() {
        assert_eq!(stat(HEALTH, 0.0), HEALTH.0);
        assert_eq!(stat(HEALTH, 1.0), HEALTH.1);
        assert_eq!(stat(PACE, 0.5), (PACE.0 + PACE.1) / 2.0);
    }
}
