use serde::{Deserialize, Serialize};

use crate::error::{CaptureErrorKind, StructuredError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CaptureModeName {
    Fullscreen,
    Window,
    #[default]
    Area,
}

impl TryFrom<&str> for CaptureModeName {
    type Error = StructuredError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim().to_lowercase().as_str() {
            "fullscreen" => Ok(Self::Fullscreen),
            "window" => Ok(Self::Window),
            "area" => Ok(Self::Area),
            _ => Err(StructuredError::from(CaptureErrorKind::InvalidCaptureMode)
                .with_context(format!("mode={value}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapturePipelineState {
    Idle,
    Capturing,
    FreezeReady,
    Selecting,
    Finalizing,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    pub fn validate(self) -> Result<Self, StructuredError> {
        if self.width == 0 || self.height == 0 {
            return Err(StructuredError::from(CaptureErrorKind::InvalidModel)
                .with_context("region width and height must be greater than zero"));
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl MonitorInfo {
    pub fn validate(&self) -> Result<(), StructuredError> {
        if self.width == 0 || self.height == 0 {
            return Err(StructuredError::from(CaptureErrorKind::InvalidModel)
                .with_context("monitor width and height must be greater than zero"));
        }
        if !self.scale_factor.is_finite() || self.scale_factor <= 0.0 {
            return Err(StructuredError::from(CaptureErrorKind::InvalidModel)
                .with_context("monitor scale_factor must be a finite positive number"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureResult {
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub is_black_warning: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreezeReadyPayload {
    pub temp_path: String,
    pub monitor: MonitorInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_capture_mode_from_str() {
        assert_eq!(
            CaptureModeName::try_from("fullscreen").expect("valid mode"),
            CaptureModeName::Fullscreen
        );
        assert_eq!(
            CaptureModeName::try_from("window").expect("valid mode"),
            CaptureModeName::Window
        );
        assert_eq!(
            CaptureModeName::try_from("area").expect("valid mode"),
            CaptureModeName::Area
        );
    }

    #[test]
    fn should_fail_for_invalid_capture_mode() {
        let error = CaptureModeName::try_from("circle").expect_err("must fail");
        assert_eq!(error.code, "INVALID_CAPTURE_MODE");
        assert!(error.context.unwrap_or_default().contains("mode=circle"));
    }

    #[test]
    fn should_validate_region_dimensions() {
        let valid = Region {
            x: 10,
            y: 12,
            width: 100,
            height: 80,
        };
        assert_eq!(valid.validate().expect("valid region"), valid);

        let invalid = Region {
            x: 10,
            y: 12,
            width: 0,
            height: 80,
        };
        let error = invalid.validate().expect_err("invalid region");
        assert_eq!(error.code, "INVALID_MODEL");
    }

    #[test]
    fn should_validate_monitor_info() {
        let valid = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
        };
        assert!(valid.validate().is_ok());

        let invalid_size = MonitorInfo {
            width: 0,
            ..valid.clone()
        };
        let err = invalid_size.validate().expect_err("must fail");
        assert_eq!(err.code, "INVALID_MODEL");

        let invalid_scale = MonitorInfo {
            scale_factor: 0.0,
            ..valid
        };
        let err = invalid_scale.validate().expect_err("must fail");
        assert_eq!(err.code, "INVALID_MODEL");
    }

    #[test]
    fn should_serialize_models_with_stable_field_names() {
        let payload = FreezeReadyPayload {
            temp_path: "/tmp/screenshot.png".to_string(),
            monitor: MonitorInfo {
                x: 100,
                y: 200,
                width: 2560,
                height: 1440,
                scale_factor: 1.5,
            },
        };

        let value = serde_json::to_value(payload).expect("must serialize");
        assert!(value.get("temp_path").is_some());
        assert!(value["monitor"].get("scale_factor").is_some());
        assert!(value["monitor"].get("scaleFactor").is_none());
    }

    #[test]
    fn capture_result_serializes_with_new_schema_fields() {
        let result = CaptureResult {
            path: "/home/user/.local/share/screenshot-tool/captures/2026-02-23_14-35-22_region.png"
                .to_string(),
            width: 800,
            height: 600,
            file_size: 245760,
            is_black_warning: false,
        };
        let json = serde_json::to_value(&result).expect("CaptureResult must serialize");
        assert!(json.get("path").is_some(), "path field must exist");
        assert!(json.get("width").is_some(), "width field must exist");
        assert!(json.get("height").is_some(), "height field must exist");
        assert!(
            json.get("file_size").is_some(),
            "file_size field must exist"
        );
        assert!(
            json.get("is_black_warning").is_some(),
            "is_black_warning field must exist"
        );
        assert!(
            json.get("file_path").is_none(),
            "deprecated file_path must NOT exist"
        );
        assert!(
            json.get("clipboard_success").is_none(),
            "deprecated clipboard_success must NOT exist"
        );
    }

    #[test]
    fn capture_result_roundtrips_through_serde_json() {
        let original = CaptureResult {
            path: "/tmp/test_2026-02-23_14-35-22_fullscreen.png".to_string(),
            width: 1920,
            height: 1080,
            file_size: 1024000,
            is_black_warning: true,
        };
        let json = serde_json::to_string(&original).expect("must serialize");
        let restored: CaptureResult = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(original, restored);
    }

    #[test]
    fn capture_result_deprecated_fields_absent() {
        let result = CaptureResult {
            path: "/tmp/screenshot.png".to_string(),
            width: 1280,
            height: 720,
            file_size: 102400,
            is_black_warning: false,
        };
        let json = serde_json::to_value(&result).expect("must serialize");
        assert!(
            json.get("file_path").is_none(),
            "deprecated field 'file_path' must NOT exist in serialized output"
        );
        assert!(
            json.get("clipboard_success").is_none(),
            "deprecated field 'clipboard_success' must NOT exist in serialized output"
        );
    }

    #[test]
    fn capture_result_is_black_warning_true_serializes_correctly() {
        let result = CaptureResult {
            path: "/tmp/possibly_black_2026-02-23_14-35-22_fullscreen.png".to_string(),
            width: 1920,
            height: 1080,
            file_size: 2048,
            is_black_warning: true,
        };
        let json = serde_json::to_string(&result).expect("must serialize");
        let restored: CaptureResult = serde_json::from_str(&json).expect("must deserialize");
        assert!(
            restored.is_black_warning,
            "is_black_warning must be true after roundtrip"
        );
        assert_eq!(restored.path, result.path);
        assert_eq!(restored.width, result.width);
        assert_eq!(restored.height, result.height);
        assert_eq!(restored.file_size, result.file_size);
        let value = serde_json::to_value(&result).expect("must serialize to value");
        assert_eq!(
            value.get("is_black_warning").and_then(|v| v.as_bool()),
            Some(true),
            "is_black_warning must serialize as JSON boolean true"
        );
    }

    #[test]
    fn capture_result_path_uses_new_naming_convention() {
        let path_region = "2026-02-23_14-35-22_region.png";
        let path_fullscreen = "2026-02-23_09-00-01_fullscreen.png";
        let path_window = "2026-02-23_23-59-59_window.png";
        let path_collision = "2026-02-23_14-35-22_region_2.png";

        for path in [path_region, path_fullscreen, path_window, path_collision] {
            let result = CaptureResult {
                path: format!("/home/user/.local/share/screenshot-tool/captures/{path}"),
                width: 800,
                height: 600,
                file_size: 245760,
                is_black_warning: false,
            };
            let json = serde_json::to_string(&result).expect("must serialize");
            let restored: CaptureResult = serde_json::from_str(&json).expect("must deserialize");
            assert!(
                restored.path.contains(path),
                "path '{path}' must be preserved through serde roundtrip"
            );
        }
    }
}
