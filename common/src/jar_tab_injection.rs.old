// The glass jar left out of the creative tabs when no mod of the series uses it (common/SPECS.md §3.3): Horizon's tab
// entries have no conditions and a tab lists every item the game has. This builds the tabs once (the body, through
// the guard), then takes the jar out of them when no Butterflies, Worms, Fireflies or Fairies item is among the mods' items.
fn creature_jar_tab(plan: &Plan<'_>, doc: &mut J) -> ::core::ops::ControlFlow<Result<()>> {
    use ::core::cell::Cell;
    use ::core::ops::ControlFlow;
    thread_local! {
        static INSIDE: Cell<bool> = const { Cell::new(false) };
    }
    const USERS: [&str; 4] = ["butterflies:", "worms:", "fireflies:", "fairies:"];
    let used = plan
        .items
        .iter()
        .any(|i| USERS.iter().any(|u| i.rename.to.starts_with(u)));
    if used || INSIDE.with(Cell::get) {
        return ControlFlow::Continue(());
    }
    INSIDE.with(|i| i.set(true));
    let result = plan.creative_tabs(doc);
    INSIDE.with(|i| i.set(false));
    let is_jar = |s: &J| matches!(s, J::Str(t) if t.contains("id:\"creature:glass_jar\""));
    if let Some(J::Arr(tabs)) = doc.get_mut("tabs") {
        for tab in tabs.iter_mut() {
            let Some(at) = (match tab.get("items") {
                Some(J::Arr(items)) => items.iter().position(is_jar),
                _ => None,
            }) else {
                continue;
            };
            // The search tab keeps its lines and tags beside its items, one each.
            let len = |tab: &J, key: &str| match tab.get(key) {
                Some(J::Arr(list)) => Some(list.len()),
                _ => None,
            };
            let items = len(tab, "items");
            for key in ["items", "lines", "tags"] {
                if len(tab, key).is_some()
                    && (key == "items" || len(tab, key) == items)
                    && let Some(J::Arr(list)) = tab.get_mut(key)
                {
                    list.remove(at);
                }
            }
        }
    }
    ControlFlow::Break(result)
}
