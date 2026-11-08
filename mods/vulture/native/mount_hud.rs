// The ridden vulture's hearts: only the ones it has, without empty hearts for the health it lost
// (`Hud.extractVehicleHealth` draws its whole max health). Returns the number of hearts drawn,
// which the HUD uses to place the rows above.
fn vulture_mount_hud(game: &mut Game<'_>, batch: &mut HudBatch, w: i32, h: i32) -> ::core::ops::ControlFlow<i32> {
    use ::core::ops::ControlFlow;
    let Some(i) = game.mount_index() else {
        return ControlFlow::Continue(());
    };
    let m = &game.mobs[i];
    if m.kind.id != "vulture:vulture" {
        return ControlFlow::Continue(());
    }
    let health = m.health;
    let hearts = ((health.ceil() as i32 + 1) / 2).clamp(1, 30);
    let draws = horizon_client::hud_layout::vehicle_health(w, h, hearts, health);
    game.gui_layer("minecraft:vehicle_health", batch, |g, batch| {
        horizon_client::hud_layout::paint(batch, &g.gui.atlas, &draws);
    });
    ControlFlow::Break(hearts)
}
