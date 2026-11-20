//! Water creatures (SPECS §3.10): the jellyfish and the flying fish, which copy the land-living wandering trader.
//!
//! - **Breathing**: they never drown (the library's `living_breathe`: water is their air); out of water they dry out
//!   instead (`tick`).
//! - **In water** they float (no gravity), their mod's behaviours moving them; **beached**, they fall, flop now and then
//!   and, after their kind's delay, dry out (a point of damage a second, `minecraft:dry_out`), dropping as usual.
//! - **The bucket**: a water bucket used on one catches it as vanilla's fish, into its mod's bucket item, keeping its look
//!   and name (as the glass jar does: `jar::held`); the bucket used on a block lets it out in water there, and gives the
//!   empty bucket back. One let out of a bucket is kept: it never despawns (`KEPT`, read by `mob::despawn_far`).
//! - **Spawning** (`spawns`): groups in the water of their biomes, out of sight of the players, under a cap.
//!
//! Single player: the hooks share the mob's id (as the rest of the series).
use std::collections::BTreeMap;
use std::sync::Mutex;

use mod_api::{
    BlockUpdateFlags, Compound, EntityInteractEvent, Host, HostSide, InteractionResult, ItemPart, ItemStack, LevelRef,
    LivingBreatheEvent, ServerHost, SoundSource, Tag, UseOnEvent,
};

use crate::territory::Ground;
use crate::{colours, hand, jar};

/// A water creature's kind, as its mod registers it.
pub struct Aquatic {
    pub kind: &'static str,
    /// Its mod's bucket item (`jellyfish:jellyfish_bucket`).
    pub bucket: &'static str,
    /// How long it lasts out of water before it dries out (ticks).
    pub dries: u32,
}

/// Where and how a water creature spawns: in groups of `group`, `depth` blocks under the surface, up to `cap` near a
/// player, one try in `chance` every `SPAWN_EVERY` ticks.
pub struct Spawning {
    pub kind: &'static str,
    pub biomes: &'static [&'static str],
    pub group: (u32, u32),
    pub cap: usize,
    pub chance: u64,
    pub depth: (f64, f64),
}

/// The fact learnt by catching one in a bucket.
pub const BUCKET_FACT: &str = "bucket";
/// The tag of a creature let out of a bucket: it never despawns.
pub const KEPT: &str = "creature.kept";

static AQUATIC: Mutex<Vec<&'static Aquatic>> = Mutex::new(Vec::new());
static SPAWNING: Mutex<Vec<&'static Spawning>> = Mutex::new(Vec::new());
/// Ticks each one has spent out of water.
static DRY: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());
/// Each one's kind and where it is, for the spawning caps.
static AT: Mutex<BTreeMap<u32, (&'static str, [f64; 3])>> = Mutex::new(BTreeMap::new());
static SPAWN_TICKS: Mutex<u32> = Mutex::new(0);

/// From a mod's `register`.
pub fn register(aquatic: &'static Aquatic) {
    let mut all = AQUATIC.lock().unwrap();
    if !all.iter().any(|a| a.kind == aquatic.kind) {
        all.push(aquatic);
    }
}

/// From a mod's `register`: its creatures spawn in water.
pub fn spawns(spawning: &'static Spawning) {
    SPAWNING.lock().unwrap().push(spawning);
}

fn of(kind: &str) -> Option<&'static Aquatic> {
    AQUATIC.lock().unwrap().iter().copied().find(|a| a.kind == kind)
}

/// Whether creatures of `kind` live in water.
pub fn is(kind: &str) -> bool {
    of(kind).is_some()
}

/// From the library's `living_breathe`: water creatures never drown (their drying out is `tick`'s).
pub(crate) fn breathe(event: &mut LivingBreatheEvent<'_>) {
    if of(event.kind).is_some() {
        event.can_breathe = true;
        event.consume = 0;
    }
}

/// How often a beached one flops (ticks), and how often it is hurt once dry.
const FLOP_EVERY: u32 = 30;
const DRY_HURT_EVERY: u32 = 20;

