//! The vulture (mods/vulture/SPECS.md).
//!
//! - `vulture:vulture` copies the wandering trader (content part, `content/`): we clear its goals and give it ours,
//!   `NoFall` and `Fly`.
//! - Mounting: a right-click on it by its owner, once it wears a harness (`interact.rs` injection), or `/ride`. Horizon then copies the rider's keys into
//!   `Mob.rider` every tick (`game/mounts.rs`, `tick_mount_riding`): forward/back, left/right, jump (Space), sprint
//!   (Ctrl). Sneaking gets off.
//! - Flight (`flight.rs`): it sets the vulture's speed directly, without gravity, flown like a plane, banking into
//!   arc turns (SPECS §6.2). `Fly` steers it from `Mob.rider`; the wild vulture's `wild::Soar` from its own AI.
//! - Wild (SPECS §4): it soars in circles high up and glides back down (`wild.rs`), walks about, and fights back
//!   in the air (`wild::DiveAttack`), its target chosen by vanilla's `HurtByTarget`.
//! - Sounds: its own, not the trader's (`sounds.rs`, played by the library's `creature::sounds`).
//! - Drawing: the Blockbench model (`assets/models/vulture.bbmodel`, converted into `model.rs`), posed by `pose` and
//!   drawn by the library's `creature::model`, in place of the trader's model; its hearts, once ridden, without the
//!   empty ones (`mount_hud.rs` injection).
//!
//! It depends on the series' library, Horizon Creature (`creature`, common/SPECS.md).
mod flight;
mod interaction;
mod model;
mod model_baby;
mod screen;
mod sounds;
mod state;
mod talons;
mod tame;
mod wild;

use std::sync::Arc;

use flight::{Controls, Flight};
use horizon_client::mobs::Who;
use horizon_client::mobs::goals::{self, HurtByTarget, LookAtPlayer, Stroll};
use horizon_client::mod_api::{
    Cx, EntityTickListenedEvent, EntityTickPostEvent, Flow, Goal, Hooks, Host, InteractEntityEvent, KeyEvent,
    LivingDeathEvent, MenuClickedEvent, MenuClosedEvent, MenuInputClosedEvent, Mob, MobSubmitEvent, NativeMobRender,
    RandomSource, RegisterGoalsEvent,
};

const KIND: &str = "vulture:vulture";
/// Its hitbox and eye height, in blocks (SPECS §4).
const WIDTH: f64 = 1.2;
const HEIGHT: f64 = 1.0;
const EYE_HEIGHT: f64 = 0.9;

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Vulture)
}

struct Vulture;

/// Walking speed on the ground, ridden, in blocks per tick.
const WALK: f64 = 0.12;
/// Its peck when it fights back (SPECS §4).
const ATTACK_DAMAGE: f32 = 3.0;

/// The pitch it takes, ridden, climbing (Space) and descending (sneak), in degrees.
const CLIMB_PITCH: f32 = 35.0;
const DESCEND_PITCH: f32 = 35.0;

/// Ridden flight (SPECS §6.2): the rider's look turns it and the keys steer `flight::Flight`. W flaps up to the
/// cruising speed, S brakes, Space climbs with strong flaps (and takes off from the ground), sneak descends (getting off
/// only on the ground, `ride_keys.rs`); on the ground it walks.
#[derive(Clone, Debug, Default)]
struct Fly {
    flight: Flight,
}

