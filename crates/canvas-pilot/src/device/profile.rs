//! Small, closed adapter registry. Add a profile here rather than branching in the editor.
use super::*;

#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    #[default]
    Midra4k,
    Aquilon,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Profile {
    #[default]
    Midra4k,
    Aquilon6,
}
impl Profile {
    pub fn family(self) -> Family {
        match self {
            Self::Midra4k => Family::Midra4k,
            Self::Aquilon6 => Family::Aquilon,
        }
    }
    pub(super) fn detect(identity: &Value) -> Result<(Self, String), String> {
        let kind = text(identity, "/type")?;
        let version: Vec<_> = ["major", "minor", "patch"]
            .iter()
            .map(|k| {
                value(identity, &format!("/version/{k}"))?
                    .as_u64()
                    .ok_or_else(|| "Invalid firmware version".to_owned())
            })
            .collect::<Result<_, _>>()?;
        let profile = if matches!(
            kind.as_str(),
            "Eikos 4K" | "Pulse 4K" | "QuickVu 4K" | "QuickMatrix 4K"
        ) {
            Self::Midra4k
        } else if kind.starts_with("AQL") {
            if kind != "AQL Cmax"
                || version[..2] != [6, 2]
                || identity.get("isLinked").and_then(Value::as_bool) != Some(false)
            {
                return Err("Unsupported Aquilon profile. Currently verified: standalone AQL Cmax firmware 6.2.x; linked systems and other firmware/models require adapter validation.".into());
            }
            Self::Aquilon6
        } else {
            return Err(format!("Unsupported device {kind}; no compatible adapter."));
        };
        Ok((
            profile,
            format!(
                "{kind} · firmware {}.{}.{}",
                version[0], version[1], version[2]
            ),
        ))
    }
    pub(crate) fn screen_context_paths(self, screen: &str) -> [String; 2] {
        match self {
            Self::Midra4k => [
                format!("/device/screenList/items/{screen}/canvas/status/size/pp"),
                format!(
                    "/device/preconfig/status/stateList/items/CURRENT/screenList/items/{screen}/pp/layerCount"
                ),
            ],
            Self::Aquilon6 => {
                let list = if screen.starts_with('A') {
                    "auxiliaryList"
                } else {
                    "screenList"
                };
                [
                    format!("/device/{list}/items/{screen}/status/size/pp"),
                    format!("/device/{list}/items/{screen}/status/pp/layerCount"),
                ]
            }
        }
    }
    pub fn layer_path(self, screen: &str, bank: &str, layer: &str) -> String {
        match self {
            Self::Midra4k => format!(
                "/device/screenList/items/{screen}/presetList/items/{bank}/liveLayerList/items/{layer}"
            ),
            Self::Aquilon6 => format!(
                "/device/{}/items/{screen}/presetList/items/{bank}/layerList/items/{layer}",
                if screen.starts_with('A') {
                    "auxiliaryList"
                } else {
                    "screenList"
                }
            ),
        }
    }
    pub fn apply_path(self) -> Vec<String> {
        match self {
            Self::Midra4k => "device/preset/control/pp/xUpdate",
            Self::Aquilon6 => "device/screenAuxGroupList/control/pp/xUpdate",
        }
        .split('/')
        .map(str::to_owned)
        .collect()
    }
    pub fn source_field(self) -> &'static str {
        match self {
            Self::Midra4k => "/source/pp/input",
            Self::Aquilon6 => "/source/pp/inputNum",
        }
    }
    pub fn input_from_source(self, source: &str) -> Option<u32> {
        source
            .strip_prefix(match self {
                Self::Midra4k => "INPUT_",
                Self::Aquilon6 => "LIVE_",
            })?
            .parse()
            .ok()
            .filter(|id| (1..=self.input_limit()).contains(id))
    }
    pub fn input_key(self, id: u32) -> String {
        format!(
            "{}{id}",
            match self {
                Self::Midra4k => "INPUT_",
                Self::Aquilon6 => "IN_",
            }
        )
    }
    pub fn input_limit(self) -> u32 {
        match self {
            Self::Midra4k => 16,
            Self::Aquilon6 => 256,
        }
    }
    pub fn advanced_inputs(self) -> bool {
        self == Self::Midra4k
    }
    pub(crate) fn prepare(
        self,
        snapshot: &Snapshot,
        layout: &crate::Layout,
        index: usize,
        mappings: &[(usize, String)],
        target: crate::device_send::Target,
        sizing: crate::device_send::Sizing,
    ) -> Result<crate::device_send::Plan, String> {
        match self {
            Self::Midra4k => crate::device_send::Plan::prepare_midra(
                snapshot, layout, index, mappings, target, sizing,
            ),
            Self::Aquilon6 => aquilon::prepare(snapshot, layout, index, mappings, target, sizing),
        }
    }
}
