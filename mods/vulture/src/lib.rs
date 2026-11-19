//! The vulture (mods/vulture/SPECS.md).
//!
//! - `vulture:vulture` copies the wandering trader: the library takes its goals off; it strolls, looks at players and
//!   turns on whoever hurts it with vanilla's goals (added when it joins), and everything else it does runs from its
//!   server tick (the library's `body`): `Sync` (its state, `state.rs`), `Fly` (ridden), `tame::Sit`,
//!   `wild::DiveAttack`, `tame::Mate`, `tame::Eat`, `tame::FollowOwner`, `wild::Soar`.
//! - Mounting: a right-click on it by its owner, once it wears a saddle (`interaction.rs`, `EntityRef::start_riding`),
//!   or `/ride`. Its rider's keys are heard by the server (`ride.rs`, `player_input`); sneaking descends in the air,
//!   getting off only on the ground. The rider sits on its harness: its seat is declared in its entity type
//!   (`passenger_attachments`).
//! - Flight (`flight.rs`): it sets the vulture's speed directly, without gravity, flown like a plane, banking into
//!   arc turns (SPECS §6.2).
//! - Wild (SPECS §4): it soars in circles high up and glides back down (`wild.rs`), walks about, and fights back
//!   in the air (`wild::DiveAttack`). Tamed, it defends its owner and attacks what they attack (`defend`).
//! - Sounds: its hurt and death are its entity type's; its call, flaps and meals ours (`sounds.rs`).
//! - Drawing: the Blockbench model (`sources/models/vulture.bbmodel`, converted into `model.rs`), posed by `pose`, by
//!   its renderer (`Draw`).
//!
//! It depends on the series' library, Sunrise Creatures (`creature`, common/SPECS.md).
mod flight;
mod interaction;
mod model;
mod model_baby;
mod ride;
mod screen;
mod sounds;
mod state;
mod talons;
mod tame;
mod wild;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};

use creature::body::{Behaviour, Body, World};
use flight::{Controls, Flight};
use mod_api::{
    BlockEntityDrawContext, ClientHost, ClientTickPostEvent, EntityInteractEvent, EntityJoinLevelEvent,
    EntityMountEvent, EntityRenderState, EntityRenderer, EntityTickPostEvent, Flow, GoalSpec, HitHost, Hooks, HostSide,
    InteractionResult, KeyRegistration, LivingDeathEvent, LivingIncomingDamageEvent, LivingRenderState,
    MenuClickedEvent, MenuClosedEvent, ParticleOptions, PayloadCodec, PayloadId, PlayerInputEvent,
    RegisterEntityRenderersEvent, Registrar, RegistrationContext, ServerHost, SoundSource, ToServer,
};

pub const KIND: &str = "vulture:vulture";
/// Its hitbox and eye height, in blocks (SPECS §4); its rider sits on its harness (its entity type's
/// `passenger_attachments`).
/// How tall its model stands, for the book's 3D view (its box is lower: the rider sits on its back).
const VIEW_HEIGHT: f32 = 1.4;
const SIZE: creature::mob::Size = creature::mob::Size {
    width: 1.2,
    height: 1.0,
    eye_height: 0.9,
};

/// Its page in the bestiary (common/SPECS.md §7), and what a player can find out about it (SPECS §5, §8).
static BESTIARY: creature::bestiary::Species = creature::bestiary::Species {
    id: KIND,
    // Where it spawns (`content/data/vulture/neoforge/biome_modifier/`: `#is_badlands`, desert, `#is_savanna`).
    biomes: &[
        "minecraft:badlands",
        "minecraft:eroded_badlands",
        "minecraft:wooded_badlands",
        "minecraft:desert",
        "minecraft:savanna",
        "minecraft:savanna_plateau",
        "minecraft:windswept_savanna",
    ],
    wild: |id| state::saved(id).owner.is_none(),
    facts: &[
        creature::bestiary::Fact {
            id: TAME_FACT,
            item: state::TAMING_FOOD,
            key: "bestiary.vulture.vulture.tame",
        },
        creature::bestiary::Fact {
            id: BREED_FACT,
            item: "minecraft:rotten_flesh",
            key: "bestiary.vulture.vulture.breed",
        },
    ],
    icon: "vulture:vulture_spawn_egg",
    scheme: None,
    rarity: creature::bestiary::Rarity::Uncommon,
    draw: book_view,
};

