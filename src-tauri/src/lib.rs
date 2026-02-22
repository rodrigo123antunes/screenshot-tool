pub mod capture;
pub mod clipboard;
pub mod commands;
pub mod error;
pub mod orchestrator;
pub mod storage;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::capture::CaptureModeName;
use crate::orchestrator::CaptureOrchestrator;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // 1. Registrar CaptureOrchestrator como managed state.
            let orchestrator: Arc<Mutex<CaptureOrchestrator>> =
                Arc::new(Mutex::new(CaptureOrchestrator::new(app.handle().clone())));
            app.manage(orchestrator.clone());

            // 2. Registrar global shortcut via plugin com handler.
            // O plugin é registrado aqui (não no Builder) para ter acesso ao managed state.
            #[cfg(desktop)]
            {
                use tauri_plugin_global_shortcut::{
                    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
                };

                #[cfg(target_os = "macos")]
                let capture_shortcut =
                    Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyS);
                #[cfg(not(target_os = "macos"))]
                let capture_shortcut =
                    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyS);

                let shortcut_copy = capture_shortcut.clone();
                let orch_for_handler = orchestrator.clone();

                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |_app, shortcut, event| {
                            if shortcut == &shortcut_copy {
                                if let ShortcutState::Pressed = event.state() {
                                    let orch = orch_for_handler.clone();
                                    // spawn_blocking para executar código síncrono do orchestrator
                                    // sem bloquear o thread do shortcut handler.
                                    tauri::async_runtime::spawn_blocking(move || {
                                        match orch.lock() {
                                            Ok(mut guard) => {
                                                if let Err(e) =
                                                    guard.start_capture(CaptureModeName::Area)
                                                {
                                                    tracing::error!(
                                                        "start_capture via shortcut failed: {:?}",
                                                        e
                                                    );
                                                }
                                            }
                                            Err(_) => {
                                                tracing::error!(
                                                    "Orchestrator mutex poisoned on shortcut trigger"
                                                );
                                            }
                                        }
                                    });
                                }
                            }
                        })
                        .build(),
                )?;

                app.global_shortcut().register(capture_shortcut)?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_capture,
            commands::finalize_capture,
            commands::cancel_capture,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
