//! Unit-safe angle aliases used by public APIs.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Radians(pub f32);

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Degrees(pub f32);

impl Degrees {
    pub fn to_radians(self) -> Radians {
        Radians(self.0.to_radians())
    }
}

impl Radians {
    pub fn to_degrees(self) -> Degrees {
        Degrees(self.0.to_degrees())
    }
}
