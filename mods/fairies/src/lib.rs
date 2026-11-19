//! Fairies (SPECS.md): shy glowing fairies of the flowery woods and meadows, of the Sunrise Creatures series.
//!
//! A fairy is a copy of the wandering trader (its behaviour is all goals) with a small box, flying with the library's
//! `flutter` (out day and night, resting in the rain, landing on flowers) under its own `Fairy` goal (`companion.rs`):
//! shy of players who come near without sneaking, drawn by a player holding glow berries (or sweet berries); fed one
//! (right-click), it may befriend them (the library's `friend`); befriended, it circles its friend's head, or stays
//! where it is told (right-click). Its friend has Speed I while it follows within reach, gets a pinch of fairy dust
//! once a day, and a soft violet light goes with it. Caught in the library's glass jar (only by its friend, once
//! befriended), it stays befriended; on display, a fairy lamp. Drawn full-bright, its wings beating.
mod companion;
mod model_fairy;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use creature::bestiary::{Fact, Species};
use creature::friend::Friend;
use creature::jar::Catchable;
use creature::particles::key;
use mod_api::{
    ActiveEffect, AttachmentCodec, AttachmentError, AttachmentType, BlockEntityDrawContext, ClientHost, ColoredLight,
    Compound, EmissiveEvent, EntityInteractEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow,
    HitHost, Holder, Hooks, HostSide, InteractionResult, ItemStack, LevelRef, LivingDeathEvent,
    LivingIncomingDamageEvent, ParticleOptions, PlayerRef, RegisterEntityRenderersEvent, Registrar,
    RegistrationContext, ServerHost, SoundSource, Tag, Text, VisualsEvent,
};

pub const FAIRY: &str = "fairies:fairy";
const TEXTURE: &str = "fairies:entity/fairy/fairy";
const DUST: &str = "fairies:fairy_dust";

/// Its own sounds (SPECS §1, `sources/sounds.toml`): wings at take-off, a chime or a tiny laugh now and then (one chance
/// in `AMBIENT_CHANCE` a tick), the sparkle of a new friendship (and of a gift); hurt and death are its entity type's.
const FLUTTER: &str = "fairies:entity.fairy.flutter";
const AMBIENT: &str = "fairies:entity.fairy.ambient";
const BEFRIEND: &str = "fairies:entity.fairy.befriend";
const AMBIENT_CHANCE: u64 = 500;
/// Its box (blocks: width, height, eye height); its model is about three quarters of a block tall.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 0.4,
    height: 0.8,
    eye_height: 0.6,
};

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
    takeoff: Some(FLUTTER),
};

/// Caught in the library's glass jar, by a sneaking player or its friend; settling on a flower; a fairy lamp on display
/// (SPECS §5).
static IN_JAR: Catchable = Catchable {
    kind: FAIRY,
    model: "fairies:fairy_jar",
    settles: Some(creature::territory::Rule::FlowerPatch),
    glows: true,
};

/// The bestiary's facts (SPECS §4).
const FRIEND_FACT: &str = "friend";

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
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.0, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    creature::model::draw(draw, &model_fairy::FAIRY, TEXTURE, &|_| [0.0; 3]);
    draw.pop();
}
const BOOK_SCALE: f32 = 1.2;

/// The speed it gives its friend (SPECS §3): Speed I while it follows within `BOOST_REACH` blocks, renewed every
/// `BOOST_EVERY` ticks for a little longer, so it lapses soon after.
const BOOST_REACH: f64 = 8.0;
const BOOST_EVERY: u32 = 40;
const BOOST_TICKS: i32 = 70;
const _: () = assert!(
    BOOST_TICKS > BOOST_EVERY as i32,
    "the speed is renewed before it lapses"
);
/// Its night light (SPECS §3), seen only: violet, wider around a fairy following its friend.
const LIGHTS: usize = 8;
const LIGHT_RGB: u32 = 0xC9A2FF;
const LIGHT_RADIUS: (f32, f32) = (3.0, 8.0);
const LIGHT_INTENSITY: f32 = 0.7;

/// Where each fairy is and whether it follows its friend, for its light (the client's `visuals` has no positions of
/// its own): kept by the server's tick, read by the drawing (single player).
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

/// They come out near players, rarely, in little groups (the library's spawning: SPECS §2).
static SPAWNING: creature::flutter::Spawning = creature::flutter::Spawning {
    kind: FAIRY,
    flight: &FLIGHT,
    biomes: BESTIARY.biomes,
    group: (2, 4),
    cap: 4,
    chance: 12,
};

