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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureResult {
    pub file_path: String,
    pub clipboard_success: bool,
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
}
