//! Cyclops (SPECS.md): a one-eyed giant shepherd at the mouths of caves, of the Horizon Creature series.
//!
//! A cyclops is a copy of the wandering trader (its behaviour is all goals) with its own 4-block box, drawn with its
//! own model (`model.rs`, from `assets/models/cyclops.bbmodel`). Hostile: it goes for a player it sees within
//! `SIGHT` blocks, and for whoever hurts it or one of its sheep, with a slow overhead blow of its club, or out of
//! reach a boulder hurled over its head (`Smash`, `boulder.rs`). Its eye is its weak spot (`eye.rs`). It appears rarely
//! at a cave mouth near a player (`spawn`), with a flock of sheep that follow it (`flock`). It drops its eye when a player kills it, and
//! sometimes its club; the eye makes a shepherd's crook, which herds farm animals (`crook.rs`).
mod boulder;
mod crook;
mod eye;
mod model;
mod model_boulder;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::time::Instant;

use creature::bestiary::{Fact, Species};
use horizon_client::mobs::Who;
use horizon_client::mobs::goals::{self, HurtByTarget, LookAtPlayer, NearestTarget, RandomLookAround, Stroll};
use horizon_client::mod_api::{
    AnvilUpdateEvent, BlockEntityDraw, Compound, Cx, EntityTickPostEvent, Flow, Goal, Hooks, Host, InteractEntityEvent,
    LevelTickPostEvent, LivingDeathEvent, LivingIncomingDamageEvent, Mob, MobSubmitEvent, NativeHeightmap,
    NativeMobRender, RegisterGoalsEvent,
};

pub const CYCLOPS: &str = "cyclops:cyclops";
const CLUB: &str = "cyclops:club";
const TEXTURE: &str = "cyclops:entity/cyclops/cyclops";

/// Its box (blocks: width, height, eye height), SPECS §2.
const SIZE: (f64, f64, f64) = (1.8, 4.0, 3.5);
/// Its blow (SPECS §2): damage, the knockback it adds, and how little it is pushed itself.
const ATTACK_DAMAGE: f32 = 12.0;
const ATTACK_KNOCKBACK: f64 = 2.0;
const KNOCKBACK_RESISTANCE: f64 = 0.6;
/// How far it sees a player to go for (blocks; its follow range).
const SIGHT: f64 = 16.0;

/// The biomes it appears in (at a cave mouth, SPECS §2): the mountain pastures and windswept heights where a
/// shepherd grazes a flock, and the rocky shores, as Polyphemus's cave by the sea.
const BIOMES: &[&str] = &[
    "minecraft:meadow",
    "minecraft:cherry_grove",
    "minecraft:grove",
    "minecraft:snowy_slopes",
    "minecraft:stony_peaks",
    "minecraft:windswept_hills",
    "minecraft:windswept_gravelly_hills",
    "minecraft:windswept_forest",
    "minecraft:windswept_savanna",
    "minecraft:savanna_plateau",
    "minecraft:stony_shore",
];

/// Its flock: the sheep that came with it carry this tag (kept with the sheep), and it is a fact in the bestiary.
const FLOCK_TAG: &str = "cyclops.flock";
const FLOCK_FACT: &str = "flock";

static BESTIARY: Species = Species {
    id: CYCLOPS,
    biomes: BIOMES,
    wild: |_| true,
    facts: &[Fact {
        id: FLOCK_FACT,
        item: "minecraft:white_wool",
        key: "bestiary.cyclops.flock",
    }],
    icon: "cyclops:cyclops_spawn_egg",
    scheme: None,
    draw: book_view,
};

/// The bestiary's 3D view: standing, its whole height in the view.
fn book_view(host: &dyn Host, draw: &mut BlockEntityDraw, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.0, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    creature::model::draw(host, draw, &model::CYCLOPS, TEXTURE, &|_| [0.0; 3]);
    draw.pop();
}
const BOOK_SCALE: f32 = 0.22;