/// Its item in the guide's Items tab (common/SPECS.md §7.2).
static DUST_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "fairies:fairy_dust",
    dropped_by: &[],
    given_by: &[creature::items::Use {
        icon: "fairies:fairy_spawn_egg",
        key: "bestiary.fairies.item.fairy_dust.given",
    }],
    uses: &[creature::items::Use {
        icon: "minecraft:potion",
        key: "bestiary.fairies.item.fairy_dust.brew",
    }],
};

#[derive(Default)]
struct Fairies;

/// A group's rotation this frame: the wings beat (fast in flight, slowly at rest), the arms sway, the hem flutters.
fn pose(name: &str, age: f32, flying: bool) -> [f32; 3] {
    let beat = if flying {
        22.0 + 26.0 * (age * 1.6).sin()
    } else {
        10.0 + 10.0 * (age * 0.1).sin()
    };
    let sway = 8.0 * (age * 0.15).sin();
    match name {
        "wing_upper_right" | "wing_lower_right" => [0.0, -beat, 0.0],
        "wing_upper_left" | "wing_lower_left" => [0.0, beat, 0.0],
        "arm_right" => [sway, 0.0, 0.0],
        "arm_left" => [-sway, 0.0, 0.0],
        "hem" => [0.0, age * 2.0 % 360.0, 0.0],
        _ => [0.0; 3],
    }
}

/// Its friend's player, if they are on the server.
fn friend_player(host: &dyn ServerHost, friend: Friend) -> Option<PlayerRef> {
    host.players()
        .server_players()
        .into_iter()
        .find(|p| friend.is(p.uuid()))
}

/// The entity id of each befriended fairy's friend, for the damage hook (which has no players to look through).
static FRIEND_IDS: Mutex<BTreeMap<u32, u32>> = Mutex::new(BTreeMap::new());

fn sparkles(host: &mut dyn ServerHost, level: LevelRef, at: [f64; 3], count: i32) {
    if let Some(k) = key("minecraft:end_rod") {
        level.send_particles(
            host,
            &ParticleOptions::simple(k),
            None,
            at,
            [0.25, 0.3, 0.25],
            0.02,
            count,
        );
    }
}

/// A wild fairy fed a berry from a player's hand (SPECS §3): one is used up (not in creative), and one time in 3 (6 for
/// a sweet berry) it befriends them: hearts, a chime, a line on their action bar; otherwise a puff of smoke.
fn fed(host: &mut dyn ServerHost, fairy: u32, stack: &ItemStack, player: PlayerRef, hand: mod_api::Hand) {
    let Some(e) = host.entity(fairy) else {
        return;
    };
    let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
        return;
    };
    let glow = creature::hand::is(&*host, stack, companion::GLOW_BERRIES);
    creature::hand::use_one(host, player, hand);
    level.play_sound(
        host,
        "minecraft:entity.generic.eat",
        at,
        SoundSource::Neutral,
        0.4,
        1.8,
        None,
    );
    let chance = if glow { GLOW_BERRY_CHANCE } else { SWEET_BERRY_CHANCE };
    let eye = [at[0], at[1] + 0.5, at[2]];
    if !creature::colours::Random::new(fairy).next().is_multiple_of(chance) {
        if let Some(k) = key("minecraft:smoke") {
            level.send_particles(host, &ParticleOptions::simple(k), None, eye, [0.2, 0.2, 0.2], 0.01, 5);
        }
        return;
    }
    creature::friend::befriend(host, fairy, player.uuid());
    if let Some(k) = key("minecraft:heart") {
        level.send_particles(host, &ParticleOptions::simple(k), None, eye, [0.3, 0.3, 0.3], 0.02, 6);
    }
    level.play_sound(host, BEFRIEND, at, SoundSource::Neutral, 1.0, 1.0, None);
    let _ = player.action_bar(host, Text::translate("fairies.befriended", vec![]));
    creature::bestiary::learn(host, player.uuid(), FAIRY, FRIEND_FACT);
}

/// One chance in this many that a glow berry befriends; a sweet berry is half as good (SPECS §3).
const GLOW_BERRY_CHANCE: u64 = 3;
const SWEET_BERRY_CHANCE: u64 = 6;
const _: () = assert!(
    GLOW_BERRY_CHANCE < SWEET_BERRY_CHANCE,
    "a glow berry befriends more often"
);