/// The vulture in the bestiary's 3D view: its model as built (wings spread), without harness or chests, scaled to about
/// a block across.
fn book_view(draw: &mut dyn BlockEntityDrawContext, _: Option<creature::colours::Look>) {
    draw.push();
    draw.translate([0.5, 0.0, 0.5]);
    draw.scale([BOOK_SCALE; 3]);
    let worn = |part: &str| !matches!(part, "harness" | "chest_left" | "chest_right");
    creature::model::draw_shown(
        draw,
        &model::VULTURE,
        TEXTURE,
        &|_| [0.0; 3],
        &worn,
        creature::model::WHITE,
    );
    draw.pop();
}
const BOOK_SCALE: f32 = 0.33;
/// Learnt once a vulture ate a raw chicken the player threw, and once the player fed one rotten flesh.
pub const TAME_FACT: &str = "tame";
pub const BREED_FACT: &str = "breed";

/// Its items in the guide's Items tab (common/SPECS.md §7.2).
static FEATHER_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "vulture:feather",
    dropped_by: &["vulture:vulture"],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "minecraft:feather",
        key: "bestiary.vulture.item.feather.use",
    }],
};
static RAW_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "vulture:raw_vulture",
    dropped_by: &["vulture:vulture"],
    given_by: &[],
    uses: &[creature::items::Use {
        icon: "vulture:raw_vulture",
        key: "bestiary.vulture.item.raw_vulture.use",
    }],
};
static COOKED_PAGE: creature::items::ItemPage = creature::items::ItemPage {
    item: "vulture:cooked_vulture",
    dropped_by: &[],
    given_by: &[creature::items::Use {
        icon: "minecraft:flint_and_steel",
        key: "bestiary.vulture.item.cooked_vulture.burnt",
    }],
    uses: &[creature::items::Use {
        icon: "vulture:cooked_vulture",
        key: "bestiary.vulture.item.cooked_vulture.use",
    }],
};

#[derive(Default)]
struct Vulture;

/// Walking speed on the ground, ridden, in blocks per tick: as fast as it walks by itself.
const WALK: f64 = 0.1;
/// Backing up on the ground (S), a share of `WALK`.
const BACK: f64 = 0.4;
/// Its peck when it fights (SPECS §4).
const ATTACK_DAMAGE: f32 = 3.0;

/// The pitch it takes, ridden, climbing (Space) and descending (sneak), in degrees.
const CLIMB_PITCH: f32 = 35.0;
const DESCEND_PITCH: f32 = 35.0;
/// How far A or D, held in the air, sets its wanted heading to that side of the look, in degrees.
const STEER: f32 = 45.0;

/// Ridden flight (SPECS §6.2): the rider's look turns it and the keys steer `flight::Flight`. W flaps up to the
/// cruising speed, S brakes, A / D steer to a side, Space climbs with strong flaps (and takes off from the ground), sneak
/// descends (getting off only on the ground, `ride.rs`); on the ground it walks, and S backs it up slowly.
#[derive(Debug, Default)]
struct Fly {
    flight: Flight,
    on: bool,
}

