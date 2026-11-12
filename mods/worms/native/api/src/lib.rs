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
use horizon_client::mobs::goals;
use horizon_client::mod_api::{
    BlockEntityDraw, Compound, Cx, EntityTickPostEvent, Flow, Goal, Hooks, Host, InteractEntityEvent,
    LevelTickPostEvent, LivingDeathEvent, Mob, MobSubmitEvent, ParticleOptions, ParticleType, RandomSource,
    RegisterGoalsEvent,
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
const WANDER: f64 = 3.0;
/// How often a settled worm checks its garden (ticks).
const CHECK_EVERY: i32 = 100;

/// Crawling about: a few blocks around where it first crawled, or over its garden's farmland when settled.
#[derive(Clone, Debug, Default)]
struct Crawl {
    target: Option<[f64; 3]>,
    ticks: i32,
    pausing: bool,
    home: Option<[f64; 3]>,
    garden: Vec<[i32; 3]>,
    check: i32,
}

fn between(m: &mut Mob, range: (i32, i32)) -> i32 {
    range.0 + m.random.next_int_bound(range.1 - range.0).unwrap_or(0)
}

impl Crawl {
    fn pick(&self, m: &mut Mob) -> [f64; 3] {
        if !self.garden.is_empty() {
            let f = self.garden[m.random.next_int_bound(self.garden.len() as i32).unwrap_or(0) as usize];
            return [f64::from(f[0]) + 0.5, f64::from(f[1]) + 1.0, f64::from(f[2]) + 0.5];
        }
        let home = self.home.unwrap_or([m.position.x, m.position.y, m.position.z]);
        let r = |m: &mut Mob| (f64::from(m.random.next_float()) * 2.0 - 1.0) * WANDER;
        [home[0] + r(m), m.position.y, home[2] + r(m)]
    }
}

impl Goal for Crawl {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        m.fall_distance = 0.0;
        if self.home.is_none() {
            self.home = Some([m.position.x, m.position.y, m.position.z]);
        }
        self.check -= 1;
        if self.check <= 0 {
            self.check = CHECK_EVERY;
            self.garden = territory::check_garden(cx, m.id);
        }
        self.ticks -= 1;
        if self.pausing {
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            if self.ticks <= 0 {
                self.pausing = false;
                self.ticks = between(m, CRAWL);
                self.target = Some(self.pick(m));
            }
            return;
        }
        let target = match self.target {
            Some(t) => t,
            None => {
                let t = self.pick(m);
                self.target = Some(t);
                t
            }
        };
        let (dx, dz) = (target[0] - m.position.x, target[2] - m.position.z);
        let len = dx.hypot(dz);
        if len < 0.2 || self.ticks <= 0 {
            self.pausing = true;
            self.ticks = between(m, PAUSE);
            self.target = None;
            return;
        }
        m.velocity.x = dx / len * SPEED;
        m.velocity.z = dz / len * SPEED;
        let yaw = (-dx).atan2(dz).to_degrees() as f32;
        m.yaw = yaw;
        m.body_yaw = yaw;
        m.head_yaw = yaw;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
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
        if random.next() % RAIN_CHANCE != 0 {
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
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id != WORM {
            return;
        }
        creature::mob::ours(event.mob, SIZE.0, SIZE.1, SIZE.2);
        event.mob.ai.goals.add(1, Crawl::default());
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
        if event.kind == WORM {
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