/// What a befriended fairy does for its friend now and then: Speed I near them, a pinch of dust once a day.
fn befriended(host: &mut dyn ServerHost, level: LevelRef, fairy: u32, at: [f64; 3], friend: Friend, ticks: u32) {
    let Some(player) = friend_player(host, friend).filter(|p| p.level(&*host) == Some(level)) else {
        return;
    };
    let Some(position) = player.entity().position(host) else {
        return;
    };
    let near = (0..3).map(|k| (position[k] - at[k]).powi(2)).sum::<f64>().sqrt() < BOOST_REACH;
    if friend.staying || !near {
        return;
    }
    if ticks.is_multiple_of(BOOST_EVERY)
        && let (Some(speed), Some(living)) = (host.effect_key("minecraft:speed"), host.living(player.entity()))
    {
        living.add_effect(host, &ActiveEffect::new(speed, BOOST_TICKS, 0, true, false, true));
    }
    // A pinch of dust at its friend's feet, once a day.
    let Some(day) = level.day_time(host).map(|t| t / 24_000) else {
        return;
    };
    let Ok(last) = DUST_DAY.get_existing(host, Holder::Entity(fairy)) else {
        return;
    };
    if last.unwrap_or(-1) >= day || DUST_DAY.set(host, Holder::Entity(fairy), day).is_err() {
        return;
    }
    if let Some(dust) = host.item_from_snbt(&format!("{{id:\"{DUST}\",count:1}}")) {
        creature::hand::drop_stack(host, level, position, dust);
    }
    sparkles(host, level, position, 8);
    level.play_sound(host, BEFRIEND, at, SoundSource::Neutral, 0.6, 1.3, None);
}

/// Its own model, glowing, its wings beating; it bobs as it hovers.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let flying = creature::flutter::flying(entity.id) || companion::active(entity.id);
        let bob = if flying { 0.04 * (entity.age * 0.2).sin() } else { 0.0 };
        draw.push();
        creature::model::living_pose(draw, entity);
        draw.translate([0.0, bob, 0.0]);
        let pose = |part: &str| pose(part, entity.age, flying);
        creature::model::draw_shown(
            draw,
            &model_fairy::FAIRY,
            TEXTURE,
            &pose,
            &|_| true,
            creature::model::tint(mob),
        );
        draw.pop();
        Ok(())
    }
}