impl Behaviour for Fly {
    fn can_use(&mut self, m: &mut Body, _: &World<'_>) -> bool {
        let on = ride::of(m.id).is_some();
        if on && !self.on {
            self.flight = Flight::default();
            m.body_yaw = m.yaw;
        }
        self.on = on;
        on
    }
    fn stop(&mut self, m: &mut Body, _: &World<'_>) {
        self.on = false;
        self.flight.land(m);
        talons::release(m.id);
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        let Some(r) = ride::of(m.id) else {
            return;
        };
        if m.on_ground && !r.jumping {
            // On the ground: a slow walk where the rider looks, backing up slower still.
            self.flight.land(m);
            m.yaw = r.yaw;
            m.body_yaw = r.yaw;
            m.head_yaw = r.yaw;
            let (s, c) = f64::from(r.yaw).to_radians().sin_cos();
            let (fwd, strafe) = (f64::from(r.forward), f64::from(r.strafe));
            let fwd = if fwd < 0.0 { fwd * BACK } else { fwd };
            m.velocity.x = (strafe * c - fwd * s) * WALK;
            m.velocity.z = (fwd * c + strafe * s) * WALK;
            return;
        }
        // Space climbs, sneak descends, nothing keeps it level; the look only turns it.
        let pitch = if r.jumping {
            -CLIMB_PITCH
        } else if r.descend {
            DESCEND_PITCH
        } else {
            0.0
        };
        // A (a positive strafe) turns it left, towards a lower yaw.
        let controls = Controls {
            yaw: r.yaw - r.strafe.clamp(-1.0, 1.0) * STEER,
            pitch,
            flap: r.forward > 0.0,
            brake: r.forward < 0.0,
            boost: r.jumping,
            ridden: true,
        };
        self.flight.tick(m, &controls);
        if talons::take_press() {
            talons::toggle(m, cx);
        }
        talons::follow(m);
    }
}

/// Every tick, beside what it does: its state (`state::sync`), no fall damage (SPECS §6), its hearts, its call.
#[derive(Debug)]
struct Sync;

impl Behaviour for Sync {
    fn claims(&self) -> bool {
        false
    }
    fn tick(&mut self, m: &mut Body, cx: &World<'_>) {
        m.fall_distance = 0.0;
        let fighting = tame::fighting(m, cx);
        // A baby fed raw meat: a burst of hearts as it grows (SPECS §8).
        if state::sync(m, ride::of(m.id).is_some(), fighting) {
            particles(
                m.id,
                "minecraft:heart",
                [m.position.x, m.position.y + 0.8, m.position.z],
                5,
            );
        }
        // In love: hearts now and then, as vanilla's animals show (SPECS §8).
        let love = state::love(m.id);
        if love > 0 && love % 10 == 0 {
            particles(
                m.id,
                "minecraft:heart",
                [m.position.x, m.position.y + 1.0, m.position.z],
                1,
            );
        }
        // A quiet bird: one call every 20 s on average (a vanilla mob: every 4 s).
        if m.random.next_int_bound(400).unwrap_or(1) == 0 {
            let pitch = 0.8 + m.random.next_float() * 0.3;
            sounds::queue(m.id, sounds::AMBIENT, 1.0, pitch);
        }
    }
}

/// A vulture's target, as its mob holds it: who, where, its eyes' height.
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub id: u32,
    pub at: [f64; 3],
    pub eye_y: f64,
}

/// Its target, if alive and not its owner (a tamed vulture never turns on its owner, even when they hit it: the
/// target is let go, `LET_GO`).
pub fn target(cx: &World<'_>, id: u32) -> Option<Target> {
    let host = cx.host();
    let t = host.entity(id).and_then(|e| host.mob(e))?.target(host)?;
    let e = t.entity();
    if !t.alive(host).unwrap_or(false) {
        return None;
    }
    let owner = state::saved(id).owner;
    if owner.is_some()
        && cx
            .player
            .as_ref()
            .is_some_and(|p| p.entity_id == e.id() && owner == Some(creature::friend::ints(p.uuid)))
    {
        LET_GO.lock().unwrap().push(id);
        return None;
    }
    let at = e.position(host)?;
    let eye_y = at[1] + e.dimensions(host).map_or(1.6, |d| d[1] * 0.85);
    Some(Target { id: e.id(), at, eye_y })
}

