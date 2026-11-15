//! Worms (SPECS.md): earthworms that make a garden grow, of the Horizon Creature series.
//!
//! A worm is a copy of the wandering trader (its behaviour is all goals) with a tiny box, crawling slowly (`Crawl`) and
//! drawn with its own segmented model (`model_worm.rs`, from `assets/models/worm.bbmodel`), wiggling as it goes. Caught
//! in the library's glass jar, released on farmland (or a crop on it) of a garden of at least 10 connected farmland
//! blocks, it settles there (the library's garden rule) and crawls only on that farmland; every `GROW_EVERY` ticks it
//! makes one crop of its garden grow a stage, with green sparkles on it, at most `MAX_WORMS` worms counting per garden
//! (SPECS §4). While it rains, worms come up near players (`rain`). Wild ones despawn far from players.
mod model_worm;

use std::collections::BTreeMap;
use std::sync::Mutex;

use creature::bestiary::{Fact, Species};
use creature::jar::Catchable;
use creature::territory::{self, Rule};
use horizon_client::mod_api::{
    BlockEntityDraw, Compound, EntityTickPostEvent, Flow, Hooks, Host, InteractEntityEvent, LevelTickPostEvent,
    LivingDeathEvent, Mob, MobSubmitEvent, ParticleOptions, ParticleType, RegisterGoalsEvent,
};

pub const WORM: &str = "worms:worm";
const TEXTURE: &str = "worms:entity/worm/worm";

/// Its box (blocks) and the scale its model is drawn at.
const SIZE: (f64, f64, f64) = (0.3, 0.15, 0.1);
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
    biomes: &[
        "minecraft:plains",
        "minecraft:sunflower_plains",
        "minecraft:meadow",
        "minecraft:forest",
        "minecraft:birch_forest",
        "minecraft:flower_forest",
        "minecraft:taiga",
        "minecraft:dark_forest",
    ],
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
    draw: book_view,
};

/// The bestiary's 3D view: curled a little, about a block across.
fn book_view(host: &dyn Host, draw: &mut BlockEntityDraw, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.05, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    creature::model::draw(host, draw, &model_worm::WORM, TEXTURE, &|part| wiggle(part, 0.0, 1.0));
    draw.pop();
}
const BOOK_SCALE: f32 = 1.5;

/// A segment's turn (degrees): a wave along the body, moving with `phase`, `amount` as wide (0 still, 1 crawling).
fn wiggle(part: &str, phase: f32, amount: f32) -> [f32; 3] {
    let Some(i) = part.strip_prefix("segment_").and_then(|n| n.parse::<f32>().ok()) else {
        return [0.0; 3];
    };
    [0.0, (phase + i * 0.9).sin() * 14.0 * amount, 0.0]
}

/// Crawling speed (blocks a tick), how long a crawl and a pause last (ticks), how far a wild one wanders from where it
/// first crawled.
const SPEED: f64 = 0.015;
const CRAWL: (i32, i32) = (60, 200);
const PAUSE: (i32, i32) = (40, 240);
/// One pause in this many, it noses into the earth with a soft rustle.
const DIG_CHANCE: u32 = 3;

/// Its sounds (`assets/sounds.toml`): a dig now and then, hurt and death.
const DIG: &str = "worms:entity.worm.dig";
const VOICE: creature::sounds::Voice = creature::sounds::Voice {
    hurt: "worms:entity.worm.hurt",
    death: "worms:entity.worm.death",
};
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
};