/// From the mod's `entity_tick_post`, for each of its water creatures: floating in water, beached out of it (flopping,
/// then drying out). Whether it is in water.
pub fn tick(host: &mut dyn ServerHost, entity: u32, kind: &str) -> bool {
    let Some(aquatic) = of(kind) else {
        return false;
    };
    let Some(e) = host.entity(entity) else {
        return false;
    };
    let wet = e.is_in_water(&*host).unwrap_or(false);
    if e.no_gravity(&*host) != Some(wet) {
        e.set_no_gravity(host, wet);
    }
    if let Some(at) = e.position(&*host) {
        AT.lock().unwrap().insert(entity, (aquatic.kind, at));
    }
    if wet {
        DRY.lock().unwrap().remove(&entity);
        return true;
    }
    let out = {
        let mut dry = DRY.lock().unwrap();
        let t = dry.entry(entity).or_insert(0);
        *t += 1;
        *t
    };
    if out.is_multiple_of(FLOP_EVERY) && e.on_ground(&*host).unwrap_or(false) {
        let mut random = colours::Random::new(entity ^ out);
        let mut side = || (random.next() % 1000) as f64 / 1000.0 * 0.2 - 0.1;
        let kick = [side(), 0.3, side()];
        e.set_velocity(host, kick);
    }
    if out >= aquatic.dries && out.is_multiple_of(DRY_HURT_EVERY) {
        e.hurt(host, "minecraft:dry_out", 1.0, None, None);
    }
    false
}

/// From the mod, when one is gone.
pub fn forget(entity: u32) {
    DRY.lock().unwrap().remove(&entity);
    AT.lock().unwrap().remove(&entity);
}

/// Whether `stack` on `kind` is a catch: a water bucket on a water creature.
pub(crate) fn catches(host: &dyn Host, event: &EntityInteractEvent<'_>) -> bool {
    event.stack.is_some_and(|s| hand::is(host, s, "minecraft:water_bucket")) && of(event.kind).is_some()
}

/// From the library's `entity_interact`, on the server: a water bucket catches a water creature into its mod's
/// bucket. Whether it did.
pub(crate) fn catch(host: &mut dyn ServerHost, event: &EntityInteractEvent<'_>) -> bool {
    if !catches(&*host, event) {
        return false;
    }
    let Some(aquatic) = of(event.kind) else {
        return false;
    };
    let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity)) else {
        return false;
    };
    let look = colours::saved(host, id);
    let name = jar::custom_name(host, id);
    let at = host.entity(id).and_then(|e| e.position(&*host));
    let Some(bucket) = filled(&*host, aquatic, look.as_ref(), name.as_deref()) else {
        return false;
    };
    if !host.entity(id).is_some_and(|e| e.discard(host)) {
        return false;
    }
    colours::forget(id);
    crate::body::forget(id);
    forget(id);
    crate::bestiary::learn(host, player.uuid(), aquatic.kind, BUCKET_FACT);
    // A water bucket doesn't stack: the one in hand becomes the full one.
    hand::set_held(host, player, event.hand, Some(bucket));
    if let (Some(at), Some(level)) = (at, player.level(&*host)) {
        level.play_sound(
            host,
            "minecraft:item.bucket.fill_fish",
            at,
            SoundSource::Neutral,
            1.0,
            1.0,
            None,
        );
    }
    true
}