/// Its sounds: a ravager's, lower (no sounds of its own yet).
const VOICE: creature::sounds::Voice = creature::sounds::Voice {
    hurt: "minecraft:entity.ravager.hurt",
    death: "minecraft:entity.ravager.death",
};
const AMBIENT: &str = "minecraft:entity.ravager.ambient";
const STEP: &str = "minecraft:entity.ravager.step";
const BLOW: &str = "minecraft:entity.ravager.attack";
/// One chance in this many, a tick it walks, of a step heard.
const STEP_CHANCE: u64 = 14;
/// One chance in this many, a tick, of its grumble.
const AMBIENT_CHANCE: u64 = 500;

/// The club's blow (SPECS §2): it walks up to its target, raises the club over its head for `WINDUP` ticks, then
/// brings it down; hit if the target is still within `REACH`. One blow every `EVERY` ticks. It lets go of a target
/// out of `GIVE_UP` blocks, dead, or a creative player.
const REACH: f64 = 4.0;
const WINDUP: i32 = 14;
const EVERY: i32 = 40;
const CHASE_SPEED: f64 = 1.1;
const GIVE_UP: f64 = 32.0;
/// The boulder (SPECS §2): thrown at a target it sees `THROW_FROM` to `THROW_TO` blocks away, raised over its head for
/// `THROW_WINDUP` ticks first, at most once every `THROW_EVERY` ticks; from just over its head.
const THROW_FROM: f64 = 7.0;
const THROW_TO: f64 = 24.0;
const THROW_WINDUP: i32 = 20;
const THROW_EVERY: i32 = 100;
const THROW_HEIGHT: f64 = 4.05;
/// Where it holds the boulder before the throw: between its raised hands (blocks from its feet).
const HELD_HEIGHT: f32 = 4.4;
const THROW: &str = "minecraft:entity.snowball.throw";

/// When each cyclops raised its club, or a boulder, last, for the drawing (single player; arismoko/horizon#2740).
static RAISED: Mutex<BTreeMap<u32, Instant>> = Mutex::new(BTreeMap::new());
static LIFTED: Mutex<BTreeMap<u32, Instant>> = Mutex::new(BTreeMap::new());

#[derive(Clone, Debug, Default)]
struct Smash {
    windup: i32,
    cooldown: i32,
    repath: i32,
    throwing: i32,
    throw_cooldown: i32,
}