impl Goal for Fly {
    fn flags(&self) -> u8 {
        goals::MOVE | goals::LOOK | goals::JUMP
    }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        m.rider.is_some()
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn start(&mut self, m: &mut Mob, _: &mut Cx) {
        self.flight = Flight::default();
        m.body_yaw = m.yaw;
    }
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        self.flight.land(m);
        talons::release(m.id);
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        let Some(r) = m.rider else {
            return;
        };
        if m.on_ground && !r.jumping {
            // On the ground: a slow walk where the rider looks.
            self.flight.land(m);
            m.yaw = r.yaw;
            m.body_yaw = r.yaw;
            m.head_yaw = r.yaw;
            let (s, c) = f64::from(r.yaw).to_radians().sin_cos();
            let (fwd, strafe) = (f64::from(r.forward), f64::from(r.strafe));
            m.velocity.x = (strafe * c - fwd * s) * WALK;
            m.velocity.z = (fwd * c + strafe * s) * WALK;
            return;
        }
        // Space climbs, sneak (or Ctrl, `ride_keys.rs`) descends, nothing keeps it level; the look only turns it.
        let pitch = if r.jumping {
            -CLIMB_PITCH
        } else if r.sprint {
            DESCEND_PITCH
        } else {
            0.0
        };
        let controls = Controls {
            yaw: r.yaw,
            pitch,
            flap: r.forward > 0.0,
            brake: r.forward < 0.0,
            boost: r.jumping,
            heavy: talons::carrying(m.id),
        };
        self.flight.tick(m, &controls);
        if talons::take_press() {
            talons::toggle(m, cx.w);
        }
        talons::follow(m);
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// No fall damage (SPECS §6), ridden or not: the fall distance is reset every tick, before the move that could land
/// it. Also keeps it muted and gives it its call.
#[derive(Clone, Debug)]
struct NoFall;

impl Goal for NoFall {
    fn flags(&self) -> u8 {
        0
    }
    fn can_use(&mut self, _: &mut Mob, _: &mut Cx) -> bool {
        true
    }
    fn every_tick(&self) -> bool {
        true
    }
    fn tick(&mut self, m: &mut Mob, cx: &mut Cx) {
        m.fall_distance = 0.0;
        state::sync_mob(m);
        // A tamed vulture never turns on its owner, even when they hit it.
        if m.extra.tame
            && m.ai.target == Some(Who::Player)
            && cx.w.player.as_ref().is_some_and(|p| m.extra.owner == Some(p.uuid))
        {
            m.ai.target = None;
        }
        // Muted as a wandering trader, every tick: a save read after `register_goals` sets its `Silent` back.
        m.silent = true;
        // A quiet bird: one call every 20 s on average (a vanilla mob: every 4 s).
        if m.random.next_int_bound(400).unwrap_or(1) == 0 {
            let pitch = 0.8 + m.random.next_float() * 0.3;
            sounds::queue(m.id, sounds::AMBIENT, 1.0, pitch);
        }
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// The variant's texture (only one for now: tawny).
const TEXTURE: &str = "vulture:entity/vulture/tawny";

/// How far it settles when sitting, in blocks.
const SIT_DROP: f32 = 0.3;
/// How long a peck lasts, in ticks.
const PECK: f32 = 10.0;

/// The baby's model is drawn at this scale.
const BABY_SCALE: f32 = 0.8;
const BABY_TEXTURE: &str = "vulture:entity/vulture/baby";

/// How far its back slopes down to the tail on the ground, in degrees.
const HUNCH: f32 = 18.0;

/// A group's rotation for this frame, in degrees (x, y, z), on top of its rest rotation.
///
/// In flight the body takes the flight's pitch (`pitch`, which `Fly` keeps in `head_pitch`) and banks with the turn:
/// the gap between its heading and its head (where the rider looks, `head_yaw`). On the ground the wings fold along the
/// flanks of its hunched body: the arm turned back with its coverts outward, the forearm folded forward over it, the hand back again, its
/// primaries along the tail.
fn pose(name: &str, mob: &NativeMobRender<'_>, flapping: bool, look: &state::Look, carrying: bool) -> [f32; 3] {
    let flying = !mob.on_ground;
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
    let leg = if flying { 60.0 } else { step.to_degrees() };
    if !flying {
        return match name {
            // Hunched: the back slopes down to the tail, the head and legs straightened against it.
            "body" => [-HUNCH, 0.0, 0.0],
            // The model faces +z: looking right (positive yaw) turns it towards -x.
            "head" => [mob.pitch + HUNCH + peck, -mob.head_yaw, 0.0],
            // Sitting: legs folded forward under the belly.
            "leg_left" | "leg_right" if look.sitting => [HUNCH - 80.0, 0.0, 0.0],
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
        "leg_left" | "leg_right" if carrying => [0.0, 0.0, 0.0],
        "leg_left" | "leg_right" => [leg, 0.0, 0.0],
        "tail" => [-10.0, 0.0, -bank * 0.3],
        _ => [0.0; 3],
    }
}

impl Hooks for Vulture {
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id != KIND {
            return;
        }
        // Without the wandering trader's sounds: its own come from `sounds.rs`.
        event.mob.silent = true;
        // Its own size (SPECS §4), in place of the trader's 0.6 × 1.95: a mob's box comes from its type, so it gets a
        // copy of its type with its own width and height.
        if event.mob.kind.width != WIDTH {
            let mut kind = (*event.mob.kind).clone();
            kind.width = WIDTH;
            kind.height = HEIGHT;
            kind.eye_height = EYE_HEIGHT;
            event.mob.kind = Arc::new(kind);
        }
        // Neutral: it fights back with its beak, diving from the air, and nearby vultures join in (SPECS §4).
        event.mob.attrs.attack_damage = ATTACK_DAMAGE;
        let ai = &mut event.mob.ai;
        ai.goals.clear();
        ai.targets.clear();
        ai.goals.add(0, NoFall);
        ai.goals.add(1, Fly::default());
        ai.goals.add(2, tame::Sit);
        ai.goals.add(2, wild::DiveAttack::default());
        ai.goals.add(3, tame::Mate::default());
        ai.goals.add(3, tame::Eat::default());
        ai.goals.add(4, tame::FollowOwner::default());
        ai.goals.add(5, wild::Soar::new());
        ai.goals.add(6, Stroll::new(0.6));
        ai.goals.add(7, LookAtPlayer::new(8.0));
        ai.targets.add(1, tame::OwnerHurtBy::default());
        ai.targets.add(2, tame::OwnerHurt::default());
        ai.targets.add(3, HurtByTarget::new().alert());
    }

    /// Right-click (`interaction.rs`), once the `interact.rs` injection didn't mount it. Always taken: no trading.
    fn interact_entity(&self, host: &mut dyn Host, event: &mut InteractEntityEvent<'_>) -> Flow {
        if event.kind != KIND {
            return Flow::Continue;
        }
        if let Ok(id) = u32::try_from(event.entity) {
            interaction::clicked(host, id);
        }
        Flow::Cancel
    }

    fn menu_clicked(&self, host: &mut dyn Host, event: &mut MenuClickedEvent) {
        screen::clicked(host, event);
    }

    fn menu_closed(&self, _: &mut dyn Host, event: &mut MenuClosedEvent) {
        screen::forget(event.uuid);
    }

    fn menu_input_closed(&self, host: &mut dyn Host, event: &mut MenuInputClosedEvent<'_>) {
        screen::closed(host, event);
    }

    fn living_death(&self, host: &mut dyn Host, event: &mut LivingDeathEvent<'_>) -> Flow {
        if event.kind == KIND {
            screen::died(host, &state::dimension(event.entity), event.entity);
            talons::release(event.entity);
        }
        Flow::Continue
    }

    /// The talons key (SPECS §6.4), taken by the ridden vulture.
    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        if event.pressed && event.key == host.mod_key("vulture", talons::KEY, talons::DEFAULT_KEY) {
            talons::press();
        }
    }

    fn entity_tick_listened(&self, _: &mut dyn Host, event: &mut EntityTickListenedEvent) {
        event.listened = true;
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind == KIND {
            creature::sounds::tick_post(host, event, &sounds::VOICE, creature::sounds::NEUTRAL);
            state::save(host, event.dimension, event.entity);
            state::look_for_food(host, event.dimension, event.entity);
            talons::carry(host, event.dimension, event.entity);
            state::births(host, event.dimension);
        }
    }

    /// The Blockbench model (`model.rs`), posed by `pose` every frame.
    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != KIND {
            return Flow::Continue;
        }
        let flapping = flight::flapping(mob.id);
        let look = state::look(mob.id);
        let carrying = talons::carrying(mob.id);
        let shown = |part: &str| match part {
            "harness" => look.harness,
            "chest_left" => look.chests >= 1,
            "chest_right" => look.chests >= 2,
            _ => true,
        };
        // A baby is its own model (SPECS §7: a chick with a big head and down), with the same groups and poses.
        let (part, texture) = if look.baby {
            (&model_baby::BABY, BABY_TEXTURE)
        } else {
            (&model::VULTURE, TEXTURE)
        };
        event.draw.push();
        creature::model::death_tilt(&mut *event.draw, mob);
        if look.baby {
            event.draw.scale([BABY_SCALE; 3]);
        }
        // Sitting: settled on its belly, legs folded under it.
        if look.sitting {
            event.draw.translate([0.0, -SIT_DROP, 0.0]);
        }
        creature::model::draw_shown(
            host,
            &mut *event.draw,
            part,
            texture,
            &|part| pose(part, mob, flapping, &look, carrying),
            &shown,
            creature::model::tint(mob),
        );
        event.draw.pop();
        Flow::Cancel
    }
}
