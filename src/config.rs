use crate::models::Config;
use color_eyre::Result;
use std::fs;
use std::path::PathBuf;

pub fn config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not find config directory"))?
        .join("carbon");

    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("config.toml"))
}

pub fn load_config() -> Result<Config> {
    let path = config_path()?;

    if path.exists() {
        let contents = fs::read_to_string(&path)?;
        let config: Config = toml::from_str(&contents)?;

        // Ensure output directory exists
        fs::create_dir_all(&config.output_directory)?;

        Ok(config)
    } else {
        // Create default config
        let config = Config::default();
        save_config(&config)?;
        Ok(config)
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let path = config_path()?;
    let contents = toml::to_string_pretty(config)?;
    fs::write(&path, contents)?;
    Ok(())
}