/// What its behaviours asked its tick to do, as they can't (`apply`): strikes, particles, a target let go.
static STRIKES: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
/// Particles asked for: (vulture, kind, where, how many).
type Particles = Vec<(u32, &'static str, [f64; 3], i32)>;
static PARTICLES: Mutex<Particles> = Mutex::new(Vec::new());
static LET_GO: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// From a behaviour: vulture `id` strikes `target` with its beak.
pub fn strike(id: u32, target: u32) {
    STRIKES.lock().unwrap().push((id, target));
}

/// From a behaviour: particles `kind` around `at`.
pub fn particles(id: u32, kind: &'static str, at: [f64; 3], count: i32) {
    PARTICLES.lock().unwrap().push((id, kind, at, count));
}

/// Hits seen by the damage hook (victim, who caused it), for the vultures defending their owners (`defend`).
static HITS: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());

/// From vulture `id`'s tick: what its behaviours asked, done; it defends its owner.
fn apply(host: &mut dyn ServerHost, id: u32) {
    let Some(me) = host.entity(id) else {
        return;
    };
    let Some(level) = me.level(host) else {
        return;
    };
    let mine = |list: &Mutex<Vec<(u32, u32)>>| {
        let mut all = list.lock().unwrap();
        let (mine, rest): (Vec<_>, Vec<_>) = all.drain(..).partition(|s| s.0 == id);
        *all = rest;
        mine
    };
    for (_, target) in mine(&STRIKES) {
        if let Some(t) = host.entity(target) {
            if let Some(l) = host.living(me) {
                l.swing(host, mod_api::Hand::Main, true);
            }
            t.hurt(host, "minecraft:mob_attack", ATTACK_DAMAGE, Some(me), Some(me));
        }
    }
    let shown: Vec<_> = {
        let mut all = PARTICLES.lock().unwrap();
        let (mine, rest): (Vec<_>, Vec<_>) = all.drain(..).partition(|p| p.0 == id);
        *all = rest;
        mine
    };
    for (_, kind, at, count) in shown {
        if let Some(k) = creature::particles::key(kind) {
            level.send_particles(
                host,
                &ParticleOptions::simple(k),
                None,
                at,
                [0.4, 0.4, 0.4],
                0.02,
                count,
            );
        }
    }
    let let_go = {
        let mut all = LET_GO.lock().unwrap();
        let had = all.contains(&id);
        all.retain(|v| *v != id);
        had
    };
    if let_go && let Some(m) = host.mob(me) {
        m.set_target(host, None);
    }
    defend(host, id);
}

/// A tamed vulture, free (not sitting, not ridden), turns on what hurt its owner and on what its owner hurt, as
/// vanilla's wolf (`OwnerHurtByTargetGoal`, `OwnerHurtTargetGoal`).
fn defend(host: &mut dyn ServerHost, id: u32) {
    let saved = state::saved(id);
    let Some(owner) = saved.owner else {
        return;
    };
    if saved.sitting || ride::of(id).is_some() {
        return;
    }
    let owner = creature::friend::uuid(owner);
    let Some(owner) = host.players().server_players().into_iter().find(|p| p.uuid() == owner) else {
        return;
    };
    let owner = owner.entity().id();
    let hits: Vec<(u32, u32)> = HITS.lock().unwrap().clone();
    let target = hits.iter().rev().find_map(|&(victim, by)| {
        if victim == owner && by != id {
            Some(by)
        } else if by == owner && victim != id {
            Some(victim)
        } else {
            None
        }
    });
    if let (Some(t), Some(m)) = (
        target.and_then(|t| host.entity(t)).and_then(|e| host.living(e)),
        host.entity(id).and_then(|e| host.mob(e)),
    ) {
        m.set_target(host, Some(t));
    }
}

/// The variant's texture (only one for now: tawny).
const TEXTURE: &str = "vulture:entity/vulture/tawny";

/// How far it settles when sitting, in blocks.
const SIT_DROP: f32 = 0.3;
/// How long a peck lasts, in ticks.
const PECK: f32 = 10.0;
/// How long a grab with the talons lasts, in ticks.
const GRAB: f32 = 12.0;

