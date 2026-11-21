//! Seagulls (SPECS.md): noisy gulls of the coasts, of the Sunrise Creatures series.
//!
//! A seagull is a copy of the wandering trader drawn with its own model (`model_seagull.rs`, from
//! `sources/models/seagull.bbmodel`). It flies with the library's flight (`creature::flight`: soaring along the coast
//! as `SOARING` says, the physics as its `WINGS`), floats on the water, walks on the shore, snatches food from hands,
//! follows boats and catches gliding flying fish (the library's prey, when the Flying Fish are installed). Fed a raw
//! fish it may become a player's friend (the library's `friend`): it follows them, dives for fish for them and circles
//! over a lucky fishing spot. Its behaviours are in `coast.rs`; what they can't do themselves, the tick does (`act`).
mod coast;
mod model_seagull;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use coast::{Action, Around};
use creature::bestiary::{Fact, Species};
use creature::flight::{Soaring, Wings};
use mod_api::{
    BlockEntityDrawContext, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, Hand,
    Hooks, HostSide, InteractionResult, ItemFishedEvent, LivingDeathEvent, ParticleOptions,
    RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost, SoundSource,
};

pub const KIND: &str = "seagulls:seagull";
const EGG: &str = "seagulls:seagull_spawn_egg";
const FEATHER: &str = "seagulls:feather";
const RAW: &str = "seagulls:raw_seagull";
const COOKED: &str = "seagulls:cooked_seagull";
const TEXTURE: &str = "seagulls:entity/seagull";

/// Its box (blocks, SPECS §1) and the scale its model (20 texels long, its wings 36 across) is drawn at.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.6,
    height: 0.6,
    eye_height: 0.5,
};
const SCALE: f32 = 0.6;

/// Its sounds (`sources/sounds.toml`, cut from CC0 freesound recordings): its cry, its laughing call, its wing beats,
/// its take-off (a loud cry), its dive's splash; hurt and death are its entity type's.
pub(crate) const CRY: &str = "seagulls:entity.seagull.cry";
pub(crate) const LAUGH: &str = "seagulls:entity.seagull.laugh";
const FLAP: &str = "seagulls:entity.seagull.flap";
const TAKEOFF: &str = "seagulls:entity.seagull.takeoff";
pub(crate) const SPLASH: &str = "seagulls:entity.seagull.splash";

/// Its flight (the library's `creature::flight`): slower than the vulture's, a quick dive.
pub(crate) static WINGS: Wings = Wings {
    cruise: 0.42,
    dive: 1.1,
    flap: FLAP,
    takeoff: TAKEOFF,
    pace: |_| 1.0,
};

/// Its soaring along the coast (SPECS §2): lower and shorter than the vulture's, by day (at night it rests on the
/// shore), landing on the sand, rocks, posts and roofs, or on the water, where it floats.
pub(crate) static SOARING: Soaring = Soaring {
    take_off: 900,
    height: (8.0, 6),
    radius: (8.0, 6),
    circle_for: (300, 400),
    idle: |m, cx| creature::friend::of(m.id).is_none() && (cx.daylight || !(m.on_ground || m.in_water)),
    perch: |name| !name.ends_with("leaves") && name != "minecraft:lava",
    lands_on_water: true,
};

/// Where it lives (SPECS §2).
const BIOMES: &[&str] = &["minecraft:beach", "minecraft:stony_shore", "minecraft:snowy_beach"];

pub(crate) const THIEF_FACT: &str = "thief";
const FISH_FACT: &str = "fish";
const CATCH_FACT: &str = "catch";
const SPOT_FACT: &str = "spot";
pub(crate) const BOAT_FACT: &str = "boat";

/// It hunts the flying fish (SPECS §2), when the Flying Fish are installed: one within 16 blocks.
static HUNTER: creature::prey::Hunter = creature::prey::Hunter {
    kind: KIND,
    range: 16.0,
    speed: 1.0,
};

