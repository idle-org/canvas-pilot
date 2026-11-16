//! Inert, explicitly saved input corrections. These are never device commands.
use crate::{DeviceLink, device::Family};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputProposal {
    pub layer_id: usize,
    pub device_link: DeviceLink,
    pub input: String,
    pub plug: String,
    /// Only the documented Midra setting suffixes are permitted; never arbitrary paths.
    pub settings: BTreeMap<String, Value>,
}
impl InputProposal {
    pub fn validate(&self) -> Result<(), String> {
        self.device_link.validate()?;
        let decimal = |s: &str| {
            !s.is_empty()
                && s.len() <= 16
                && s.bytes().all(|b| b.is_ascii_digit())
                && !s.starts_with('0')
        };
        if self.layer_id == 0
            || self.device_link.family != Family::Midra4k
            || !self.input.strip_prefix("INPUT_").is_some_and(decimal)
            || !decimal(&self.plug)
            || self.settings.is_empty()
            || self.settings.len() > 9
        {
            return Err("Invalid input proposal identity or settings count.".into());
        }
        for (key, value) in &self.settings {
            let options: &[&str] = match key.as_str() {
                "aspect/pp/signal" => &[
                    "NATIVE", "5_4", "4_3", "16_10", "15_9", "16_9", "21_9", "256_135", "64_27",
                ],
                "aspect/pp/transformTo" => &[
                    "SIGNAL", "5_4", "4_3", "16_10", "15_9", "16_9", "21_9", "CUSTOM",
                ],
                "aspect/pp/layerFill" => {
                    &["LAYER_SETTING", "1_1", "CENTERED", "FULLSCREEN", "CROPPED"]
                }
                "cropping/control/pp/predefined" => &[
                    "NONE",
                    "LETTERBOX_1_78",
                    "LETTERBOX_1_85",
                    "LETTERBOX_2_35",
                    "PILLARBOX_1_33",
                ],
                "aspect/pp/customRatio" => {
                    if !value.as_i64().is_some_and(|v| (250..=10000).contains(&v)) {
                        return Err("Invalid proposal custom ratio.".into());
                    }
                    continue;
                }
                "cropping/control/pp/left"
                | "cropping/control/pp/right"
                | "cropping/control/pp/top"
                | "cropping/control/pp/bottom" => {
                    if !value.as_i64().is_some_and(|v| (0..=8191).contains(&v)) {
                        return Err("Invalid proposal input crop.".into());
                    }
                    continue;
                }
                _ => return Err("Unsupported input proposal setting.".into()),
            };
            if !value.as_str().is_some_and(|v| options.contains(&v)) {
                return Err(format!("Invalid input proposal value for {key}."));
            }
        }
        Ok(())
    }
    pub fn base(&self) -> String {
        format!(
            "/device/inputList/items/{}/plugList/items/{}/settings",
            self.input, self.plug
        )
    }
}
