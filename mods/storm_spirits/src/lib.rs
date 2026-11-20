//! Storm Spirits (SPECS.md): spirits of living lightning, born in storms, of the Sunrise Creatures series.
//!
//! A storm spirit is a copy of the wandering trader (its template's goals taken off by the library) drawn with its own
//! model (`model_storm_spirit.rs`, from `sources/models/storm_spirit.bbmodel`), full-bright (`emissive`) with a soft
//! blue light around it (the library's `glow`). It is born where a weather lightning bolt strikes, one time in
//! `BIRTH_CHANCE` (`lightning_strike`, the spirit summoned on the next level tick), and fades once the storm is over.
//! It flies by hand (`flight.rs`, the library's `body`): short jerky dashes, or at whoever hit it, to zap them (the
//! library's `anger`). Lightning and fire don't hurt it; what touches it is zapped. Its sparks call lightning
//! (`spark.rs`).
mod flight;
mod model_storm_spirit;
mod spark;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use creature::bestiary::{Fact, Species};
use mod_api::{
    BlockEntityDrawContext, ClientHost, EmissiveEvent, EntityInteractEvent, EntityRenderState, EntityRenderer,
    EntityStruckByLightningEvent, EntityTickPostEvent, Flow, HitHost, Hooks, HostSide, InteractionResult,
    LevelTickPostEvent, LightningStrikeEvent, LivingDeathEvent, LivingIncomingDamageEvent, ParticleOptions,
    RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost, SoundSource,
};

pub const KIND: &str = "storm_spirits:storm_spirit";
const EGG: &str = "storm_spirits:storm_spirit_spawn_egg";
const TEXTURE: &str = "storm_spirits:entity/storm_spirit/storm_spirit";

/// Its box (blocks) and the scale its model (34 texels tall) is drawn at: about a player's height.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.8,
    height: 1.8,
    eye_height: 1.5,
};
const SCALE: f32 = 0.85;

/// One weather strike in this many gives birth to a spirit (SPECS §2).
const BIRTH_CHANCE: u64 = 10;
/// Out of the storm, it fades after 1 to 3 minutes (ticks).
const FADE: (u64, u64) = (1200, 3600);
/// What touches it is zapped (damage), at most once every `TOUCH_EVERY` ticks.
const TOUCH_DAMAGE: f32 = 2.0;
const TOUCH_EVERY: i32 = 10;
/// A shock (SPECS §2): damage and a moment of Slowness (ticks).
const SHOCK_DAMAGE: f32 = 6.0;
const SHOCK_SLOWNESS: i32 = 30;

/// Its sounds, vanilla's: the crackle of its birth and its fading, its zap.
const CRACKLE: &str = "minecraft:entity.firework_rocket.twinkle";
const ZAP: &str = "minecraft:entity.lightning_bolt.impact";

pub const BORN_FACT: &str = "born";
pub const SHOCK_FACT: &str = "shock";
pub const BOLT_FACT: &str = "bolt";

/// Its page in the bestiary: met in any biome of the overworld, where storms are.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: &["@minecraft:overworld"],
    wild: |_| true,
    facts: &[
        Fact {
            id: BORN_FACT,
            item: "minecraft:lightning_rod",
            key: "bestiary.storm_spirits.born",
        },
        Fact {
            id: SHOCK_FACT,
            item: spark::SPARK,
            key: "bestiary.storm_spirits.shock",
        },
        Fact {
            id: BOLT_FACT,
            item: spark::SPARK,
            key: "bestiary.storm_spirits.bolt",
        },
    ],
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::VeryRare,
    // Never in a jar (SPECS §5): too big for it.
    draw: |_, _| {},
    draw_scale: 1.0,
    draw_lift: 0.0,
};

/// Its item in the guide's Items tab (common/SPECS.md §7.2).
static SPARK_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: spark::SPARK,
    dropped_by: &[KIND],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:lightning_rod",
        key: "bestiary.storm_spirits.item.spark.use",
    }],
};

/// The pale blue light around it (seen only, it lights nothing in the world).
static LIGHT: creature::glow::Light = creature::glow::Light {
    rgb: 0x9EC8FF,
    radius: 6.0,
    intensity: 0.9,
    lift: 1.0,
    shown: |_| true,
};

