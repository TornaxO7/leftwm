use leftwm_core::Config;

#[cfg(feature = "toml_config")]
pub mod toml;

#[must_use]
pub fn get_config<C: Config + Default>() -> C {
    #[cfg(feature = "toml_config")]
    toml::get_config()
}