impl Goal for Smash {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        m.target_info(cx.w).is_some_and(|t| t.alive && !t.creative)
    }
    fn can_continue(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        let Some(t) = m.target_info(cx.w) else {
            return false;
        };
        let far = (t.position.x - m.position.x).hypot(t.position.z - m.position.z) > GIVE_UP;
        if far || !t.alive || t.creative {
            m.ai.target = None;
            return false;
        }
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, _: &mut Mob, _: &mut Cx) {
        self.repath = 0;
        self.throw_cooldown = self.throw_cooldown.max(THROW_WINDUP);
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.windup = 0;
        self.throwing = 0;
        m.ai.nav.stop();
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(who) = m.ai.target else {
            return;
        };
        let Some(t) = cx.w.target_info(who) else {
            return;
        };
        let at = [t.position.x, t.eye_y, t.position.z];
        m.ai.look.set_look_at(at, 30.0, 30.0);
        let reach = (t.position.x - m.position.x).hypot(t.position.z - m.position.z);
        self.cooldown -= 1;
        if self.windup > 0 {
            // The club up: it stands, then strikes.
            self.windup -= 1;
            if self.windup == 0 {
                if reach < REACH + 0.5 {
                    m.swing();
                    horizon_client::mobs::ai::do_hurt_target(m, who, cx);
                }
                creature::sounds::queue(m.id, BLOW, 1.2, 0.7);
                self.cooldown = EVERY;
            }
            return;
        }
        if self.throwing > 0 {
            // The boulder up: it stands, then hurls it at where the target is.
            self.throwing -= 1;
            if self.throwing == 0 {
                let from = [m.position.x, m.position.y + THROW_HEIGHT, m.position.z];
                let to = [t.position.x, t.position.y + 0.9, t.position.z];
                let velocity = boulder::aim(from, to);
                boulder::THROWS.lock().unwrap().push(boulder::Throw {
                    from,
                    velocity,
                    target: who,
                });
                m.swing();
                creature::sounds::queue(m.id, THROW, 1.2, 0.4);
                self.throw_cooldown = THROW_EVERY;
            }
            return;
        }
        self.throw_cooldown -= 1;
        if (THROW_FROM..THROW_TO).contains(&reach) && self.throw_cooldown <= 0 && m.sees(cx.w, who) {
            self.throwing = THROW_WINDUP;
            m.ai.nav.stop();
            LIFTED.lock().unwrap().insert(m.id, Instant::now());
            return;
        }
        if reach < REACH && self.cooldown <= 0 {
            self.windup = WINDUP;
            m.ai.nav.stop();
            RAISED.lock().unwrap().insert(m.id, Instant::now());
            return;
        }
        self.repath -= 1;
        if self.repath <= 0 {
            m.nav_move_to(cx.w, [t.position.x, t.position.y, t.position.z], CHASE_SPEED);
            self.repath = 10;
        }
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Whom each cyclops is to turn on, for hurting one of its sheep (from `living_incoming_damage`, which has no host): a
/// mob, or none for the player (Horizon gives a player's hit no attacker id yet).
static ANGERED: Mutex<BTreeMap<u32, Option<u32>>> = Mutex::new(BTreeMap::new());

/// Takes as its target whoever hurt one of its sheep.
#[derive(Clone, Debug)]
struct Shepherd;

impl Goal for Shepherd {
    fn flags(&self) -> u8 {
        goals::TARGET
    }
    fn can_use(&mut self, m: &mut Mob, cx: &mut Cx) -> bool {
        if let Some(attacker) = ANGERED.lock().unwrap().remove(&m.id) {
            m.ai.target = Some(match attacker {
                Some(id) if cx.w.seen(id).is_some() => Who::Mob(id),
                _ => Who::Player,
            });
        }
        false
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Where each cyclops is (from its tick): for the flock and for keeping cyclopes apart.
static CYCLOPS_AT: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());
/// Each sheep seen near a cyclops: whether it is of a flock (its tag, read once).
static FLOCK: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());
/// Each flock sheep's cyclops (the nearest).
static SHEPHERD: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());
/// The players who have noted the flock fact this session (`bestiary::learn` keeps it).
static NOTED: Mutex<BTreeSet<u128>> = Mutex::new(BTreeSet::new());

/// The flock follows: every `FLOCK_EVERY` ticks, a flock sheep within `FLOCK_RANGE` of a cyclops and more than
/// `FOLLOW_FROM` blocks away walks to it. A player within `SEE_FLOCK` of a cyclops with its flock notes it.
const FLOCK_EVERY: u32 = 20;
const FLOCK_RANGE: f64 = 48.0;
const FOLLOW_FROM: f64 = 6.0;
const SEE_FLOCK: f64 = 16.0;
static FLOCK_TICKS: Mutex<u32> = Mutex::new(0);

fn flock(host: &mut dyn Host, dimension: &str) {
    {
        let mut t = FLOCK_TICKS.lock().unwrap();
        *t += 1;
        if *t < FLOCK_EVERY {
            return;
        }
        *t = 0;
    }
    let entities = host.level_entities(dimension);
    let cyclopes: Vec<(u32, [f64; 3])> = entities
        .iter()
        .filter(|(_, kind, _)| kind == CYCLOPS)
        .map(|&(id, _, at)| (id, at))
        .collect();
    if cyclopes.is_empty() {
        return;
    }
    let mut with_flock = BTreeSet::new();
    for (sheep, _, at) in entities.iter().filter(|(_, kind, _)| kind == "minecraft:sheep") {
        let Some((shepherd, distance)) = cyclopes
            .iter()
            .map(|&(id, c)| (id, (c[0] - at[0]).hypot(c[2] - at[2])))
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            continue;
        };
        if distance > FLOCK_RANGE {
            continue;
        }
        let known = FLOCK.lock().unwrap().get(sheep).copied();
        let of_flock = known.unwrap_or_else(|| {
            let tagged = host
                .entity(*sheep)
                .and_then(|e| host.entity_tags(e))
                .is_some_and(|tags| tags.iter().any(|t| t == FLOCK_TAG));
            FLOCK.lock().unwrap().insert(*sheep, tagged);
            tagged
        });
        // Not while a player's crook herds it.
        if !of_flock || crook::HERDED.lock().unwrap().contains(sheep) {
            continue;
        }
        SHEPHERD.lock().unwrap().insert(*sheep, shepherd);
        with_flock.insert(shepherd);
        if distance > FOLLOW_FROM {
            host.mob_follow(dimension, *sheep, shepherd, 1.0);
        }
    }
    for player in host.server_players().into_iter().filter(|p| p.dimension == dimension) {
        let sees = cyclopes.iter().any(|(id, c)| {
            with_flock.contains(id)
                && (0..3).map(|k| (c[k] - player.position[k]).powi(2)).sum::<f64>() < SEE_FLOCK * SEE_FLOCK
        });
        if sees && NOTED.lock().unwrap().insert(player.uuid) {
            creature::bestiary::learn(host, player.uuid, CYCLOPS, FLOCK_FACT);
        }
    }
}

