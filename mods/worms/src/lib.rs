//! Worms (SPECS.md): earthworms that make a garden grow, of the Sunrise Creatures series.
//!
//! A worm is a copy of the wandering trader (its behaviour is all goals) with a tiny box, crawling slowly (`Crawl`) and
//! drawn with its own segmented model (`model_worm.rs`, from `sources/models/worm.bbmodel`), wiggling as it goes. Caught
//! in the library's glass jar, released on farmland (or a crop on it) of a garden of at least 10 connected farmland
//! blocks, it settles there (the library's garden rule) and crawls only on that farmland; every `GROW_EVERY` ticks it
//! makes one crop of its garden grow a stage, with green sparkles on it, at most `MAX_WORMS` worms counting per garden
//! (SPECS §4). While it rains, worms come up near players (`rain`). Wild ones despawn far from players.
mod model_worm;

use std::collections::BTreeMap;
use std::sync::Mutex;

use std::sync::{Arc, OnceLock};

use creature::bestiary::{Fact, Species};
use creature::jar::Catchable;
use creature::territory::{self, Rule};
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityPart, EntityRenderState, EntityRenderer, EntityTickPostEvent,
    Flow, GoalContext, GoalRegistrar, Hooks, HostSide, InteractionResult, LevelTickPostEvent, LivingDeathEvent,
    ParticleKey, ParticleOptions, RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost,
    SoundSource, SpawnReason,
};

pub const WORM: &str = "worms:worm";
const TEXTURE: &str = "worms:entity/worm/worm";

/// Its box (blocks) and the scale its model is drawn at.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.3,
    height: 0.15,
    eye_height: 0.1,
};
const SCALE: f32 = 0.6;

/// Caught in the library's glass jar, settling in a garden (SPECS §3).
static IN_JAR: Catchable = Catchable {
    kind: WORM,
    model: "worms:worm_jar",
    settles: Some(Rule::Garden),
    glows: false,
};

static BESTIARY: Species = Species {
    id: WORM,
    biomes: BIOMES,
    wild: |id| !territory::settled(id),
    facts: &[
        Fact {
            id: creature::jar::JAR_FACT,
            item: creature::jar::ITEM,
            key: "bestiary.worms.jar",
        },
        Fact {
            id: creature::jar::GARDEN_FACT,
            item: "minecraft:wooden_hoe",
            key: "bestiary.worms.garden",
        },
    ],
    icon: "worms:worm_spawn_egg",
    scheme: None,
    rarity: creature::bestiary::Rarity::Common,
    draw: jar_view,
};

/// Drawn in a placed jar: curled a little, about a block across.
fn jar_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.05, 0.5]);
    draw.scale([JAR_SCALE; 3]);
    creature::model::draw(draw, &model_worm::WORM, TEXTURE, &|part| wiggle(part, 0.0, 1.0));
    draw.pop();
}
const JAR_SCALE: f32 = 1.5;

/// The biomes it lives in (its spawns, and where rain brings it up).
const BIOMES: &[&str] = &[
    "minecraft:plains",
    "minecraft:sunflower_plains",
    "minecraft:meadow",
    "minecraft:forest",
    "minecraft:birch_forest",
    "minecraft:flower_forest",
    "minecraft:taiga",
    "minecraft:dark_forest",
];

/// A segment's turn (degrees): a wave along the body, moving with `phase`, `amount` as wide (0 still, 1 crawling).
fn wiggle(part: &str, phase: f32, amount: f32) -> [f32; 3] {
    let Some(i) = part.strip_prefix("segment_").and_then(|n| n.parse::<f32>().ok()) else {
        return [0.0; 3];
    };
    [0.0, (phase + i * 0.9).sin() * 14.0 * amount, 0.0]
}

/// Crawling speed (the multiplier of its movement speed, 0.1 in its entity type: a slow crawl), how long a crawl and a pause last (ticks), how far a wild one wanders from where it
/// first crawled.
const SPEED: f64 = 1.0;
const CRAWL: (i32, i32) = (60, 200);
const PAUSE: (i32, i32) = (40, 240);
/// One pause in this many, it noses into the earth with a soft rustle.
const DIG_CHANCE: u32 = 3;

/// Its dig now and then (`sources/sounds.toml`); its hurt and death sounds are its entity type's.
const DIG: &str = "worms:entity.worm.dig";
const WANDER: f64 = 3.0;
/// How often a settled worm checks its garden (ticks).
const CHECK_EVERY: i32 = 100;

/// Crawling about (the library's small crawlers): a few blocks around where it first crawled, or over its garden's
/// farmland when settled, digging into the earth now and then at a pause.
const CRAWL_CFG: creature::crawl::Cfg = creature::crawl::Cfg {
    speed: SPEED,
    crawl: CRAWL,
    pause: PAUSE,
    wander: WANDER,
    check_every: CHECK_EVERY,
    settled_area: territory::check_garden,
    on_pause: Some(dig),
    busy: None,
};

