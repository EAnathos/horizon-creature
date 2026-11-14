//! Fairies (SPECS.md): shy glowing fairies of the flowery woods and meadows, of the Horizon Creature series.
//!
//! A fairy is a copy of the wandering trader (its behaviour is all goals) with a small box, flying with the library's
//! `flutter` (out day and night, resting in the rain, landing on flowers) under its own `Fairy` goal (`companion.rs`):
//! shy of players who come near without sneaking, drawn by a sneaking player holding glow berries (or sweet berries),
//! which it nibbles and may befriend them for (the library's `friend`); befriended, it circles its friend's head, or
//! stays where it is told (right-click). Its friend has Speed I while it follows within reach, gets a pinch of fairy
//! dust once a day, and a soft violet light goes with it. Caught in the library's glass jar (a sneaking player, or its
//! friend), it stays befriended; on display, a fairy lamp. Drawn full-bright, its wings beating.
mod companion;
mod model_fairy;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Instant;

use creature::bestiary::{Fact, Species};
use creature::friend::Friend;
use creature::jar::Catchable;
use horizon_client::mod_api::{
    AttachmentCodec, AttachmentError, AttachmentType, BlockEntityDraw, Compound, EmissiveEvent, EntityTickPostEvent,
    Flow, Holder, Hooks, Host, InteractEntityEvent, LivingDeathEvent, LivingIncomingDamageEvent, MobSubmitEvent,
    NativeColoredLight, NativeEffect, NativeMobRender, ParticleOptions, ParticleType, RegisterGoalsEvent, Tag, Text,
    UseItemOnEvent, VisualsEvent,
};

pub const FAIRY: &str = "fairies:fairy";
const TEXTURE: &str = "fairies:entity/fairy/fairy";
const DUST: &str = "fairies:fairy_dust";
/// Its box (blocks: width, height, eye height); its model is about three quarters of a block tall.
const SIZE: (f64, f64, f64) = (0.4, 0.8, 0.6);

/// How it flies wild (SPECS §2): day and night, resting in the rain, flitting between flowers.
static FLIGHT: creature::flutter::Kind = creature::flutter::Kind {
    hours: creature::flutter::Hours::Always,
    speed: 0.1,
    height: (1.0, 2.5),
    flight: (120, 320),
    rest: (40, 140),
    wander: 5.0,
    drawn_to_light: false,
    lands_on_flowers: true,
    takeoff: None,
};

/// Caught in the library's glass jar, by a sneaking player or its friend; settling on a flower; a fairy lamp on display
/// (SPECS §5).
static IN_JAR: Catchable = Catchable {
    kind: FAIRY,
    model: "fairies:fairy_jar",
    settles: Some(creature::territory::Rule::FlowerPatch),
    glows: true,
    shy: true,
};

/// The bestiary's facts (SPECS §4).
const FRIEND_FACT: &str = "friend";
const DUST_FACT: &str = "dust";
const SHY_FACT: &str = "shy";

static BESTIARY: Species = Species {
    id: FAIRY,
    biomes: &[
        "minecraft:flower_forest",
        "minecraft:meadow",
        "minecraft:cherry_grove",
        "minecraft:birch_forest",
    ],
    wild: |id| creature::friend::of(id).is_none() && !creature::territory::settled(id),
    facts: &[
        Fact {
            id: FRIEND_FACT,
            item: "minecraft:glow_berries",
            key: "bestiary.fairies.friend",
        },
        Fact {
            id: DUST_FACT,
            item: DUST,
            key: "bestiary.fairies.dust",
        },
        Fact {
            id: SHY_FACT,
            item: "minecraft:leather_boots",
            key: "bestiary.fairies.shy",
        },
        Fact {
            id: creature::jar::JAR_FACT,
            item: creature::jar::ITEM,
            key: "bestiary.fairies.jar",
        },
    ],
    icon: "fairies:fairy_spawn_egg",
    scheme: None,
    rarity: creature::bestiary::Rarity::Rare,
    draw: book_view,
};

/// The bestiary's 3D view: hovering, its wings open, about a block tall.
fn book_view(host: &dyn Host, draw: &mut BlockEntityDraw, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.0, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    creature::model::draw(host, draw, &model_fairy::FAIRY, TEXTURE, &|_| [0.0; 3]);
    draw.pop();
}
const BOOK_SCALE: f32 = 1.2;