/// Where it appears (SPECS §2): rarely, at a cave mouth near a player. Every `SPAWN_EVERY` ticks, one chance in
/// `SPAWN_CHANCE` for each player with no cyclops within `ALONE` blocks, `SPAWN_TRIES` columns `SPAWN_MIN` to
/// `SPAWN_MAX` blocks away are tried (`cave_mouth`); it comes with `FLOCK_SIZE` sheep.
const SPAWN_EVERY: u32 = 1200;
const SPAWN_CHANCE: u64 = 4;
const ALONE: f64 = 160.0;
const SPAWN_TRIES: usize = 24;
const SPAWN_MIN: f64 = 24.0;
const SPAWN_MAX: f64 = 64.0;
const FLOCK_SIZE: (u64, u64) = (3, 5);
static SPAWN_TICKS: Mutex<u32> = Mutex::new(0);

fn spawn(host: &mut dyn Host, dimension: &str) {
    {
        let mut t = SPAWN_TICKS.lock().unwrap();
        *t += 1;
        if *t < SPAWN_EVERY {
            return;
        }
        *t = 0;
    }
    let mut random = creature::colours::Random::new(SPAWN_EVERY);
    for player in host.server_players().into_iter().filter(|p| p.dimension == dimension) {
        if random.next() % SPAWN_CHANCE != 0 {
            continue;
        }
        let p = player.position;
        let crowded = CYCLOPS_AT
            .lock()
            .unwrap()
            .values()
            .any(|c| (c[0] - p[0]).hypot(c[2] - p[2]) < ALONE);
        if crowded {
            continue;
        }
        for _ in 0..SPAWN_TRIES {
            let angle = ((random.next() % 3600) as f64 / 10.0).to_radians();
            let distance = SPAWN_MIN + (random.next() % 1000) as f64 / 1000.0 * (SPAWN_MAX - SPAWN_MIN);
            let x = (p[0] + angle.cos() * distance).floor() as i32;
            let z = (p[2] + angle.sin() * distance).floor() as i32;
            let Some(at) = cave_mouth(host, dimension, x, z) else {
                continue;
            };
            if !host.biome(at).is_some_and(|b| BIOMES.contains(&b.as_str())) {
                continue;
            }
            let feet = [f64::from(at[0]) + 0.5, f64::from(at[1]), f64::from(at[2]) + 0.5];
            if let Some(id) = host.summon(dimension, CYCLOPS, feet, Compound::new()) {
                CYCLOPS_AT.lock().unwrap().insert(id, feet);
                let count = FLOCK_SIZE.0 + random.next() % (FLOCK_SIZE.1 - FLOCK_SIZE.0 + 1);
                for _ in 0..count {
                    sheep(host, dimension, at, &mut random);
                }
            }
            break;
        }
    }
}

const AIR: [&str; 2] = ["minecraft:air", "minecraft:cave_air"];

/// Room to stand in: air, or what a block placed there would replace (grass, ferns, snow layers), but not liquids.
fn is_open(host: &dyn Host, pos: [i32; 3]) -> bool {
    let block = host.block(pos);
    AIR.contains(&block.as_str())
        || (!block.contains("water")
            && !block.contains("lava")
            && host.block_name_in_tag(&block, "minecraft:replaceable").unwrap_or(false))
}

