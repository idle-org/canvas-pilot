//! Directional routing, independent of controls and device protocol paths.
use crate::DeviceLink;
use serde::{Deserialize, Serialize};

/// Connection scope is explicit, but only the current connection exists today.
/// A future saved-connection identity can be added without changing fan-out shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTarget {
    Current,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendDestination {
    pub connection: ConnectionTarget,
    pub layer: DeviceLink,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendMapping {
    pub destinations: Vec<SendDestination>,
}

impl SendMapping {
    pub fn linked(link: Option<DeviceLink>) -> Self {
        Self {
            destinations: link
                .into_iter()
                .map(|layer| SendDestination {
                    connection: ConnectionTarget::Current,
                    layer,
                })
                .collect(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        for (i, destination) in self.destinations.iter().enumerate() {
            destination.layer.validate()?;
            if self.destinations[..i].contains(destination) {
                return Err("A send mapping contains a duplicate destination.".into());
            }
        }
        Ok(())
    }

    pub fn current_links(&self) -> impl Iterator<Item = &DeviceLink> {
        self.destinations
            .iter()
            .map(|destination| match destination.connection {
                ConnectionTarget::Current => &destination.layer,
            })
    }
}
