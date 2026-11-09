// Right-click on a vulture (SPECS §6), before its wandering trader's trading screen (always taken).
// Compiled into the game: it reaches the mob's own state (`Mob.extra`, which the vulture's goals keep
// in step with its saved state) and the acting player's hand, and mounts with `start_riding` (the
// trait hook cannot run `/ride` in single player: `native_run_command` refuses during a player's turn).
//
// Only its owner gets anything from it:
// - holding the vulture harness: puts it on; a chest: adds one (two at most, harness needed);
// - holding raw meat while it is hurt: heals 4;
// - otherwise: mounts it when it wears a harness, else tells it to sit or follow.
fn vulture_interact(game: &mut crate::game::Game<'_>, entity: i32, kind: &str, claimed: &mut bool) {
    use crate::mod_api::Host as _;
    if *claimed || kind != "vulture:vulture" {
        return;
    }
    // No trading screen, whatever happens.
    *claimed = true;
    let Ok(id) = u32::try_from(entity) else {
        return;
    };
    let me = game.level.player_uuid.unwrap_or_default();
    let held = game.held().map(|s| s.item.to_string()).unwrap_or_default();
    let Some(m) = game.mobs.iter().find(|m| m.id == id && m.alive()) else {
        return;
    };
    if !m.extra.tame || !crate::mobs::tame::owned_by(m.extra.owner, me) {
        return;
    }
    let (saddled, chests, hurt) = (m.extra.saddled, m.extra.variant2, m.health < m.attrs.max_health);
    let meat = matches!(
        held.as_str(),
        "minecraft:chicken" | "minecraft:beef" | "minecraft:porkchop" | "minecraft:mutton" | "minecraft:rabbit"
    );
    let take = |game: &mut crate::game::Game<'_>, item: &str| game.take_items(item, 1) == 1;
    if held == "vulture:harness" && !saddled {
        if take(game, "vulture:harness") {
            vulture_with_mob(game, id, |m| m.extra.saddled = true);
        }
        return;
    }
    if held == "minecraft:chest" && saddled && chests < 2 {
        if take(game, "minecraft:chest") {
            vulture_with_mob(game, id, |m| m.extra.variant2 = chests + 1);
        }
        return;
    }
    if meat && hurt {
        if take(game, &held) {
            vulture_with_mob(game, id, |m| m.heal(4.0));
        }
        return;
    }
    if !saddled {
        vulture_with_mob(game, id, |m| m.extra.sitting = !m.extra.sitting);
        return;
    }
    if game.riding.is_some() || !game.native_may_board(id) {
        return;
    }
    let mut yaw = game.yaw;
    vulture_with_mob(game, id, |m| {
        m.extra.flag = false;
        m.extra.counter = 0;
        m.extra.sitting = false;
        yaw = m.yaw;
    });
    game.yaw = yaw;
    game.mount.jump_ticks = 0;
    game.mount.jump_scale = 0.0;
    game.start_riding(id);
}

fn vulture_with_mob(game: &mut crate::game::Game<'_>, id: u32, f: impl FnOnce(&mut crate::mobs::Mob)) {
    if let Some(m) = game.mobs.iter_mut().find(|m| m.id == id) {
        f(m);
    }
}