/// Weather strikes that give birth to a spirit, summoned on the next level tick (`level_tick_post`).
static BIRTHS: Mutex<Vec<[f64; 3]>> = Mutex::new(Vec::new());
/// The ticks each spirit has left once the storm is over.
static FADING: Mutex<BTreeMap<u32, u64>> = Mutex::new(BTreeMap::new());
/// Who touched each spirit last and how long until it zaps them again.
static TOUCHED: Mutex<BTreeMap<(u32, u32), i32>> = Mutex::new(BTreeMap::new());
/// Shocks its behaviours made (spirit, target), dealt by the tick hook (a behaviour can't hurt).
static SHOCKS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

/// From a behaviour: `spirit` shocks `target`.
pub(crate) fn shock(spirit: u32, target: u32) {
    SHOCKS.lock().unwrap().push((spirit, target));
}

/// Bolts called down on a target: sparks crackle over its head for `BOLT_WARNING` ticks, then the lightning strikes
/// where it stands (spirit, target, ticks left).
static BOLTS: Mutex<Vec<(u32, u32, i32)>> = Mutex::new(Vec::new());
const BOLT_WARNING: i32 = 20;

/// From a behaviour: `spirit` calls a lightning bolt down on `target`.
pub(crate) fn call_bolt(spirit: u32, target: u32) {
    let mut bolts = BOLTS.lock().unwrap();
    if !bolts.iter().any(|b| b.0 == spirit) {
        bolts.push((spirit, target, BOLT_WARNING));
    }
}

/// From `entity_tick_post`: `spirit`'s bolt on its way, its warning sparks, then the strike (a mod's bolt: it never
/// gives birth to a spirit, and the spirit is immune to it).
fn bolts(host: &mut dyn ServerHost, dimension: &str, spirit: u32) {
    let due: Vec<(u32, i32)> = {
        let mut all = BOLTS.lock().unwrap();
        for b in all.iter_mut().filter(|b| b.0 == spirit) {
            b.2 -= 1;
        }
        let due = all.iter().filter(|b| b.0 == spirit).map(|b| (b.1, b.2)).collect();
        all.retain(|b| b.0 != spirit || b.2 > 0);
        due
    };
    for (target, left) in due {
        let Some(at) = host.entity(target).and_then(|e| e.position(host)) else {
            continue;
        };
        if left > 0 {
            if left % 4 == 0 {
                sparks(host, dimension, [at[0], at[1] + 2.6, at[2]], 3);
            }
            continue;
        }
        if let Some(level) = host.level(dimension) {
            level.strike_lightning(host, at, false);
        }
    }
}

/// A few random numbers, for its birth and its fading.
fn roll(seed: u64) -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as u64);
    creature::colours::Random::new((seed ^ nanos) as u32).next()
}

/// Blue-white sparks at `at`.
fn sparks(host: &mut dyn ServerHost, dimension: &str, at: [f64; 3], count: i32) {
    let Some(level) = host.level(dimension) else {
        return;
    };
    if let Some(k) = creature::particles::key("minecraft:electric_spark") {
        level.send_particles(host, &ParticleOptions::simple(k), None, at, [0.4, 0.6, 0.4], 0.1, count);
    }
}

/// A rough sound at `at`, heard by everyone near.
fn sound(host: &mut dyn ServerHost, dimension: &str, at: [f64; 3], id: &str, volume: f32, pitch: f32) {
    if let Some(level) = host.level(dimension) {
        level.play_sound(host, id, at, SoundSource::Neutral, volume, pitch, None);
    }
}

/// The player whose entity is `entity`, by UUID.
fn player_of(host: &dyn ServerHost, entity: u32) -> Option<u128> {
    host.players()
        .server_players()
        .into_iter()
        .find(|p| p.entity().id() == entity)
        .map(|p| p.uuid())
}

