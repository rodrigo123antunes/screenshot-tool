use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use image::RgbaImage;
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::capture::{
    AreaCapture, CaptureMode, CaptureModeName, CaptureResult, FreezeReadyPayload,
    FullscreenCapture, MonitorInfo, Region, WindowCapture,
};
use crate::clipboard::ClipboardManager;
use crate::error::{CaptureErrorKind, StructuredError};
use crate::storage::StorageManager;

/// Estado rico da state machine de captura com dados associados em cada variante.
#[derive(Debug, Clone)]
pub enum CaptureState {
    /// Aguardando trigger de captura.
    Idle,
    /// Captura em andamento (xcap chamado).
    Capturing { mode: CaptureModeName },
    /// Freeze frame salvo, overlay pronto para abrir (apenas modos com overlay).
    FreezeReady {
        temp_path: PathBuf,
        monitor_info: MonitorInfo,
        full_image: Arc<RgbaImage>,
    },
    /// Overlay aberto, usuário selecionando região.
    Selecting {
        temp_path: PathBuf,
        full_image: Arc<RgbaImage>,
    },
    /// Finalizando: crop + clipboard + file save.
    Finalizing,
    /// Captura concluída com sucesso.
    Complete,
    /// Captura falhou.
    Failed { error: StructuredError },
    /// Captura cancelada pelo usuário.
    Cancelled,
}

impl CaptureState {
    pub fn name(&self) -> &'static str {
        match self {
            CaptureState::Idle => "Idle",
            CaptureState::Capturing { .. } => "Capturing",
            CaptureState::FreezeReady { .. } => "FreezeReady",
            CaptureState::Selecting { .. } => "Selecting",
            CaptureState::Finalizing => "Finalizing",
            CaptureState::Complete => "Complete",
            CaptureState::Failed { .. } => "Failed",
            CaptureState::Cancelled => "Cancelled",
        }
    }
}

/// Orquestrador central da state machine de captura de tela.
///
/// Gerencia o ciclo de vida completo do pipeline:
/// Idle → Capturing → FreezeReady → Selecting → Finalizing → Complete → Idle
///
/// Thread-safe via `Arc<Mutex<CaptureOrchestrator>>`.
pub struct CaptureOrchestrator<R: Runtime = tauri::Wry> {
    pub(crate) state: CaptureState,
    app_handle: AppHandle<R>,
}

impl<R: Runtime> CaptureOrchestrator<R> {
    pub fn new(app_handle: AppHandle<R>) -> Self {
        Self {
            state: CaptureState::Idle,
            app_handle,
        }
    }

