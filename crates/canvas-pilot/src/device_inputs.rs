//! Adapter-aware input-owned preprocessing drafts. No layer crop or routing mutations.
use crate::{
    device::{Profile, Snapshot},
    device_send::Change,
    formats::Crop,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub enum Kind {
    Number(i64, i64),
    Choice(Vec<String>),
    Toggle,
}
#[derive(Clone, Debug)]
pub struct Field {
    pub path: String,
    pub label: String,
    pub group: &'static str,
    pub kind: Kind,
}
#[derive(Clone, Debug)]
pub struct InputDraft {
    pub input: u32,
    pub profile: Profile,
    pub plug: String,
    pub native: Option<[f64; 2]>,
    pub fields: Vec<Field>,
    pub values: BTreeMap<String, Value>,
    original: BTreeMap<String, Value>,
    observed: Vec<(String, Value)>,
    kept_groups: BTreeSet<String>,
}
impl InputDraft {
    pub fn read(snapshot: &Snapshot, input: u32) -> Result<Self, String> {
        let aquilon = snapshot.profile == Profile::Aquilon6;
        if !(1..=snapshot.profile.input_limit()).contains(&input) {
            return Err("Invalid device input.".into());
        }
        let prefix = if aquilon { "IN" } else { "INPUT" };
        let root = format!("/device/inputList/items/{prefix}_{input}");
        let item = snapshot.raw.pointer(&root).ok_or("Input unavailable")?;
        if item
            .pointer("/status/pp/isAvailable")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err("Input unavailable".into());
        }
        let plug = item
            .pointer("/status/pp/plug")
            .and_then(Value::as_str)
            .ok_or("Active plug unavailable")?
            .to_owned();
        if plug.is_empty() || !plug.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Invalid input plug".into());
        }
        let base = format!("{root}/plugList/items/{plug}");
        let p = snapshot.raw.pointer(&base).ok_or("Plug unavailable")?;
        let dimensions = ["imageWidth", "imageHeight"].map(|key| {
            p.pointer(&format!(
                "/status/signal/pp/{}",
                key.replace("image", "aspect")
            ))
            .or_else(|| p.pointer(&format!("/status/signal/pp/{key}")))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
        });
        let native = dimensions
            .iter()
            .all(|v| v.is_finite() && *v >= 1.0)
            .then_some(dimensions)
            .filter(|_| {
                p.pointer("/status/signal/pp/isValid")
                    .and_then(Value::as_bool)
                    == Some(true)
            });
        let mut fields = Vec::new();
        let mut add = |suffix: &str, label: &str, group, kind| {
            if let Some(value) = p.pointer(&format!("/{suffix}")) {
                let valid = match &kind {
                    Kind::Number(_, _) => value.as_i64().is_some(),
                    Kind::Choice(_) => value.is_string(),
                    Kind::Toggle => value.is_boolean(),
                };
                if valid {
                    fields.push(Field {
                        path: format!("{base}/{suffix}"),
                        label: label.into(),
                        group,
                        kind,
                    });
                }
            }
        };
        let choices =
            |options: &[&str]| Kind::Choice(options.iter().map(|v| (*v).into()).collect());
        for (key, label, options) in [
            (
                "signal",
                "Content ratio",
                &[
                    "NATIVE", "5_4", "4_3", "16_10", "15_9", "16_9", "21_9", "256_135", "64_27",
                ][..],
            ),
            (
                "transformTo",
                "Transform to",
                &[
                    if aquilon {
                        "KEEP_INPUT_RATIO"
                    } else {
                        "SIGNAL"
                    },
                    "5_4",
                    "4_3",
                    "16_10",
                    "15_9",
                    "16_9",
                    "21_9",
                    "CUSTOM",
                ][..],
            ),
            (
                "layerFill",
                "Layer fill option",
                &["LAYER_SETTING", "1_1", "CENTERED", "FULLSCREEN", "CROPPED"][..],
            ),
        ] {
            add(
                &format!("settings/aspect/pp/{key}"),
                label,
                "Aspect",
                choices(
                    &options
                        .iter()
                        .copied()
                        .filter(|v| !aquilon || !matches!(*v, "256_135" | "LAYER_SETTING"))
                        .collect::<Vec<_>>(),
                ),
            );
        }
        add(
            "settings/aspect/pp/customRatio",
            "Custom ratio",
            "Aspect",
            Kind::Number(250, 10000),
        );
        add(
            "settings/cropping/control/pp/predefined",
            "Crop preset",
            "Crop",
            choices(&[
                "NONE",
                "LETTERBOX_1_78",
                "LETTERBOX_1_85",
                "LETTERBOX_2_35",
                "PILLARBOX_1_33",
            ]),
        );
        for edge in ["left", "right", "top", "bottom"] {
            add(
                &format!("settings/cropping/control/pp/{edge}"),
                edge,
                "Crop",
                Kind::Number(
                    0,
                    if aquilon && matches!(edge, "left" | "right") {
                        32767
                    } else {
                        8191
                    },
                ),
            );
        }
        for (key, label) in [
            ("brightness", "Brightness"),
            ("contrast", "Contrast"),
            ("saturation", "Saturation"),
            ("hue", "Hue"),
            ("gainR", "Red contrast"),
            ("gainG", "Green contrast"),
            ("gainB", "Blue contrast"),
            ("offsetR", "Red brightness"),
            ("offsetG", "Green brightness"),
            ("offsetB", "Blue brightness"),
        ] {
            add(
                &format!("settings/color/pp/{key}"),
                label,
                "Color",
                match (aquilon, key) {
                    (true, "hue") => Kind::Number(90, 270),
                    (true, _) => Kind::Number(0, 255),
                    (false, "hue") => Kind::Number(-90, 90),
                    (false, _) => Kind::Number(-128, 127),
                },
            );
        }
        add(
            "settings/processing/pp/sharpness",
            "Sharpness",
            "Processing",
            choices(&["LOW", "MEDIUM", "HIGH"]),
        );
        for key in ["pulldown22", "pulldown32"] {
            add(
                &format!("settings/processing/pp/{key}"),
                key,
                "Processing",
                Kind::Toggle,
            );
        }
        if aquilon
            || item
                .pointer("/status/keying/pp/isAvailable")
                .and_then(Value::as_bool)
                == Some(true)
        {
            let mut modes = vec!["DISABLE", "CHROMA", "LUMA"];
            if item
                .pointer("/status/keying/cutNFill/pp/isAvailable")
                .and_then(Value::as_bool)
                == Some(true)
            {
                modes.push("CUT_AND_FILL");
            }
            if aquilon {
                let available = p
                    .pointer("/settings/keying/status/pp/modeValidity")
                    .and_then(Value::as_array);
                modes.retain(|mode| {
                    available.is_some_and(|values| values.iter().any(|v| v.as_str() == Some(mode)))
                });
            }
            add(
                "settings/keying/control/pp/mode",
                "Keying mode",
                "Keying",
                choices(&modes),
            );
            for (kind, key, max) in [
                ("chroma", "hue", 360),
                ("chroma", "transparency", 255),
                ("chroma", "colorCorrection", 255),
                ("chroma", "foreground", 255),
                ("chroma", "background", 255),
                ("luma", "luma", 1023),
                ("luma", "foreground", 255),
                ("luma", "transparency", 255),
            ] {
                add(
                    &format!("settings/keying/{kind}/pp/{key}"),
                    &format!("{kind}: {key}"),
                    "Keying",
                    Kind::Number(0, max),
                );
            }
            add(
                "settings/keying/luma/pp/invert",
                "Invert luma key",
                "Keying",
                Kind::Toggle,
            );
        }
        // Use the device's published validity list for resource-dependent LUT selection.
        if p.pointer("/settings/correctionLut/status/pp/isEnabled")
            .and_then(Value::as_bool)
            == Some(true)
        {
            add(
                "settings/correctionLut/control/pp/mode",
                "Correction mode",
                "LUT",
                choices(&["MANUAL", "LUT"]),
            );
            if let Some(list) = p
                .pointer("/settings/correctionLut/status/pp/sourceValidity")
                .and_then(Value::as_array)
            {
                let options = list
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect();
                add(
                    "settings/correctionLut/control/pp/source",
                    "Correction LUT",
                    "LUT",
                    Kind::Choice(options),
                );
            }
        }
        let original: BTreeMap<_, _> = fields
            .iter()
            .map(|f| {
                (
                    f.path.clone(),
                    snapshot.raw.pointer(&f.path).unwrap().clone(),
                )
            })
            .collect();
        let mut observed = vec![(
            format!("{root}/status/pp"),
            item.pointer("/status/pp").unwrap().clone(),
        )];
        for suffix in [
            "status/signal/pp",
            "settings/cropping/control/pp",
            "settings/cropping/status/pp",
            "settings/aspect/pp",
            "settings/keying/control/pp",
            "settings/correctionLut/status/pp",
        ] {
            if let Some(value) = p.pointer(&format!("/{suffix}")) {
                observed.push((format!("{base}/{suffix}"), value.clone()));
            }
        }
        Ok(Self {
            input,
            profile: snapshot.profile,
            plug,
            native,
            fields,
            values: original.clone(),
            original,
            observed,
            kept_groups: BTreeSet::new(),
        })
    }
    pub fn dirty(&self) -> bool {
        self.fields.iter().any(|f| {
            !self.keeps_group(f.group) && self.values.get(&f.path) != self.original.get(&f.path)
        })
    }
    pub fn keeps_group(&self, group: &str) -> bool {
        self.kept_groups.contains(group)
    }
    /// Keep means no writes, never restoring stale device values.
    pub fn keep_group(&mut self, group: &str, keep: bool) {
        if keep {
            self.kept_groups.insert(group.to_owned());
        } else {
            self.kept_groups.remove(group);
        }
        self.restore_kept_groups();
    }
    pub fn restore_kept_groups(&mut self) {
        for field in &self.fields {
            if self.keeps_group(field.group) {
                self.values
                    .insert(field.path.clone(), self.original[&field.path].clone());
            }
        }
    }
    pub fn reset(&mut self) {
        self.values = self.original.clone();
    }
    pub fn crop(&self) -> Crop {
        let value = |edge| {
            self.values
                .get(&self.crop_path(edge))
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
        };
        Crop {
            left: value("left"),
            right: value("right"),
            top: value("top"),
            bottom: value("bottom"),
        }
    }
    fn crop_path(&self, edge: &str) -> String {
        format!(
            "/device/inputList/items/{}_{}/plugList/items/{}/settings/cropping/control/pp/{edge}",
            if self.profile == Profile::Aquilon6 {
                "IN"
            } else {
                "INPUT"
            },
            self.input,
            self.plug
        )
    }
    pub fn set_crop(&mut self, crop: Crop) -> Result<(), String> {
        let crop = Crop {
            left: crop.left.round(),
            right: crop.right.round(),
            top: crop.top.round(),
            bottom: crop.bottom.round(),
        };
        crop.remaining(self.native.ok_or("No valid input dimensions")?)?;
        for (edge, v) in [
            ("left", crop.left),
            ("right", crop.right),
            ("top", crop.top),
            ("bottom", crop.bottom),
        ] {
            let max = if self.profile == Profile::Aquilon6 && matches!(edge, "left" | "right") {
                32767.
            } else {
                8191.
            };
            if !(0.0..=max).contains(&v) {
                return Err("Input crop exceeds device range".into());
            }
        }
        if ["left", "right", "top", "bottom"]
            .iter()
            .any(|edge| !self.values.contains_key(&self.crop_path(edge)))
        {
            return Err("Input crop field unavailable".into());
        }
        for (edge, v) in [
            ("left", crop.left),
            ("right", crop.right),
            ("top", crop.top),
            ("bottom", crop.bottom),
        ] {
            let path = self.crop_path(edge);
            if !self.values.contains_key(&path) {
                return Err("Input crop field unavailable".into());
            }
            self.values.insert(path, json!(v as i64));
        }
        let path = self.crop_path("predefined");
        if self.values.contains_key(&path) {
            self.values.insert(path, json!("NONE"));
        }
        Ok(())
    }
    pub fn zoom_crop(&mut self, factor: f64, pivot: [f64; 2]) -> Result<(), String> {
        if !factor.is_finite()
            || factor <= 0.0
            || pivot
                .iter()
                .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
        {
            return Err("Invalid crop zoom/pivot".into());
        }
        let native = self.native.ok_or("No valid input dimensions")?;
        let c = self.crop();
        let retained = c.remaining(native)?;
        let size = [
            (retained[0] / factor).clamp(1.0, native[0]),
            (retained[1] / factor).clamp(1.0, native[1]),
        ];
        let left = (c.left + (retained[0] - size[0]) * pivot[0]).clamp(0.0, native[0] - size[0]);
        let top = (c.top + (retained[1] - size[1]) * pivot[1]).clamp(0.0, native[1] - size[1]);
        self.set_crop(Crop {
            left,
            top,
            right: native[0] - left - size[0],
            bottom: native[1] - top - size[1],
        })
    }
    pub fn changes(&self, fresh: &Snapshot) -> Result<Vec<Change>, String> {
        if fresh.profile != self.profile {
            return Err("Device family changed".into());
        }
        for (path, value) in &self.observed {
            if fresh.raw.pointer(path) != Some(value) {
                return Err("Input signal, plug or preprocessing changed. Refresh and review before sending.".into());
            }
        }
        if self.values.keys().ne(self.original.keys()) {
            return Err("Incomplete input draft".into());
        }
        let mut changes = Vec::new();
        for (path, after) in &self.values {
            let before = self.original.get(path).ok_or("Unknown input setting")?;
            if after == before {
                continue;
            }
            let field = self
                .fields
                .iter()
                .find(|f| &f.path == path)
                .ok_or("Unknown input setting")?;
            if self.keeps_group(field.group) {
                continue;
            }
            let valid = match &field.kind {
                Kind::Number(min, max) => {
                    after.as_i64().is_some_and(|v| (*min..=*max).contains(&v))
                }
                Kind::Choice(options) => after
                    .as_str()
                    .is_some_and(|v| options.iter().any(|o| o == v)),
                Kind::Toggle => after.is_boolean(),
            };
            if !valid {
                return Err(format!("Invalid {}", field.label));
            }
            if fresh.raw.pointer(path) != Some(before) {
                return Err(format!("{} changed on device; refresh first", field.label));
            }
            changes.push(Change {
                path: path
                    .trim_start_matches('/')
                    .split('/')
                    .map(str::to_owned)
                    .collect(),
                before: before.clone(),
                after: after.clone(),
            });
        }
        if changes
            .iter()
            .any(|c| c.path.iter().any(|p| p == "cropping"))
        {
            self.crop().remaining(
                self.native
                    .ok_or("A valid input signal is required to change crop")?,
            )?;
        }
        changes.sort_by_key(|c| {
            !(c.path.iter().any(|p| p == "cropping")
                && c.after
                    .as_i64()
                    .zip(c.before.as_i64())
                    .is_some_and(|(a, b)| a < b))
        });
        Ok(changes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> Snapshot {
        Snapshot {
            identity: "test".into(),
            profile: Profile::Midra4k,
            screens: vec![],
            raw: json!({"device":{"inputList":{"items":{"INPUT_1":{
                "status":{"pp":{"isAvailable":true,"plug":"1"}},
                "plugList":{"items":{"1":{
                    "status":{"signal":{"pp":{"isValid":true,"imageWidth":1920,"imageHeight":1080}}},
                    "settings":{
                        "cropping":{"control":{"pp":{"left":0,"right":0,"top":0,"bottom":0,"predefined":"NONE","xUpdate":true}}},
                        "aspect":{"pp":{"signal":"NATIVE","transformTo":"SIGNAL","customRatio":1778,"layerFill":"LAYER_SETTING"}},
                        "color":{"pp":{"brightness":0,"hue":0}}
                    }
                }}}
            }}}}}),
        }
    }
    #[test]
    fn kept_categories_never_send_and_discard_their_edits() {
        let s = snapshot();
        let mut d = InputDraft::read(&s, 1).unwrap();
        let brightness = d
            .fields
            .iter()
            .find(|f| f.label == "Brightness")
            .unwrap()
            .path
            .clone();
        d.values.insert(brightness.clone(), json!(10));
        d.keep_group("Color", true);
        assert_eq!(d.values[&brightness], json!(0));
        assert!(!d.dirty());
        // Even a stale undo value cannot bypass the send boundary.
        d.values.insert(brightness.clone(), json!(20));
        assert!(d.changes(&s).unwrap().is_empty());
        assert!(!d.dirty());
        d.zoom_crop(2., [0.5, 0.5]).unwrap();
        assert!(
            d.changes(&s)
                .unwrap()
                .iter()
                .all(|c| !c.path.iter().any(|p| p == "color"))
        );
        d.restore_kept_groups();
        d.keep_group("Color", false);
        d.values.insert(brightness, json!(15));
        assert!(
            d.changes(&s)
                .unwrap()
                .iter()
                .any(|c| c.path.iter().any(|p| p == "color"))
        );
        d.keep_group("Crop", true);
        assert_eq!(d.crop(), Crop::default());
        d.reset();
        assert!(!d.dirty());
        assert!(d.keeps_group("Crop"));
    }
    #[test]
    fn aquilon_uses_aspect_bounds_and_unsigned_color_ranges() {
        let mut s = snapshot();
        s.profile = Profile::Aquilon6;
        let mut input = s.raw["device"]["inputList"]["items"]["INPUT_1"].take();
        let plug = &mut input["plugList"]["items"]["1"];
        plug["status"]["signal"]["pp"]["aspectWidth"] = json!(3840);
        plug["status"]["signal"]["pp"]["aspectHeight"] = json!(2160);
        plug["settings"]["aspect"]["pp"]["transformTo"] = json!("KEEP_INPUT_RATIO");
        plug["settings"]["color"]["pp"]["brightness"] = json!(128);
        plug["settings"]["color"]["pp"]["hue"] = json!(180);
        s.raw["device"]["inputList"]["items"]["IN_1"] = input;
        let mut d = InputDraft::read(&s, 1).unwrap();
        assert_eq!(d.native, Some([3840., 2160.]));
        let path = d
            .fields
            .iter()
            .find(|f| f.label == "Brightness")
            .unwrap()
            .path
            .clone();
        d.values.insert(path.clone(), json!(255));
        assert!(d.changes(&s).is_ok());
        d.values.insert(path, json!(-1));
        assert!(d.changes(&s).is_err());
        d.reset();
        d.zoom_crop(2., [0.5, 0.5]).unwrap();
        assert_eq!(d.crop().left, 960.);
    }
    #[test]
    fn crop_zoom_anchors_and_rounds_native_pixels() {
        let s = snapshot();
        let mut d = InputDraft::read(&s, 1).unwrap();
        d.zoom_crop(2., [0.5, 0.5]).unwrap();
        assert_eq!(
            d.crop(),
            Crop {
                left: 480.,
                right: 480.,
                top: 270.,
                bottom: 270.
            }
        );
        assert_eq!(d.changes(&s).unwrap().len(), 4);
        d.reset();
        d.zoom_crop(2., [0., 1.]).unwrap();
        assert_eq!(
            d.crop(),
            Crop {
                left: 0.,
                right: 960.,
                top: 540.,
                bottom: 0.
            }
        );
        d.zoom_crop(0.5, [0., 1.]).unwrap();
        assert!(!d.dirty());
    }
    #[test]
    fn invalid_or_incomplete_crop_is_atomic() {
        let mut d = InputDraft::read(&snapshot(), 1).unwrap();
        let before = d.values.clone();
        assert!(
            d.set_crop(Crop {
                left: 1920.,
                ..Crop::default()
            })
            .is_err()
        );
        assert_eq!(before, d.values);
        d.values.remove(&d.crop_path("bottom"));
        let before = d.values.clone();
        assert!(d.zoom_crop(2., [0.5, 0.5]).is_err());
        assert_eq!(before, d.values);
    }
    #[test]
    fn preflight_rejects_stale_signal_plug_crop_and_changed_value() {
        let s = snapshot();
        let mut d = InputDraft::read(&s, 1).unwrap();
        let key = d
            .fields
            .iter()
            .find(|f| f.label == "Brightness")
            .unwrap()
            .path
            .clone();
        d.values.insert(key.clone(), json!(12));
        assert_eq!(d.changes(&s).unwrap().len(), 1);
        for (path, value) in [
            ("/device/inputList/items/INPUT_1/status/pp/plug", json!("2")),
            (
                "/device/inputList/items/INPUT_1/plugList/items/1/status/signal/pp/imageWidth",
                json!(1280),
            ),
            (&d.crop_path("left"), json!(30)),
            (&key, json!(1)),
        ] {
            let mut fresh = s.clone();
            *fresh.raw.pointer_mut(path).unwrap() = value;
            assert!(d.changes(&fresh).is_err(), "{path}");
        }
        d.values.insert(key, json!(128));
        assert!(d.changes(&s).is_err());
    }
    #[test]
    fn unavailable_input_and_unknown_family_are_not_editable() {
        let mut s = snapshot();
        s.profile = Profile::Aquilon6;
        assert!(InputDraft::read(&s, 1).is_err());
        s.profile = Profile::Midra4k;
        s.raw["device"]["inputList"]["items"]["INPUT_1"]["status"]["pp"]["isAvailable"] =
            json!(false);
        assert!(InputDraft::read(&s, 1).is_err());
    }
}
