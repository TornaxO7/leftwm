//! `LeftWM` general configuration

mod checks;
mod default;
mod keybind;
mod filetype;

use self::keybind::Modifier;

use super::{BaseCommand, ThemeSetting};
use crate::config::keybind::Keybind;
use anyhow::Result;
use leftwm_core::{
    config::{InsertBehavior, ScratchPad, Workspace},
    layouts::{Layout, LAYOUTS},
    models::{FocusBehaviour, Gutter, LayoutMode, Margins, Size, Window},
    state::State,
    DisplayServer, Manager,
};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::default::Default;
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use xdg::BaseDirectories;

/// Path to file where state will be dumper upon soft reload.
const STATE_FILE: &str = "/tmp/leftwm.state";

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

/// General configuration
#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Config {
    pub modkey: String,
    pub mousekey: Option<Modifier>,
    pub workspaces: Option<Vec<Workspace>>,
    pub tags: Option<Vec<String>>,
    pub max_window_width: Option<Size>,
    pub layouts: Vec<Layout>,
    pub layout_mode: LayoutMode,
    pub insert_behavior: InsertBehavior,
    pub scratchpad: Option<Vec<ScratchPad>>,
    pub window_rules: Option<Vec<WindowHook>>,
    //of you are on tag "1" and you goto tag "1" this takes you to the previous tag
    pub disable_current_tag_swap: bool,
    pub disable_tile_drag: bool,
    pub disable_window_snap: bool,
    pub focus_behaviour: FocusBehaviour,
    pub focus_new_windows: bool,
    pub sloppy_mouse_follows_focus: bool,
    pub keybind: Vec<Keybind>,
    pub state: Option<PathBuf>,
    // NOTE: any newly added parameters must be inserted before `pub keybind: Vec<Keybind>,`
    //       at least when `TOML` is used as config language
    #[serde(skip)]
    pub theme_setting: ThemeSetting,
}

#[must_use]
pub fn get_config() -> Config {
    // load_from_file()
    //     .map_err(|err| eprintln!("ERROR LOADING CONFIG: {:?}", err))
    //     .unwrap_or_default()
}

impl Config {
    fn state_file(&self) -> &Path {
        self.state
            .as_deref()
            .unwrap_or_else(|| Path::new(STATE_FILE))
    }
}
