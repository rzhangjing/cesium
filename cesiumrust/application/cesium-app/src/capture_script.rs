//! M3.4 — batch screenshot script (`CESIUM_SCREENSHOT_SCRIPT`).
//!
//! Extends the single-shot `CESIUM_SCREENSHOT_AT_FRAME` harness into an
//! ordered multi-view capture. A TOML script lists `[[shot]]` entries, each
//! carrying the frame to capture at, the deterministic camera pose
//! (`pos`/`quat`/`fov_y`) applied that frame, and the output `name`.
//!
//! The camera pose schema is **byte-identical** to the `.camera.json` the
//! single-shot harness already writes (`{"pos":[x,y,z],"quat":[x,y,z,w],
//! "fov_y":<deg>}`), so a script entry round-trips with the captured metadata.
//!
//! # Determinism
//!
//! Poses are applied in `PostUpdate` (after the orbit camera's `Update`), so
//! the scripted transform wins for that frame's render regardless of orbit
//! state — no coupling to `orbit_camera` internals. Each shot also emits a
//! `.meta.json` stamping frame / pose / env / git SHA for `pixel_diff` gating.
//!
//! This module is pure parsing + scheduling (no Bevy, no GPU) so it is fully
//! unit-testable headless.

use serde::Deserialize;
use std::path::Path;

/// A deterministic camera pose. Mirrors the `.camera.json` schema:
/// `pos = [x, y, z]`, `quat = [x, y, z, w]`, `fov_y` in **degrees**.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct CameraPose {
    /// Camera translation (render units; 1 render unit = 6378137 m).
    pub pos: [f64; 3],
    /// Camera orientation as a quaternion `[x, y, z, w]`.
    pub quat: [f64; 4],
    /// Vertical field of view in degrees (optional; keeps current if absent).
    #[serde(default)]
    pub fov_y: Option<f64>,
}

/// `[meta]` block of a screenshot script (optional, descriptive only).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScriptMeta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// One `[[shot]]` entry: capture at `frame`, from this pose, into `name`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ShotEntry {
    /// Frame index (1-based, matching the harness counter) to capture at.
    pub frame: u32,
    /// Output base name (no extension) → `{name}.png` / `{name}.camera.json`
    /// / `{name}.meta.json` under the capture output dir.
    pub name: String,
    /// Camera translation `[x, y, z]`.
    pub pos: [f64; 3],
    /// Camera orientation quaternion `[x, y, z, w]`.
    pub quat: [f64; 4],
    /// Vertical FOV in degrees (optional).
    #[serde(default)]
    pub fov_y: Option<f64>,
}

impl ShotEntry {
    /// The entry's pose as a [`CameraPose`] (the fixed-camera schema), so a
    /// script entry can be re-applied through the same path as `FIXED_CAMERA`.
    #[allow(dead_code)]
    pub fn pose(&self) -> CameraPose {
        CameraPose {
            pos: self.pos,
            quat: self.quat,
            fov_y: self.fov_y,
        }
    }
}

/// A parsed screenshot script: optional `[meta]` + ordered `[[shot]]` list.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ShotScript {
    #[serde(default)]
    pub meta: Option<ScriptMeta>,
    #[serde(default)]
    pub shot: Vec<ShotEntry>,
}

impl ShotScript {
    /// Parses a script from a TOML string.
    ///
    /// # Errors
    /// Returns a human-readable message on malformed TOML.
    pub fn from_toml_str(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| format!("parse screenshot script: {e}"))
    }

    /// Reads and parses a script from disk.
    ///
    /// # Errors
    /// Returns a human-readable message on IO or parse failure.
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        Self::from_toml_str(&text)
    }

    /// Sorts shots by ascending `frame` for deterministic scheduling (a script
    /// authored out of order still fires in frame order).
    pub fn sorted(mut self) -> Self {
        self.shot.sort_by_key(|e| e.frame);
        self
    }
}

/// Pure scheduling helper: given shots sorted by frame, the next unfired index
/// `cursor`, and the current `frame`, returns the index of the next shot that
/// is due (`frame >= shot.frame`), or `None` when nothing is due yet.
///
/// Uses `>=` (not `==`) so a shot whose exact frame was skipped still fires
/// exactly once — every entry is captured regardless of frame hitches.
pub fn next_due(shots: &[ShotEntry], cursor: usize, frame: u32) -> Option<usize> {
    shots
        .iter()
        .enumerate()
        .skip(cursor)
        .find(|(_, e)| frame >= e.frame)
        .map(|(i, _)| i)
}

