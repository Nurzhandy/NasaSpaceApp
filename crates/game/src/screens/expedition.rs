//! B8/B9 — new expedition: survival or creative (only here, never during play), Junior or
//! Advanced, one or two players, and the crew's roles.

use super::*;

pub(super) fn expedition(p: &mut ChildSpawnerCommands, art: &Art, prof: &Prof, l: Lang, setup: &Setup) {
    let creative = setup.mode == ja_core::Mode::Sandbox;
    let junior = prof.0.cadet;
    let pick = |p: &mut ChildSpawnerCommands, title: &str, opts: [(Action, &str, &str, &str, bool); 2], order: u16| {
        p.spawn(kit::col(6.0)).with_children(|c| {
            kit::text(c, title, &art.head, 20.0, C_TEXT);
            c.spawn(kit::row(12.0)).with_children(|r| {
                for (i, (act, icon, name, desc, on)) in opts.into_iter().enumerate() {
                    card(r, art, act, order + i as u16, icon, name, desc, on, 300.0);
                }
            });
        });
    };
    body(p, |c| {
        kit::text(c, tr(l, "exp.title"), &art.head, 32.0, C_TEXT);
        pick(c, tr(l, "exp.mode"), [
            (Action::SetCreative(false), "shield", tr(l, "exp.survival"), tr(l, "exp.survival.desc"), !creative),
            (Action::SetCreative(true), "paint", tr(l, "exp.creative"), tr(l, "exp.creative.desc"), creative),
        ], 0);
        pick(c, tr(l, "exp.level"), [
            (Action::SetJunior(true), "seedling", "Junior 7–12", tr(l, "exp.junior.desc"), junior),
            (Action::SetJunior(false), "rocket", "Advanced 12+", tr(l, "exp.advanced.desc"), !junior),
        ], 2);
        pick(c, tr(l, "exp.players"), [
            (Action::SetPlayers(false), "astronaut", tr(l, "exp.one"), tr(l, "exp.one.desc"), !setup.coop),
            (Action::SetPlayers(true), "coop", tr(l, "exp.two"), tr(l, "exp.two.desc"), setup.coop),
        ], 4);
        c.spawn(kit::col(6.0)).with_children(|c| {
            kit::text(c, tr(l, "exp.roles"), &art.head, 20.0, C_TEXT);
            c.spawn(kit::row(12.0)).with_children(|r| {
                // player 1's role lives in the profile (changed on the character screen)
                let r1 = prof.0.role;
                card(r, art, Action::Go(AppState::Character), 6, r1.icon(), &format!("{}: {}", tr(l, "exp.p1"), role_name(l, r1)), role_bonus(l, r1), false, 300.0);
                if setup.coop {
                    let r2 = setup.roles[1];
                    card(r, art, Action::CycleRole2, 7, r2.icon(), &format!("{}: {}", tr(l, "exp.p2"), role_name(l, r2)), role_bonus(l, r2), false, 300.0);
                }
            });
        });
        c.spawn(kit::row(12.0)).with_children(|r| {
            kit::button(r, art, Action::Go(AppState::Menu), 9, "back", tr(l, "common.back"), false);
            kit::button(r, art, Action::StartExpedition, 8, "rocket", tr(l, "exp.launch"), true);
        });
    });
}