/// At a pause, one time in `DIG_CHANCE` it noses into the earth with a soft rustle.
fn dig(cx: &mut GoalContext<'_>) {
    creature::crawl::rustle(cx, DIG_CHANCE, DIG);
}

/// A settled worm makes a crop of its garden grow a stage this often (ticks); at most `MAX_WORMS` worms count in one
/// garden (SPECS §4: a slow bonus, capped).
const GROW_EVERY: u32 = 400;
const MAX_WORMS: usize = 3;

/// Ticks since each worm last helped a crop.
static TICKS: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

/// The age a crop is grown at, by block (those with an `age`); others are left alone.
fn grown_age(block: &str) -> Option<u32> {
    match block {
        "minecraft:wheat"
        | "minecraft:carrots"
        | "minecraft:potatoes"
        | "minecraft:melon_stem"
        | "minecraft:pumpkin_stem" => Some(7),
        "minecraft:beetroots" | "minecraft:nether_wart" => Some(3),
        "minecraft:torchflower_crop" => Some(1),
        "minecraft:pitcher_crop" => Some(4),
        _ => None,
    }
}

/// From `entity_tick_post`: now and then, a settled worm makes one crop of its garden grow a stage.
fn tend(host: &mut dyn ServerHost, entity: u32) {
    if !territory::settled(entity) {
        return;
    }
    {
        let mut ticks = TICKS.lock().unwrap();
        let t = ticks.entry(entity).or_insert(entity % GROW_EVERY);
        *t += 1;
        if *t < GROW_EVERY {
            return;
        }
        *t = 0;
    }
    let garden = territory::garden_of(host, entity);
    if garden.is_empty() {
        return;
    }
    let mut random = creature::colours::Random::new(entity);
    // More than `MAX_WORMS` worms in this garden: each helps less, so that together they count as `MAX_WORMS`.
    let worms = TICKS
        .lock()
        .unwrap()
        .keys()
        .filter(|w| territory::anchor(**w).is_some_and(|a| garden.contains(&a)))
        .count()
        .max(1);
    if worms > MAX_WORMS && (random.next() % worms as u64) as usize >= MAX_WORMS {
        return;
    }
    let Some(ground) = territory::Ground::of_entity(&*host, entity) else {
        return;
    };
    let crops: Vec<([i32; 3], u32, u32)> = garden
        .iter()
        .filter_map(|f| {
            let crop = [f[0], f[1] + 1, f[2]];
            let grown = grown_age(&ground.block(crop))?;
            let age: u32 = ground.level.block_property(ground.host, crop, "age")?.parse().ok()?;
            (age < grown).then_some((crop, age, grown))
        })
        .collect();
    let level = ground.level;
    if crops.is_empty() {
        return;
    }
    let (crop, age, _) = crops[(random.next() % crops.len() as u64) as usize];
    if host.set_block_property(crop, "age", &(age + 1).to_string()) {
        // A few green sparkles on the crop it helped, as bone meal shows.
        let at = [
            f64::from(crop[0]) + 0.5,
            f64::from(crop[1]) + 0.4,
            f64::from(crop[2]) + 0.5,
        ];
        if let Some(&key) = SPARKLE.get() {
            level.send_particles(host, &ParticleOptions::simple(key), None, at, [0.3, 0.25, 0.3], 0.0, 6);
        }
    }
}

/// After rain (SPECS §2): while it rains, worms come up on the grass and dirt near players, in the biomes they live in.
/// Every `RAIN_EVERY` ticks, one chance in `RAIN_CHANCE` for each player, while fewer than `RAIN_CAP` worms are within
/// `RAIN_NEAR` blocks of them; it comes up `RAIN_MIN` to `RAIN_MAX` blocks away.
const RAIN_EVERY: u32 = 100;
const RAIN_CHANCE: u64 = 3;
const RAIN_CAP: usize = 6;
const RAIN_NEAR: f64 = 24.0;
const RAIN_MIN: f64 = 4.0;
const RAIN_MAX: f64 = 16.0;
const SOIL: [&str; 5] = [
    "minecraft:grass_block",
    "minecraft:dirt",
    "minecraft:coarse_dirt",
    "minecraft:rooted_dirt",
    "minecraft:podzol",
];

/// The green sparkles of a crop helped (bone meal's), asked for at registration.
static SPARKLE: OnceLock<ParticleKey> = OnceLock::new();

/// Where each worm is, from its tick (for the rain's cap).
static WORMS_AT: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());
static RAIN_TICKS: Mutex<u32> = Mutex::new(0);

