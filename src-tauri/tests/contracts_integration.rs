use screenshot_tool_lib::capture::{
    CaptureModeName, CapturePipelineState, CaptureResult, FreezeReadyPayload, MonitorInfo, Region,
};
use screenshot_tool_lib::error::{CaptureError, CaptureErrorKind, StructuredError};

#[test]
fn should_preserve_structured_error_contract_through_json() {
    let original = StructuredError::new("STORAGE_ERROR", "Falha ao salvar arquivo")
        .with_context("path=/tmp/screenshot.png");

    let json = serde_json::to_string(&original).expect("must serialize");
    let restored: StructuredError = serde_json::from_str(&json).expect("must deserialize");

    assert_eq!(restored.code, "STORAGE_ERROR");
    assert_eq!(restored.message, "Falha ao salvar arquivo");
    assert_eq!(
        restored.context.as_deref(),
        Some("path=/tmp/screenshot.png")
    );
}

#[test]
fn should_map_capture_error_kind_consistently_for_ipc() {
    let error = CaptureError::new(CaptureErrorKind::InvalidState).with_context("state=Idle");
    let structured = StructuredError::from(error);

    assert_eq!(structured.code, "INVALID_STATE");
    assert!(structured.message.contains("Operacao invalida"));
    assert_eq!(structured.context.as_deref(), Some("state=Idle"));
}

#[test]
fn should_validate_models_for_valid_and_invalid_cases() {
    let valid_region = Region {
        x: 12,
        y: 8,
        width: 640,
        height: 480,
    };
    assert!(valid_region.validate().is_ok());

    let invalid_region = Region {
        width: 0,
        ..valid_region
    };
    let region_err = invalid_region.validate().expect_err("must fail");
    assert_eq!(region_err.code, "INVALID_MODEL");

    let valid_monitor = MonitorInfo {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
        scale_factor: 1.0,
    };
    assert!(valid_monitor.validate().is_ok());

    let invalid_monitor = MonitorInfo {
        scale_factor: -1.0,
        ..valid_monitor
    };
    let monitor_err = invalid_monitor.validate().expect_err("must fail");
    assert_eq!(monitor_err.code, "INVALID_MODEL");
}

#[test]
fn should_keep_stable_field_names_for_shared_payloads() {
    let payload = FreezeReadyPayload {
        temp_path: "/tmp/freeze.png".to_string(),
        monitor: MonitorInfo {
            x: 0,
            y: 0,
            width: 2560,
            height: 1440,
            scale_factor: 2.0,
        },
    };

    let serialized = serde_json::to_value(&payload).expect("must serialize");
    assert!(serialized.get("temp_path").is_some());
    assert!(serialized["monitor"].get("scale_factor").is_some());
    assert!(serialized["monitor"].get("scaleFactor").is_none());
}

#[test]
fn should_serialize_clipboard_error_to_clipboard_error_code() {
    let error = CaptureError::new(CaptureErrorKind::ClipboardError);
    let structured = StructuredError::from(error);

    assert_eq!(
        structured.code, "CLIPBOARD_ERROR",
        "ClipboardError deve serializar para 'CLIPBOARD_ERROR'"
    );
    assert!(!structured.message.is_empty(), "message não deve ser vazia");
    assert!(
        structured.context.is_none(),
        "context deve ser None quando não fornecido"
    );
}

#[test]
fn should_serialize_clipboard_error_with_context_via_structured_error() {
    let error = CaptureError::new(CaptureErrorKind::ClipboardError)
        .with_context("arboard set_image failed: display not found");
    let structured = StructuredError::from(error);

    let json = serde_json::to_string(&structured).expect("must serialize");
    let restored: StructuredError = serde_json::from_str(&json).expect("must deserialize");

    assert_eq!(restored.code, "CLIPBOARD_ERROR");
    assert_eq!(
        restored.context.as_deref(),
        Some("arboard set_image failed: display not found")
    );
}

#[test]
fn should_serialize_enums_using_stable_wire_format() {
    let mode = serde_json::to_string(&CaptureModeName::Area).expect("must serialize");
    assert_eq!(mode, "\"area\"");

    let state = serde_json::to_string(&CapturePipelineState::FreezeReady).expect("must serialize");
    assert_eq!(state, "\"freeze_ready\"");

    let result = CaptureResult {
        file_path: "/tmp/final.png".to_string(),
        clipboard_success: false,
    };
    let value = serde_json::to_value(&result).expect("must serialize");
    assert!(value.get("file_path").is_some());
    assert!(value.get("clipboard_success").is_some());
}
