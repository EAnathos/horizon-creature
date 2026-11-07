// Clic droit sur un vautour : le joueur monte dessus, comme un cheval (`interact_mount`) ou `/ride`.
// Le hook trait ne peut pas lancer `/ride` en solo (`native_run_command` refuse pendant le tour du
// joueur), d'où cette injection, compilée dans le jeu, qui appelle directement `start_riding`.
fn vulture_mount_click(
    game: &mut crate::game::Game<'_>,
    entity: i32,
    kind: &str,
    claimed: &mut bool,
) {
    if *claimed || kind != "vulture:vulture" {
        return;
    }
    // Pas d'écran de marchand, monté ou non.
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
