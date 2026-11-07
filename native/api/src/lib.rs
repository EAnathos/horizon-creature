//! Vautour, phase 0 : le prototype technique (docs/SPECS.md §1.2).
//!
//! - `vulture:vulture` copie le marchand ambulant (partie contenu, `content/`) : on vide ses goals
//!   et on lui donne le nôtre, `Fly`.
//! - Monter : un clic droit dessus (injection `mount_click.rs`), ou `/ride`. Horizon copie alors à chaque
//!   tick les touches du cavalier dans `Mob.rider` (`game/mounts.rs`, `tick_mount_riding`) :
//!   avancer/reculer, gauche/droite, saut (Espace), sprint (Ctrl). Accroupi fait descendre.
//! - Voler : `Fly` lit `Mob.rider` et règle directement la vitesse du vautour, sans gravité.
//! - Dessin : quelques cubes (`mob_submit`) à la place du modèle du marchand ; ses cœurs, une fois
//!   monté, sans les cœurs vides (injection `mount_hud.rs`).
use horizon_client::mobs::goals;
use horizon_client::mod_api::{
    CubeFace, Cx, Flow, Goal, Hooks, Host, Mob, MobSubmitEvent, RegisterGoalsEvent,
};

const KIND: &str = "vulture:vulture";

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Vulture)
}

struct Vulture;

/// Vitesses en blocs par tick, avant la traînée de l'air (×0,91 par tick).
const FLY_SPEED: f64 = 0.6;
const DIVE_SPEED: f64 = 1.0;
const CLIMB: f64 = 0.35;
const GLIDE_SINK: f64 = -0.08;
const DIVE_SINK: f64 = -0.7;

/// Le vol monté : tant qu'un joueur est dessus, ses touches donnent la vitesse du vautour.
#[derive(Clone, Debug)]
struct Fly;

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
    fn stop(&mut self, m: &mut Mob, _: &mut Cx) {
        m.no_gravity = false;
    }
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        let Some(r) = m.rider else {
            return;
        };
        // Il regarde là où regarde son cavalier.
        m.yaw = r.yaw;
        m.body_yaw = r.yaw;
        m.head_yaw = r.yaw;
        m.head_pitch = r.pitch;
        let flying = !m.on_ground || r.jumping;
        m.no_gravity = flying;
        if !flying {
            // Au sol : il ne bouge pas (la marche viendra plus tard).
            m.velocity.x = 0.0;
            m.velocity.z = 0.0;
            return;
        }
        let dive = r.sprint;
        let speed = if dive { DIVE_SPEED } else { FLY_SPEED };
        // `Entity.moveRelative` : strafe positif = gauche.
        let (s, c) = f64::from(r.yaw).to_radians().sin_cos();
        let (fwd, strafe) = (f64::from(r.forward), f64::from(r.strafe));
        let len = (fwd * fwd + strafe * strafe).sqrt().max(1.0);
        m.velocity.x = (strafe * c - fwd * s) / len * speed;
        m.velocity.z = (fwd * c + strafe * s) / len * speed;
        m.velocity.y = if r.jumping {
            CLIMB
        } else if dive {
            DIVE_SINK
        } else {
            GLIDE_SINK
        };
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Pas de dégâts de chute (SPECS §6), monté ou non : la distance de chute est remise à zéro à
/// chaque tick, avant le déplacement qui pourrait le faire atterrir.
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
    fn tick(&mut self, m: &mut Mob, _: &mut Cx) {
        m.fall_distance = 0.0;
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

fn faces(sprite: &str) -> [Option<CubeFace<'_>>; 6] {
    std::array::from_fn(|_| Some(CubeFace { sprite, uv: None }))
}

impl Hooks for Vulture {
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id != KIND {
            return;
        }
        let ai = &mut event.mob.ai;
        ai.goals.clear();
        ai.targets.clear();
        ai.goals.add(0, NoFall);
        ai.goals.add(1, Fly);
    }

    /// Le modèle provisoire : un corps, une tête, deux ailes qui battent en vol.
    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != KIND {
            return Flow::Continue;
        }
        let draw = &mut *event.draw;
        let body = "block/brown_terracotta";
        let head = "block/pink_terracotta";
        let wing = "block/dark_oak_planks";
        // Corps et tête (texels, 16 par bloc ; +z devant, +x à sa gauche).
        host.draw_cube(draw, [-5.0, 8.0, -8.0], [5.0, 17.0, 8.0], faces(body), 0xFFFFFF);
        host.draw_cube(draw, [-2.5, 15.0, 8.0], [2.5, 21.0, 13.0], faces(head), 0xFFFFFF);
        // Ailes : repliées au sol, battantes en l'air (pivot à l'épaule, en blocs).
        let flap = if mob.on_ground {
            70.0
        } else {
            (mob.age * 0.5).sin() * 35.0
        };
        let shoulder_y = 16.0 / 16.0;
        draw.push();
        draw.rotate_about([0.0, 0.0, 1.0], -flap, [5.0 / 16.0, shoulder_y, 0.0]);
        host.draw_cube(draw, [5.0, 15.0, -6.0], [25.0, 16.5, 6.0], faces(wing), 0xFFFFFF);
        draw.pop();
        draw.push();
        draw.rotate_about([0.0, 0.0, 1.0], flap, [-5.0 / 16.0, shoulder_y, 0.0]);
        host.draw_cube(draw, [-25.0, 15.0, -6.0], [-5.0, 16.5, 6.0], faces(wing), 0xFFFFFF);
        draw.pop();
        Flow::Cancel
    }
}