/// Ground a cyclops or a sheep may stand on: not open, not a liquid or leaves.
fn is_ground(host: &dyn Host, pos: [i32; 3]) -> bool {
    let block = host.block(pos);
    !(is_open(host, pos) || block.contains("water") || block.contains("lava") || block.contains("leaves"))
}

/// A cave mouth at column (`x`, `z`): the open ground there (sky above, 4 blocks of room for the cyclops) at the foot of
/// a covered passage going in, in one of the 4 directions: within 2 blocks of it, at least 4 cells in a row with air
/// (or grass) for 3 blocks, ground under them and the land at least 3 blocks over them. Where the cyclops stands, or none.
fn cave_mouth(host: &dyn Host, dimension: &str, x: i32, z: i32) -> Option<[i32; 3]> {
    const DIRS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    const DEPTH: i32 = 8;
    let mut columns = vec![(x, z)];
    for (dx, dz) in DIRS {
        columns.extend((1..=DEPTH).map(|d| (x + dx * d, z + dz * d)));
    }
    let heights = host
        .surface_heights(dimension, &columns, NativeHeightmap::MotionBlockingNoLeaves)
        .ok()?;
    let y = heights[0]?;
    if !is_ground(host, [x, y - 1, z]) || !(0..4).all(|k| is_open(host, [x, y + k, z])) {
        return None;
    }
    for (i, (dx, dz)) in DIRS.into_iter().enumerate() {
        let mut run = 0;
        for d in 1..=DEPTH {
            let (cx, cz) = (x + dx * d, z + dz * d);
            let covered = heights[1 + i * DEPTH as usize + (d - 1) as usize].is_some_and(|h| h > y + 3);
            let open = (0..3).all(|k| is_open(host, [cx, y + k, cz])) && is_ground(host, [cx, y - 1, cz]);
            if covered && open {
                run += 1;
                if run >= 4 {
                    return Some([x, y, z]);
                }
            } else if run > 0 || d > 2 {
                break;
            }
        }
    }
    None
}

/// A sheep of the flock, on open ground 2 to 5 blocks from `at`, at about its height, tagged as the flock's.
fn sheep(host: &mut dyn Host, dimension: &str, at: [i32; 3], random: &mut creature::colours::Random) {
    for _ in 0..6 {
        let angle = ((random.next() % 360) as f64).to_radians();
        let distance = 2.0 + (random.next() % 300) as f64 / 100.0;
        let x = at[0] + (angle.cos() * distance).round() as i32;
        let z = at[2] + (angle.sin() * distance).round() as i32;
        let Ok(heights) = host.surface_heights(dimension, &[(x, z)], NativeHeightmap::MotionBlockingNoLeaves) else {
            return;
        };
        let Some(y) = heights[0] else {
            continue;
        };
        if (y - at[1]).abs() > 2 || !is_ground(host, [x, y - 1, z]) {
            continue;
        }
        let feet = [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5];
        if let Some(id) = host.summon(dimension, "minecraft:sheep", feet, Compound::new()) {
            if let Some(e) = host.entity(id) {
                host.entity_add_tag(e, FLOCK_TAG);
            }
            FLOCK.lock().unwrap().insert(id, true);
        }
        return;
    }
}

/// Where each cyclops stood last tick, for its steps.
static LAST_AT: Mutex<BTreeMap<u32, [f64; 3]>> = Mutex::new(BTreeMap::new());

/// Its heavy steps and its grumble, from its tick.
fn voice(entity: u32, at: [f64; 3], random: &mut creature::colours::Random) {
    let moved = LAST_AT
        .lock()
        .unwrap()
        .insert(entity, at)
        .is_some_and(|last| (at[0] - last[0]).hypot(at[2] - last[2]) > 0.04);
    if moved && random.next() % STEP_CHANCE == 0 {
        creature::sounds::queue(entity, STEP, 1.0, 0.6);
    }
    if random.next() % AMBIENT_CHANCE == 0 {
        creature::sounds::queue(entity, AMBIENT, 1.2, 0.6);
    }
}

pub fn hooks() -> Box<dyn Hooks> {
    creature::bestiary::register(&BESTIARY);
    Box::new(Cyclops)
}

struct Cyclops;

