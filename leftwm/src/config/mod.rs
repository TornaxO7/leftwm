//! `LeftWM` general configuration

mod checks;
mod default;
mod filtetypes;
mod keybind;

use std::path::PathBuf;
use std::{env, fs};
use leftwm_core::{Window, Config, Workspace};
use serde::{Serialize, Deserialize};

/// Path to file where state will be dumper upon soft reload.
pub const STATE_FILE: &str = "/tmp/leftwm.state";

/// Selecting by `WM_CLASS` and/or window title, allow the user to define if a
/// window should spawn on a specified tag and/or its floating state.
///
/// # Example
///
/// In `config.toml`
///
/// ```toml
/// [[window_config_by_class]]
/// wm_class = "krita"
/// spawn_on_tag = 3
/// spawn_floating = false
/// ```
///
/// windows whose `WM_CLASS` is "krita" will spawn on tag 3 (1-indexed) and not floating.
#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct WindowHook {
    /// `WM_CLASS` in X11
    pub window_class: Option<String>,
    /// `_NET_WM_NAME` in X11
    pub window_title: Option<String>,
    pub spawn_on_tag: Option<usize>,
    pub spawn_floating: Option<bool>,
}

impl WindowHook {
    /// Score the similarity between a [`leftwm_core::models::Window`] and a [`WindowHook`].
    ///
    /// Multiple [`WindowHook`]s might match a `WM_CLASS` but we want the most
    /// specific one to apply: matches by title are scored greater than by `WM_CLASS`.
    fn score_window(&self, window: &Window) -> u8 {
        u8::from(
            self.window_class.is_some()
                & (self.window_class == window.res_name || self.window_class == window.res_class),
        ) + 2 * u8::from(
            self.window_title.is_some()
                & ((self.window_title == window.name) | (self.window_title == window.legacy_name)),
        )
    }

    fn apply(&self, window: &mut Window) {
        if let Some(tag) = self.spawn_on_tag {
            window.tags = vec![tag];
        }
        if let Some(should_float) = self.spawn_floating {
            window.set_floating(should_float);
        }
    }
}

#[must_use]
pub fn get_config<C: Config + Default>() -> C {
    filtetypes::get_config()
}

#[must_use]
pub fn check_workspace_ids(config: &dyn Config) -> bool {
    config.workspaces.clone().map_or(true, |wss| {
        let ids = get_workspace_ids(&wss);
        if ids.iter().any(Option::is_some) {
            all_ids_some(&ids) && all_ids_unique(&ids)
        } else {
            true
        }
    })
}

pub fn get_workspace_ids(wss: &[Workspace]) -> Vec<Option<i32>> {
    wss.iter().map(|ws| ws.id).collect()
}

pub fn all_ids_some(ids: &[Option<i32>]) -> bool {
    ids.iter().all(Option::is_some)
}

#[must_use]
pub fn all_ids_unique(ids: &[Option<i32>]) -> bool {
    let mut sorted = ids.to_vec();
    sorted.sort();
    sorted.dedup();
    ids.len() == sorted.len()
}

#[must_use]
pub fn is_program_in_path(program: &str) -> bool {
    if let Ok(path) = env::var("PATH") {
        for p in path.split(':') {
            let p_str = format!("{}/{}", p, program);
            if fs::metadata(p_str).is_ok() {
                return true;
            }
        }
    }
    false
}

/// Returns a terminal to set for the default mod+shift+enter keybind.
fn default_terminal<'s>() -> &'s str {
    // order from least common to most common.
    // the thinking is if a machine has an uncommon terminal installed, it is intentional
    let terms = &[
        "alacritty",
        "termite",
        "kitty",
        "urxvt",
        "rxvt",
        "st",
        "roxterm",
        "eterm",
        "xterm",
        "terminator",
        "terminology",
        "gnome-terminal",
        "xfce4-terminal",
        "konsole",
        "uxterm",
        "guake", // at the bottom because of odd behaviour. guake wants F12 and should really be
                 // started using autostart instead of LeftWM keybind.
    ];

    // If no terminal found in path, default to a good one
    terms
        .iter()
        .find(|terminal| is_program_in_path(terminal))
        .unwrap_or(&"termite")
}

/// Returns default keybind value for exiting `LeftWM`.
// On systems that have elogind and/or systemd, the recommended way to
// kill LeftWM is to use loginctl. As we have no consistent way of knowing
// whether it is implemented on non-systemd machines,so we instead look
// to see if loginctl is in the path. If it isn't then we default to
// `pkill leftwm`, which may leave zombie processes on a machine.
fn exit_strategy<'s>() -> &'s str {
    if is_program_in_path("loginctl") {
        return "loginctl kill-session $XDG_SESSION_ID";
    }
    "pkill leftwm"
}

fn absolute_path(path: &str) -> Option<PathBuf> {
    let exp_path = shellexpand::full(path).ok()?;
    std::fs::canonicalize(exp_path.as_ref()).ok()
}