/// Its page (SPECS §4): the biomes met in, its facts shown as they are learnt.
static BESTIARY: Species = Species {
    id: KIND,
    biomes: BIOMES,
    wild: |id| creature::friend::of(id).is_none(),
    facts: &[
        Fact {
            id: THIEF_FACT,
            item: "minecraft:bread",
            key: "bestiary.seagulls.thief",
        },
        Fact {
            id: FISH_FACT,
            item: "minecraft:cod",
            key: "bestiary.seagulls.fish",
        },
        Fact {
            id: CATCH_FACT,
            item: "minecraft:salmon",
            key: "bestiary.seagulls.catch",
        },
        Fact {
            id: SPOT_FACT,
            item: "minecraft:fishing_rod",
            key: "bestiary.seagulls.spot",
        },
        Fact {
            id: BOAT_FACT,
            item: "minecraft:oak_boat",
            key: "bestiary.seagulls.boat",
        },
    ],
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::Common,
    // Never in a glass jar.
    draw: |_, _| {},
    draw_scale: 1.0,
    draw_lift: 0.0,
};

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static FEATHER_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: FEATHER,
    dropped_by: &[KIND],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:arrow",
        key: "bestiary.seagulls.item.feather.use",
    }],
};
static RAW_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: RAW,
    dropped_by: &[KIND],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:chicken",
        key: "bestiary.seagulls.item.raw_seagull.use",
    }],
};
static COOKED_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: COOKED,
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:cooked_chicken",
        key: "bestiary.seagulls.item.cooked_seagull.use",
    }],
};

/// One fish in this many befriends it (SPECS §3).
const FRIEND_CHANCE: u64 = 4;
/// How far it looks for food on the ground, a boat to follow, its friend's bobber (blocks), and how often (ticks).
const FOOD_RANGE: f64 = 10.0;
const BOAT_RANGE: f64 = 24.0;
const BOBBER_RANGE: f64 = 24.0;
const LOOK_EVERY: i32 = 10;
/// The raw fish that befriend it.
const BEFRIENDING: &[&str] = &["minecraft:cod", "minecraft:salmon"];

/// The players fishing: where they cast from and along which look, and when (the game's time). Sunrise's fishing
/// bobber is no entity the mod can find, so its place is guessed from the cast (`bobber`), and it is forgotten at the
/// reel-in, when the rod leaves the player's hands, or after `CAST_FOR` ticks.
/// A cast: the player's eyes and look, and the game's time.
type Cast = ([f64; 3], [f64; 3], i64);
static CASTS: Mutex<BTreeMap<u128, Cast>> = Mutex::new(BTreeMap::new());
const CAST_FOR: i64 = 1200;
/// How far ahead of the player a cast bobber lands (blocks across).
const CAST_REACH: f64 = 6.0;

/// Where player `player`'s bobber is, guessed: `CAST_REACH` ahead of where they cast along their look, on the water
/// there; none when there is no water there or no cast.
fn bobber(host: &dyn ServerHost, level: mod_api::LevelRef, player: u128, now: i64) -> Option<[f64; 3]> {
    let (eye, look, at) = CASTS.lock().unwrap().get(&player).copied()?;
    if now - at > CAST_FOR {
        CASTS.lock().unwrap().remove(&player);
        return None;
    }
    let n = look[0].hypot(look[2]).max(0.01);
    let (x, z) = (eye[0] + look[0] / n * CAST_REACH, eye[2] + look[2] / n * CAST_REACH);
    let ground = creature::territory::Ground::new(host, level);
    let (bx, bz) = (x.floor() as i32, z.floor() as i32);
    let top = (eye[1].floor() as i32 - 12..=eye[1].floor() as i32)
        .rev()
        .find(|&y| ground.block([bx, y, bz]) != "minecraft:air")?;
    (ground.block([bx, top, bz]) == "minecraft:water").then_some([x, f64::from(top) + 0.9, z])
}

/// Whether each seagull is in the air, and in the water, from its server tick, for the drawing (single player).
static FLYING: Mutex<BTreeMap<u32, (bool, bool)>> = Mutex::new(BTreeMap::new());

fn is_boat(kind: &str) -> bool {
    kind.ends_with("boat") || kind.ends_with("raft")
}

/// From `entity_tick_post`, for each seagull: its friend's save, its behaviours, what they queued, what it finds
/// around it, its sounds, despawning far from players.
fn tick(host: &mut dyn ServerHost, dimension: &str, id: u32) {
    creature::friend::tick(host, id);
    creature::body::tick(host, id, coast::behaviours);
    for action in coast::actions() {
        act(host, dimension, action);
    }
    creature::sounds::play_queued(host, id, SoundSource::Neutral);
    let Some(e) = host.entity(id) else {
        return;
    };
    let flying = !e.on_ground(&*host).unwrap_or(true) && !e.is_in_water(&*host).unwrap_or(false);
    FLYING
        .lock()
        .unwrap()
        .insert(id, (flying, e.is_in_water(&*host).unwrap_or(false)));
    if (e.tick_count(&*host).unwrap_or(0) + id as i32) % LOOK_EVERY == 0 {
        look_around(host, id);
    }
    let kept = creature::friend::of(id).is_some();
    if creature::mob::despawn_far(host, dimension, id, kept) {
        forget(id);
    }
}

