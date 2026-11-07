// Les cœurs du vautour monté : seulement ceux qu'il a, sans les cœurs vides de sa vie perdue
// (`Hud.extractVehicleHealth` dessine toute sa vie maximale). Renvoie le nombre de cœurs dessinés,
// que le HUD utilise pour placer les lignes au-dessus.
fn vulture_mount_hud(
    game: &mut Game<'_>,
    batch: &mut HudBatch,
    w: i32,
    h: i32,
) -> ::core::ops::ControlFlow<i32> {
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
