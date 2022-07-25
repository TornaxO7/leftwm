use std::{
    fs::{self, File},
    path::{Path, PathBuf}, io::Write,
};

use anyhow::Result;
use leftwm_core::{
    config::{FocusBehaviour, Gutter, InsertBehavior, Margins, ScratchPad, Size, Workspace},
    layouts::Layout,
    models::LayoutMode,
    Config, DisplayServer, Manager, State, Window,
};
use serde::{Deserialize, Serialize};
use xdg::BaseDirectories;

use crate::{
    check_workspace_ids,
    config::{
        absolute_path,
        keybind::{Keybind, Modifier},
    },
    ThemeSetting, WindowHook, STATE_FILE,
};

const CONFIG_FILE: &str = "config.toml";

/// # Panics
///
/// Function can only panic if toml cannot be serialized. This should not occur as it is defined
/// globally.
///
/// # Errors
///
/// Function will throw an error if `BaseDirectories` doesn't exist, if user doesn't have
/// permissions to place config.toml, if config.toml cannot be read (access writes, malformed file,
/// etc.).
/// Function can also error from inability to save config.toml (if it is the first time running
/// `LeftWM`).
pub fn get_config<C: Config + Default>() -> C {
    load_config_file()
        .map_err(|err| eprintln!("ERROR LOADING CONFIG: {:?}", err))
        .unwrap_or_default()
}

fn load_config_file<C: Config + Default>() -> Result<C> {
    let path = BaseDirectories::with_prefix(CONFIG_FILE)?;
    let config_filename = path.place_config_file(CONFIG_FILE)?;
    if Path::new(&config_filename).exists() {
        let contents = fs::read_to_string(config_filename)?;
        let config = toml::from_str(&contents)?;
        if check_workspace_ids(&config) {
            Ok(config)
        } else {
            log::warn!("Invalid workspace ID configuration in config.toml. Falling back to default config.");
            Ok(TomlConfig::default())
        }
    } else {
        let config = TomlConfig::default();
        let toml = toml::to_string(&config).unwrap();
        let mut file = File::create(&config_filename)?;
        file.write_all(toml.as_bytes())?;
        Ok(config)
    }
}

/// General configuration
#[allow(clippy::struct_excessive_bools)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct TomlConfig {
    pub modkey: String,
    pub mousekey: Option<Modifier>,
    pub workspaces: Option<Vec<Workspace>>,
    pub tags: Option<Vec<String>>,
    pub max_window_width: Option<leftwm_core::config::Size>,
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

impl leftwm_core::Config for TomlConfig {
    fn mapped_bindings(&self) -> Vec<leftwm_core::Keybind> {
        // copy keybinds substituting "modkey" modifier with a new "modkey".
        self.keybind
            .clone()
            .into_iter()
            .map(|mut keybind| {
                if let Some(ref mut modifier) = keybind.modifier {
                    match modifier {
                        Modifier::Single(m) if m == "modkey" => *m = self.modkey.clone(),
                        Modifier::List(ms) => {
                            for m in ms {
                                if m == "modkey" {
                                    *m = self.modkey.clone();
                                }
                            }
                        }
                        Modifier::Single(_) => {}
                    }
                }

                keybind
            })
            .filter_map(|keybind| match keybind.try_convert_to_core_keybind(self) {
                Ok(internal_keybind) => Some(internal_keybind),
                Err(err) => {
                    log::error!("Invalid key binding: {}\n{:?}", err, keybind);
                    None
                }
            })
            .collect()
    }

    fn create_list_of_tag_labels(&self) -> Vec<String> {
        if let Some(tags) = &self.tags {
            return tags.clone();
        }
        Self::default()
            .tags
            .expect("we created it in the Default impl; qed")
    }

    fn workspaces(&self) -> Option<Vec<Workspace>> {
        self.workspaces.clone()
    }

    fn focus_behaviour(&self) -> FocusBehaviour {
        self.focus_behaviour
    }

    fn mousekey(&self) -> Vec<String> {
        self.mousekey
            .as_ref()
            .unwrap_or(&"Mod4".into())
            .clone()
            .into()
    }

    fn create_list_of_scratchpads(&self) -> Vec<ScratchPad> {
        if let Some(scratchpads) = &self.scratchpad {
            return scratchpads.clone();
        }
        return vec![];
    }

    fn layouts(&self) -> Vec<Layout> {
        self.layouts.clone()
    }

    fn layout_mode(&self) -> LayoutMode {
        self.layout_mode
    }

    fn insert_behavior(&self) -> InsertBehavior {
        self.insert_behavior
    }

    fn focus_new_windows(&self) -> bool {
        self.focus_new_windows
    }

