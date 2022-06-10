use crate::error::{Error, Result};
use serde::de::{Deserializer, Error as SerdeError};
use serde::{Deserialize, Serialize};
use sscanf::scanf;
use std::fs;
use std::result::Result as StdResult;
use std::str::FromStr;

/// Name of the default configuration file.
pub const DEFAULT_CONFIG: &str = concat!(env!("CARGO_PKG_NAME"), ".toml");

/// Configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Global configuration.
    pub global: GlobalConfig,
}

impl Config {
    /// Parses the configuration file.
    pub fn parse(file: &str) -> Result<Self> {
        let contents = fs::read_to_string(file)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }
}

/// Global configuration.
#[derive(Debug, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// Geometry of the notification window.
    #[serde(deserialize_with = "deserialize_geometry_from_string")]
    pub geometry: Geometry,
    /// Text font.
    pub font: String,
}

/// Custom deserializer implementation for converting `String` to [`Geometry`]
fn deserialize_geometry_from_string<'de, D>(deserializer: D) -> StdResult<Geometry, D::Error>
where
    D: Deserializer<'de>,
{
    let value: &str = Deserialize::deserialize(deserializer)?;
    Geometry::from_str(value).map_err(SerdeError::custom)
}

/// Window geometry.
#[derive(Debug, Deserialize, Serialize)]
pub struct Geometry {
    /// Width of the window.
    pub width: u32,
    /// Height of the window.
    pub height: u32,
    /// X coordinate.
    pub x: u32,
    /// Y coordinate.
    pub y: u32,
}

impl<'a> FromStr for Geometry {
    type Err = Error;
    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        let (width, height, x, y) =
            scanf!(s, "{u32}x{u32}+{u32}+{u32}").map_err(|e| Error::Scanf(e.to_string()))?;
        Ok(Self {
            width,
            height,
            x,
            y,
        })
    }
}
