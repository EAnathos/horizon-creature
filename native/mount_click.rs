// Right-click on a vulture: the player mounts it, as on a horse (`interact_mount`) or with `/ride`.
// The trait hook cannot run `/ride` in single player (`native_run_command` refuses during the player's
// turn), hence this injection, compiled into the game, which calls `start_riding` directly.
fn vulture_mount_click(
    game: &mut crate::game::Game<'_>,
    entity: i32,
    kind: &str,
    claimed: &mut bool,
) {
    if *claimed || kind != "vulture:vulture" {
        return;
    }
    // No trading screen, whether it mounts or not.
    *claimed = true;
    let Ok(id) = u32::try_from(entity) else {
        return;
    };
    if game.riding.is_some() || !game.may_board() || !game.native_may_board(id) {
        return;
    }
    let Some(m) = game.mobs.iter_mut().find(|m| m.id == id && m.alive()) else {
        return;
    };
    m.extra.flag = false;
    m.extra.counter = 0;
    game.yaw = m.yaw;
    game.mount.jump_ticks = 0;
    game.mount.jump_scale = 0.0;
    game.start_riding(id);
}
