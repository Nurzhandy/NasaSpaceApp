//! Main menu over the Moon backdrop: «Начать», continue the saved mission, and every screen.

use super::*;

pub(super) fn main_menu(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, music: bool, saved: &SavedMission, l: Lang, sky: &Sky) {
    p.spawn(Node { flex_grow: 1.0, flex_direction: FlexDirection::Row, padding: UiRect::all(Val::Px(26.0)), column_gap: Val::Px(30.0), ..default() }).with_children(|row| {
        row.spawn(kit::col(14.0)).insert(Node { width: Val::Px(640.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), ..default() }).with_children(|c| {
            kit::text(c, "SPACE EXPLORERS", &art.head, 44.0, kit::C_ACCENT);
            kit::wrap_text(c, tr(l, "menu.subtitle"), &art.head, 28.0, C_TEXT, 620.0);
            c.spawn(kit::row(8.0)).with_children(|r| {
                for ic in ["oxygen", "food", "shield", "science", "power"] {
                    kit::icon(r, art, ic, 34.0);
                }
                kit::text(r, tr(l, "menu.power_hint"), &art.body, 19.0, C_MUTED);
            });
            if let Some((m, sol, sols, coop)) = saved.0 {
                // the saved mission comes first: one big green button
                let md = map(m);
                let bg = Color::srgb(0.18, 0.62, 0.42);
                c.spawn((
                    Button,
                    kit::UiAction(Action::Continue),
                    kit::Focusable(0),
                    Node { width: Val::Px(530.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(14.0), padding: UiRect::axes(Val::Px(18.0), Val::Px(10.0)), border: UiRect::all(Val::Px(4.0)), ..default() },
                    BackgroundColor(bg),
                    kit::Base(bg),
                    BorderColor(C_LINE),
                    BorderRadius::all(Val::Px(10.0)),
                ))
                .with_children(|b| {
                    kit::icon(b, art, "play", 44.0);
                    b.spawn(kit::col(2.0)).with_children(|t| {
                        kit::text(t, tr(l, "menu.continue"), &art.head, 24.0, C_TEXT);
                        t.spawn(kit::row(6.0)).with_children(|x| {
                            kit::icon(x, art, md.icon, 22.0);
                            kit::text(x, format!("{} · {} {}/{}", l.pick(md.name_ru, md.name_en), tr(l, "menu.sol"), sol, sols), &art.body, 18.0, C_TEXT);
                            if coop {
                                kit::icon(x, art, "coop", 22.0);
                            }
                        });
                    });
                });
            }
            c.spawn(kit::row(12.0)).with_children(|r| {
                kit::tile(r, art, Action::Go(AppState::Expedition), 1, "rocket", tr(l, "menu.start"), 200.0, Color::srgb(0.9, 0.42, 0.22));
                kit::tile(r, art, Action::Go(AppState::Lobby), 2, "satellite", tr(l, "menu.network"), 140.0, Color::srgb(0.2, 0.6, 0.55));
                kit::tile(r, art, Action::Go(AppState::ProfilePage), 3, prof.0.role.icon(), tr(l, "menu.profile"), 140.0, kit::C_PANEL);
                kit::tile(r, art, Action::Go(AppState::Shop), 4, "shop", tr(l, "menu.shop"), 120.0, kit::C_PANEL);
            });
            c.spawn(kit::row(12.0)).with_children(|r| {
                kit::tile(r, art, Action::Go(AppState::Inventory), 5, "box", tr(l, "menu.inventory"), 140.0, kit::C_PANEL);
                kit::tile(r, art, Action::Go(AppState::Achievements), 6, "trophy", tr(l, "menu.achievements"), 140.0, kit::C_PANEL);
                kit::tile(r, art, Action::Go(AppState::Info), 7, "book", tr(l, "menu.nasa"), 140.0, kit::C_PANEL);
                kit::tile(r, art, Action::Go(AppState::Settings), 8, "gear", tr(l, "menu.settings"), 140.0, kit::C_PANEL);
            });
            c.spawn(kit::row(12.0)).with_children(|r| {
                kit::tile(r, art, Action::Go(AppState::Encyclopedia), 13, "planet", tr(l, "menu.encyclopedia"), 200.0, Color::srgb(0.3, 0.35, 0.7));
            });
            c.spawn(kit::row(8.0)).with_children(|r| {
                kit::button(r, art, Action::ToggleLang, 9, "globe", prof.0.lang.code(), false);
                kit::button(r, art, Action::ToggleSound, 10, if prof.0.sound { "sound" } else { "mute" }, "", false);
                kit::button(r, art, Action::ToggleMusic, 11, if music { "s_music" } else { "mute" }, "", false);
                kit::button(r, art, Action::Quit, 12, "exit", "", false);
            });
        });
        row.spawn(kit::col(14.0)).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(12.0), align_items: AlignItems::Center, ..default() }).with_children(|c| {
            avatar(c, art, &prof.0, 8.0);
            kit::text(c, prof.0.name(l), &art.head, 24.0, C_TEXT);
            weather_card(c, art, l, sky);
        });
    });
}

fn weather_card(p: &mut ChildSpawnerCommands, art: &Art, l: Lang, sky: &Sky) {
    let w = &sky.weather;
    p.spawn(kit::panel()).insert(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(6.0), padding: UiRect::all(Val::Px(12.0)), border: UiRect::all(Val::Px(3.0)), max_width: Val::Px(470.0), ..default() }).with_children(|c| {
        c.spawn(kit::row(8.0)).with_children(|r| {
            kit::icon(r, art, "sun", 28.0);
            kit::text(r, l.pick("Космическая погода", "Space weather"), &art.head, 19.0, C_TEXT);
            let (tag, col) = if sky.loading { (l.pick("загрузка…", "loading…"), C_MUTED) } else if w.live { ("LIVE · NASA DONKI", C_GOOD) } else { (l.pick("АРХИВ · NASA DONKI", "ARCHIVE · NASA DONKI"), C_WARN) };
            kit::text(r, tag, &art.head, 14.0, col);
        });
        for e in w.events.iter().take(3) {
            c.spawn(kit::row(6.0)).with_children(|r| {
                let col = match e.kind.as_str() { "FLR" => C_ACCENT, "GST" => Color::srgb(0.65, 0.5, 1.0), "SEP" => C_BAD, _ => C_BLUE };
                r.spawn((Node { padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)), ..default() }, BackgroundColor(col), BorderRadius::all(Val::Px(3.0)))).with_children(|t| {
                    kit::text(t, &e.kind, &art.head, 12.0, Color::BLACK);
                });
                kit::wrap_text(r, format!("{} · {}", l.pick(&e.label_ru, &e.label_en), e.date), &art.body, 14.0, C_MUTED, 380.0);
            });
        }
        kit::text(c, l.pick("Вспышка в игре — настоящая вспышка на Солнце.", "A flare in the game is a real flare from the Sun."), &art.body, 13.0, C_MUTED);
    });
}