/// The speed it gives its friend (SPECS §3): Speed I while it follows within `BOOST_REACH` blocks, renewed every
/// `BOOST_EVERY` ticks for a little longer, so it lapses soon after.
const BOOST_REACH: f64 = 8.0;
const BOOST_EVERY: u32 = 40;
const BOOST_TICKS: i32 = 70;
/// Its night light (SPECS §3), seen only: violet, wider around a fairy following its friend.
const LIGHTS: usize = 8;
const LIGHT_RGB: u32 = 0xC9A2FF;
const LIGHT_RADIUS: (f32, f32) = (3.0, 8.0);
const LIGHT_INTENSITY: f32 = 0.7;

/// Where each fairy is and whether it follows its friend, for its light (the client's `visuals` has no positions of
/// its own): kept by the server's tick, read by the drawing (single player; arismoko/horizon#2740).
static AT: Mutex<BTreeMap<u32, Seen>> = Mutex::new(BTreeMap::new());

struct Seen {
    at: [f64; 3],
    following: bool,
    when: Instant,
}
/// Ticks seen per fairy, for what it does now and then.
static TICKS: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

/// The day it last gave its friend fairy dust (SPECS §3), saved on it (`fairies:dust_day`).
static DUST_DAY: AttachmentType<i64> = AttachmentType {
    namespace: "fairies",
    name: "dust_day",
    default: || -1,
    codec: Some(AttachmentCodec {
        encode: |day| Ok([("Day".into(), Tag::Long(*day))].into()),
        decode: |c: &Compound| -> Result<i64, AttachmentError> {
            Ok(match c.get(&"Day".into()) {
                Some(Tag::Long(d)) => *d,
                _ => -1,
            })
        },
    }),
    copy_on_death: false,
    sync: false,
};

pub fn hooks() -> Box<dyn Hooks> {
    creature::hand::server_used(DUST);
    creature::jar::register(&IN_JAR);
    creature::bestiary::register(&BESTIARY);
    Box::new(Fairies)
}

struct Fairies;