/// At a pause, one time in `DIG_CHANCE` it noses into the earth with a soft rustle.
fn dig(m: &mut Mob) {
    creature::crawl::rustle(m, DIG_CHANCE, DIG);
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
fn tend(host: &mut dyn Host, entity: u32) {
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
    let crops: Vec<([i32; 3], u32, u32)> = garden
        .iter()
        .filter_map(|f| {
            let crop = [f[0], f[1] + 1, f[2]];
            let grown = grown_age(&host.block(crop))?;
            let age: u32 = host.block_property(crop, "age")?.parse().ok()?;
            (age < grown).then_some((crop, age, grown))
        })
        .collect();
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
        let sparkle = ParticleOptions::simple(ParticleType::HappyVillager);
        host.send_particles(&sparkle, None, at, [0.3, 0.25, 0.3], 0.0, 6);
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

/// Where each worm is, from its tick (for the rain's cap).
static WORMS_AT: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());
static RAIN_TICKS: Mutex<u32> = Mutex::new(0);

/// From `level_tick_post`, in the overworld: worms come up near players while it rains.
fn rain(host: &mut dyn Host, dimension: &str) {
    if dimension != "minecraft:overworld" {
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
    if host.weather().0 < 0.2 {
        return;
    }
    let mut random = creature::colours::Random::new(RAIN_EVERY);
    for player in host.server_players().into_iter().filter(|p| p.dimension == dimension) {
        if !random.next().is_multiple_of(RAIN_CHANCE) {
            continue;
        }
        let p = player.position;
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
        let top = (p[1] as i32 - 8..=p[1] as i32 + 8)
            .rev()
            .find(|&y| SOIL.contains(&host.block([x, y, z]).as_str()) && host.block([x, y + 1, z]) == "minecraft:air");
        let Some(y) = top else {
            continue;
        };
        let lives_here = host
            .biome([x, y + 1, z])
            .is_some_and(|b| BESTIARY.biomes.contains(&b.as_str()));
        if lives_here {
            let at = [f64::from(x) + 0.5, f64::from(y + 1), f64::from(z) + 0.5];
            host.summon(dimension, WORM, at, Compound::new());
        }
    }
}

pub fn hooks() -> Box<dyn Hooks> {
    creature::jar::register(&IN_JAR);
    creature::bestiary::register(&BESTIARY);
    Box::new(Worms)
}

struct Worms;

impl Hooks for Worms {
    /// Its row in Options > Mods, which switches it off and on at once (`creature::list_mod`).
    fn mods(&self, _: &mut dyn Host, event: &mut horizon_client::mod_api::ModsEvent<'_>) {
        creature::list_mod(
            event,
            "worms",
            concat!("Worms ", env!("CARGO_PKG_VERSION")),
            "Earthworms that make your garden grow. Needs Horizon Creature.",
        );
    }

    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id != WORM {
            return;
        }
        creature::mob::ours(event.mob, SIZE.0, SIZE.1, SIZE.2);
        event.mob.ai.goals.add(1, creature::crawl::Crawl::new(&CRAWL_CFG));
    }

    /// Right-click: nothing (no trading screen, which its template would open).
    fn interact_entity(&self, _: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if event.kind == WORM {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn level_tick_post(&self, host: &mut dyn Host, event: &mut LevelTickPostEvent<'_>) {
        rain(host, event.dimension);
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind != WORM {
            return;
        }
        creature::sounds::tick_post(host, event, &VOICE, creature::sounds::NEUTRAL);
        territory::tick(host, event.entity);
        tend(host, event.entity);
        if let Some(at) = host.entity(event.entity).and_then(|e| host.entity_position(e)) {
            WORMS_AT.lock().unwrap().insert(event.entity, at);
        }
        if creature::mob::despawn_far(host, event.dimension, event.entity, territory::settled(event.entity)) {
            territory::forget(event.entity);
            TICKS.lock().unwrap().remove(&event.entity);
            WORMS_AT.lock().unwrap().remove(&event.entity);
        }
    }

    fn living_death(&self, _: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        if creature::mob::kind(event.kind) == WORM {
            territory::forget(event.entity);
            TICKS.lock().unwrap().remove(&event.entity);
            WORMS_AT.lock().unwrap().remove(&event.entity);
        }
        Flow::Continue
    }

    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != WORM {
            return Flow::Continue;
        }
        let amount = (mob.walk_speed * 8.0).clamp(0.15, 1.0);
        event.draw.push();
        creature::model::death_tilt(&mut *event.draw, mob);
        event.draw.scale([SCALE; 3]);
        creature::model::draw_shown(
            host,
            &mut *event.draw,
            &model_worm::WORM,
            TEXTURE,
            &|part| wiggle(part, mob.age * 0.3, amount),
            &|_| true,
            creature::model::tint(mob),
        );
        event.draw.pop();
        Flow::Cancel
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