/// From `level_tick_post`, in the overworld: worms come up near players while it rains.
fn rain(host: &mut dyn ServerHost, level: mod_api::LevelRef) {
    if level.dimension() != mod_api::DimensionKey::OVERWORLD {
        return;
    }
    {
        let mut t = RAIN_TICKS.lock().unwrap();
        *t += 1;
        if *t < RAIN_EVERY {
            return;
        }
        *t = 0;
    }
    if level.weather(host).is_none_or(|(rain, _)| rain < 0.2) {
        return;
    }
    let mut random = creature::colours::Random::new(RAIN_EVERY);
    let players = host.players().server_players();
    for player in players {
        if player.level(host) != Some(level) || !random.next().is_multiple_of(RAIN_CHANCE) {
            continue;
        }
        let Some(p) = player.entity().position(host) else {
            continue;
        };
        let ground = territory::Ground::new(&*host, level);
        let near = WORMS_AT
            .lock()
            .unwrap()
            .values()
            .filter(|w| (w[0] - p[0]).hypot(w[2] - p[2]) < RAIN_NEAR)
            .count();
        if near >= RAIN_CAP {
            continue;
        }
        let angle = (random.next() % 360) as f64;
        let distance = RAIN_MIN + (random.next() % 1000) as f64 / 1000.0 * (RAIN_MAX - RAIN_MIN);
        let (x, z) = (
            (p[0] + angle.to_radians().cos() * distance).floor() as i32,
            (p[2] + angle.to_radians().sin() * distance).floor() as i32,
        );
        // The ground there: the first soil block under air, from a little above the player down.
        let top = (p[1] as i32 - 8..=p[1] as i32 + 8).rev().find(|&y| {
            SOIL.contains(&ground.block([x, y, z]).as_str()) && ground.block([x, y + 1, z]) == "minecraft:air"
        });
        let Some(y) = top else {
            continue;
        };
        let lives_here = level
            .biome_at(host, [x, y + 1, z])
            .is_some_and(|b| BIOMES.contains(&b.as_str()));
        if lives_here {
            let at = [f64::from(x) + 0.5, f64::from(y + 1), f64::from(z) + 0.5];
            level.spawn_mob(host, WORM, at, SpawnReason::Natural);
        }
    }
}

/// Its goals, added to each worm as it is built (its template's are taken off by the library as it joins).
struct Goals;

#[mod_api::sunrise_part]
impl EntityPart for Goals {
    fn register_goals(&self, goals: &mut GoalRegistrar<'_>) {
        goals.add_goal(1, creature::crawl::Crawl::new(&CRAWL_CFG));
    }
}

/// Its own segmented model, wiggling as it crawls.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let amount = mob.map_or(1.0, |m| (m.walk_animation_speed * 8.0).clamp(0.15, 1.0));
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        creature::model::draw_shown(
            draw,
            &model_worm::WORM,
            TEXTURE,
            &|part| wiggle(part, entity.age * 0.3, amount),
            &|_| true,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

/// A worm forgotten (dead, despawned).
fn forget(entity: u32) {
    territory::forget(entity);
    TICKS.lock().unwrap().remove(&entity);
    WORMS_AT.lock().unwrap().remove(&entity);
}

#[derive(Default)]
struct Worms;

#[mod_api::sunrise_mod]
impl Hooks for Worms {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(WORM, SIZE);
        creature::jar::register(&IN_JAR);
        creature::bestiary::register(&BESTIARY);
        r.entity_part(WORM, Goals);
        let _ = SPARKLE.set(r.particle_key("minecraft:happy_villager"));
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(WORM.into(), Arc::new(Draw));
    }

    /// Right-click: nothing (no trading screen, which its template would open), on both sides. The library's jar
    /// answers first.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == WORM).then_some(InteractionResult::Consume)
    }

    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        if let HostSide::Server(host) = host {
            rain(host, event.level);
        }
    }

    fn entity_tick_post(&self, host: &mut dyn ServerHost, event: &mut EntityTickPostEvent<'_>) {
        if event.kind != WORM {
            return;
        }
        creature::sounds::play_queued(host, event.entity, SoundSource::Neutral);
        territory::tick(host, event.entity);
        tend(host, event.entity);
        if let Some(at) = host.entity(event.entity).and_then(|e| e.position(host)) {
            WORMS_AT.lock().unwrap().insert(event.entity, at);
        }
        if creature::mob::despawn_far(host, event.dimension, event.entity, territory::settled(event.entity)) {
            forget(event.entity);
        }
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == WORM {
            forget(event.entity);
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crops_grow_to_their_last_age_and_other_blocks_not_at_all() {
        assert_eq!(grown_age("minecraft:wheat"), Some(7));
        assert_eq!(grown_age("minecraft:beetroots"), Some(3));
        assert_eq!(grown_age("minecraft:torchflower_crop"), Some(1));
        assert_eq!(grown_age("minecraft:grass_block"), None);
    }
}