/// A group's rotation this frame: the wings beat (fast in flight, slowly at rest), the arms sway, the hem flutters.
fn pose(name: &str, mob: &NativeMobRender<'_>, flying: bool) -> [f32; 3] {
    let beat = if flying {
        22.0 + 26.0 * (mob.age * 1.6).sin()
    } else {
        10.0 + 10.0 * (mob.age * 0.1).sin()
    };
    let sway = 8.0 * (mob.age * 0.15).sin();
    match name {
        "wing_upper_right" | "wing_lower_right" => [0.0, -beat, 0.0],
        "wing_upper_left" | "wing_lower_left" => [0.0, beat, 0.0],
        "arm_right" => [sway, 0.0, 0.0],
        "arm_left" => [-sway, 0.0, 0.0],
        "hem" => [0.0, mob.age * 2.0 % 360.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Its friend's player, if they are on the server.
fn friend_player(host: &dyn Host, friend: Friend) -> Option<horizon_client::mod_api::NativeServerPlayer> {
    host.server_players().into_iter().find(|p| friend.is(p.uuid))
}

fn sparkles(host: &mut dyn Host, at: [f64; 3], count: i32) {
    let options = ParticleOptions::simple(ParticleType::EndRod);
    host.send_particles(&options, None, at, [0.25, 0.3, 0.25], 0.02, count);
}

fn chime(host: &mut dyn Host, at: [f64; 3], pitch: f32) {
    host.play_sound(
        "minecraft:block.amethyst_block.chime",
        at,
        creature::sounds::NEUTRAL,
        1.0,
        pitch,
    );
}

/// A sneaking player held out berries and it nibbled one (`companion::take_nibbles`): one is taken from their hand
/// (not in creative); it befriends them when the goal's roll said so.
fn nibbled(host: &mut dyn Host, fairy: u32, at: [f64; 3], nibble: companion::Nibble) {
    let uuid = creature::friend::uuid(nibble.player);
    let Some(player) = host.server_players().into_iter().find(|p| p.uuid == uuid) else {
        return;
    };
    let item = if nibble.glow {
        "minecraft:glow_berries"
    } else {
        "minecraft:sweet_berries"
    };
    if player.game_mode != 1 {
        // Captured: the command's answer reaches no one (`run_command_as` would show it in their chat).
        host.run_command_captured(&format!("clear {} {item} 1", player.name));
    }
    host.play_sound("minecraft:entity.generic.eat", at, creature::sounds::NEUTRAL, 0.4, 1.8);
    let eye = [at[0], at[1] + 0.5, at[2]];
    if !nibble.befriends {
        let smoke = ParticleOptions::simple(ParticleType::Smoke);
        host.send_particles(&smoke, None, eye, [0.2, 0.2, 0.2], 0.01, 5);
        return;
    }
    creature::friend::befriend(host, fairy, uuid);
    let hearts = ParticleOptions::simple(ParticleType::Heart);
    host.send_particles(&hearts, None, eye, [0.3, 0.3, 0.3], 0.02, 6);
    chime(host, at, 1.4);
    host.action_bar(uuid, Text::translate("fairies.befriended", &[]));
    creature::bestiary::learn(host, uuid, FAIRY, FRIEND_FACT);
}

/// What a befriended fairy does for its friend now and then: Speed I near them, a pinch of dust once a day.
fn befriended(host: &mut dyn Host, dimension: &str, fairy: u32, at: [f64; 3], friend: Friend, ticks: u32) {
    let Some(player) = friend_player(host, friend).filter(|p| p.dimension == dimension) else {
        return;
    };
    let near = (0..3).map(|k| (player.position[k] - at[k]).powi(2)).sum::<f64>().sqrt() < BOOST_REACH;
    if friend.staying || !near {
        return;
    }
    if ticks.is_multiple_of(BOOST_EVERY) {
        host.add_effect(
            player.uuid,
            &NativeEffect {
                id: "minecraft:speed".into(),
                amplifier: 0,
                duration: BOOST_TICKS,
                ambient: true,
                particles: false,
            },
        );
    }
    // A pinch of dust at its friend's feet, once a day.
    let Some(day) = host.level(dimension).and_then(|l| l.day_time(host)).map(|t| t / 24_000) else {
        return;
    };
    let Ok(last) = DUST_DAY.get_existing(host, Holder::Entity(fairy)) else {
        return;
    };
    if last.unwrap_or(-1) >= day {
        return;
    }
    if DUST_DAY.set(host, Holder::Entity(fairy), day).is_err() {
        return;
    }
    let mut item = Compound::new();
    item.insert("id".into(), Tag::String(DUST.into()));
    item.insert("count".into(), Tag::Int(1));
    let nbt: Compound = [("Item".into(), Tag::Compound(item))].into();
    host.summon(dimension, "minecraft:item", player.position, nbt);
    sparkles(host, player.position, 8);
    chime(host, at, 1.8);
    creature::bestiary::learn(host, player.uuid, FAIRY, DUST_FACT);
}

/// Fairy dust on a crop or a sapling (SPECS §6), as bone meal: a crop grows 2 to 5 stages, a sapling is ready to grow
/// into a tree; one pinch is used (not in creative). Whether it acted.
fn dust_on(host: &mut dyn Host, stack: &horizon_client::items::ItemStack, pos: [i32; 3]) -> bool {
    let block = host.block(pos);
    let grew = if creature::grow::growing(host, pos).is_some() {
        let stages = 2
            + (creature::colours::Random::new(pos[0] as u32 ^ pos[2] as u32 ^ host.day_time() as u32).next() % 4)
                as u32;
        creature::grow::grow(host, pos, stages)
    } else if block.ends_with("_sapling") && host.block_property(pos, "stage").as_deref() == Some("0") {
        host.set_block_property(pos, "stage", "1")
    } else {
        false
    };
    if grew {
        sparkles(
            host,
            [
                f64::from(pos[0]) + 0.5,
                f64::from(pos[1]) + 0.5,
                f64::from(pos[2]) + 0.5,
            ],
            6,
        );
        creature::hand::use_one(host, stack);
    }
    grew
}

impl Hooks for Fairies {
    /// Fairy dust used on a block: as bone meal on a crop or a sapling.
    fn use_item_on(&self, host: &mut dyn Host, event: &mut UseItemOnEvent<'_>) -> Flow {
        if &*event.stack.item == DUST && dust_on(host, event.stack, event.pos) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id == FAIRY {
            creature::flutter::register(event.mob, SIZE, &FLIGHT);
            event.mob.ai.goals.add(0, companion::Fairy::default());
        }
    }

    /// Right-click by its friend: it stays where it is, or follows again. A name tag names it as vanilla's would; the
    /// jar is the library's (its hook runs first). Never a trading screen, which its template would open.
    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if event.kind != FAIRY {
            return Flow::Continue;
        }
        if event.stack.is_some_and(|s| &*s.item == "minecraft:name_tag") {
            return Flow::Continue;
        }
        let (Some(player), Ok(id)) = (host.acting_player(), u32::try_from(event.entity)) else {
            return Flow::Cancel;
        };
        if let Some(friend) = creature::friend::of(id).filter(|f| f.is(player.uuid)) {
            let staying = !friend.staying;
            creature::friend::set_staying(id, staying);
            let key = if staying { "fairies.stays" } else { "fairies.follows" };
            host.action_bar(player.uuid, Text::translate(key, &[]));
        }
        Flow::Cancel
    }

    /// A befriended fairy can't be hurt by a player's blow (no accidents, SPECS §2; Horizon gives a player's hit no
    /// attacker id, so it is any player's: its friend's in single player).
    fn living_incoming_damage(&self, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        let ours = creature::mob::kind(event.kind) == FAIRY;
        let player_hit = matches!(
            event.damage_type,
            "minecraft:player_attack" | "minecraft:arrow" | "minecraft:trident"
        );
        let befriended = u32::try_from(event.entity).is_ok_and(|id| creature::friend::of(id).is_some());
        if ours && player_hit && befriended {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind != FAIRY {
            return;
        }
        let fairy = event.entity;
        creature::friend::tick(host, fairy);
        creature::flutter::tick_post(host, event.dimension, fairy);
        let Some(at) = host.entity(fairy).and_then(|e| host.entity_position(e)) else {
            return;
        };
        let ticks = {
            let mut all = TICKS.lock().unwrap();
            let t = all.entry(fairy).or_default();
            *t = t.wrapping_add(1);
            *t
        };
        for nibble in companion::take_nibbles(fairy) {
            nibbled(host, fairy, at, nibble);
        }
        for player in companion::take_fled(fairy) {
            creature::bestiary::learn(host, creature::friend::uuid(player), FAIRY, SHY_FACT);
        }
        let friend = creature::friend::of(fairy);
        if let Some(f) = friend {
            befriended(host, event.dimension, fairy, at, f, ticks);
            // A friend more than `companion::TELEPORT` blocks away: it comes to them at once.
            if !f.staying
                && let Some(p) = friend_player(host, f).filter(|p| p.dimension == event.dimension)
                && (0..3).map(|k| (p.position[k] - at[k]).powi(2)).sum::<f64>().sqrt() > companion::TELEPORT
            {
                let near = [p.position[0], p.position[1] + 2.0, p.position[2]];
                host.teleport_mob(event.dimension, fairy, near);
                sparkles(host, near, 10);
            }
            // A faint trail of sparkles as it follows.
            if !f.staying && ticks.is_multiple_of(8) {
                sparkles(host, [at[0], at[1] + 0.4, at[2]], 1);
            }
        }
        let following = friend.is_some_and(|f| !f.staying);
        AT.lock().unwrap().insert(
            fairy,
            Seen {
                at,
                following,
                when: Instant::now(),
            },
        );
    }

    /// Killed, it vanishes in a puff of sparkles (it drops nothing: fairies are not loot).
    fn living_death(&self, host: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        if creature::mob::kind(event.kind) != FAIRY {
            return Flow::Continue;
        }
        if let Some(at) = host.entity(event.entity).and_then(|e| host.entity_position(e)) {
            sparkles(host, [at[0], at[1] + 0.4, at[2]], 20);
        }
        creature::flutter::gone(event.entity);
        creature::friend::forget(event.entity);
        companion::forget(event.entity);
        AT.lock().unwrap().remove(&event.entity);
        TICKS.lock().unwrap().remove(&event.entity);
        Flow::Continue
    }

    /// It glows: its whole texture is full-bright.
    fn emissive(&self, _: &dyn Host, event: &mut EmissiveEvent<'_>) {
        event.layers.push((TEXTURE, TEXTURE));
    }

    /// A soft violet light around each fairy (seen only, it lights nothing in the world), wider around one following
    /// its friend: its night light.
    fn visuals(&self, _: &dyn Host, event: &mut VisualsEvent<'_>) {
        let mut at = AT.lock().unwrap();
        at.retain(|_, seen| seen.when.elapsed().as_secs() < 2);
        let mut lights: Vec<&Seen> = at.values().collect();
        // The ones following first: their light matters most.
        lights.sort_by_key(|seen| !seen.following);
        for seen in lights.into_iter().take(LIGHTS) {
            let p = seen.at;
            event.lights.push(NativeColoredLight {
                at: [p[0], p[1] + 0.4, p[2]],
                rgb: LIGHT_RGB,
                radius: if seen.following { LIGHT_RADIUS.1 } else { LIGHT_RADIUS.0 },
                intensity: LIGHT_INTENSITY,
            });
        }
    }

    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != FAIRY {
            return Flow::Continue;
        }
        let flying = creature::flutter::flying(mob.id) || companion::active(mob.id);
        // Hovering, it bobs gently.
        let bob = if flying { 0.04 * (mob.age * 0.2).sin() } else { 0.0 };
        event.draw.push();
        creature::model::death_tilt(&mut *event.draw, mob);
        event.draw.translate([0.0, bob, 0.0]);
        creature::model::draw_shown(
            host,
            &mut *event.draw,
            &model_fairy::FAIRY,
            TEXTURE,
            &|part| pose(part, mob, flying),
            &|_| true,
            creature::model::tint(mob),
        );
        event.draw.pop();
        Flow::Cancel
    }
}
