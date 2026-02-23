mod models;

pub use models::{
    CaptureModeName, CapturePipelineState, CaptureResult, FreezeReadyPayload, MonitorInfo, Region,
};

use image::RgbaImage;
use xcap::Monitor;

use crate::error::{CaptureError, CaptureErrorKind};

impl From<xcap::XCapError> for CaptureError {
    fn from(e: xcap::XCapError) -> Self {
        CaptureError::new(CaptureErrorKind::CaptureFailure).with_context(e.to_string())
    }
}

/// Abstração de modo de captura de tela.
///
/// Cada modo determina como a imagem raw é obtida e se um overlay de seleção
/// é necessário. O caller (CaptureOrchestrator) é responsável por envolver
/// `capture()` em `tokio::task::spawn_blocking`, pois `capture_image()` é blocking.
pub trait CaptureMode: Send + Sync {
    /// Executa a captura e retorna a imagem raw RGBA.
    fn capture(&self, monitor: &Monitor) -> Result<RgbaImage, CaptureError>;

    /// Indica se este modo requer overlay para seleção de região.
    fn requires_overlay(&self) -> bool;
}

/// Captura a tela cheia do monitor ativo e salva diretamente, sem overlay.
pub struct FullscreenCapture;

impl CaptureMode for FullscreenCapture {
    fn capture(&self, monitor: &Monitor) -> Result<RgbaImage, CaptureError> {
        monitor.capture_image().map_err(CaptureError::from)
    }

    fn requires_overlay(&self) -> bool {
        false
    }
}

/// Captura a tela cheia como freeze frame para que o usuário selecione
/// uma região via overlay. O crop acontece em `finalize_capture`.
pub struct AreaCapture;

impl CaptureMode for AreaCapture {
    fn capture(&self, monitor: &Monitor) -> Result<RgbaImage, CaptureError> {
        monitor.capture_image().map_err(CaptureError::from)
    }

    fn requires_overlay(&self) -> bool {
        true
    }
}

/// Captura a tela cheia como freeze frame para o overlay apresentar a lista
/// de janelas ao usuário. A seleção e captura individual da janela acontece
/// via `xcap::Window::all()` no CaptureOrchestrator após a seleção do usuário.
pub struct WindowCapture;

impl CaptureMode for WindowCapture {
    fn capture(&self, monitor: &Monitor) -> Result<RgbaImage, CaptureError> {
        monitor.capture_image().map_err(CaptureError::from)
    }

    fn requires_overlay(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CaptureErrorKind;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn capture_mode_impls_are_send_sync() {
        assert_send_sync::<FullscreenCapture>();
        assert_send_sync::<AreaCapture>();
        assert_send_sync::<WindowCapture>();
    }

    #[test]
    fn fullscreen_capture_requires_no_overlay() {
        let capture = FullscreenCapture;
        assert!(!capture.requires_overlay());
    }

    #[test]
    fn area_capture_requires_overlay() {
        let capture = AreaCapture;
        assert!(capture.requires_overlay());
    }

    #[test]
    fn window_capture_requires_overlay() {
        let capture = WindowCapture;
        assert!(capture.requires_overlay());
    }

    #[test]
    fn capture_failure_error_maps_to_correct_kind() {
        let xcap_error = xcap::XCapError::Error("test capture error".to_string());
        let capture_error = CaptureError::from(xcap_error);

        assert_eq!(capture_error.kind, CaptureErrorKind::CaptureFailure);
        assert!(capture_error.context.is_some());
        assert!(!capture_error.context.unwrap().is_empty());
    }

    #[test]
    #[ignore = "requires display server"]
    fn xcap_capture_image_integration() {
        let monitors = xcap::Monitor::all().expect("must list monitors");
        assert!(!monitors.is_empty(), "must have at least one monitor");

        let monitor = &monitors[0];
        let capture = FullscreenCapture;
        let image = capture.capture(monitor).expect("must capture image");

        assert!(image.width() > 0);
        assert!(image.height() > 0);
    }
}