/// The club raised over its head, then brought down (degrees on the right arm), `since` seconds after it was raised.
fn club(since: f32) -> f32 {
    let up = WINDUP as f32 / 20.0;
    if since < up {
        let t = since / up;
        -170.0 * t * (2.0 - t)
    } else if since < up + 0.12 {
        -170.0 + 150.0 * (since - up) / 0.12
    } else if since < up + 0.6 {
        -20.0 * (1.0 - (since - up - 0.12) / 0.48)
    } else {
        0.0
    }
}

/// The boulder raised over its head with both hands, then hurled forward (degrees on both arms), `since` seconds after
/// it was lifted.
fn hurl(since: f32) -> f32 {
    let up = THROW_WINDUP as f32 / 20.0;
    if since < up {
        let t = since / up;
        -170.0 * t * (2.0 - t)
    } else if since < up + 0.15 {
        -170.0 + 130.0 * (since - up) / 0.15
    } else if since < up + 0.6 {
        -40.0 * (1.0 - (since - up - 0.15) / 0.45)
    } else {
        0.0
    }
}

/// What it is doing, for its drawing: seconds since it raised its club, lifted a boulder, was struck in the eye.
struct Moment {
    raised: Option<f32>,
    lifted: Option<f32>,
    dazed: Option<f32>,
}

impl Moment {
    fn of(id: u32) -> Moment {
        let since = |m: &Mutex<BTreeMap<u32, Instant>>| {
            m.lock()
                .unwrap()
                .get(&id)
                .map(|at| at.elapsed().as_secs_f32())
                .filter(|s| *s < 2.0)
        };
        Moment {
            raised: since(&RAISED),
            lifted: since(&LIFTED),
            dazed: eye::dazed(id),
        }
    }
    /// Whether it holds a boulder over its head now.
    fn holds_boulder(&self) -> bool {
        self.lifted.is_some_and(|s| s < THROW_WINDUP as f32 / 20.0)
    }
}

