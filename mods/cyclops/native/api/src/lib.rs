//! Cyclops (SPECS.md): a one-eyed giant shepherd at the mouths of caves, of the Sunrise Creatures series.
//!
//! A cyclops is a copy of the wandering trader (its behaviour is all goals) with its own 4-block box, drawn with its
//! own model (`model.rs`, from `assets/models/cyclops.bbmodel`). Hostile: it goes for a player it sees within
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
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use creature::bestiary::{Fact, Species};
use horizon_client::mobs::Who;
use horizon_client::mobs::goals::{self, HurtByTarget, LookAtPlayer, NearestTarget, RandomLookAround, Stroll};
use horizon_client::mod_api::{
    AnvilUpdateEvent, BlockEntityDraw, Compound, Cx, EntityTickPostEvent, Flow, Goal, Hooks, Host, InteractEntityEvent,
    LevelTickPostEvent, LivingDeathEvent, LivingIncomingDamageEvent, Mob, MobSubmitEvent, NativeHeightmap,
    NativeMobRender, RandomSource, RegisterGoalsEvent, Text, TooltipEvent,
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
    icon: EGG,
    scheme: None,
    rarity: creature::bestiary::Rarity::Rare,
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

/// Its sounds (`assets/sounds.toml`, cut from CC0 freesound recordings).
const VOICE: creature::sounds::Voice = creature::sounds::Voice {
    hurt: "cyclops:entity.cyclops.hurt",
    death: "cyclops:entity.cyclops.death",
};
const AMBIENT: &str = "cyclops:entity.cyclops.ambient";
const STEP: &str = "cyclops:entity.cyclops.step";
/// The club: its whoosh as it comes down, then the blow landing.
const SWING: &str = "cyclops:entity.cyclops.swing";
const HIT: &str = "cyclops:entity.cyclops.hit";
/// Its roar as it lifts a boulder.
const ROAR: &str = "cyclops:entity.cyclops.roar";
/// One chance in this many, a tick it walks, of a step heard.
const STEP_CHANCE: u64 = 14;
/// One chance in this many, a tick, of its grumble.
const AMBIENT_CHANCE: u64 = 500;

const EGG: &str = "cyclops:cyclops_spawn_egg";
/// Vanilla's red (`§c`), packed as tooltip lines take it (`rgba(255, 85, 85, 255)`).
const PEACEFUL_RED: u32 = 0xFF55_55FF;
/// Whether the world is on Peaceful, for the egg's tooltip (from the server's tick).
static PEACEFUL: AtomicBool = AtomicBool::new(false);

/// Whether the world is on Peaceful: no cyclops appears, and those there go, as vanilla's monsters.
fn peaceful(host: &dyn Host, dimension: &str) -> bool {
    host.level(dimension).and_then(|l| l.difficulty(host)) == Some("peaceful")
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
    if moved && random.next().is_multiple_of(STEP_CHANCE) {
        // One recorded stomp, a little higher or lower each time.
        let pitch = 0.9 + (random.next() % 100) as f32 / 100.0 * 0.2;
        creature::sounds::queue(entity, STEP, 1.0, pitch);
    }
    if random.next().is_multiple_of(AMBIENT_CHANCE) {
        creature::sounds::queue(entity, AMBIENT, 1.2, 1.0);
    }
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

pub fn hooks() -> Box<dyn Hooks> {
    creature::bestiary::register(&BESTIARY);
    creature::bestiary::register_item(&EYE_PAGE);
    creature::bestiary::register_item(&CLUB_PAGE);
    creature::bestiary::register_item(&CROOK_PAGE);
    Box::new(Cyclops)
}

struct Cyclops;

impl Hooks for Cyclops {
    /// Its row in Options > Mods, which switches it off and on at once (`creature::list_mod`).
    fn mods(&self, _: &mut dyn Host, event: &mut horizon_client::mod_api::ModsEvent<'_>) {
        creature::list_mod(
            event,
            "cyclops",
            concat!("Cyclops ", env!("CARGO_PKG_VERSION")),
            "A one-eyed giant shepherd at the mouths of caves. Needs Sunrise Creatures.",
        );
    }

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

    /// Its spawn egg says "Disabled in Peaceful" in red there, as a vanilla monster's (Horizon only knows vanilla's
    /// monsters; the cyclops copies the wandering trader). The client can't read the difficulty: the server's tick
    /// keeps it in `PEACEFUL` (single player; arismoko/horizon#2740).
    fn tooltip(&self, _: &dyn Host, event: &mut TooltipEvent<'_>) {
        if &*event.stack.item == EGG && PEACEFUL.load(Ordering::Relaxed) {
            let at = event.lines.len().min(1);
            event
                .lines
                .insert(at, (Text::translate("item.spawn_egg.peaceful", &[]), PEACEFUL_RED));
        }
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
        PEACEFUL.store(peaceful(host, event.dimension), Ordering::Relaxed);
        if event.dimension == "minecraft:overworld" {
            spawn(host, event.dimension);
        }
        crook::tick(host, event.dimension);
        flock(host, event.dimension);
        eye::watch(host, event.dimension);
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if (event.kind == CYCLOPS || event.kind == boulder::BOULDER) && peaceful(host, event.dimension) {
            host.remove_mob(event.dimension, event.entity);
            return;
        }
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
        smash::tick(host, event.dimension, event.entity);
        boulder::launch(host, event.dimension);
        creature::sounds::tick_post(host, event, &VOICE, creature::sounds::HOSTILE);
    }

    fn living_death(&self, _: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        let kind = creature::mob::kind(event.kind);
        if kind == CYCLOPS {
            CYCLOPS_AT.lock().unwrap().remove(&event.entity);
            RAISED.lock().unwrap().remove(&event.entity);
            LIFTED.lock().unwrap().remove(&event.entity);
            smash::forget(event.entity);
            LAST_AT.lock().unwrap().remove(&event.entity);
            ANGERED.lock().unwrap().remove(&event.entity);
            eye::forget(event.entity);
        } else if kind == boulder::BOULDER {
            boulder::forget(event.entity);
        } else if kind == "minecraft:sheep" {
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