    fn command_handler<SERVER: DisplayServer>(
        command: &str,
        manager: &mut Manager<Self, SERVER>,
    ) -> bool {
        if let Some((command, value)) = command.split_once(' ') {
            match command {
                "LoadTheme" => {
                    if let Some(absolute) = absolute_path(value.trim()) {
                        manager.config.theme_setting.load(absolute);
                    } else {
                        log::warn!("Path submitted does not exist.");
                    }
                    return manager.reload_config();
                }
                "UnloadTheme" => {
                    manager.config.theme_setting = ThemeSetting::default();
                    return manager.reload_config();
                }
                _ => {
                    log::warn!("Command not recognized: {}", command);
                    return false;
                }
            }
        }
        false
    }

    fn border_width(&self) -> i32 {
        self.theme_setting.border_width
    }

    fn margin(&self) -> Margins {
        match self.theme_setting.margin.clone().try_into() {
            Ok(margins) => margins,
            Err(err) => {
                log::warn!("Could not read margin: {}", err);
                Margins::new(0)
            }
        }
    }

    fn workspace_margin(&self) -> Option<Margins> {
        self.theme_setting
            .workspace_margin
            .clone()
            .and_then(|custom_margin| match custom_margin.try_into() {
                Ok(margins) => Some(margins),
                Err(err) => {
                    log::warn!("Could not read margin: {}", err);
                    None
                }
            })
    }

    fn gutter(&self) -> Option<Vec<Gutter>> {
        self.theme_setting.gutter.clone()
    }

    fn default_border_color(&self) -> String {
        self.theme_setting.default_border_color.clone()
    }

    fn floating_border_color(&self) -> String {
        self.theme_setting.floating_border_color.clone()
    }

    fn disable_window_snap(&self) -> bool {
        self.disable_window_snap
    }

    fn always_float(&self) -> bool {
        self.theme_setting.always_float.unwrap_or(false)
    }

    fn default_width(&self) -> i32 {
        self.theme_setting.default_width.unwrap_or(800)
    }

    fn default_height(&self) -> i32 {
        self.theme_setting.default_height.unwrap_or(600)
    }

    fn focused_border_color(&self) -> String {
        self.theme_setting.focused_border_color.clone()
    }

    fn on_new_window_cmd(&self) -> Option<String> {
        self.theme_setting.on_new_window_cmd.clone()
    }

    fn get_list_of_gutters(&self) -> Vec<Gutter> {
        self.theme_setting.gutter.clone().unwrap_or_default()
    }

    fn max_window_width(&self) -> Option<Size> {
        self.max_window_width
    }

    fn disable_tile_drag(&self) -> bool {
        self.disable_tile_drag
    }

    fn save_state(&self, state: &State) {
        let path = self.state_file();
        let state_file = match File::create(&path) {
            Ok(file) => file,
            Err(err) => {
                log::error!("Cannot create file at path {}: {}", path.display(), err);
                return;
            }
        };
        if let Err(err) = serde_json::to_writer(state_file, state) {
            log::error!("Cannot save state: {}", err);
        }
    }

    fn load_state(&self, state: &mut State) {
        let path = self.state_file().to_owned();
        match File::open(&path) {
            Ok(file) => {
                match serde_json::from_reader(file) {
                    Ok(old_state) => state.restore_state(&old_state),
                    Err(err) => log::error!("Cannot load old state: {}", err),
                }
                // Clean old state.
                if let Err(err) = std::fs::remove_file(&path) {
                    log::error!("Cannot remove old state file: {}", err);
                }
            }
            Err(err) => log::error!("Cannot open old state: {}", err),
        }
    }

    /// Pick the best matching [`WindowHook`], if any, and apply its config.
    fn setup_predefined_window(&self, window: &mut Window) -> bool {
        if let Some(window_rules) = &self.window_rules {
            let best_match = window_rules
                .iter()
                // map first instead of using max_by_key directly...
                .map(|wh| (wh, wh.score_window(window)))
                // ...since this filter is required (0 := non-match)
                .filter(|(_wh, score)| score != &0)
                .max_by_key(|(_wh, score)| *score);
            if let Some((hook, _)) = best_match {
                hook.apply(window);
                log::debug!(
                    "Window [[ TITLE={:?}, {:?}; WM_CLASS={:?}, {:?} ]] spawned in tag={:?} with floating={:?}",
                    window.name,
                    window.legacy_name,
                    window.res_name,
                    window.res_class,
                    hook.spawn_on_tag,
                    hook.spawn_floating,
                );
                return true;
            }
            return false;
        }
        false
    }

    fn sloppy_mouse_follows_focus(&self) -> bool {
        self.sloppy_mouse_follows_focus
    }
}

impl TomlConfig {
    fn state_file(&self) -> &Path {
        self.state
            .as_deref()
            .unwrap_or_else(|| Path::new(STATE_FILE))
    }
}