/// `spirit` zaps `target`: damage from it, sparks and a zap; a shock also slows a player and teaches them the fact.
fn zap(host: &mut dyn ServerHost, dimension: &str, spirit: u32, target: u32, damage: f32, shock: bool) {
    let (Some(me), Some(t)) = (host.entity(spirit), host.entity(target)) else {
        return;
    };
    let Some(at) = t.position(host) else {
        return;
    };
    t.hurt(host, "minecraft:mob_attack", damage, Some(me), Some(me));
    sparks(host, dimension, [at[0], at[1] + 1.0, at[2]], if shock { 12 } else { 4 });
    sound(host, dimension, at, ZAP, if shock { 0.5 } else { 0.2 }, 1.9);
    if shock {
        creature::mob::give_effect(host, target, "minecraft:slowness", SHOCK_SLOWNESS);
        if let Some(player) = player_of(host, target) {
            creature::bestiary::learn(host, player, KIND, SHOCK_FACT);
        }
    }
}

/// From `entity_tick_post`, for each spirit: its shocks dealt, what touches it zapped, its light, and its fading once
/// the storm is over.
fn tick(host: &mut dyn ServerHost, dimension: &str, spirit: u32) {
    let mine: Vec<u32> = {
        let mut all = SHOCKS.lock().unwrap();
        let (mine, rest): (Vec<_>, Vec<_>) = all.drain(..).partition(|s| s.0 == spirit);
        *all = rest;
        mine.into_iter().map(|s| s.1).collect()
    };
    for target in mine {
        zap(host, dimension, spirit, target, SHOCK_DAMAGE, true);
    }
    bolts(host, dimension, spirit);
    let Some(e) = host.entity(spirit) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    // Fire never stays on it.
    if e.fire_ticks(host).unwrap_or(0) > 0 {
        e.clear_fire(host);
    }
    let middle = [at[0], at[1] + 0.9, at[2]];
    for target in creature::projectile::hits(&*host, level, middle, KIND, spirit, 0.6) {
        let ready = {
            let mut touched = TOUCHED.lock().unwrap();
            let wait = touched.entry((spirit, target)).or_insert(0);
            *wait -= 1;
            if *wait <= 0 {
                *wait = TOUCH_EVERY;
                true
            } else {
                false
            }
        };
        if ready {
            zap(host, dimension, spirit, target, TOUCH_DAMAGE, false);
        }
    }
    creature::glow::shine(&*host, dimension, spirit, LIGHT);
    if level.is_thundering(&*host).unwrap_or(false) {
        FADING.lock().unwrap().remove(&spirit);
        return;
    }
    let left = {
        let mut fading = FADING.lock().unwrap();
        let left = fading
            .entry(spirit)
            .or_insert_with(|| FADE.0 + roll(u64::from(spirit)) % (FADE.1 - FADE.0));
        *left = left.saturating_sub(1);
        *left
    };
    if left == 0 {
        // A last spark, no drops.
        sparks(host, dimension, middle, 20);
        sound(host, dimension, middle, CRACKLE, 0.8, 1.2);
        e.discard(host);
        forget(spirit);
    }
}

/// Forget a gone spirit in every static that holds it.
fn forget(spirit: u32) {
    creature::body::forget(spirit);
    creature::glow::dim(spirit);
    FADING.lock().unwrap().remove(&spirit);
    TOUCHED.lock().unwrap().retain(|k, _| k.0 != spirit && k.1 != spirit);
    SHOCKS.lock().unwrap().retain(|s| s.0 != spirit);
    BOLTS.lock().unwrap().retain(|b| b.0 != spirit);
}

/// From the overworld's `level_tick_post`: the spirits born of last tick's weather strikes.
fn births(host: &mut dyn ServerHost) {
    let born: Vec<[f64; 3]> = std::mem::take(&mut *BIRTHS.lock().unwrap());
    let dimension = "minecraft:overworld";
    for at in born {
        if host.summon(dimension, KIND, at, mod_api::Compound::new()).is_none() {
            continue;
        }
        let middle = [at[0], at[1] + 0.9, at[2]];
        sparks(host, dimension, middle, 30);
        sound(host, dimension, middle, CRACKLE, 1.0, 0.8);
        for player in creature::bestiary::players_near(&*host, dimension, at) {
            creature::bestiary::learn(host, player, KIND, BORN_FACT);
        }
    }
}

