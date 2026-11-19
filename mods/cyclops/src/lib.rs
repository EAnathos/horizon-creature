//! Cyclops (SPECS.md): a one-eyed giant shepherd at the mouths of caves, of the Sunrise Creatures series.
//!
//! A cyclops is a copy of the wandering trader (its behaviour is all goals) with its own 4-block box, drawn with its
//! own model (`model.rs`, from `sources/models/cyclops.bbmodel`). Hostile: it goes for a player it sees within
//! `SIGHT` blocks, and for whoever hurts it or one of its sheep, with a slow overhead blow of its club, or out of
//! reach a boulder hurled over its head (`Smash`, `boulder.rs`). Its eye is its weak spot (`eye.rs`). It appears rarely
//! at a cave mouth near a player (`spawn`), with a flock of sheep that follow it (`flock`). It drops its eye when a player kills it, and
//! sometimes its club; the eye makes a shepherd's crook, which herds farm animals (`crook.rs`).
mod boulder;
mod crook;
mod draw;
mod eye;
mod fight;
mod flock;
mod model;
mod model_boulder;
mod smash;
mod spawn;

use draw::*;
use fight::*;
use flock::*;
use spawn::*;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use creature::bestiary::{Fact, Species};
use creature::body::{Behaviour, Body, World};
use creature::territory::Ground;
use mod_api::{
    AnvilUpdateEvent, BlockEntityDrawContext, ClientHost, Compound, EntityInteractEvent, EntityJoinLevelEvent,
    EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, GoalSpec, Heightmap, HitHost, Hooks, Host, HostSide,
    InteractionResult, ItemPart, LevelRef, LevelTickPostEvent, LivingDeathEvent, LivingIncomingDamageEvent,
    LivingRenderState, MobFilter, RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost,
    SoundSource, Text, TooltipEvent,
};

pub const CYCLOPS: &str = "cyclops:cyclops";
const CLUB: &str = "cyclops:club";
const TEXTURE: &str = "cyclops:entity/cyclops/cyclops";

/// Its box (blocks: width, height, eye height), SPECS §2.
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 1.8,
    height: 4.0,
    eye_height: 3.5,
};
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
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::Rare,
    draw: book_view,
    draw_scale: BOOK_SCALE,
    draw_lift: 0.0,
};

/// Its drawing in a placed jar: standing, its whole height in the view.
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    creature::model::draw(draw, &model::CYCLOPS, TEXTURE, &|_| [0.0; 3]);
}
const BOOK_SCALE: f32 = 0.22;

/// Its sounds (`sources/sounds.toml`, cut from CC0 freesound recordings): its steps, its grumble, hurt and death are
/// its entity type's; the club's whoosh as it comes down, then the blow landing, and its roar as it lifts a boulder,
/// ours.
const SWING: &str = "cyclops:entity.cyclops.swing";
const HIT: &str = "cyclops:entity.cyclops.hit";
const ROAR: &str = "cyclops:entity.cyclops.roar";

const EGG: &str = "cyclops:cyclops_spawn_egg";
/// Vanilla's red (`§c`), packed as tooltip lines take it (`rgba(255, 85, 85, 255)`).
const PEACEFUL_RED: u32 = 0xFF55_55FF;
/// Whether the world is on Peaceful, for the egg's tooltip (from the server's tick).
static PEACEFUL: AtomicBool = AtomicBool::new(false);

/// Whether the world is on Peaceful: no cyclops appears, and those there go, as vanilla's monsters.
fn peaceful(host: &dyn Host, level: LevelRef) -> bool {
    level.difficulty(host) == Some("peaceful")
}

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static EYE_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "cyclops:cyclops_eye",
    dropped_by: &["cyclops:cyclops"],
    given_by: &[],
    uses: &[],
};
static CLUB_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "cyclops:club",
    dropped_by: &["cyclops:cyclops"],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "cyclops:club",
        key: "bestiary.cyclops.item.club.use",
    }],
};
static CROOK_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "cyclops:shepherds_crook",
    dropped_by: &[],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:sheep_spawn_egg",
        key: "bestiary.cyclops.item.shepherds_crook.use",
    }],
};

/// Its spawn egg says "Disabled in Peaceful" in red there, as a vanilla monster's (the game only knows vanilla's
/// monsters; the cyclops copies the wandering trader). The client can't read the difficulty: the server's tick keeps
/// it in `PEACEFUL` (single player).
struct Egg;