/// Builds the `.meta.json` payload for one captured shot: the fired frame, the
/// scripted pose, the git SHA, and the offline/env snapshot (`env`), so a
/// reviewer can reproduce the exact capture. Never fails (pretty-prints a
/// `serde_json` value).
pub fn metadata_json(
    entry: &ShotEntry,
    captured_frame: u32,
    git_sha: &str,
    env: &serde_json::Value,
) -> String {
    let meta = serde_json::json!({
        "name": entry.name,
        "scripted_frame": entry.frame,
        "captured_frame": captured_frame,
        "pos": entry.pos,
        "quat": entry.quat,
        "fov_y": entry.fov_y,
        "git_sha": git_sha,
        "env": env,
    });
    serde_json::to_string_pretty(&meta).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[meta]
name = "baseline_v0"
description = "v0 six-view baseline"

[[shot]]
frame = 90
name = "globe_default"
pos = [0.0, 0.0, 3.0]
quat = [0.0, 0.0, 0.0, 1.0]
fov_y = 60.0

[[shot]]
frame = 120
name = "pole_north"
pos = [0.0, 0.0, 3.0]
quat = [0.7071, 0.0, 0.0, 0.7071]
"#;

    #[test]
    fn parses_sample_script() {
        let s = ShotScript::from_toml_str(SAMPLE).unwrap();
        assert_eq!(s.meta.as_ref().and_then(|m| m.name.clone()).as_deref(), Some("baseline_v0"));
        assert_eq!(s.shot.len(), 2);
        assert_eq!(s.shot[0].name, "globe_default");
        assert_eq!(s.shot[0].frame, 90);
        assert_eq!(s.shot[0].pos, [0.0, 0.0, 3.0]);
        assert_eq!(s.shot[0].quat, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(s.shot[0].fov_y, Some(60.0));
        // fov_y is optional.
        assert_eq!(s.shot[1].fov_y, None);
    }

    #[test]
    fn pose_round_trips_entry() {
        let s = ShotScript::from_toml_str(SAMPLE).unwrap();
        let p = s.shot[0].pose();
        assert_eq!(p.pos, s.shot[0].pos);
        assert_eq!(p.quat, s.shot[0].quat);
        assert_eq!(p.fov_y, s.shot[0].fov_y);
    }

    #[test]
    fn sorted_orders_by_frame() {
        let s = ShotScript::from_toml_str(SAMPLE).unwrap().sorted();
        assert_eq!(s.shot[0].frame, 90);
        assert_eq!(s.shot[1].frame, 120);
    }

    #[test]
    fn rejects_malformed_toml() {
        let err = ShotScript::from_toml_str("[[shot]]\nframe = \"not-a-number\"\n").unwrap_err();
        assert!(err.contains("parse screenshot script"));
    }

    #[test]
    fn empty_script_has_no_shots() {
        let s = ShotScript::from_toml_str("").unwrap();
        assert!(s.shot.is_empty());
        assert!(s.meta.is_none());
    }

    #[test]
    fn next_due_fires_each_entry_once_in_order() {
        let s = ShotScript::from_toml_str(SAMPLE).unwrap().sorted();
        // Before frame 90: nothing due.
        assert_eq!(next_due(&s.shot, 0, 89), None);
        // At frame 90: entry 0 due.
        assert_eq!(next_due(&s.shot, 0, 90), Some(0));
        // After firing 0 (cursor=1), at frame 91 nothing due until 120.
        assert_eq!(next_due(&s.shot, 1, 91), None);
        assert_eq!(next_due(&s.shot, 1, 120), Some(1));
        // Both fired (cursor=2): nothing left.
        assert_eq!(next_due(&s.shot, 2, 999), None);
    }

    #[test]
    fn next_due_fires_skipped_frame_late() {
        let s = ShotScript::from_toml_str(SAMPLE).unwrap().sorted();
        // If frame 90 was missed, at frame 95 entry 0 still fires (>= semantics).
        assert_eq!(next_due(&s.shot, 0, 95), Some(0));
    }

    #[test]
    fn metadata_contains_frame_sha_and_env() {
        let s = ShotScript::from_toml_str(SAMPLE).unwrap();
        let env = serde_json::json!({"strict_offline": true, "offline_imagery_root": "x"});
        let meta = metadata_json(&s.shot[0], 90, "abc1234", &env);
        assert!(meta.contains("\"captured_frame\": 90"));
        assert!(meta.contains("\"git_sha\": \"abc1234\""));
        assert!(meta.contains("\"strict_offline\": true"));
        assert!(meta.contains("globe_default"));
        // Must be valid JSON.
        assert!(serde_json::from_str::<serde_json::Value>(&meta).is_ok());
    }

    // Guards the checked-in v0 baseline against accidental corruption: the
    // real `specs/scripts/baseline_v0.toml` must parse into exactly the six
    // M0.1 views, in ascending frame order, with normalized quaternions.
    #[test]
    fn baseline_v0_toml_is_valid_six_view_script() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../specs/scripts/baseline_v0.toml");
        let s = ShotScript::from_file(&path)
            .unwrap_or_else(|e| panic!("baseline_v0.toml must parse: {e}"));
        assert_eq!(
            s.meta.as_ref().and_then(|m| m.name.clone()).as_deref(),
            Some("baseline_v0")
        );
        let names: Vec<&str> = s.shot.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "globe_default",
                "pole_north",
                "pole_south",
                "zoom_mid",
                "zoom_close",
                "terrain_off"
            ]
        );
        // Frames strictly ascending (deterministic scheduling order).
        assert!(s.shot.windows(2).all(|w| w[0].frame < w[1].frame));
        // Unit quaternions (valid rotation) + 60° FOV matching CAMERA_FOV_Y.
        for e in &s.shot {
            let [x, y, z, w] = e.quat;
            let norm = (x * x + y * y + z * z + w * w).sqrt();
            assert!((norm - 1.0).abs() < 1e-3, "{}: quat not unit ({norm})", e.name);
            assert_eq!(e.fov_y, Some(60.0));
        }
    }
}