/// What seagull `id` finds around it (`coast::Around`).
fn look_around(host: &mut dyn ServerHost, id: u32) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    let mut around = Around::default();
    let near = |r: f64| ([at[0] - r, at[1] - r, at[2] - r], [at[0] + r, at[1] + r, at[2] + r]);
    // Food on the ground, the nearest.
    let (from, to) = near(FOOD_RANGE);
    let distance = |p: [f64; 3]| creature::body::distance(p, at);
    around.food = level
        .entities_in(&*host, from, to)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let item = host.item_entity(e)?;
            let stack = item.stack(&*host)?;
            let kind = creature::hand::id(&*host, &stack);
            let fish = BEFRIENDING.contains(&kind.as_str());
            (fish || coast::FOODS.contains(&kind.as_str())).then_some((
                e.id(),
                e.position(&*host)?,
                fish,
                item.owner(&*host).map(|o| o.id()),
            ))
        })
        .min_by(|a, b| distance(a.1).total_cmp(&distance(b.1)));
    let players = host.players().server_players();
    let friend = creature::friend::of(id).map(|f| creature::friend::uuid(f.uuid));
    for p in players {
        let pe = p.entity();
        let Some(there) = pe.position(&*host) else {
            continue;
        };
        let boating = pe
            .vehicle(&*host)
            .and_then(|v| v.kind(&*host))
            .is_some_and(|k| is_boat(&k.id(&*host)));
        if Some(p.uuid()) == friend {
            around.friend_boating = boating;
            // Its friend's bobber in the water, while they hold their rod.
            let rod = [Hand::Main, Hand::Off].into_iter().any(|hand| {
                creature::hand::held(&*host, p, hand)
                    .is_some_and(|s| creature::hand::is(&*host, &s, "minecraft:fishing_rod"))
            });
            if !rod {
                CASTS.lock().unwrap().remove(&p.uuid());
            }
            let now = level.game_time(&*host).unwrap_or(0);
            around.bobber = bobber(&*host, level, p.uuid(), now).filter(|b| distance(*b) < BOBBER_RANGE);
        } else if boating && distance(there) < BOAT_RANGE && around.boater.is_none() {
            around.boater = Some((pe.id(), p.uuid(), there));
        }
    }
    coast::set_around(id, around);
}

/// Particles of `kind` around seagull `id`.
fn particles(host: &mut dyn ServerHost, id: u32, kind: &str) {
    let Some(e) = host.entity(id) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) else {
        return;
    };
    if let Some(k) = creature::particles::key(kind) {
        let at = [at[0], at[1] + 0.6, at[2]];
        level.send_particles(host, &ParticleOptions::simple(k), None, at, [0.3, 0.3, 0.3], 0.02, 6);
    }
}

fn sound(host: &mut dyn ServerHost, id: u32, sound: &str) {
    let Some(e) = host.entity(id) else {
        return;
    };
    if let (Some(at), Some(level)) = (e.position(&*host), e.level(&*host)) {
        level.play_sound(host, sound, at, SoundSource::Neutral, 1.0, 1.0, None);
    }
}

/// An item `item` dropped at `at` in seagull `id`'s level.
fn drop_item(host: &mut dyn ServerHost, id: u32, item: &str, at: [f64; 3]) -> bool {
    let Some(level) = host.entity(id).and_then(|e| e.level(&*host)) else {
        return false;
    };
    let Some(stack) = host.item_from_snbt(&format!("{{id:\"{item}\",count:1}}")) else {
        return false;
    };
    creature::hand::drop_stack(host, level, at, stack);
    true
}

/// The oceans of the flying fish, where a seagull diving for its friend sometimes brings one back.
const FLYING_FISH_OCEANS: &[&str] = &[
    "minecraft:lukewarm_ocean",
    "minecraft:deep_lukewarm_ocean",
    "minecraft:warm_ocean",
];

