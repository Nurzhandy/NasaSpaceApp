//! The active profile: callsign, level and experience, stars, the six directions at a glance
//! and the expedition history.

use super::*;
use ja_core::landing::SITES;

pub(super) fn profile(p: &mut ChildSpawnerCommands, art: &Art, prof: &Profile, l: Lang) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(22.0)), column_gap: Val::Px(22.0), ..default() }).with_children(|row| {
        row.spawn(kit::panel()).insert(Node { width: Val::Px(360.0), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(16.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|c| {
            avatar(c, art, prof, 6.0);
            kit::text(c, prof.name(l), &art.head, 28.0, C_TEXT);
            c.spawn(kit::row(6.0)).with_children(|r| {
                kit::icon(r, art, prof.role.icon(), 24.0);
                kit::text(r, role_name(l, prof.role), &art.body, 18.0, C_MUTED);
            });
            kit::text(c, trf(l, "prof.level", &[("n", &prof.level().to_string())]), &art.head, 22.0, C_ACCENT);
            bar(c, prof.xp_in_level() as f32 / ja_core::progress::XP_PER_LEVEL as f32, 260.0, C_ACCENT);
            kit::text(c, trf(l, "prof.xp", &[("a", &prof.xp_in_level().to_string()), ("b", &ja_core::progress::XP_PER_LEVEL.to_string())]), &art.body, 15.0, C_MUTED);
            for (ic, label, v) in [("star", tr(l, "prof.stars"), prof.stars), ("medal", tr(l, "prof.points"), prof.total), ("flag", tr(l, "prof.missions"), prof.missions)] {
                c.spawn(kit::row(8.0)).with_children(|r| {
                    kit::icon(r, art, ic, 24.0);
                    kit::text(r, label, &art.body, 17.0, C_MUTED);
                    kit::text(r, v.to_string(), &art.head, 18.0, C_TEXT);
                });
            }
            kit::button(c, art, Action::Go(AppState::Character), 0, "shirt", tr(l, "prof.character"), true);
            kit::button(c, art, Action::Go(AppState::Profiles), 1, "coop", tr(l, "prof.switch"), false);
        });
        row.spawn(kit::col(12.0)).insert(Node { flex_grow: 1.0, flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), ..default() }).with_children(|c| {
            c.spawn(kit::row(12.0)).with_children(|r| {
                kit::text(r, tr(l, "ach.title"), &art.head, 22.0, C_TEXT);
                kit::button(r, art, Action::Go(AppState::Achievements), 2, "trophy", "", false);
            });
            track_medals(c, art, prof, 34.0);
            kit::text(c, tr(l, "prof.history"), &art.head, 22.0, C_TEXT);
            if prof.history.is_empty() {
                kit::text(c, tr(l, "prof.no_history"), &art.body, 17.0, C_MUTED);
            }
            for h in prof.history.iter().take(8) {
                c.spawn((kit::row(10.0), BackgroundColor(kit::C_BG), BorderRadius::all(Val::Px(6.0)))).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(10.0), padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)), ..default() }).with_children(|r| {
                    let md = map(h.map);
                    let mode_icon = match h.mode {
                        ja_core::Mode::Survival => "shield",
                        ja_core::Mode::Sandbox => "paint",
                        _ => md.icon,
                    };
                    kit::icon(r, art, mode_icon, 26.0);
                    let place = match h.site.and_then(|s| SITES.get(s as usize)) {
                        Some(s) => l.pick(s.name_ru, s.name_en).to_string(),
                        None => l.pick(md.name_ru, md.name_en).to_string(),
                    };
                    kit::text(r, place, &art.body, 17.0, C_TEXT);
                    kit::text(r, format!("{} {}/{}", tr(l, "menu.sol"), h.sol, h.sols), &art.body, 16.0, C_MUTED);
                    kit::pips(r, h.stars, 3, C_ACCENT, 14.0);
                    kit::text(r, if h.won { tr(l, "prof.won") } else { tr(l, "prof.lost") }, &art.body, 16.0, if h.won { C_GOOD } else { C_WARN });
                });
            }
        });
    });
}