/// A bucket holding `aquatic` with `look`, named `name` (the item data the glass jar uses, `jar::held`).
fn filled(host: &dyn Host, aquatic: &Aquatic, look: Option<&Compound>, name: Option<&str>) -> Option<ItemStack> {
    let text = |l: &Compound, key: &str| match l.get(&key.into()) {
        Some(Tag::String(s)) => s.to_string_lossy(),
        _ => String::new(),
    };
    let look_fields = look.map(|l| [text(l, "Base"), text(l, "Pattern"), text(l, "PatternColour")]);
    let look_data = look
        .map(|l| {
            let fields: Vec<String> = l
                .iter()
                .filter_map(|(k, v)| match v {
                    Tag::String(s) => Some(format!("{}:{}", k.to_string_lossy(), jar::quoted(&s.to_string_lossy()))),
                    _ => None,
                })
                .collect();
            format!(",look:{{{}}}", fields.join(","))
        })
        .unwrap_or_default();
    let name_data = name.map(|n| format!(",name:{}", jar::quoted(n))).unwrap_or_default();
    let snbt = format!(
        "{{id:{item},count:1,components:{{\"minecraft:custom_data\":{{{data}:{{kind:{kind}{look_data}{name_data}}}}},\
         \"minecraft:lore\":[{lore}]}}}}",
        item = jar::quoted(aquatic.bucket),
        data = jar::DATA,
        kind = jar::quoted(aquatic.kind),
        lore = jar::lore(aquatic.kind, look_fields, false, name),
    );
    host.item_from_snbt(&snbt)
}

/// From the mod's bucket (`Bucket`): it lets its creature out in water on the face clicked, and the empty bucket comes
/// back (not in creative). Whether the click was the bucket's.
pub fn release(host: &mut dyn ServerHost, event: &UseOnEvent<'_>) -> bool {
    let Some(held) = jar::held(event.stack) else {
        return false;
    };
    let Some(player) = host.player(event.player) else {
        return true;
    };
    let Some(level) = player.level(&*host) else {
        return true;
    };
    // Its mod isn't installed: the bucket stays full.
    if of(&held.kind).is_none() {
        return true;
    }
    let (pos, face) = (event.pos, event.face);
    let block = [pos[0] + face[0], pos[1] + face[1], pos[2] + face[2]];
    if !pour(host, level, block) {
        return true;
    }
    let at = [
        f64::from(block[0]) + 0.5,
        f64::from(block[1]) + 0.1,
        f64::from(block[2]) + 0.5,
    ];
    let mut nbt = Compound::new();
    if let Some(n) = &held.name {
        nbt.insert("CustomName".into(), Tag::String(n.as_str().into()));
    }
    nbt.insert("Tags".into(), Tag::List(vec![Tag::String(KEPT.into())]));
    let dimension = level.dimension().id(&*host).unwrap_or_default();
    let Some(id) = host.summon(&dimension, &held.kind, at, nbt) else {
        return true;
    };
    if let Some(look) = &held.look {
        colours::restore(host, id, look);
    }
    level.play_sound(
        host,
        "minecraft:item.bucket.empty_fish",
        at,
        SoundSource::Neutral,
        1.0,
        1.0,
        None,
    );
    if !player.has_infinite_materials(&*host).unwrap_or(false) {
        let empty = host.item_from_snbt("{id:\"minecraft:bucket\",count:1}");
        hand::set_held(host, player, event.hand, empty);
    }
    true
}

/// Water at `block`: poured there in air or a plant a bucket would wash away, kept where there is water already.
fn pour(host: &mut dyn ServerHost, level: LevelRef, block: [i32; 3]) -> bool {
    let here = Ground::new(&*host, level).block(block);
    if here == "minecraft:water" {
        return true;
    }
    if !matches!(
        here.as_str(),
        "minecraft:air" | "minecraft:cave_air" | "minecraft:short_grass"
    ) {
        return false;
    }
    let Some(water) = host.block_state_id("minecraft:water") else {
        return false;
    };
    level
        .set_block(
            host,
            block,
            water,
            BlockUpdateFlags::NEIGHBORS | BlockUpdateFlags::CLIENTS,
        )
        .unwrap_or(false)
}

/// A mod's bucket of a water creature: its right-click on a block lets the creature out (`release`).
pub struct Bucket;

#[mod_api::sunrise_part]
impl ItemPart for Bucket {
    fn use_on(&self, host: HostSide, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        match host {
            HostSide::Server(host) => release(host, event).then_some(InteractionResult::Success),
            HostSide::Client(_) => jar::held(event.stack).map(|_| InteractionResult::Success),
        }
    }
}

