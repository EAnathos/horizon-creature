//! Vautour, phase 0 : le prototype technique (docs/SPECS.md §1.2).
//!
//! - `vulture:vulture` copie le marchand ambulant (partie contenu, `content/`) : on vide ses goals
//!   et on lui donne le nôtre, `Fly`.
//! - Monter : un clic droit dessus (injection `mount_click.rs`), ou `/ride`. Horizon copie alors à chaque
//!   tick les touches du cavalier dans `Mob.rider` (`game/mounts.rs`, `tick_mount_riding`) :
//!   avancer/reculer, gauche/droite, saut (Espace), sprint (Ctrl). Accroupi fait descendre.
//! - Voler : `Fly` lit `Mob.rider` et règle directement la vitesse du vautour, sans gravité.
//! - Sons : les siens, pas ceux du marchand (`sounds.rs`).
//! - Dessin : le modèle de Blockbench (`assets/models/vulture.bbmodel`, converti en `model.rs`), animé
//!   par `pose`, à la place du modèle du marchand ; ses cœurs, une fois monté, sans les cœurs vides
//!   (injection `mount_hud.rs`).
mod model;
mod sounds;

use horizon_client::mobs::goals;
use horizon_client::mod_api::{
    CubeFace, Cx, EntityTickListenedEvent, EntityTickPostEvent, Flow, Goal, Hooks, Host, Mob,
    MobSubmitEvent, NativeMobRender, RandomSource, RegisterGoalsEvent, BlockEntityDraw,
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
        // Un battement d'ailes toutes les 8 ticks tant qu'il monte.
        if r.jumping && m.age % 8 == 0 {
            let pitch = 0.9 + m.random.next_float() * 0.2;
            sounds::queue(m.id, sounds::FLAP, 0.6, pitch);
        }
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
        // Muet côté marchand ambulant, à chaque tick : une sauvegarde lue après `register_goals`
        // remet son `Silent`.
        m.silent = true;
        // Un oiseau discret : un cri toutes les 20 s en moyenne (un mob vanilla : toutes les 4 s).
        if m.random.next_int_bound(400).unwrap_or(1) == 0 {
            let pitch = 0.8 + m.random.next_float() * 0.3;
            sounds::queue(m.id, sounds::AMBIENT, 1.0, pitch);
        }
    }
    fn box_clone(&self) -> Box<dyn Goal> {
        Box::new(self.clone())
    }
}

/// Un groupe du modèle : son pivot (texels), sa rotation de repos (degrés, x puis y puis z), ses cubes
/// et ses sous-groupes. Les données viennent de `tools/bbmodel_to_rust.py`.
pub struct Part {
    pub name: &'static str,
    pub pivot: [f32; 3],
    pub rotation: [f32; 3],
    pub cubes: &'static [Cube],
    pub children: &'static [Part],
}

/// Un cube (texels, 16 par bloc) et les UV de ses faces dans l'ordre de `draw_cube`.
#[allow(dead_code)] // `name` : pour s’y retrouver dans `model.rs`.
pub struct Cube {
    pub name: &'static str,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub uv: [[f32; 4]; 6],
}

/// La texture de la variante (une seule pour l'instant : le vautour fauve).
const TEXTURE: &str = "vulture:entity/vulture/fauve";

/// La rotation d'un groupe à cette image, en degrés (x, y, z), en plus de sa rotation de repos.
fn pose(name: &str, mob: &NativeMobRender<'_>) -> [f32; 3] {
    let flying = !mob.on_ground;
    // Battement : les ailes montent et descendent, les pointes suivent avec un temps de retard.
    let beat = (mob.age * 0.5).sin();
    let lag = (mob.age * 0.5 - 0.8).sin();
    // Au sol, les ailes pendent le long du corps et les pointes se replient vers le haut, côté corps,
    // sous les couvertures beiges.
    let (arm, tip) = if flying { (beat * 35.0, lag * 20.0) } else { (85.0, 165.0) };
    // Marche : comme les pattes d'un quadrupède vanilla ; en vol, les pattes sont rentrées.
    let step = (mob.walk_position * 0.6662).cos() * 1.4 * mob.walk_speed.min(1.0);
    let leg = if flying { 60.0 } else { step.to_degrees() };
    match name {
        // Le modèle regarde vers +z : un regard à droite (lacet positif) le tourne vers -x.
        "head" => [mob.pitch, -mob.head_yaw, 0.0],
        "wing_left" => [0.0, 0.0, -arm],
        "wing_right" => [0.0, 0.0, arm],
        "wing_left_tip" => [0.0, 0.0, -tip],
        "wing_right_tip" => [0.0, 0.0, tip],
        "leg_left" => [leg, 0.0, 0.0],
        "leg_right" => [if flying { leg } else { -leg }, 0.0, 0.0],
        "tail" if flying => [-10.0, 0.0, 0.0],
        _ => [0.0; 3],
    }
}

fn draw_part(host: &dyn Host, draw: &mut BlockEntityDraw, part: &Part, mob: &NativeMobRender<'_>) {
    let animated = pose(part.name, mob);
    let pivot = part.pivot.map(|v| v / 16.0);
    draw.push();
    for (k, axis) in [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].into_iter().enumerate() {
        let degrees = part.rotation[k] + animated[k];
        if degrees != 0.0 {
            draw.rotate_about(axis, degrees, pivot);
        }
    }
    for cube in part.cubes {
        let faces = cube.uv.map(|uv| Some(CubeFace { sprite: TEXTURE, uv: Some(uv) }));
        host.draw_cube(draw, cube.from, cube.to, faces, 0xFFFFFF);
    }
    for child in part.children {
        draw_part(host, draw, child, mob);
    }
    draw.pop();
}

impl Hooks for Vulture {
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id != KIND {
            return;
        }
        // Sans les sons du marchand ambulant : les siens viennent de `sounds.rs`.
        event.mob.silent = true;
        let ai = &mut event.mob.ai;
        ai.goals.clear();
        ai.targets.clear();
        ai.goals.add(0, NoFall);
        ai.goals.add(1, Fly);
    }

    fn entity_tick_listened(&self, _: &mut dyn Host, event: &mut EntityTickListenedEvent) {
        event.listened = true;
    }

    fn entity_tick_post(&self, host: &mut dyn Host, event: &mut EntityTickPostEvent<'_>) {
        if event.kind == KIND {
            sounds::tick_post(host, event);
        }
    }

    /// Le modèle de Blockbench (`model.rs`), posé par `pose` à chaque image.
    fn mob_submit(&self, host: &dyn Host, event: &mut MobSubmitEvent<'_>) -> Flow {
        let mob = event.mob;
        if mob.kind != KIND {
            return Flow::Continue;
        }
        draw_part(host, &mut *event.draw, &model::VULTURE, mob);
        Flow::Cancel
    }
}