/// The chick's model is drawn from the first scale at birth to the second; the juvenile is the adult's model, from
/// `JUVENILE_SCALE` (as tall as the grown chick: 14 × 0.8 ≈ 18 × 0.62 pixels) to full size.
const CHICK_SCALE: (f32, f32) = (0.6, 0.8);
const JUVENILE_SCALE: f32 = 0.62;
const BABY_TEXTURE: &str = "vulture:entity/vulture/baby";

/// How far its back slopes down to the tail on the ground, in degrees.
const HUNCH: f32 = 18.0;

/// What a frame of a vulture shows, for `pose`.
struct Frame {
    age: f32,
    /// Its head's turn from its body (where it looks: the bank), and its pitch (degrees).
    head_yaw: f32,
    pitch: f32,
    walk_position: f32,
    walk_speed: f32,
    flying: bool,
}

/// A group's rotation for this frame, in degrees (x, y, z), on top of its rest rotation.
///
/// In flight the body takes the flight's pitch and banks with the turn: the gap between its heading and its head
/// (where the rider looks). On the ground the wings fold along the flanks of its hunched body: the arm turned back with
/// its coverts outward, the forearm folded forward over it, the hand back again, its primaries along the tail.
fn pose(name: &str, mob: &Frame, flapping: bool, look: &state::Look, carrying: bool) -> [f32; 3] {
    let flying = mob.flying;
    // A peck: the head goes down and back up after it ate; a strike, quicker.
    let since_meal = mob.age - look.ate_at as f32;
    let peck = if look.ate_at > 0 && (0.0..PECK).contains(&since_meal) {
        (since_meal / PECK * std::f32::consts::PI).sin() * 70.0
    } else if look.striking {
        (mob.age * 1.2).sin().abs() * 45.0
    } else {
        0.0
    };

    let bank = (mob.head_yaw * 0.8).clamp(-50.0, 50.0);
    // Flapping: the wings go up and down, the outer segments following a little later; gliding, they are held still
    // with a slight dihedral, swept back the steeper it dives.
    let (beat, lag, late) = if flapping {
        (
            (mob.age * 0.5).sin() * 35.0,
            (mob.age * 0.5 - 0.8).sin() * 20.0,
            (mob.age * 0.5 - 1.4).sin() * 15.0,
        )
    } else {
        (6.0, 0.0, 4.0)
    };
    let sweep = if flying {
        (mob.pitch - 20.0).clamp(0.0, 40.0)
    } else {
        0.0
    };
    // Walking: like a vanilla quadruped's legs; in flight, the legs are tucked in.
    let step = (mob.walk_position * 0.6662).cos() * 1.4 * mob.walk_speed.min(1.0);
    let mut leg = if flying { 60.0 } else { step.to_degrees() };
    // A grab with the talons (SPECS §6.4): the legs thrust down and forward, then fold back, caught or missed.
    let since_grab = mob.age - look.grabbed_at as f32;
    if flying && look.grabbed_at > 0 && (0.0..GRAB).contains(&since_grab) {
        leg -= (since_grab / GRAB * std::f32::consts::PI).sin() * 110.0;
    }
    if !flying {
        return match name {
            // Hunched: the back slopes down to the tail, the head and legs straightened against it.
            "body" => [-HUNCH, 0.0, 0.0],
            // The model faces +z: looking right (positive yaw) turns it towards -x.
            "head" => [mob.pitch + HUNCH + peck, -mob.head_yaw, 0.0],
            // Sitting: legs folded forward under the belly.
            "leg_left" | "leg_right" if look.sitting => [HUNCH - 80.0, 0.0, 0.0],
            // The saddlebags swing out from under the folded wings.
            "chest_left" => [0.0, 0.0, 35.0],
            "chest_right" => [0.0, 0.0, -35.0],
            "wing_left" => [-90.0, 0.0, 90.0],
            "wing_right" => [-90.0, 0.0, -90.0],
            "wing_left_mid" | "wing_right_mid" => [0.0, 0.0, 180.0],
            "wing_left_tip" => [0.0, 178.0, 0.0],
            "wing_right_tip" => [0.0, -178.0, 0.0],
            "leg_left" => [HUNCH + leg, 0.0, 0.0],
            "leg_right" => [HUNCH - leg, 0.0, 0.0],
            _ => [0.0; 3],
        };
    }
    match name {
        "body" => [mob.pitch, 0.0, bank],
        // The head looks into the turn, and a little up against a dive.
        "head" => [-mob.pitch * 0.5 + peck, -mob.head_yaw * 0.5, 0.0],
        "wing_left" => [0.0, sweep, beat],
        "wing_right" => [0.0, -sweep, -beat],
        "wing_left_mid" => [0.0, sweep * 0.5, lag],
        "wing_right_mid" => [0.0, -sweep * 0.5, -lag],
        "wing_left_tip" => [0.0, 0.0, late],
        "wing_right_tip" => [0.0, 0.0, -late],
        // Carrying, the legs hang down to hold the catch.
        "leg_left" | "leg_right" if carrying => [leg.min(0.0), 0.0, 0.0],
        "leg_left" | "leg_right" => [leg, 0.0, 0.0],
        "tail" => [-10.0, 0.0, -bank * 0.3],
        _ => [0.0; 3],
    }
}