/// How often a try is made (ticks), how near a player the cap counts (blocks), and how far from them a group comes out.
const SPAWN_EVERY: u32 = 200;
const SPAWN_NEAR: f64 = 48.0;
const SPAWN_MIN: f64 = 20.0;
const SPAWN_MAX: f64 = 40.0;
/// A group spreads this far around its spot (blocks); the water surface is looked for this far above and below a
/// player.
const GROUP_SPREAD: f64 = 3.0;
const SEARCH: (i32, i32) = (16, 24);

/// From the library's `level_tick_post`, in the overworld: groups come out in the water near players.
pub(crate) fn spawn(host: &mut dyn ServerHost, level: LevelRef) {
    if level.dimension() != mod_api::DimensionKey::OVERWORLD {
        return;
    }
    {
        let mut t = SPAWN_TICKS.lock().unwrap();
        *t += 1;
        if *t < SPAWN_EVERY {
            return;
        }
        *t = 0;
    }
    let spawning: Vec<&'static Spawning> = SPAWNING.lock().unwrap().clone();
    if spawning.is_empty() {
        return;
    }
    let seed = level.game_time(&*host).unwrap_or(0) as u32;
    let mut random = colours::Random::new(seed);
    let mut float = || (random.next() % 10_000) as f64 / 10_000.0;
    let dimension = level.dimension().id(&*host).unwrap_or_default();
    for player in host.players().server_players() {
        if player.level(&*host) != Some(level) {
            continue;
        }
        let Some(p) = player.entity().position(&*host) else {
            continue;
        };
        for s in &spawning {
            if s.chance > 1 && (float() * s.chance as f64) as u64 != 0 {
                continue;
            }
            let near = AT
                .lock()
                .unwrap()
                .values()
                .filter(|(kind, at)| *kind == s.kind && (at[0] - p[0]).hypot(at[2] - p[2]) < SPAWN_NEAR)
                .count();
            if near >= s.cap {
                continue;
            }
            let angle = float() * std::f64::consts::TAU;
            let distance = SPAWN_MIN + float() * (SPAWN_MAX - SPAWN_MIN);
            let (x, z) = (p[0] + angle.cos() * distance, p[2] + angle.sin() * distance);
            let ground = Ground::new(&*host, level);
            let (bx, bz) = (x.floor() as i32, z.floor() as i32);
            let Some(surface) = water_surface(ground, bx, p[1] as i32, bz) else {
                continue;
            };
            let depth = s.depth.0 + float() * (s.depth.1 - s.depth.0);
            let y = f64::from(surface) - depth;
            if ground.block([bx, y.floor() as i32, bz]) != "minecraft:water" {
                continue;
            }
            if !level
                .biome_at(&*host, [bx, y.floor() as i32, bz])
                .is_some_and(|b| s.biomes.contains(&b.as_str()))
            {
                continue;
            }
            let count = s.group.0 + (float() * f64::from(s.group.1 - s.group.0 + 1)) as u32;
            for _ in 0..count.min(s.group.1) {
                let at = [
                    x + (float() * 2.0 - 1.0) * GROUP_SPREAD,
                    y + (float() * 2.0 - 1.0),
                    z + (float() * 2.0 - 1.0) * GROUP_SPREAD,
                ];
                let block = at.map(|k| k.floor() as i32);
                if Ground::new(&*host, level).block(block) == "minecraft:water" {
                    let _ = host.summon(&dimension, s.kind, at, Compound::new());
                }
            }
        }
    }
}

/// The height of the top water block of the column at `x`, `z`, near height `y`, if the column has water there.
fn water_surface(ground: Ground<'_>, x: i32, y: i32, z: i32) -> Option<i32> {
    (y - SEARCH.1..=y + SEARCH.0)
        .rev()
        .find(|&by| ground.block([x, by, z]) == "minecraft:water" && ground.block([x, by + 1, z]) != "minecraft:water")
}
