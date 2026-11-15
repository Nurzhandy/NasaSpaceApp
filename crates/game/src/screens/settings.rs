//! Settings: language (RU / KK / EN), sounds, music, fullscreen, touch controls, default level.

use super::*;

pub(super) fn settings(p: &mut ChildSpawnerCommands, art: &Art, prof: &Profile, music: bool, l: Lang) {
    body(p, |c| {
        kit::text(c, tr(l, "set.title"), &art.head, 30.0, C_TEXT);
        c.spawn(kit::panel()).insert(Node { width: Val::Px(900.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(10.0), padding: UiRect::all(Val::Px(18.0)), border: UiRect::all(Val::Px(3.0)), ..default() }).with_children(|c| {
            let line = |c: &mut ChildSpawnerCommands, icon: &str, label: &str, f: &mut dyn FnMut(&mut ChildSpawnerCommands)| {
                c.spawn(kit::row(12.0)).with_children(|r| {
                    kit::icon(r, art, icon, 30.0);
                    r.spawn(Node { width: Val::Px(270.0), ..default() }).with_children(|t| {
                        kit::text(t, label, &art.body, 19.0, C_TEXT);
                    });
                    f(r);
                });
            };
            let onoff = |on: bool| if on { tr(l, "set.on") } else { tr(l, "set.off") };
            line(c, "globe", tr(l, "set.lang"), &mut |r| {
                for (i, (lang, name)) in [(Lang::Ru, "Русский"), (Lang::Kk, "Қазақша"), (Lang::En, "English")].into_iter().enumerate() {
                    kit::button(r, art, Action::SetLang(lang), i as u16, "", name, prof.lang == lang);
                }
            });
            line(c, if prof.sound { "sound" } else { "mute" }, tr(l, "set.sound"), &mut |r| {
                kit::button(r, art, Action::ToggleSound, 3, "", onoff(prof.sound), prof.sound);
            });
            line(c, if music { "s_music" } else { "mute" }, tr(l, "set.music"), &mut |r| {
                kit::button(r, art, Action::ToggleMusic, 4, "", onoff(music), music);
            });
            line(c, "map", tr(l, "set.fullscreen"), &mut |r| {
                kit::button(r, art, Action::ToggleFullscreen, 5, "", onoff(prof.fullscreen), prof.fullscreen);
            });
            line(c, "gamepad", tr(l, "set.touch"), &mut |r| {
                for (i, key) in ["set.touch.auto", "set.touch.on", "set.touch.off"].into_iter().enumerate() {
                    kit::button(r, art, Action::SetTouch(i as u8), 6 + i as u16, "", tr(l, key), prof.touch == i as u8);
                }
            });
            kit::wrap_text(c, tr(l, "set.touch.desc"), &art.body, 14.0, C_MUTED, 840.0);
            line(c, "seedling", tr(l, "set.level"), &mut |r| {
                kit::button(r, art, Action::SetJunior(true), 9, "", "Junior 7–12", prof.cadet);
                kit::button(r, art, Action::SetJunior(false), 10, "", "Advanced 12+", !prof.cadet);
            });
            c.spawn(kit::row(12.0)).insert(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::FlexStart, column_gap: Val::Px(12.0), ..default() }).with_children(|r| {
                kit::icon(r, art, "info", 30.0);
                r.spawn(kit::col(4.0)).with_children(|t| {
                    kit::text(t, tr(l, "set.controls"), &art.body, 19.0, C_TEXT);
                    kit::wrap_text(t, tr(l, "set.keys"), &art.body, 15.0, C_MUTED, 780.0);
                    if prof.lang == Lang::Kk {
                        kit::wrap_text(t, tr(l, "set.kk_note"), &art.body, 15.0, C_WARN, 780.0);
                    }
                    kit::wrap_text(t, tr(l, "set.nasa"), &art.body, 15.0, C_MUTED, 780.0);
                });
            });
        });
    });
}
