use std::sync::{Arc, Mutex};

use tauri::State;

use crate::capture::{CaptureModeName, CaptureResult, Region};
use crate::error::StructuredError;
use crate::orchestrator::CaptureOrchestrator;

/// Inicia o pipeline de captura no modo especificado.
///
/// Thin wrapper que delega ao `CaptureOrchestrator` via managed state.
/// Sem lógica de negócio neste handler.
#[tauri::command]
pub async fn start_capture(
    orchestrator: State<'_, Arc<Mutex<CaptureOrchestrator>>>,
    mode: CaptureModeName,
) -> Result<(), StructuredError> {
    let orch = Arc::clone(&*orchestrator);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = orch
            .lock()
            .map_err(|_| StructuredError::internal("Orchestrator mutex poisoned"))?;
        guard.start_capture(mode)
    })
    .await
    .map_err(|e| StructuredError::internal(format!("start_capture task panicked: {e}")))?
}

/// Finaliza captura com a região selecionada pelo usuário no overlay.
///
/// Thin wrapper que delega ao `CaptureOrchestrator` via managed state.
/// Válido apenas quando orchestrator está em estado `Selecting`.
#[tauri::command]
pub async fn finalize_capture(
    orchestrator: State<'_, Arc<Mutex<CaptureOrchestrator>>>,
    region: Region,
) -> Result<CaptureResult, StructuredError> {
    let orch = Arc::clone(&*orchestrator);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = orch
            .lock()
            .map_err(|_| StructuredError::internal("Orchestrator mutex poisoned"))?;
        guard.finalize_capture(region)
    })
    .await
    .map_err(|e| StructuredError::internal(format!("finalize_capture task panicked: {e}")))?
}

/// Cancela captura em andamento.
///
/// Thin wrapper que delega ao `CaptureOrchestrator` via managed state.
/// Limpa recursos (temp file, overlay) e emite `capture:cancelled`.
#[tauri::command]
pub async fn cancel_capture(
    orchestrator: State<'_, Arc<Mutex<CaptureOrchestrator>>>,
) -> Result<(), StructuredError> {
    let orch = Arc::clone(&*orchestrator);
    tauri::async_runtime::spawn_blocking(move || {
        let mut guard = orch
            .lock()
            .map_err(|_| StructuredError::internal("Orchestrator mutex poisoned"))?;
        guard.cancel_capture()
    })
    .await
    .map_err(|e| StructuredError::internal(format!("cancel_capture task panicked: {e}")))?
}

#[cfg(test)]
mod tests {
    // Garante que os 3 commands existem e compilam no módulo.
    // A compilação deste módulo confirma que os commands estão implementados corretamente.
    // O registro no invoke_handler é validado em lib.rs via tauri::generate_handler!.
    use super::*;

    #[test]
    fn commands_module_compiles_with_all_three_handlers() {
        // Funções async #[tauri::command] não podem ser coercidas para fn pointers.
        // O fato deste módulo compilar confirma que start_capture, finalize_capture
        // e cancel_capture existem com as assinaturas corretas esperadas pelo Tauri.
        let _ = std::mem::size_of_val(&start_capture);
        let _ = std::mem::size_of_val(&finalize_capture);
        let _ = std::mem::size_of_val(&cancel_capture);
    }
}
