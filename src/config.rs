use std::fs::File;
use std::io::Read;

use serde::{Deserialize};
use serenity::prelude::TypeMapKey;

#[derive(Deserialize)]
pub struct ConfigData {
	pub general: General,
	pub database: Database
}

#[derive(Deserialize)]
pub struct General {
	pub token: String,
	pub prefix: String
}

#[derive(Deserialize)]
pub struct Database {
	pub database_url: String,
}

impl TypeMapKey for ConfigData {
	type Value = ConfigData;
}


pub fn read_config() -> ConfigData {
	let mut config_file = File::open("./config.toml").expect("Configuration file not found");
	let mut config_content = String::new();
	config_file.read_to_string(&mut config_content).unwrap();

	let config_data: ConfigData = toml::from_str(&config_content)
		.expect("Config file was not a valid TOML file or something was missing");

	config_data
}