/// What a behaviour queued (`coast::Action`).
fn act(host: &mut dyn ServerHost, dimension: &str, action: Action) {
    match action {
        Action::Steal { gull, player } => {
            let Some(p) = host
                .players()
                .server_players()
                .into_iter()
                .find(|p| p.entity().id() == player)
            else {
                return;
            };
            let Some(stack) = creature::hand::held(&*host, p, Hand::Main) else {
                return;
            };
            let item = creature::hand::id(&*host, &stack);
            if !coast::FOODS.contains(&item.as_str()) {
                return;
            }
            creature::hand::use_one(host, p, Hand::Main);
            coast::set_carried(gull, Some(item));
            sound(host, gull, LAUGH);
            creature::bestiary::learn(host, p.uuid(), KIND, THIEF_FACT);
        }
        Action::Swallow { gull } => coast::set_carried(gull, None),
        Action::DropCarried { gull } => {
            if let Some(item) = coast::carried(gull)
                && let Some(at) = host.entity(gull).and_then(|e| e.position(&*host))
            {
                drop_item(host, gull, &item, at);
            }
            coast::set_carried(gull, None);
        }
        Action::Eat { gull, item, befriends } => {
            let Some(e) = host.entity(item) else {
                return;
            };
            let Some(it) = host.item_entity(e) else {
                return;
            };
            let Some(stack) = it.stack(&*host) else {
                return;
            };
            match stack.shrink(1) {
                Some(rest) => {
                    let _ = it.set_stack(host, rest);
                }
                None => {
                    e.discard(host);
                }
            }
            sound(host, gull, "minecraft:entity.generic.eat");
            if let Some(player) = befriends {
                if creature::colours::Random::new(gull ^ item)
                    .next()
                    .is_multiple_of(FRIEND_CHANCE)
                {
                    creature::friend::befriend(host, gull, player);
                    particles(host, gull, "minecraft:heart");
                    creature::bestiary::learn(host, player, KIND, FISH_FACT);
                } else {
                    particles(host, gull, "minecraft:smoke");
                }
            }
        }
        Action::GiveFish { gull, at } => {
            let roll = creature::colours::Random::new(gull).next() % 100;
            let biome = host.entity(gull).and_then(|e| e.level(&*host)).and_then(|l| {
                let p = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
                l.biome_at(&*host, p)
            });
            let warm = biome.is_some_and(|b| FLYING_FISH_OCEANS.contains(&b.as_str()));
            let fish = match roll {
                0..3 => "minecraft:tropical_fish",
                3..15 => "minecraft:salmon",
                _ => "minecraft:cod",
            };
            // A raw flying fish now and then in their oceans, when the Flying Fish are installed.
            let given = (warm && roll.is_multiple_of(4) && drop_item(host, gull, "flying_fish:raw_flying_fish", at))
                || drop_item(host, gull, fish, at);
            if given && let Some(f) = creature::friend::of(gull) {
                creature::bestiary::learn(host, creature::friend::uuid(f.uuid), KIND, CATCH_FACT);
            }
            let _ = dimension;
        }
        Action::Learn { player, fact } => creature::bestiary::learn(host, player, KIND, fact),
    }
}

/// What fishing brings up that isn't a fish or a treasure (vanilla's junk), which a lucky spot turns into a cod.
const JUNK: &[&str] = &[
    "minecraft:lily_pad",
    "minecraft:leather_boots",
    "minecraft:leather",
    "minecraft:bone",
    "minecraft:potion",
    "minecraft:string",
    "minecraft:fishing_rod",
    "minecraft:bowl",
    "minecraft:stick",
    "minecraft:ink_sac",
    "minecraft:tripwire_hook",
    "minecraft:rotten_flesh",
    "minecraft:bamboo",
];
/// How near its lucky spot the bobber must be (blocks across), how near a wild seagull steals a fresh catch, and one
/// catch in how many it steals.
const LUCKY_REACH: f64 = 4.0;
const STEAL_REACH: f64 = 10.0;
const STEAL_ONE_IN: u64 = 4;