#[mod_api::sunrise_part]
impl ItemPart for Egg {
    fn tooltip(&self, _: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
        if PEACEFUL.load(Ordering::Relaxed) {
            let at = event.lines.len().min(1);
            let line = Text::translate("item.spawn_egg.peaceful", vec![]);
            event.lines.insert(at, (line, PEACEFUL_RED));
        }
    }
}

/// Its own model: strides, the club's blow, a boulder lifted over its head, dazed.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob = entity.living.as_ref();
        let now = Moment::of(entity.id);
        draw.push();
        creature::model::living_pose(draw, entity);
        // Its scale is for its client box only (`creature::mob::scale_client_box`): the model keeps its size, whatever
        // scale this frame says.
        let scale = mob.map_or(1.0, |m| m.scale);
        if scale > 0.0 && scale != 1.0 {
            draw.scale([1.0 / scale; 3]);
        }
        let pose = |part: &str| pose(part, mob, &now);
        creature::model::draw_shown(
            draw,
            &model::CYCLOPS,
            TEXTURE,
            &pose,
            &|_| true,
            creature::model::tint(mob),
        );
        if now.holds_boulder() {
            boulder::held(draw, [0.0, HELD_HEIGHT, 0.0]);
        }
        draw.pop();
        Ok(())
    }
}

#[derive(Default)]
struct Cyclops;

