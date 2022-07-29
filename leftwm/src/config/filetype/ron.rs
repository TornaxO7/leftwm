use crate::{Config, CONFIG_DIR_PATH};


fn load_from_file() -> Result<Config> {
    let config_filename = CONFIG_DIR_PATH.place_config_file("config.ron")?;

        let config = ron::from_str(&contents)?;
}