/// Fishing (SPECS §2, §3): over its friend's seagull's lucky spot, no junk; near a wild seagull, a fish just caught
/// may be snatched.
fn fished(host: &mut dyn ServerHost, event: &mut ItemFishedEvent<'_>) {
    let at = event.fished.position;
    if coast::lucky_spots(event.player)
        .iter()
        .any(|s| (s[0] - at[0]).hypot(s[2] - at[2]) <= LUCKY_REACH)
    {
        let mut lucky = false;
        for stack in event.drops.iter_mut() {
            if JUNK.contains(&creature::hand::id(&*host, stack).as_str())
                && let Some(cod) = host.item_from_snbt("{id:\"minecraft:cod\",count:1}")
            {
                *stack = cod;
                lucky = true;
            }
        }
        creature::bestiary::learn(host, event.player, KIND, SPOT_FACT);
        let _ = lucky;
        return;
    }
    let Some(level) = host.player(event.player).and_then(|p| p.entity().level(&*host)) else {
        return;
    };
    let r = STEAL_REACH;
    let thief = level
        .entities_in(
            &*host,
            [at[0] - r, at[1] - r, at[2] - r],
            [at[0] + r, at[1] + r, at[2] + r],
        )
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.kind(&*host).is_some_and(|k| k.id(&*host) == KIND))
        .map(|e| e.id())
        .find(|&g| creature::friend::of(g).is_none() && coast::carried(g).is_none());
    let Some(gull) = thief else {
        return;
    };
    if !creature::colours::Random::new(gull ^ event.fished.hook)
        .next()
        .is_multiple_of(STEAL_ONE_IN)
    {
        return;
    }
    let fish = event.drops.iter().position(|s| {
        let id = creature::hand::id(&*host, s);
        BEFRIENDING.contains(&id.as_str()) || id == "minecraft:tropical_fish"
    });
    if let Some(i) = fish {
        let stack = event.drops.remove(i);
        coast::set_carried(gull, Some(creature::hand::id(&*host, &stack)));
        sound(host, gull, LAUGH);
        creature::bestiary::learn(host, event.player, KIND, THIEF_FACT);
    }
}

fn forget(id: u32) {
    creature::body::forget(id);
    creature::friend::forget(id);
    coast::forget(id);
    FLYING.lock().unwrap().remove(&id);
}

/// How it holds itself this frame: in flight its body takes the flight's pitch and banks with the turn, its wings
/// beat or glide, swept back in a dive and folded back for a plunge; on the ground or the water its wings fold along
/// its flanks.
fn pose(name: &str, frame: &Frame) -> [f32; 3] {
    let bank = (frame.head_yaw * 0.8).clamp(-50.0, 50.0);
    if !frame.flying {
        let step = (frame.walk_position * 0.6662).cos() * 1.4 * frame.walk_speed.min(1.0);
        let leg = if frame.floating { 30.0 } else { step.to_degrees() };
        return match name {
            "head" => [frame.pitch, -frame.head_yaw, 0.0],
            // Folded along its back, raised to the spine like a roof (turned back, then tilted 20 degrees about the
            // wing's length: x, y and z as the drawing applies them), the outer parts bent in so the black tips
            // cross over the tail.
            "wing_left" => [95.5, 69.9, -95.8],
            "wing_right" => [95.5, -69.9, 95.8],
            "wing_left_mid" | "wing_left_tip" => [0.0, 6.0, 0.0],
            "wing_right_mid" | "wing_right_tip" => [0.0, -6.0, 0.0],
            "leg_left" => [leg, 0.0, 0.0],
            "leg_right" => [-leg, 0.0, 0.0],
            "tail" => [-5.0, 0.0, 0.0],
            _ => [0.0; 3],
        };
    }
    let (beat, lag, late) = if frame.diving {
        (0.0, 0.0, 0.0)
    } else if frame.flapping {
        (
            (frame.age * 0.6).sin() * 35.0,
            (frame.age * 0.6 - 0.8).sin() * 20.0,
            (frame.age * 0.6 - 1.4).sin() * 15.0,
        )
    } else {
        (6.0, 0.0, 4.0)
    };
    let sweep = if frame.diving {
        75.0
    } else {
        (frame.pitch - 20.0).clamp(0.0, 40.0)
    };
    match name {
        "body" => [frame.pitch, 0.0, bank],
        "head" => [-frame.pitch * 0.5, -frame.head_yaw * 0.5, 0.0],
        "wing_left" => [0.0, sweep, beat],
        "wing_right" => [0.0, -sweep, -beat],
        "wing_left_mid" => [0.0, sweep * 0.5, lag],
        "wing_right_mid" => [0.0, -sweep * 0.5, -lag],
        "wing_left_tip" => [0.0, 0.0, late],
        "wing_right_tip" => [0.0, 0.0, -late],
        "leg_left" | "leg_right" => [60.0, 0.0, 0.0],
        "tail" => [-10.0, 0.0, -bank * 0.3],
        _ => [0.0; 3],
    }
}