#[mod_api::sunrise_mod]
impl Hooks for Cyclops {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(CYCLOPS, SIZE);
        creature::mob::scale_client_box(CYCLOPS);
        creature::projectile::register(boulder::BOULDER, boulder::SIZE);
        // Their attribute rows (a type only its content file declares has none to add to), with the values the
        // content files set, then the attributes their template (a trader) lacks: the blow's and the boulder's.
        let _ = r.declare_entity_attributes(
            "cyclops",
            "minecraft:wandering_trader",
            vec![
                ("minecraft:max_health".into(), 110.0),
                ("minecraft:movement_speed".into(), 0.25),
            ],
        );
        let _ = r.declare_entity_attributes(
            "boulder",
            "minecraft:wandering_trader",
            vec![
                ("minecraft:max_health".into(), 1.0),
                ("minecraft:movement_speed".into(), 0.0),
            ],
        );
        let _ = r.modify_entity_attributes(
            CYCLOPS,
            vec![
                ("minecraft:attack_damage".into(), Some(f64::from(ATTACK_DAMAGE))),
                ("minecraft:attack_knockback".into(), Some(ATTACK_KNOCKBACK)),
                ("minecraft:knockback_resistance".into(), Some(KNOCKBACK_RESISTANCE)),
                ("minecraft:follow_range".into(), Some(SIGHT)),
            ],
        );
        let _ = r.modify_entity_attributes(
            boulder::BOULDER,
            vec![
                ("minecraft:attack_damage".into(), Some(boulder::DAMAGE)),
                ("minecraft:attack_knockback".into(), Some(boulder::KNOCKBACK)),
            ],
        );
        let egg = r.item_key(EGG);
        r.item_part(egg, Egg);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&EYE_PAGE);
        creature::bestiary::register_item(&CLUB_PAGE);
        creature::bestiary::register_item(&CROOK_PAGE);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(CYCLOPS.into(), Arc::new(Draw));
        event
            .renderers
            .register(boulder::BOULDER.into(), Arc::new(boulder::Draw));
    }

    /// A cyclops joining takes vanilla's goals for its wandering and its targets (the library took its template's off
    /// just before, in dependency order): it strolls and looks about, turns on whoever hurts it, goes for a player it
    /// sees. Its fight and its daze run from its tick (`Smash`, `eye::Stunned`).
    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if event.kind != CYCLOPS {
            return Flow::Continue;
        }
        let Some(m) = host.entity(event.entity).and_then(|e| host.mob(e)) else {
            return Flow::Continue;
        };
        // Its client box as tall as it is (`creature::mob::scale_client_box`): set on each cyclops, a change the
        // clients are sent (vanilla scales its shadow too: `cyclops.json`'s radius is its own over this scale).
        if let Some(living) = host.entity(event.entity).and_then(|e| host.living(e)) {
            let scale = creature::mob::client_box_scale(CYCLOPS);
            living.set_attribute_base(host, "minecraft:scale", f64::from(scale));
        }
        m.add_goal(
            host,
            5,
            &GoalSpec::Stroll {
                speed: 0.6,
                interval: 120,
                water_avoiding: Some(0.001),
                swim: false,
                flying: false,
            },
        );
        m.add_goal(
            host,
            6,
            &GoalSpec::LookAtPlayer {
                range: 12.0,
                probability: 0.02,
                only_horizontal: false,
                mobs: false,
                only: None,
            },
        );
        m.add_goal(host, 7, &GoalSpec::RandomLookAround);
        m.add_target_goal(host, 1, &GoalSpec::HurtByTarget { alert: false });
        m.add_target_goal(
            host,
            3,
            &GoalSpec::NearestTarget {
                prey: MobFilter::Player,
                must_see: true,
                must_reach: false,
                interval: 10,
            },
        );
        Flow::Continue
    }

    /// The club and the crook take no enchantment at an anvil either (the table offers none: no `enchantable`): no
    /// result with an enchanted book or an enchanted item on the right. Repairing and renaming still work.
    fn anvil_update(&self, host: HostSide, event: &mut AnvilUpdateEvent<'_>) -> Flow {
        let host: &dyn Host = match &host {
            HostSide::Server(h) => &**h,
            HostSide::Client(h) => &**h,
        };
        let update = &event.update;
        let ours = [CLUB, crook::CROOK].contains(&creature::hand::id(host, &update.left).as_str());
        let enchants = update.right.as_ref().is_some_and(|r| {
            let enchanted = r
                .components()
                .is_some_and(|c| c.as_nbt().contains_key(&"minecraft:enchantments".into()));
            creature::hand::is(host, r, "minecraft:enchanted_book") || enchanted
        });
        if ours && enchants { Flow::Cancel } else { Flow::Continue }
    }

    /// Right-click: nothing (no trading screen, which their template would open), on both sides.
    fn entity_interact(&self, _: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        (event.kind == CYCLOPS || event.kind == boulder::BOULDER).then_some(InteractionResult::Consume)
    }

    /// An arrow or trident in its eye (`eye.rs`); one of its sheep hurt: its cyclops turns on the attacker.
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        eye::struck(event);
        if event.kind == "minecraft:sheep"
            && let Ok(attacker) = u32::try_from(event.source.causing)
            && let Ok(sheep) = u32::try_from(event.entity)
            && let Some(&shepherd) = SHEPHERD.lock().unwrap().get(&sheep)
        {
            ANGERED.lock().unwrap().insert(shepherd, attacker);
        }
        Flow::Continue
    }

    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let level = event.level;
        PEACEFUL.store(peaceful(&*host, level), Ordering::Relaxed);
        if level.dimension() == mod_api::DimensionKey::OVERWORLD {
            spawn(host, level);
        }
        crook::tick(host, level);
        flock(host, level);
        eye::watch(host, level);
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != CYCLOPS && event.kind != boulder::BOULDER {
                return;
            }
            let Some(e) = host.entity(event.entity) else {
                return;
            };
            let Some(level) = e.level(host) else {
                return;
            };
            if peaceful(&*host, level) {
                e.discard(host);
                return;
            }
            if event.kind == boulder::BOULDER {
                boulder::tick(host, event.entity);
                return;
            }
            let id = event.entity;
            if let Some(at) = e.position(host) {
                CYCLOPS_AT.lock().unwrap().insert(id, at);
                eye::tick(host, level, id, at);
            }
            creature::body::tick(host, id, || {
                vec![Box::new(eye::Stunned::default()), Box::new(Smash::default())]
            });
            fight::apply(host, id);
            anger(host, id);
            smash::tick(host, level, id);
            boulder::launch(host, event.dimension);
            creature::sounds::play_queued(host, id, SoundSource::Hostile);
        });
    }

    fn living_death(&self, _: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == CYCLOPS {
            CYCLOPS_AT.lock().unwrap().remove(&event.entity);
            RAISED.lock().unwrap().remove(&event.entity);
            LIFTED.lock().unwrap().remove(&event.entity);
            smash::forget(event.entity);
            ANGERED.lock().unwrap().remove(&event.entity);
            eye::forget(event.entity);
            creature::body::forget(event.entity);
        } else if event.kind == boulder::BOULDER {
            boulder::forget(event.entity);
        } else if event.kind == "minecraft:sheep" {
            FLOCK.lock().unwrap().remove(&event.entity);
            SHEPHERD.lock().unwrap().remove(&event.entity);
        }
        Flow::Continue
    }
}