/// A group's rotation this frame: heavy strides, arms swinging, the head looking about, the club's blow, the boulder
/// lifted and hurled, the head lolling while dazed.
fn pose(name: &str, mob: &NativeMobRender<'_>, now: &Moment) -> [f32; 3] {
    let stride = ((mob.walk_position * 0.6662).cos() * 1.4 * mob.walk_speed).to_degrees() * 0.6;
    let blow = now.raised.map_or(0.0, club);
    let hurl = now.lifted.map_or(0.0, hurl);
    let loll = now.dazed.map_or(0.0, |s| (s * 9.0).sin() * 18.0);
    match name {
        "head" => [mob.pitch + loll.abs() * 0.5, -mob.head_yaw, loll],
        "leg_left" => [stride, 0.0, 0.0],
        "leg_right" => [-stride, 0.0, 0.0],
        "arm_left" => [-stride * 0.6 + hurl, 0.0, 0.0],
        "arm_right" => [stride * 0.6 + blow + hurl, 0.0, 0.0],
        "body" => [(blow + hurl) * 0.06, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

impl Hooks for Cyclops {
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id == boulder::BOULDER {
            let (w, h, e) = boulder::SIZE;
            creature::mob::ours(event.mob, w, h, e);
            event.mob.attrs.attack_damage = boulder::DAMAGE;
            event.mob.attrs.attack_knockback = boulder::KNOCKBACK;
            event.mob.ai.goals.add(1, boulder::Flight::default());
            return;
        }
        if event.mob.kind.id != CYCLOPS {
            return;
        }
        creature::mob::ours(event.mob, SIZE.0, SIZE.1, SIZE.2);
        event.mob.attrs.attack_damage = ATTACK_DAMAGE;
        event.mob.attrs.attack_knockback = ATTACK_KNOCKBACK;
        event.mob.attrs.knockback_resistance = KNOCKBACK_RESISTANCE;
        event.mob.attrs.follow_range = SIGHT;
        let ai = &mut event.mob.ai;
        ai.goals.add(0, goals::Float);
        ai.goals.add(0, eye::Stunned::default());
        ai.goals.add(1, Smash::default());
        ai.goals.add(5, Stroll::new(0.6));
        ai.goals.add(6, LookAtPlayer::new(12.0));
        ai.goals.add(7, RandomLookAround::default());
        ai.targets.add(1, HurtByTarget::new());
        ai.targets.add(2, Shepherd);
        ai.targets.add(3, NearestTarget::player(true));
    }

    /// The club and the crook take no enchantment at an anvil either (the table offers none: no `enchantable`): no
    /// result with an enchanted book or an enchanted item on the right. Repairing and renaming still work.
    fn anvil_update(&self, _: &mut dyn Host, event: &mut AnvilUpdateEvent<'_>) -> Flow {
        let ours = [CLUB, crook::CROOK].contains(&&*event.left.item);
        let enchants = event
            .right
            .is_some_and(|r| &*r.item == "minecraft:enchanted_book" || !r.enchantments().is_empty());
        if ours && enchants { Flow::Cancel } else { Flow::Continue }
    }

    /// Right-click: nothing (no trading screen, which their template would open).
    fn interact_entity(&self, _: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if event.kind == CYCLOPS || event.kind == boulder::BOULDER {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// An arrow or trident in its eye (`eye.rs`); one of its sheep hurt by a mob or a player's blow: its cyclops turns
    /// on the attacker.
    fn living_incoming_damage(&self, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        eye::struck(event);
        let attacker = match u32::try_from(event.causing) {
            Ok(id) if id > 0 => Some(Some(id)),
            _ if event.damage_type == "minecraft:player_attack" => Some(None),
            _ => None,
        };
        if event.kind == "minecraft:sheep"
            && let Some(attacker) = attacker
            && let Ok(sheep) = u32::try_from(event.entity)
            && let Some(&shepherd) = SHEPHERD.lock().unwrap().get(&sheep)
        {
            ANGERED.lock().unwrap().insert(shepherd, attacker);
        }
        Flow::Continue
    }

    fn level_tick_post(&self, host: &mut dyn Host, event: &mut LevelTickPostEvent<'_>) {
        if event.dimension == "minecraft:overworld" {
            spawn(host, event.dimension);
        }
        crook::tick(host, event.dimension);
        flock(host, event.dimension);
        eye::watch(host, event.dimension);
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind == boulder::BOULDER {
            boulder::tick(host, event.dimension, event.entity);
            return;
        }
        if event.kind != CYCLOPS {
            return;
        }
        if let Some(at) = host.entity(event.entity).and_then(|e| host.entity_position(e)) {
            CYCLOPS_AT.lock().unwrap().insert(event.entity, at);
            let mut random = creature::colours::Random::new(event.entity);
            voice(event.entity, at, &mut random);
            eye::tick(host, event.entity, at);
        }
        boulder::launch(host, event.dimension);
        creature::sounds::tick_post(host, event, &VOICE, creature::sounds::NEUTRAL);
    }

    fn living_death(&self, _: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == CYCLOPS {
            CYCLOPS_AT.lock().unwrap().remove(&event.entity);
            RAISED.lock().unwrap().remove(&event.entity);
            LIFTED.lock().unwrap().remove(&event.entity);
            LAST_AT.lock().unwrap().remove(&event.entity);
            ANGERED.lock().unwrap().remove(&event.entity);
            eye::forget(event.entity);
        } else if event.kind == boulder::BOULDER {
            boulder::forget(event.entity);
        } else if event.kind == "minecraft:sheep" {
            FLOCK.lock().unwrap().remove(&event.entity);
            SHEPHERD.lock().unwrap().remove(&event.entity);
        }
        Flow::Continue
    }

    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind == boulder::BOULDER {
            boulder::submit(host, event);
            return Flow::Cancel;
        }
        if mob.kind != CYCLOPS {
            return Flow::Continue;
        }
        let now = Moment::of(mob.id);
        event.draw.push();
        creature::model::death_tilt(&mut *event.draw, mob);
        creature::model::draw_shown(
            host,
            &mut *event.draw,
            &model::CYCLOPS,
            TEXTURE,
            &|part| pose(part, mob, &now),
            &|_| true,
            creature::model::tint(mob),
        );
        if now.holds_boulder() {
            boulder::held(host, &mut *event.draw, [0.0, HELD_HEIGHT, 0.0]);
        }
        event.draw.pop();
        Flow::Cancel
    }
}