/// What a frame of a seagull shows, for `pose`.
struct Frame {
    age: f32,
    head_yaw: f32,
    pitch: f32,
    walk_position: f32,
    walk_speed: f32,
    flying: bool,
    floating: bool,
    flapping: bool,
    diving: bool,
}

/// Its own model, posed by `pose` every frame.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let (flying, floating) = FLYING
            .lock()
            .unwrap()
            .get(&entity.id)
            .copied()
            .unwrap_or((false, false));
        let frame = Frame {
            age: entity.age,
            head_yaw: mob.map_or(0.0, |m| m.y_rot),
            pitch: mob.map_or(0.0, |m| m.x_rot),
            walk_position: mob.map_or(0.0, |m| m.walk_animation_pos),
            walk_speed: mob.map_or(0.0, |m| m.walk_animation_speed),
            flying,
            floating,
            flapping: creature::flight::flapping(entity.id),
            diving: coast::diving(entity.id),
        };
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.scale([SCALE; 3]);
        let pose = |part: &str| pose(part, &frame);
        let all = |_: &str| true;
        creature::model::draw_shown(
            draw,
            &model_seagull::SEAGULL,
            TEXTURE,
            &pose,
            &all,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Seagulls;

#[mod_api::sunrise_mod]
impl Hooks for Seagulls {
    fn register(&self, _: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(KIND, SIZE);
        creature::prey::add_hunter("flying_fish:flying_fish", &HUNTER);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&FEATHER_PAGE);
        creature::bestiary::register_item(&RAW_PAGE);
        creature::bestiary::register_item(&COOKED_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Right-click by its friend: it stays, or follows again. A name tag names it as vanilla's would. Never a trading
    /// screen, which its template would open, on both sides.
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != KIND {
            return None;
        }
        let host = match host {
            HostSide::Server(host) => host,
            HostSide::Client(client) => {
                let name_tag = event
                    .stack
                    .is_some_and(|s| creature::hand::is(&*client, s, "minecraft:name_tag"));
                return (!name_tag).then_some(InteractionResult::Consume);
            }
        };
        if event
            .stack
            .is_some_and(|s| creature::hand::is(&*host, s, "minecraft:name_tag"))
        {
            return None;
        }
        if let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity))
            && let Some(f) = creature::friend::of(id).filter(|f| f.is(player.uuid()))
        {
            creature::friend::set_staying(id, !f.staying);
            particles(host, id, "minecraft:happy_villager");
        }
        Some(InteractionResult::Consume)
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind == KIND {
                tick(host, event.dimension, event.entity);
            }
        });
    }

    /// A cast (state 0) is remembered, a reel-in (2 to 4) forgets it (`CASTS`).
    fn player_fish(&self, host: &mut dyn ServerHost, event: &mut mod_api::PlayerFishEvent<'_>) -> Flow {
        match event.state {
            0 => {
                if let Some(p) = host.player(event.player)
                    && let Some((eye, look)) = creature::hand::eye_and_look(&*host, p)
                {
                    let now = p.entity().level(&*host).and_then(|l| l.game_time(&*host)).unwrap_or(0);
                    CASTS.lock().unwrap().insert(event.player, (eye, look, now));
                }
            }
            2..=4 => {
                CASTS.lock().unwrap().remove(&event.player);
            }
            _ => {}
        }
        Flow::Continue
    }

    fn item_fished(&self, host: &mut dyn ServerHost, event: &mut ItemFishedEvent<'_>) -> Flow {
        fished(host, event);
        Flow::Continue
    }

    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            // Food it was carrying falls.
            if let Some(item) = coast::carried(event.entity)
                && let Some(at) = host.entity(event.entity).and_then(|e| e.position(&*host))
            {
                drop_item(host, event.entity, &item, at);
            }
            forget(event.entity);
        }
        Flow::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(flying: bool, diving: bool) -> Frame {
        Frame {
            age: 0.0,
            head_yaw: 0.0,
            pitch: 0.0,
            walk_position: 0.0,
            walk_speed: 0.0,
            flying,
            floating: false,
            flapping: false,
            diving,
        }
    }

    #[test]
    fn its_wings_fold_on_the_ground_and_sweep_back_in_a_dive() {
        assert_eq!(pose("wing_left", &frame(false, false)), [95.5, 69.9, -95.8]);
        assert_eq!(pose("wing_left", &frame(true, true))[1], 75.0);
        assert!(is_boat("minecraft:oak_boat") && is_boat("minecraft:bamboo_raft") && !is_boat("minecraft:pig"));
    }
}