/// How it holds itself this frame: its arms swaying, its tail of arcs flickering side to side.
fn pose(name: &str, age: f32) -> [f32; 3] {
    let sway = (age * 0.15).sin();
    let flick = |phase: f32| 14.0 * (age * 0.5 + phase).sin();
    match name {
        "arm_left" => [10.0 * sway, 0.0, -25.0 - 8.0 * sway],
        "arm_right" => [-10.0 * sway, 0.0, 25.0 + 8.0 * sway],
        "forearm_left" => [-20.0, 0.0, -10.0],
        "forearm_right" => [-20.0, 0.0, 10.0],
        "head" => [0.0, 6.0 * (age * 0.07).sin(), 0.0],
        "tail" => [8.0, 0.0, flick(0.0)],
        "tail_mid" => [8.0, 0.0, flick(1.2)],
        "tail_tip" => [10.0, 0.0, flick(2.4)],
        // The sparks circle it.
        "sparks" => [0.0, age * 6.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Whether part `name` shows this frame: each spark flickers on and off on its own.
fn shown(name: &str, age: f32, id: u32) -> bool {
    if !name.starts_with("spark_") {
        return true;
    }
    let seed = name
        .bytes()
        .fold(id, |h, b| h.wrapping_mul(31).wrapping_add(u32::from(b)));
    let frame = (age / 2.0) as u32;
    !creature::colours::Random::new(seed ^ frame.wrapping_mul(2_654_435_761))
        .next()
        .is_multiple_of(3)
}

/// Its own model, full-bright.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let pose = |part: &str| pose(part, entity.age);
        let shown = |part: &str| shown(part, entity.age, entity.id);
        creature::model::draw_shown(
            draw,
            &model_storm_spirit::STORM_SPIRIT,
            TEXTURE,
            &pose,
            &shown,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct StormSpirits;

#[mod_api::sunrise_mod]
impl Hooks for StormSpirits {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(KIND, SIZE);
        creature::anger::register(KIND);
        let spark = r.item_key(spark::SPARK);
        r.item_part(spark, spark::Spark);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&SPARK_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Right-click: nothing (no trading screen, which its template would open), on both sides.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == KIND).then_some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != KIND {
                return;
            }
            creature::body::tick(host, event.entity, || {
                vec![Box::new(flight::Shock::default()), Box::new(flight::Dash::default())]
            });
            creature::sounds::play_queued(host, event.entity, SoundSource::Neutral);
            tick(host, event.dimension, event.entity);
        });
    }

    /// A weather bolt gives birth to a spirit, one time in `BIRTH_CHANCE`; a trident's, a command's or a spark's never
    /// does (no farming them).
    fn lightning_strike(&self, _: &mut dyn ServerHost, event: &mut LightningStrikeEvent<'_>) -> Flow {
        let p = event.pos;
        let seed = (p[0] as u64).wrapping_mul(73_856_093) ^ (p[2] as u64).wrapping_mul(19_349_663);
        if event.cause == "weather" && roll(seed).is_multiple_of(BIRTH_CHANCE) {
            BIRTHS
                .lock()
                .unwrap()
                .push([f64::from(p[0]) + 0.5, f64::from(p[1]) + 1.0, f64::from(p[2]) + 0.5]);
        }
        Flow::Continue
    }

    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        if let HostSide::Server(host) = host
            && event.level.dimension() == mod_api::DimensionKey::OVERWORLD
        {
            births(host);
        }
    }

    /// Lightning doesn't touch it: it is lightning.
    fn entity_struck_by_lightning(&self, host: &mut dyn ServerHost, event: &mut EntityStruckByLightningEvent) -> Flow {
        let kind = event.entity.kind(&*host).map(|k| k.id(&*host)).unwrap_or_default();
        if kind == KIND { Flow::Cancel } else { Flow::Continue }
    }

    /// Nor does fire.
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        let burning = matches!(
            event.source.damage_type,
            "minecraft:in_fire"
                | "minecraft:on_fire"
                | "minecraft:campfire"
                | "minecraft:lava"
                | "minecraft:hot_floor"
                | "minecraft:lightning_bolt"
        );
        if event.kind == KIND && burning {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            forget(event.entity);
        }
        Flow::Continue
    }

    /// It glows: its whole texture is full-bright.
    fn emissive(&self, _: &dyn ClientHost, event: &mut EmissiveEvent<'_>) {
        event.layers.push((TEXTURE, TEXTURE));
    }
}