/// Whether each vulture is off the ground, from its server tick, for the drawing (single player).
static FLYING: Mutex<BTreeMap<u32, bool>> = Mutex::new(BTreeMap::new());

/// The Blockbench model (`model.rs`), posed by `pose` every frame.
struct Draw;

impl EntityRenderer for Draw {
    fn submit(&self, entity: &EntityRenderState, draw: &mut dyn BlockEntityDrawContext) -> Result<(), String> {
        let mob: Option<&LivingRenderState> = entity.living.as_ref();
        let frame = Frame {
            age: entity.age,
            head_yaw: mob.map_or(0.0, |m| m.y_rot),
            pitch: mob.map_or(0.0, |m| m.x_rot),
            walk_position: mob.map_or(0.0, |m| m.walk_animation_pos),
            walk_speed: mob.map_or(0.0, |m| m.walk_animation_speed),
            flying: FLYING.lock().unwrap().get(&entity.id).copied().unwrap_or(false),
        };
        let flapping = flight::flapping(entity.id);
        let look = state::look(entity.id);
        let carrying = talons::carrying(entity.id);
        let shown = |part: &str| match part {
            "harness" => look.harness,
            "chest_left" => look.chests >= 1,
            "chest_right" => look.chests >= 2,
            _ => true,
        };
        // A baby grows little by little (SPECS §8): a chick of its own model (§7: a big head and down) for the first
        // half, then a juvenile, the adult's model growing to full size; both have the same groups and poses.
        let (part, texture, scale) = if look.growth < state::JUVENILE {
            let t = look.growth / state::JUVENILE;
            (
                &model_baby::BABY,
                BABY_TEXTURE,
                CHICK_SCALE.0 + (CHICK_SCALE.1 - CHICK_SCALE.0) * t,
            )
        } else {
            let t = (look.growth - state::JUVENILE) / (1.0 - state::JUVENILE);
            (&model::VULTURE, TEXTURE, JUVENILE_SCALE + (1.0 - JUVENILE_SCALE) * t)
        };
        draw.push();
        creature::model::living_pose(draw, entity);
        if scale < 1.0 {
            draw.scale([scale; 3]);
        }
        // Sitting: settled on its belly, legs folded under it.
        if look.sitting {
            draw.translate([0.0, -SIT_DROP, 0.0]);
        }
        let pose = |p: &str| pose(p, &frame, flapping, &look, carrying);
        creature::model::draw_shown(draw, part, texture, &pose, &shown, creature::model::tint(mob));
        draw.pop();
        Ok(())
    }
}

/// The talons key, sent from the client to the server.
#[derive(PayloadCodec)]
struct TalonsPress {
    key: String,
}

