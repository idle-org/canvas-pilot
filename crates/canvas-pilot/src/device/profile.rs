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
// Model identifiers documented by Analog Way's AWJ guide V6.2, section 2.1.
// Keep exact matches: an unknown AQL product must not inherit this adapter.
pub(super) fn aquilon_model(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "AQL RS alpha" | "AQL RS Alpha" => "NLC_RSALPHA",
        "AQL RS1" => "NLC_RS1",
        "AQL RS2" => "NLC_RS2",
        "AQL RS3" => "NLC_RS3",
        "AQL RS4" => "NLC_RS4",
        "AQL RS5" => "NLC_RS5",
        "AQL RS6" => "NLC_RS6",
        "AQL Cmini" => "NLC_CMINI",
        "AQL C" => "NLC_C",
        "AQL C+" => "NLC_CPLUS",
        "AQL Cmax" => "NLC_CMAX",
        _ => return None,
    })
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
            if aquilon_model(&kind).is_none()
                || identity.get("isLinked").and_then(Value::as_bool) != Some(false)
            {
                return Err("Unsupported Aquilon model or linked system. Select a standalone Aquilon RS alpha, RS1–RS6, Cmini, C, C+ or Cmax.".into());
            }
            Self::Aquilon6
        } else {
            return Err(format!("Unsupported device {kind}; no compatible adapter."));
        };
        let mut label = format!(
            "{kind} · firmware {}.{}.{}",
            version[0], version[1], version[2]
        );
        if profile == Self::Aquilon6 && version[..2] != [6, 2] {
            label.push_str(" · Warning: firmware outside validated 6.2.x; backward compatibility is not guaranteed. Transfers require compatible protocol fields.");
        }
        Ok((profile, label))
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