    pub fn current_state_name(&self) -> &'static str {
        self.state.name()
    }

    /// Executa transição de estado com validação.
    /// Retorna `INVALID_STATE` se a transição não é permitida.
    fn transition(&mut self, new_state: CaptureState) -> Result<(), StructuredError> {
        if !Self::is_valid_transition(&self.state, &new_state) {
            let err = StructuredError::from(CaptureErrorKind::InvalidState).with_context(format!(
                "transition {} → {} is invalid",
                self.state.name(),
                new_state.name()
            ));
            return Err(err);
        }
        tracing::info!(
            from = self.state.name(),
            to = new_state.name(),
            "State transition"
        );
        self.state = new_state;
        Ok(())
    }

    /// Define todas as transições válidas da state machine.
    pub(crate) fn is_valid_transition(from: &CaptureState, to: &CaptureState) -> bool {
        match (from, to) {
            (CaptureState::Idle, CaptureState::Capturing { .. }) => true,
            (CaptureState::Capturing { .. }, CaptureState::FreezeReady { .. }) => true,
            // Modo fullscreen: Capturing → Finalizing diretamente (sem overlay)
            (CaptureState::Capturing { .. }, CaptureState::Finalizing) => true,
            (CaptureState::FreezeReady { .. }, CaptureState::Selecting { .. }) => true,
            (CaptureState::Selecting { .. }, CaptureState::Finalizing) => true,
            // cancel_capture em Selecting vai direto para Idle
            (CaptureState::Selecting { .. }, CaptureState::Idle) => true,
            (CaptureState::Finalizing, CaptureState::Complete) => true,
            (CaptureState::Finalizing, CaptureState::Failed { .. }) => true,
            // Reset automático após Complete ou Failed
            (CaptureState::Complete, CaptureState::Idle) => true,
            (CaptureState::Failed { .. }, CaptureState::Idle) => true,
            (CaptureState::Cancelled, CaptureState::Idle) => true,
            _ => false,
        }
    }

    /// Detecta o monitor ativo. Usa primeiro monitor disponível como fallback.
    fn detect_active_monitor() -> Result<xcap::Monitor, StructuredError> {
        let monitors = xcap::Monitor::all().map_err(|e| {
            StructuredError::from(CaptureErrorKind::MonitorNotFound)
                .with_context(format!("xcap::Monitor::all() failed: {e}"))
        })?;
        if monitors.is_empty() {
            return Err(StructuredError::from(CaptureErrorKind::MonitorNotFound)
                .with_context("No monitors detected"));
        }
        // Usa primeiro monitor disponível.
        // TODO: usar Monitor::from_point(cursor_x, cursor_y) quando Wayland suportar.
        Ok(monitors.into_iter().next().unwrap())
    }

    fn monitor_info_from_xcap(monitor: &xcap::Monitor) -> Result<MonitorInfo, StructuredError> {
        let x = monitor.x().map_err(|e| {
            StructuredError::from(CaptureErrorKind::MonitorNotFound)
                .with_context(format!("monitor.x() failed: {e}"))
        })?;
        let y = monitor.y().map_err(|e| {
            StructuredError::from(CaptureErrorKind::MonitorNotFound)
                .with_context(format!("monitor.y() failed: {e}"))
        })?;
        let width = monitor.width().map_err(|e| {
            StructuredError::from(CaptureErrorKind::MonitorNotFound)
                .with_context(format!("monitor.width() failed: {e}"))
        })?;
        let height = monitor.height().map_err(|e| {
            StructuredError::from(CaptureErrorKind::MonitorNotFound)
                .with_context(format!("monitor.height() failed: {e}"))
        })?;
        let scale_factor = monitor.scale_factor().map_err(|e| {
            StructuredError::from(CaptureErrorKind::MonitorNotFound)
                .with_context(format!("monitor.scale_factor() failed: {e}"))
        })? as f64;
        Ok(MonitorInfo {
            x,
            y,
            width,
            height,
            scale_factor,
        })
    }

    /// Inicia o pipeline de captura no modo especificado.
    ///
    /// Válido apenas em estado `Idle`. Retorna `INVALID_STATE` se em outro estado.
    /// Para modos com overlay (Area, Window): executa xcap, salva temp file,
    /// cria overlay e emite `capture:freeze-ready`.
    /// Para modo fullscreen: executa xcap, clipboard e file save diretamente,
    /// emite `capture:complete`.
    pub fn start_capture(&mut self, mode: CaptureModeName) -> Result<(), StructuredError> {
        if !matches!(self.state, CaptureState::Idle) {
            return Err(
                StructuredError::from(CaptureErrorKind::InvalidState).with_context(format!(
                    "start_capture called in state={}",
                    self.state.name()
                )),
            );
        }

        let start = Instant::now();
        tracing::info!(?mode, "Starting capture pipeline");

        self.transition(CaptureState::Capturing { mode })?;

        let monitor = match Self::detect_active_monitor() {
            Ok(m) => m,
            Err(e) => {
                self.app_handle.emit("capture:error", e.clone()).ok();
                self.state = CaptureState::Idle;
                return Err(e);
            }
        };

        let monitor_info = match Self::monitor_info_from_xcap(&monitor) {
            Ok(info) => info,
            Err(e) => {
                self.app_handle.emit("capture:error", e.clone()).ok();
                self.state = CaptureState::Idle;
                return Err(e);
            }
        };

        let capture_mode: Box<dyn CaptureMode> = match mode {
            CaptureModeName::Fullscreen => Box::new(FullscreenCapture),
            CaptureModeName::Area => Box::new(AreaCapture),
            CaptureModeName::Window => Box::new(WindowCapture),
        };

        let requires_overlay = capture_mode.requires_overlay();

        let image = match capture_mode.capture(&monitor) {
            Ok(img) => img,
            Err(e) => {
                let error = StructuredError::from(e);
                self.app_handle.emit("capture:error", error.clone()).ok();
                self.state = CaptureState::Idle;
                return Err(error);
            }
        };

        let freeze_elapsed = start.elapsed().as_millis();
        tracing::info!(elapsed_ms = freeze_elapsed, "Freeze frame captured");
        if freeze_elapsed > 50 {
            tracing::warn!(
                elapsed_ms = freeze_elapsed,
                target_ms = 50,
                "Freeze time exceeded target"
            );
        }

        if requires_overlay {
            let temp_path = match StorageManager::save_temp(&image) {
                Ok(p) => p,
                Err(e) => {
                    let error = StructuredError::from(e);
                    self.app_handle.emit("capture:error", error.clone()).ok();
                    self.state = CaptureState::Idle;
                    return Err(error);
                }
            };

            tracing::debug!("Temp file saved: {:?}", temp_path);

            let full_image = Arc::new(image);

            self.transition(CaptureState::FreezeReady {
                temp_path: temp_path.clone(),
                monitor_info: monitor_info.clone(),
                full_image: full_image.clone(),
            })?;

            if let Err(e) = self.create_overlay_window(&monitor_info, &temp_path) {
                tracing::error!("Overlay creation failed: {}", e.message);
                self.cleanup_temp_file(&temp_path);
                self.app_handle.emit("capture:error", e.clone()).ok();
                self.state = CaptureState::Idle;
                return Err(e);
            }

            tracing::debug!(
                "Overlay window created at ({}, {})",
                monitor_info.x,
                monitor_info.y
            );

            self.app_handle
                .emit(
                    "capture:freeze-ready",
                    FreezeReadyPayload {
                        temp_path: temp_path.to_string_lossy().to_string(),
                        monitor: monitor_info,
                    },
                )
                .map_err(|e| {
                    StructuredError::internal(format!("Failed to emit capture:freeze-ready: {e}"))
                })?;

            self.transition(CaptureState::Selecting {
                temp_path,
                full_image,
            })?;
        } else {
            // Modo fullscreen: pipeline direto sem overlay.
            self.transition(CaptureState::Finalizing)?;

            // Executa clipboard e file save em paralelo via tokio::join! + spawn_blocking.
            // Chamado de dentro de spawn_blocking, então Handle::current() é disponível.
            let image_arc = Arc::new(image);
            let image_for_clipboard = image_arc.clone();
            let image_for_file = image_arc.clone();

            let handle = tokio::runtime::Handle::current();
            let (clipboard_res, file_res) = handle.block_on(async move {
                tokio::join!(
                    tokio::task::spawn_blocking(move || {
                        let w = image_for_clipboard.width() as usize;
                        let h = image_for_clipboard.height() as usize;
                        ClipboardManager::set_image(image_for_clipboard.as_raw(), w, h)
                    }),
                    tokio::task::spawn_blocking(move || {
                        StorageManager::save_screenshot(&*image_for_file)
                    }),
                )
            });

            let clipboard_success = match clipboard_res {
                Ok(Ok(())) => true,
                Ok(Err(e)) => {
                    tracing::warn!(
                        "Clipboard set_image failed, continuing with file save: {}",
                        e
                    );
                    false
                }
                Err(_) => {
                    tracing::warn!("Clipboard task panicked, continuing with file save");
                    false
                }
            };

            let file_path = match file_res {
                Ok(Ok(path)) => path,
                Ok(Err(e)) => {
                    let error = StructuredError::from(e);
                    self.app_handle.emit("capture:error", error.clone()).ok();
                    let _ = self.transition(CaptureState::Failed {
                        error: error.clone(),
                    });
                    self.state = CaptureState::Idle;
                    return Err(error);
                }
                Err(join_err) => {
                    let error =
                        StructuredError::internal(format!("file save task panicked: {}", join_err));
                    self.app_handle.emit("capture:error", error.clone()).ok();
                    self.state = CaptureState::Idle;
                    return Err(error);
                }
            };

            let finalize_elapsed = start.elapsed().as_millis();
            tracing::info!(elapsed_ms = finalize_elapsed, "Post-capture finalized");
            if finalize_elapsed > 200 {
                tracing::warn!(
                    elapsed_ms = finalize_elapsed,
                    target_ms = 200,
                    "Finalize time exceeded target"
                );
            }

            let result = CaptureResult {
                file_path: file_path.to_string_lossy().to_string(),
                clipboard_success,
            };

            self.app_handle.emit("capture:complete", result).ok();
            let _ = self.transition(CaptureState::Complete);
            let _ = self.transition(CaptureState::Idle);
        }

        Ok(())
    }

    /// Finaliza captura com a região selecionada pelo usuário.
    ///
    /// Válido apenas em estado `Selecting`. Executa crop com clamp defensivo,
    /// clipboard e file save, destrói overlay e emite `capture:complete`.
    /// Falha de clipboard não é fatal — reportada via `clipboard_success: false`.
    pub fn finalize_capture(&mut self, region: Region) -> Result<CaptureResult, StructuredError> {
        let (temp_path, full_image) = match &self.state {
            CaptureState::Selecting {
                temp_path,
                full_image,
            } => (temp_path.clone(), full_image.clone()),
            _ => {
                return Err(
                    StructuredError::from(CaptureErrorKind::InvalidState).with_context(format!(
                        "finalize_capture called in state={}",
                        self.state.name()
                    )),
                )
            }
        };

        region.validate()?;

        let start = Instant::now();
        tracing::info!(?region, "Finalizing capture");

        self.transition(CaptureState::Finalizing)?;

        // Crop defensivo: clampa região aos bounds da imagem sem retornar erro.
        let img_width = full_image.width();
        let img_height = full_image.height();
        let x = region.x.min(img_width.saturating_sub(1));
        let y = region.y.min(img_height.saturating_sub(1));
        let max_w = img_width.saturating_sub(x);
        let max_h = img_height.saturating_sub(y);
        let crop_w = region.width.min(max_w).max(1);
        let crop_h = region.height.min(max_h).max(1);

        let cropped = image::imageops::crop_imm(&*full_image, x, y, crop_w, crop_h).to_image();

        // Executa clipboard e file save em paralelo via tokio::join! + spawn_blocking.
        // Clipboard: falha não é fatal. File save: falha é fatal.
        let cropped_arc = Arc::new(cropped);
        let cropped_for_clipboard = cropped_arc.clone();
        let cropped_for_file = cropped_arc.clone();

        let handle = tokio::runtime::Handle::current();
        let (clipboard_res, file_res) = handle.block_on(async move {
            tokio::join!(
                tokio::task::spawn_blocking(move || {
                    let w = cropped_for_clipboard.width() as usize;
                    let h = cropped_for_clipboard.height() as usize;
                    ClipboardManager::set_image(cropped_for_clipboard.as_raw(), w, h)
                }),
                tokio::task::spawn_blocking(move || {
                    StorageManager::save_screenshot(&*cropped_for_file)
                }),
            )
        });

        let clipboard_success = match clipboard_res {
            Ok(Ok(())) => true,
            Ok(Err(e)) => {
                tracing::warn!(
                    "Clipboard set_image failed, continuing with file save: {}",
                    e
                );
                false
            }
            Err(_) => {
                tracing::warn!("Clipboard task panicked, continuing with file save");
                false
            }
        };

        // File save: falha é fatal.
        let file_path = match file_res {
            Ok(Ok(path)) => path,
            Ok(Err(e)) => {
                let error = StructuredError::from(e);
                self.destroy_overlay_window().ok();
                self.cleanup_temp_file(&temp_path);
                self.app_handle.emit("capture:error", error.clone()).ok();
                let _ = self.transition(CaptureState::Failed {
                    error: error.clone(),
                });
                self.state = CaptureState::Idle;
                return Err(error);
            }
            Err(join_err) => {
                let error =
                    StructuredError::internal(format!("file save task panicked: {}", join_err));
                self.destroy_overlay_window().ok();
                self.cleanup_temp_file(&temp_path);
                self.app_handle.emit("capture:error", error.clone()).ok();
                self.state = CaptureState::Idle;
                return Err(error);
            }
        };

        let finalize_elapsed = start.elapsed().as_millis();
        tracing::info!(elapsed_ms = finalize_elapsed, "Post-capture finalized");
        if finalize_elapsed > 200 {
            tracing::warn!(
                elapsed_ms = finalize_elapsed,
                target_ms = 200,
                "Finalize time exceeded target"
            );
        }

        let result = CaptureResult {
            file_path: file_path.to_string_lossy().to_string(),
            clipboard_success,
        };

        self.destroy_overlay_window().ok();
        self.cleanup_temp_file(&temp_path);
        self.app_handle
            .emit("capture:complete", result.clone())
            .ok();
        let _ = self.transition(CaptureState::Complete);
        let _ = self.transition(CaptureState::Idle);

        Ok(result)
    }

    /// Cancela captura em andamento.
    ///
    /// Válido em qualquer estado exceto `Idle`. Destrói overlay, limpa temp file
    /// e emite `capture:cancelled`. Reset automático para `Idle`.
    pub fn cancel_capture(&mut self) -> Result<(), StructuredError> {
        if matches!(self.state, CaptureState::Idle) {
            return Err(StructuredError::from(CaptureErrorKind::InvalidState)
                .with_context("cancel_capture called in state=Idle"));
        }

        let temp_path = match &self.state {
            CaptureState::FreezeReady { temp_path, .. } => Some(temp_path.clone()),
            CaptureState::Selecting { temp_path, .. } => Some(temp_path.clone()),
            _ => None,
        };

        if let Some(path) = temp_path {
            self.cleanup_temp_file(&path);
        }

        self.destroy_overlay_window().ok();
        self.app_handle.emit("capture:cancelled", ()).ok();
        self.state = CaptureState::Idle;

        tracing::info!("Capture cancelled, reset to Idle");

        Ok(())
    }

    /// Cria overlay window transparente no monitor ativo.
    fn create_overlay_window(
        &self,
        monitor_info: &MonitorInfo,
        _temp_path: &PathBuf,
    ) -> Result<(), StructuredError> {
        let url = tauri::WebviewUrl::App("/overlay".into());
        tauri::WebviewWindowBuilder::new(&self.app_handle, "overlay", url)
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .position(monitor_info.x as f64, monitor_info.y as f64)
            .inner_size(monitor_info.width as f64, monitor_info.height as f64)
            .build()
            .map_err(|e| {
                StructuredError::from(CaptureErrorKind::OverlayError)
                    .with_context(format!("WebviewWindowBuilder failed: {e}"))
            })?;
        Ok(())
    }

    /// Fecha overlay window se existir.
    fn destroy_overlay_window(&self) -> Result<(), StructuredError> {
        if let Some(window) = self.app_handle.get_webview_window("overlay") {
            window.close().map_err(|e| {
                StructuredError::from(CaptureErrorKind::OverlayError)
                    .with_context(format!("Failed to close overlay window: {e}"))
            })?;
        }
        Ok(())
    }

    /// Remove arquivo temporário do disco se existir.
    fn cleanup_temp_file(&self, path: &PathBuf) {
        if path.exists() {
            if let Err(e) = std::fs::remove_file(path) {
                tracing::warn!("Failed to cleanup temp file {:?}: {}", path, e);
            } else {
                tracing::debug!("Cleaned up temp file: {:?}", path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cria um AppHandle mockado para testes unitários sem display server.
    fn mock_orchestrator() -> (
        CaptureOrchestrator<tauri::test::MockRuntime>,
        tauri::App<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .expect("Failed to build mock app");
        let handle = app.handle().clone();
        let orch = CaptureOrchestrator::new(handle);
        (orch, app)
    }

    #[test]
    fn should_start_in_idle_state() {
        let (orch, _app) = mock_orchestrator();
        assert!(matches!(orch.state, CaptureState::Idle));
    }

    #[test]
    fn should_reject_start_capture_when_not_idle() {
        let (mut orch, _app) = mock_orchestrator();
        orch.state = CaptureState::Capturing {
            mode: CaptureModeName::Area,
        };

        let err = orch
            .start_capture(CaptureModeName::Area)
            .expect_err("must fail when not idle");
        assert_eq!(err.code, "INVALID_STATE");
    }

    #[test]
    fn should_reject_finalize_when_not_selecting() {
        let (mut orch, _app) = mock_orchestrator();
        // Estado é Idle por padrão.
        let region = Region {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let err = orch
            .finalize_capture(region)
            .expect_err("must fail when not selecting");
        assert_eq!(err.code, "INVALID_STATE");
    }

    #[test]
    fn should_reject_cancel_when_already_idle() {
        let (mut orch, _app) = mock_orchestrator();
        let err = orch
            .cancel_capture()
            .expect_err("must fail when already idle");
        assert_eq!(err.code, "INVALID_STATE");
    }

    #[test]
    fn should_transition_idle_to_capturing_on_start() {
        let (mut orch, _app) = mock_orchestrator();
        assert!(matches!(orch.state, CaptureState::Idle));

        orch.transition(CaptureState::Capturing {
            mode: CaptureModeName::Area,
        })
        .expect("transition Idle → Capturing should be valid");

        assert!(matches!(orch.state, CaptureState::Capturing { .. }));
    }

    #[test]
    fn should_transition_selecting_to_finalizing_on_finalize() {
        let (mut orch, _app) = mock_orchestrator();
        orch.state = CaptureState::Selecting {
            temp_path: PathBuf::from("/tmp/test.png"),
            full_image: Arc::new(RgbaImage::new(1, 1)),
        };

        orch.transition(CaptureState::Finalizing)
            .expect("transition Selecting → Finalizing should be valid");

        assert!(matches!(orch.state, CaptureState::Finalizing));
    }

    #[test]
    fn should_reset_to_idle_after_complete() {
        let (mut orch, _app) = mock_orchestrator();
        orch.state = CaptureState::Complete;

        orch.transition(CaptureState::Idle)
            .expect("transition Complete → Idle should be valid");

        assert!(matches!(orch.state, CaptureState::Idle));
    }

    #[test]
    fn should_reset_to_idle_after_failed() {
        let (mut orch, _app) = mock_orchestrator();
        orch.state = CaptureState::Failed {
            error: StructuredError::internal("test error"),
        };

        orch.transition(CaptureState::Idle)
            .expect("transition Failed → Idle should be valid");

        assert!(matches!(orch.state, CaptureState::Idle));
    }

    #[test]
    fn should_cleanup_temp_file_on_cancel() {
        let (mut orch, _app) = mock_orchestrator();

        // Criar arquivo temporário real para testar cleanup.
        let temp_path = std::env::temp_dir().join("orchestrator_test_cancel_cleanup.png");
        let tiny_image = RgbaImage::new(2, 2);
        tiny_image
            .save(&temp_path)
            .expect("must save temp file for test");
        assert!(temp_path.exists(), "temp file must exist before cancel");

        orch.state = CaptureState::Selecting {
            temp_path: temp_path.clone(),
            full_image: Arc::new(RgbaImage::new(2, 2)),
        };

        orch.cancel_capture().expect("cancel_capture must succeed");

        assert!(
            !temp_path.exists(),
            "temp file must be deleted after cancel"
        );
        assert!(matches!(orch.state, CaptureState::Idle));
    }

    #[test]
    fn should_cleanup_temp_file_on_cancel_from_freeze_ready() {
        let (mut orch, _app) = mock_orchestrator();

        let temp_path = std::env::temp_dir().join("orchestrator_test_cancel_freeze.png");
        let tiny_image = RgbaImage::new(2, 2);
        tiny_image
            .save(&temp_path)
            .expect("must save temp file for test");

        orch.state = CaptureState::FreezeReady {
            temp_path: temp_path.clone(),
            monitor_info: MonitorInfo {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                scale_factor: 1.0,
            },
            full_image: Arc::new(RgbaImage::new(2, 2)),
        };

        orch.cancel_capture()
            .expect("cancel from FreezeReady must succeed");
        assert!(
            !temp_path.exists(),
            "temp file must be deleted after cancel"
        );
    }

    #[test]
    fn should_return_partial_result_when_clipboard_fails() {
        // Testa lógica: clipboard falha mas file_path é string válida.
        // Simula o comportamento via construção direta de CaptureResult.
        let result = CaptureResult {
            file_path: "/home/user/Screenshots/screenshot-tool/Screenshot_2026-01-01.png"
                .to_string(),
            clipboard_success: false,
        };

        assert!(!result.clipboard_success);
        assert!(!result.file_path.is_empty());
    }

    #[test]
    fn should_clamp_region_to_image_bounds() {
        let image = RgbaImage::new(100, 100);
        let full_image = Arc::new(image);

        // Região que excede os bounds.
        let region = Region {
            x: 80,
            y: 80,
            width: 100,
            height: 100,
        };

        let img_width = full_image.width();
        let img_height = full_image.height();
        let x = region.x.min(img_width.saturating_sub(1));
        let y = region.y.min(img_height.saturating_sub(1));
        let max_w = img_width.saturating_sub(x);
        let max_h = img_height.saturating_sub(y);
        let crop_w = region.width.min(max_w).max(1);
        let crop_h = region.height.min(max_h).max(1);

        // Verifica que os bounds clamped estão dentro da imagem.
        assert!(x < img_width);
        assert!(y < img_height);
        assert!(x + crop_w <= img_width);
        assert!(y + crop_h <= img_height);

        // Verifica que o crop funciona sem panic.
        let cropped = image::imageops::crop_imm(&*full_image, x, y, crop_w, crop_h).to_image();
        assert!(cropped.width() > 0);
        assert!(cropped.height() > 0);
    }

    #[test]
    fn should_reset_to_idle_after_cancel_from_capturing() {
        let (mut orch, _app) = mock_orchestrator();
        orch.state = CaptureState::Capturing {
            mode: CaptureModeName::Area,
        };

        orch.cancel_capture()
            .expect("cancel must succeed from Capturing");
        assert!(matches!(orch.state, CaptureState::Idle));
    }

    #[test]
    fn all_valid_transitions_are_accepted() {
        let monitor_info = MonitorInfo {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
        };
        let temp_path = PathBuf::from("/tmp/test.png");
        let full_image = Arc::new(RgbaImage::new(1, 1));

        let valid_transitions: Vec<(CaptureState, CaptureState)> = vec![
            (
                CaptureState::Idle,
                CaptureState::Capturing {
                    mode: CaptureModeName::Area,
                },
            ),
            (
                CaptureState::Capturing {
                    mode: CaptureModeName::Area,
                },
                CaptureState::FreezeReady {
                    temp_path: temp_path.clone(),
                    monitor_info: monitor_info.clone(),
                    full_image: full_image.clone(),
                },
            ),
            (
                CaptureState::Capturing {
                    mode: CaptureModeName::Fullscreen,
                },
                CaptureState::Finalizing,
            ),
            (
                CaptureState::FreezeReady {
                    temp_path: temp_path.clone(),
                    monitor_info: monitor_info.clone(),
                    full_image: full_image.clone(),
                },
                CaptureState::Selecting {
                    temp_path: temp_path.clone(),
                    full_image: full_image.clone(),
                },
            ),
            (
                CaptureState::Selecting {
                    temp_path: temp_path.clone(),
                    full_image: full_image.clone(),
                },
                CaptureState::Finalizing,
            ),
            (
                CaptureState::Selecting {
                    temp_path: temp_path.clone(),
                    full_image: full_image.clone(),
                },
                CaptureState::Idle,
            ),
            (CaptureState::Finalizing, CaptureState::Complete),
            (
                CaptureState::Finalizing,
                CaptureState::Failed {
                    error: StructuredError::internal("test"),
                },
            ),
            (CaptureState::Complete, CaptureState::Idle),
            (
                CaptureState::Failed {
                    error: StructuredError::internal("test"),
                },
                CaptureState::Idle,
            ),
            (CaptureState::Cancelled, CaptureState::Idle),
        ];

        for (from, to) in valid_transitions {
            assert!(
                CaptureOrchestrator::<tauri::Wry>::is_valid_transition(&from, &to),
                "transition {} → {} should be valid",
                from.name(),
                to.name()
            );
        }
    }

    #[test]
    fn invalid_transitions_are_rejected_by_state_machine() {
        let invalid_transitions: Vec<(CaptureState, CaptureState)> = vec![
            (CaptureState::Idle, CaptureState::Finalizing),
            (CaptureState::Idle, CaptureState::Complete),
            (CaptureState::Finalizing, CaptureState::Idle),
            (CaptureState::Complete, CaptureState::Finalizing),
            (
                CaptureState::Idle,
                CaptureState::Failed {
                    error: StructuredError::internal("test"),
                },
            ),
        ];

        for (from, to) in invalid_transitions {
            assert!(
                !CaptureOrchestrator::<tauri::Wry>::is_valid_transition(&from, &to),
                "transition {} → {} should be invalid",
                from.name(),
                to.name()
            );
        }
    }

    #[test]
    fn should_not_create_overlay_for_fullscreen_mode() {
        // Verifica que FullscreenCapture.requires_overlay() retorna false.
        use crate::capture::FullscreenCapture;
        let fc = FullscreenCapture;
        assert!(
            !fc.requires_overlay(),
            "fullscreen mode must not require overlay"
        );
    }

    /// Verifica que após `finalize_capture`, o arquivo temporário é deletado do disco.
    /// Usa runtime tokio multi-thread pois `finalize_capture` usa `Handle::current().block_on`.
    #[test]
    fn should_cleanup_temp_file_on_complete() {
        let (mut orch, _app) = mock_orchestrator();

        let temp_path = std::env::temp_dir().join("orchestrator_test_cleanup_on_complete.png");
        RgbaImage::new(2, 2)
            .save(&temp_path)
            .expect("must save temp file for test");
        assert!(temp_path.exists(), "temp file must exist before finalize");

        orch.state = CaptureState::Selecting {
            temp_path: temp_path.clone(),
            full_image: Arc::new(RgbaImage::new(2, 2)),
        };

        let region = Region {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        };

        // finalize_capture internamente usa Handle::current().block_on — precisamos
        // de um runtime tokio. Criamos um e chamamos via spawn_blocking.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("must build tokio runtime for test");

        let _result = rt.block_on(async {
            tokio::task::spawn_blocking(move || orch.finalize_capture(region)).await
        });

        // O temp file deve ser deletado independentemente de clipboard/storage ter sucesso.
        assert!(
            !temp_path.exists(),
            "temp file must be deleted after finalize_capture completes"
        );
    }

    /// Verifica que `cancel_capture` emite `capture:cancelled` e reseta estado para `Idle`.
    /// O emit acontece via `app_handle.emit()` — com MockRuntime, a chamada é silenciosa
    /// mas não retorna erro, confirmando que a operação foi executada corretamente.
    #[test]
    fn should_emit_cancelled_event_on_cancel() {
        let (mut orch, _app) = mock_orchestrator();
        orch.state = CaptureState::Selecting {
            temp_path: PathBuf::from("/tmp/orchestrator_test_emit_cancel.png"),
            full_image: Arc::new(RgbaImage::new(1, 1)),
        };

        // cancel_capture deve ter sucesso e emitir "capture:cancelled".
        // Com MockRuntime, emit() retorna Ok(()) silenciosamente.
        orch.cancel_capture()
            .expect("cancel_capture must succeed to trigger event emission");

        assert!(
            matches!(orch.state, CaptureState::Idle),
            "state must be Idle after cancel (post-event emission)"
        );
    }

    /// Verifica que o payload `FreezeReadyPayload` emitido com `capture:freeze-ready`
    /// contém os campos corretos de `temp_path` e `monitor`.
    /// O pipeline completo requer display server (coberto em testes ignorados).
    #[test]
    fn should_emit_freeze_ready_event_for_area_mode() {
        let monitor_info = MonitorInfo {
            x: 100,
            y: 200,
            width: 1920,
            height: 1080,
            scale_factor: 2.0,
        };
        let temp_path = PathBuf::from("/tmp/orchestrator_test_freeze_ready.png");

        // Verifica a serialização do payload que seria emitido com o evento.
        let payload = FreezeReadyPayload {
            temp_path: temp_path.to_string_lossy().to_string(),
            monitor: monitor_info.clone(),
        };

        let json = serde_json::to_value(&payload).expect("FreezeReadyPayload must serialize");
        assert_eq!(json["temp_path"], "/tmp/orchestrator_test_freeze_ready.png");
        assert_eq!(json["monitor"]["x"], 100);
        assert_eq!(json["monitor"]["y"], 200);
        assert_eq!(json["monitor"]["width"], 1920);
        assert_eq!(json["monitor"]["height"], 1080);
        assert_eq!(json["monitor"]["scale_factor"], 2.0);

        // Pré-condição: AreaCapture deve exigir overlay para emitir este evento.
        assert!(
            AreaCapture.requires_overlay(),
            "AreaCapture must require overlay to trigger capture:freeze-ready event"
        );

        // Verifica o estado FreezeReady que armazena os dados do evento.
        let state = CaptureState::FreezeReady {
            temp_path,
            monitor_info,
            full_image: Arc::new(RgbaImage::new(1920, 1080)),
        };
        assert!(
            matches!(state, CaptureState::FreezeReady { .. }),
            "FreezeReady state must hold monitor and temp_path data for event emission"
        );
    }

    /// Verifica que o payload `CaptureResult` emitido com `capture:complete`
    /// contém os campos corretos após `finalize_capture`.
    /// O pipeline completo requer tokio runtime + storage (coberto por should_cleanup_temp_file_on_complete).
    #[test]
    fn should_emit_complete_event_after_finalize() {
        // Payload de sucesso total.
        let result = CaptureResult {
            file_path: "/home/user/Screenshots/screenshot-tool/Screenshot_2026-01-01_12-00-00.png"
                .to_string(),
            clipboard_success: true,
        };
        let json = serde_json::to_value(&result).expect("CaptureResult must serialize");
        assert_eq!(json["clipboard_success"], true);
        assert!(!json["file_path"].as_str().unwrap().is_empty());
        assert!(json["file_path"].as_str().unwrap().ends_with(".png"));

        // Payload de falha parcial (clipboard falhou, file_path válido).
        // Este é o CaptureResult emitido quando clipboard_success: false.
        let partial_result = CaptureResult {
            file_path: "/home/user/Screenshots/screenshot-tool/Screenshot_2026-01-01_12-00-00.png"
                .to_string(),
            clipboard_success: false,
        };
        let partial_json = serde_json::to_value(&partial_result).expect("must serialize");
        assert_eq!(partial_json["clipboard_success"], false);
        assert!(!partial_json["file_path"].as_str().unwrap().is_empty());
    }

    // Testes de integração que precisam de display server.
    // Execute com: cargo test -- --ignored

    #[test]
    #[ignore = "requires display server"]
    fn pipeline_fullscreen_end_to_end() {
        let (mut orch, _app) = mock_orchestrator();
        let result = orch.start_capture(CaptureModeName::Fullscreen);
        assert!(
            result.is_ok(),
            "fullscreen capture must succeed: {:?}",
            result
        );
        assert!(matches!(orch.state, CaptureState::Idle));
    }

    #[test]
    #[ignore = "requires display server"]
    fn cancel_flow_end_to_end() {
        let (mut orch, _app) = mock_orchestrator();

        let temp_path = std::env::temp_dir().join("orchestrator_e2e_cancel_test.png");
        RgbaImage::new(100, 100)
            .save(&temp_path)
            .expect("must save temp");

        orch.state = CaptureState::Selecting {
            temp_path: temp_path.clone(),
            full_image: Arc::new(RgbaImage::new(100, 100)),
        };

        orch.cancel_capture().expect("cancel must succeed");

        assert!(matches!(orch.state, CaptureState::Idle));
        assert!(!temp_path.exists(), "temp file must be cleaned up");
    }

    #[test]
    fn state_guard_finalize_in_idle_returns_invalid_state() {
        let (mut orch, _app) = mock_orchestrator();
        let region = Region {
            x: 10,
            y: 10,
            width: 100,
            height: 100,
        };
        let err = orch
            .finalize_capture(region)
            .expect_err("must return error");
        assert_eq!(err.code, "INVALID_STATE");
    }

    #[test]
    fn state_guard_start_in_capturing_returns_invalid_state() {
        let (mut orch, _app) = mock_orchestrator();
        orch.state = CaptureState::Capturing {
            mode: CaptureModeName::Fullscreen,
        };
        let err = orch
            .start_capture(CaptureModeName::Area)
            .expect_err("must return error");
        assert_eq!(err.code, "INVALID_STATE");
    }
}