static TALONS: OnceLock<ToServer<TalonsPress>> = OnceLock::new();

#[mod_api::sunrise_mod]
impl Hooks for Vulture {
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        creature::mob::ours(KIND, SIZE);
        // Its model stands taller than its box (its neck, its folded wings): the book shows it by that.
        creature::bestiary::view_size(KIND, SIZE.width, VIEW_HEIGHT);
        let _ = r.register_attachment_type("vulture:state", false);
        screen::declare(r);
        creature::bestiary::register(&BESTIARY);
        creature::bestiary::register_item(&FEATHER_PAGE);
        creature::bestiary::register_item(&RAW_PAGE);
        creature::bestiary::register_item(&COOKED_PAGE);
        r.register_key_mapping(
            KeyRegistration::new(talons::KEY, talons::DEFAULT_KEY)
                .label("key.vulture.talons")
                .category("key.categories.vulture"),
        );
        let mut payloads = r.payload_handlers();
        let mut view = payloads.registrar("1");
        let token = view.play_to_server_typed::<TalonsPress>(PayloadId("vulture:talons".into()), |_, _| {
            talons::press();
            Ok(())
        });
        let _ = TALONS.set(token);
    }

    fn register_entity_renderers(&self, _: &RegistrationContext, event: &mut RegisterEntityRenderersEvent<'_>) {
        event.renderers.register(KIND.into(), Arc::new(Draw));
    }

    /// Its vanilla goals, added when it joins (the library took its template's off just before): it strolls, looks at
    /// players, and turns on whoever hurts it, with the vultures near it (SPECS §4).
    fn entity_join_level(&self, host: &mut dyn ServerHost, event: &mut EntityJoinLevelEvent<'_>) -> Flow {
        if event.kind != KIND {
            return Flow::Continue;
        }
        let Some(m) = host.entity(event.entity).and_then(|e| host.mob(e)) else {
            return Flow::Continue;
        };
        m.add_goal(
            host,
            6,
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
            7,
            &GoalSpec::LookAtPlayer {
                range: 8.0,
                probability: 0.02,
                only_horizontal: false,
                mobs: false,
                only: None,
            },
        );
        m.add_target_goal(host, 3, &GoalSpec::HurtByTarget { alert: true });
        Flow::Continue
    }

    /// Right-click (`interaction.rs`). Always taken, on both sides: no trading.
    fn entity_interact(&self, host: HostSide, event: &mut EntityInteractEvent<'_>) -> Option<InteractionResult> {
        if event.kind != KIND {
            return None;
        }
        if let (HostSide::Server(host), Ok(id)) = (host, u32::try_from(event.entity)) {
            interaction::clicked(host, id, event);
        }
        Some(InteractionResult::Consume)
    }

    /// The rider's keys, for the ridden flight.
    fn player_input(&self, _: &mut dyn ServerHost, event: &mut PlayerInputEvent<'_>) {
        ride::heard(event.player, event.flags);
    }

    /// Sneaking in the air descends: a rider getting off a vulture that isn't on the ground stays on.
    fn entity_mount(&self, host: &mut dyn ServerHost, event: &mut EntityMountEvent<'_>) -> Flow {
        if event.vehicle_kind == KIND && !event.mounting && ride::keep_seated(&*host, event.vehicle) {
            return Flow::Cancel;
        }
        Flow::Continue
    }

    /// Hits on and by its owner, for `defend`.
    fn living_incoming_damage(&self, _: &mut dyn HitHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow {
        if let (Ok(victim), Ok(by)) = (u32::try_from(event.entity), u32::try_from(event.source.causing)) {
            let mut hits = HITS.lock().unwrap();
            hits.push((victim, by));
            let excess = hits.len().saturating_sub(16);
            hits.drain(..excess);
        }
        Flow::Continue
    }

    /// Its screen's clicks (`screen.rs`).
    fn menu_clicked(&self, host: &mut dyn ServerHost, event: &mut MenuClickedEvent) {
        screen::clicked(host, event);
    }

    fn menu_closed(&self, _: &mut dyn ServerHost, event: &mut MenuClosedEvent) {
        screen::closed(event);
    }

    fn living_death(&self, host: &mut dyn ServerHost, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            screen::died(host, event.entity);
            talons::release(event.entity);
            ride::forget(event.entity);
            creature::friend::forget(event.entity);
            creature::body::forget(event.entity);
            FLYING.lock().unwrap().remove(&event.entity);
        }
        Flow::Continue
    }

    /// The talons key (SPECS §6.4), sent to the server, where the ridden vulture takes it. A mod's own mapping is read
    /// from its clicks (`consume_click`): `key_mapping_pre` tells only vanilla's.
    fn client_tick_post(&self, host: &mut dyn ClientHost, _: &mut ClientTickPostEvent) {
        while host.consume_click("vulture", talons::KEY) {
            if let Some(token) = TALONS.get() {
                let _ = token.send(
                    host,
                    &TalonsPress {
                        key: talons::KEY.into(),
                    },
                );
            }
        }
    }

    fn entity_tick_post(&self, host: HostSide, event: &mut EntityTickPostEvent<'_>) {
        creature::mob::each_ticked(host, event, |host, event| {
            if event.kind != KIND {
                return;
            }
            let id = event.entity;
            water_escape(host, id);
            ride::read(&*host, id);
            creature::body::tick(host, id, || {
                vec![
                    Box::new(Sync),
                    Box::new(Fly::default()),
                    Box::new(tame::Sit),
                    Box::new(wild::DiveAttack::default()),
                    Box::new(tame::Mate::default()),
                    Box::new(tame::Eat::default()),
                    Box::new(tame::FollowOwner::default()),
                    Box::new(wild::Soar::new()),
                ]
            });
            apply(host, id);
            creature::sounds::play_queued(host, id, SoundSource::Neutral);
            state::save(host, event.dimension, id);
            if let Some(e) = host.entity(id) {
                FLYING.lock().unwrap().insert(id, !e.on_ground(host).unwrap_or(true));
                if let Some(level) = e.level(host) {
                    state::look_for_food(host, level, id);
                }
            }
            talons::carry(host, id);
            state::births(host, event.dimension);
            for player in state::take_taming_seen() {
                creature::bestiary::learn(host, player, KIND, TAME_FACT);
            }
        });
    }
}