#[mod_api::sunrise_mod]
impl Hooks for Fairies {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let _ = r.register_attachment_type("fairies:dust_day", false);
        creature::flutter::register(FAIRY, SIZE);
        creature::bestiary::register_item(&DUST_PAGE);
        creature::flutter::spawns(&SPAWNING);
        creature::jar::register(&IN_JAR);
        creature::bestiary::register(&BESTIARY);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(FAIRY.into(), Arc::new(Draw));
    }

    /// Right-click: with berries, a wild one eats one (`fed`); by its friend, it stays where it is, or follows again. A
    /// name tag names it as vanilla's would; the jar is the library's (its hook runs first). Never a trading screen,
    /// which its template would open.
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != FAIRY {
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
        let (Some(player), Ok(id)) = (host.players().acting_player(), u32::try_from(event.entity)) else {
            return Some(InteractionResult::Consume);
        };
        let berries = event.stack.filter(|s| {
            let item = creature::hand::id(&*host, s);
            item == companion::GLOW_BERRIES || item == companion::SWEET_BERRIES
        });
        if let Some(stack) = berries
            && creature::friend::of(id).is_none()
        {
            fed(host, id, stack, player, event.hand);
            return Some(InteractionResult::Consume);
        }
        if let Some(friend) = creature::friend::of(id).filter(|f| f.is(player.uuid())) {
            let staying = !friend.staying;
            creature::friend::set_staying(id, staying);
            let key = if staying { "fairies.stays" } else { "fairies.follows" };
            let _ = player.action_bar(host, Text::translate(key, vec![]));
        }
        Some(InteractionResult::Consume)
    }

    /// A befriended fairy can't be hurt by its friend (no accidents, SPECS §2): their blow, their arrow or their
    /// trident (the hit names who caused it).
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        if event.kind != FAIRY {
            return Flow::Continue;
        }
        let friend = u32::try_from(event.entity)
            .ok()
            .and_then(|id| FRIEND_IDS.lock().unwrap().get(&id).copied());
        let by_friend = friend.is_some_and(|f| i64::from(f) == i64::from(event.source.causing));
        if by_friend { Flow::Cancel } else { Flow::Continue }
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != FAIRY {
                return;
            }
            let fairy = event.entity;
            creature::friend::tick(host, fairy);
            creature::flutter::tick_post(host, event.dimension, event.kind, fairy, || {
                vec![
                    Box::new(companion::Fairy::default()),
                    Box::new(creature::flutter::Flutter::new(&FLIGHT)),
                ]
            });
            let Some(e) = host.entity(fairy) else {
                return;
            };
            let (Some(at), Some(level)) = (e.position(host), e.level(host)) else {
                return;
            };
            if creature::colours::Random::new(fairy)
                .next()
                .is_multiple_of(AMBIENT_CHANCE)
            {
                level.play_sound(host, AMBIENT, at, SoundSource::Neutral, 0.7, 1.0, None);
            }
            let ticks = {
                let mut all = TICKS.lock().unwrap();
                let t = all.entry(fairy).or_default();
                *t = t.wrapping_add(1);
                *t
            };
            let friend = creature::friend::of(fairy);
            if let Some(f) = friend {
                let player = friend_player(host, f);
                if let Some(p) = player {
                    FRIEND_IDS.lock().unwrap().insert(fairy, p.entity().id());
                }
                befriended(host, level, fairy, at, f, ticks);
                // A friend more than `companion::TELEPORT` blocks away: it comes to them at once.
                if !f.staying
                    && let Some(p) = player.filter(|p| p.level(&*host) == Some(level))
                    && let Some(pos) = p.entity().position(host)
                    && (0..3).map(|k| (pos[k] - at[k]).powi(2)).sum::<f64>().sqrt() > companion::TELEPORT
                {
                    let near = [pos[0], pos[1] + 2.0, pos[2]];
                    e.teleport_to(host, near, None);
                    sparkles(host, level, near, 10);
                }
                // A faint trail of sparkles as it follows.
                if !f.staying && ticks.is_multiple_of(8) {
                    sparkles(host, level, [at[0], at[1] + 0.4, at[2]], 1);
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
        });
    }

    /// Killed, it vanishes in a puff of sparkles (it drops nothing: fairies are not loot).
    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind != FAIRY {
            return Flow::Continue;
        }
        if let Some(e) = host.entity(event.entity)
            && let (Some(at), Some(level)) = (e.position(host), e.level(host))
        {
            sparkles(host, level, [at[0], at[1] + 0.4, at[2]], 20);
        }
        creature::flutter::gone(event.entity);
        creature::friend::forget(event.entity);
        companion::forget(event.entity);
        AT.lock().unwrap().remove(&event.entity);
        TICKS.lock().unwrap().remove(&event.entity);
        FRIEND_IDS.lock().unwrap().remove(&event.entity);
        Flow::Continue
    }

    /// It glows: its whole texture is full-bright.
    fn emissive(&self, _: &dyn ClientHost, event: &mut EmissiveEvent<'_>) {
        event.layers.push((TEXTURE, TEXTURE));
    }

    /// A soft violet light around each fairy (seen only, it lights nothing in the world), wider around one following
    /// its friend: its night light.
    fn visuals(&self, _: &dyn ClientHost, event: &mut VisualsEvent<'_>) {
        let mut at = AT.lock().unwrap();
        at.retain(|_, seen| seen.when.elapsed().as_secs() < 2);
        let mut lights: Vec<&Seen> = at.values().collect();
        // The ones following first: their light matters most.
        lights.sort_by_key(|seen| !seen.following);
        for seen in lights.into_iter().take(LIGHTS) {
            let p = seen.at;
            let radius = if seen.following { LIGHT_RADIUS.1 } else { LIGHT_RADIUS.0 };
            event.lights.push(ColoredLight::new(
                [p[0], p[1] + 0.4, p[2]],
                LIGHT_RGB,
                radius,
                LIGHT_INTENSITY,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn they_spawn_where_their_bestiary_says_at_any_hour() {
        assert_eq!(SPAWNING.biomes, BESTIARY.biomes);
        assert_eq!(FLIGHT.hours, creature::flutter::Hours::Always);
        assert!(SPAWNING.group.0 <= SPAWNING.group.1);
    }
}