/// A vulture cannot carry a rider into water. Eject everyone as soon as it enters, then keep swimming it upward and
/// toward the nearest dry shore block until it leaves the water.
fn water_escape(host: &mut dyn ServerHost, id: u32) {
    let Some(entity) = host.entity(id) else { return };
    if !entity.is_in_water(host).unwrap_or(false) {
        return;
    }
    entity.eject_passengers(host);
    let (Some(at), Some(level)) = (entity.position(host), entity.level(host)) else {
        return;
    };
    let ground = creature::territory::Ground::new(&*host, level);
    let wet = |name: &str| name.contains("water") || name == "minecraft:kelp" || name.contains("seagrass");
    let origin = [at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32];
    let mut shore = None;
    'rings: for radius in 1_i32..=8 {
        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if dx.abs() != radius && dz.abs() != radius {
                    continue;
                }
                let feet = [origin[0] + dx, origin[1], origin[2] + dz];
                let (block, below) = (ground.block(feet), ground.block([feet[0], feet[1] - 1, feet[2]]));
                if !block.is_empty() && !wet(&block) && !wet(&below) && !below.ends_with("air") {
                    shore = Some([f64::from(feet[0]) + 0.5, at[1], f64::from(feet[2]) + 0.5]);
                    break 'rings;
                }
            }
        }
    }
    let (vx, vz) = shore.map_or((0.0, 0.0), |to| {
        let dx = to[0] - at[0];
        let dz = to[2] - at[2];
        let length = dx.hypot(dz).max(0.001);
        (dx / length * 0.18, dz / length * 0.18)
    });
    entity.set_velocity(host, [vx, 0.3, vz]);
}
