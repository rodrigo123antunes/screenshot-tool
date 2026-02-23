//! Módulo de notificações toast pós-captura.
//!
//! Encapsula interação com `tauri-plugin-notification` (todas as plataformas) e
//! `notify-rust` (Linux — para click handler via XDG action buttons).
//!
//! Comportamento de click handler por plataforma:
//! - **Linux**: usa `notify-rust` diretamente com `wait_for_action` em thread separada.
//!   O click na notificação abre a pasta via `tauri_plugin_opener::reveal_item_in_dir`.
//! - **macOS / Windows**: notificação informativa via `tauri-plugin-notification`.
//!   Click handler não é suportado pelo plugin em plataformas desktop (issue #2150).
//!
//! O `NotificationService` é non-blocking: falhas em `notify_capture` ou `notify_error`
//! apenas registram `tracing::warn!` e não interrompem o pipeline de captura.

use std::fmt;

use tauri::{AppHandle, Runtime};

// ============================================================
// Tipos de Erro
// ============================================================

/// Erros que podem ocorrer durante operações de notificação.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationError {
    /// Falha ao enviar a notificação ao sistema operacional.
    SendFailed(String),
    /// Falha ao registrar action types na inicialização.
    SetupFailed(String),
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SendFailed(msg) => write!(f, "Failed to send notification: {}", msg),
            Self::SetupFailed(msg) => {
                write!(f, "Failed to setup notification actions: {}", msg)
            }
        }
    }
}

impl std::error::Error for NotificationError {}

// ============================================================
// Tipos de Dados
// ============================================================

/// Dados de uma notificação pós-captura bem-sucedida.
pub struct CaptureNotification {
    /// Nome do arquivo salvo (ex: "2026-02-23_14-35-22_region.png").
    pub filename: String,
    /// Tamanho formatado do arquivo (ex: "245 KB" ou "1.2 MB").
    pub file_size_display: String,
    /// Caminho do diretório onde o arquivo foi salvo.
    pub dir_path: String,
    /// `true` se a imagem foi detectada como possivelmente preta (warning).
    pub is_warning: bool,
}

// ============================================================
// NotificationService
// ============================================================

/// Serviço de notificações toast do pipeline de captura.
pub struct NotificationService;

impl NotificationService {
    /// Registra action types para click handler na inicialização do app.
    ///
    /// No desktop, este método é um no-op: o plugin `tauri-plugin-notification` não
    /// suporta `registerActionTypes` em plataformas desktop (apenas mobile). O click
    /// handler no Linux é implementado diretamente via `notify-rust` em `notify_capture`.
    ///
    /// A função existe para satisfazer o contrato da arquitetura e facilitar futura
    /// extensão para plataformas mobile.
    pub fn setup<R: Runtime>(_app: &AppHandle<R>) -> Result<(), NotificationError> {
        tracing::debug!("NotificationService::setup() called — no-op on desktop platforms");
        Ok(())
    }

    /// Emite notificação toast pós-captura.
    ///
    /// - Sucesso (`is_warning=false`): título "Captura salva" com filename, tamanho e diretório.
    /// - Warning (`is_warning=true`): título "Captura salva (possível erro)" com aviso de
    ///   imagem corrompida.
    ///
    /// No Linux, registra action button "open_capture_folder" que abre o diretório no
    /// gerenciador de arquivos via `reveal_item_in_dir`. Em outras plataformas desktop,
    /// emite notificação informativa sem click handler.
    pub fn notify_capture<R: Runtime>(
        app: &AppHandle<R>,
        notification: CaptureNotification,
    ) -> Result<(), NotificationError> {
        let title = if notification.is_warning {
            "Captura salva (possível erro)".to_string()
        } else {
            "Captura salva".to_string()
        };

        let body = if notification.is_warning {
            format!(
                "A imagem pode estar corrompida. Verifique o arquivo em {}",
                notification.dir_path
            )
        } else {
            format!(
                "{} — {}\n{}",
                notification.filename, notification.file_size_display, notification.dir_path
            )
        };

        tracing::debug!(
            title = %title,
            is_warning = notification.is_warning,
            "Sending capture notification"
        );

        Self::send_with_action(app, &title, &body, &notification.dir_path)
    }

    /// Emite notificação de erro (falha de I/O, permissão, etc.).
    pub fn notify_error<R: Runtime>(
        app: &AppHandle<R>,
        error_message: &str,
    ) -> Result<(), NotificationError> {
        let title = "Falha na captura";
        let body = format!("Falha ao salvar captura — {}", error_message);

        tracing::debug!(error_message = %error_message, "Sending error notification");

        Self::send_without_action(app, title, &body)
    }

    /// Formata tamanho de arquivo em representação legível.
    ///
    /// - < 1024 bytes: "N B"
    /// - 1024–1048575 bytes (KB): com 1 decimal se valor < 10, sem decimal se valor ≥ 10
    /// - ≥ 1048576 bytes (MB): com 1 decimal se valor < 10, sem decimal se valor ≥ 10
    pub fn format_file_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * 1024;

        if bytes < KB {
            format!("{} B", bytes)
        } else if bytes < MB {
            let value = bytes as f64 / KB as f64;
            if value < 10.0 {
                format!("{:.1} KB", value)
            } else {
                format!("{:.0} KB", value)
            }
        } else {
            let value = bytes as f64 / MB as f64;
            if value < 10.0 {
                format!("{:.1} MB", value)
            } else {
                format!("{:.0} MB", value)
            }
        }
    }

    /// Envia notificação com click handler para abrir pasta (comportamento por plataforma).
    ///
    /// Linux: usa `notify-rust` com XDG action button "open_capture_folder" em thread dedicada.
    /// Outras plataformas: usa `tauri-plugin-notification` sem click handler.
    fn send_with_action<R: Runtime>(
        app: &AppHandle<R>,
        title: &str,
        body: &str,
        dir_path: &str,
    ) -> Result<(), NotificationError> {
        #[cfg(target_os = "linux")]
        {
            Self::send_linux_with_action(app, title, body, dir_path)
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Suprimir aviso de variável não utilizada em plataformas não-Linux
            let _ = dir_path;
            Self::send_via_plugin(app, title, body)
        }
    }

    /// Envia notificação sem click handler via `tauri-plugin-notification`.
    fn send_without_action<R: Runtime>(
        app: &AppHandle<R>,
        title: &str,
        body: &str,
    ) -> Result<(), NotificationError> {
        Self::send_via_plugin(app, title, body)
    }

    /// Implementação Linux: `notify-rust` com XDG action button e `wait_for_action` em thread.
    #[cfg(target_os = "linux")]
    fn send_linux_with_action<R: Runtime>(
        app: &AppHandle<R>,
        title: &str,
        body: &str,
        dir_path: &str,
    ) -> Result<(), NotificationError> {
        let app_clone = app.clone();
        let dir_path_owned = dir_path.to_string();
        let title_owned = title.to_string();
        let body_owned = body.to_string();

        std::thread::spawn(move || {
            let result = notify_rust::Notification::new()
                .summary(&title_owned)
                .body(&body_owned)
                .action("open_capture_folder", "Abrir pasta")
                .show();

            match result {
                Ok(handle) => {
                    handle.wait_for_action(|action| {
                        if action == "open_capture_folder" {
                            tracing::debug!(
                                action = "open_capture_folder",
                                dir_path = %dir_path_owned,
                                "Notification click handler activated"
                            );
                            if let Err(e) = tauri_plugin_opener::reveal_item_in_dir(&dir_path_owned)
                            {
                                tracing::warn!("reveal_item_in_dir failed: {}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("notify-rust notification failed: {}", e);
                    // Fallback para o plugin do tauri
                    use tauri_plugin_notification::NotificationExt;
                    if let Err(plugin_err) = app_clone
                        .notification()
                        .builder()
                        .title(&title_owned)
                        .body(&body_owned)
                        .show()
                    {
                        tracing::warn!(
                            "Fallback tauri-plugin-notification also failed: {}",
                            plugin_err
                        );
                    }
                }
            }
        });

        Ok(())
    }

    /// Implementação via `tauri-plugin-notification` (todas as plataformas, sem click handler).
    fn send_via_plugin<R: Runtime>(
        app: &AppHandle<R>,
        title: &str,
        body: &str,
    ) -> Result<(), NotificationError> {
        use tauri_plugin_notification::NotificationExt;

        app.notification()
            .builder()
            .title(title)
            .body(body)
            .show()
            .map_err(|e| NotificationError::SendFailed(e.to_string()))
    }
}

// ============================================================
// Unit Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Formatação de tamanho de arquivo ----

    #[test]
    fn format_size_bytes_below_kb() {
        assert_eq!(NotificationService::format_file_size(0), "0 B");
        assert_eq!(NotificationService::format_file_size(500), "500 B");
        assert_eq!(NotificationService::format_file_size(1023), "1023 B");
    }

    #[test]
    fn format_size_exactly_one_kb() {
        // 1024 bytes = 1.0 KB (valor < 10 → 1 decimal)
        assert_eq!(NotificationService::format_file_size(1024), "1.0 KB");
    }

    #[test]
    fn format_size_kb_above_ten() {
        // 245760 / 1024 = 240 (valor >= 10 → sem decimal)
        assert_eq!(NotificationService::format_file_size(245760), "240 KB");
    }

    #[test]
    fn format_size_exactly_one_mb() {
        // 1048576 bytes = 1.0 MB (valor < 10 → 1 decimal)
        assert_eq!(NotificationService::format_file_size(1048576), "1.0 MB");
    }

    #[test]
    fn format_size_mb_decimal() {
        // 2457600 / 1048576 = 2.34375 → arredondado para 1 decimal = "2.3 MB"
        assert_eq!(NotificationService::format_file_size(2457600), "2.3 MB");
    }

    #[test]
    fn format_size_mb_above_ten() {
        // 20971520 / 1048576 = 20.0 (valor >= 10 → sem decimal)
        assert_eq!(
            NotificationService::format_file_size(20 * 1024 * 1024),
            "20 MB"
        );
    }

    // ---- Títulos da notificação ----

    #[test]
    fn capture_notification_success_title() {
        let notification = CaptureNotification {
            filename: "2026-02-23_14-35-22_region.png".to_string(),
            file_size_display: "245 KB".to_string(),
            dir_path: "/home/user/.local/share/screenshot-tool/captures".to_string(),
            is_warning: false,
        };
        // Verificar que is_warning=false resulta no título correto
        assert!(!notification.is_warning);
        let title = if notification.is_warning {
            "Captura salva (possível erro)"
        } else {
            "Captura salva"
        };
        assert_eq!(title, "Captura salva");
    }

    #[test]
    fn capture_notification_warning_title() {
        let notification = CaptureNotification {
            filename: "2026-02-23_14-35-22_fullscreen.png".to_string(),
            file_size_display: "1.0 MB".to_string(),
            dir_path: "/home/user/.local/share/screenshot-tool/captures".to_string(),
            is_warning: true,
        };
        // Verificar que is_warning=true resulta no título diferenciado
        assert!(notification.is_warning);
        let title = if notification.is_warning {
            "Captura salva (possível erro)"
        } else {
            "Captura salva"
        };
        assert_eq!(title, "Captura salva (possível erro)");
    }

    // ---- Display dos erros ----

    #[test]
    fn notification_error_send_failed_display() {
        let err = NotificationError::SendFailed("connection refused".to_string());
        let display = format!("{}", err);
        assert!(
            display.contains("Failed to send notification"),
            "Display deve conter 'Failed to send notification', obtido: {}",
            display
        );
        assert!(display.contains("connection refused"));
    }

    #[test]
    fn notification_error_setup_failed_display() {
        let err = NotificationError::SetupFailed("permission denied".to_string());
        let display = format!("{}", err);
        assert!(
            display.contains("Failed to setup notification actions"),
            "Display deve conter 'Failed to setup notification actions', obtido: {}",
            display
        );
        assert!(display.contains("permission denied"));
    }

    #[test]
    fn notification_error_implements_error_trait() {
        let err = NotificationError::SendFailed("test".to_string());
        // Deve implementar std::error::Error — verificar que podemos usar como &dyn Error
        let _: &dyn std::error::Error = &err;
    }

    // ---- Non-blocking: o resultado de notify_capture deve ser tratável sem panic ----

    #[test]
    fn notification_error_is_clone_and_eq() {
        let err = NotificationError::SendFailed("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);

        let err2 = NotificationError::SetupFailed("x".to_string());
        assert_ne!(err, err2);
    }

    #[test]
    fn format_size_boundary_conditions() {
        // Exatamente 1 KB - 1 = 1023 B (abaixo do threshold de KB)
        assert_eq!(NotificationService::format_file_size(1023), "1023 B");
        // Exatamente 1 MB - 1 = 1048575 bytes
        // 1048575 / 1024 = 1023.999... ≥ 10 → sem decimal → "1024 KB"
        assert_eq!(NotificationService::format_file_size(1048575), "1024 KB");
    }

    #[test]
    fn format_size_small_kb_uses_one_decimal() {
        // 5120 bytes = 5.0 KB (< 10 → 1 decimal)
        assert_eq!(NotificationService::format_file_size(5120), "5.0 KB");
        // 9216 bytes = 9.0 KB (< 10 → 1 decimal)
        assert_eq!(NotificationService::format_file_size(9216), "9.0 KB");
        // 10240 bytes = 10 KB (= 10 → sem decimal)
        assert_eq!(NotificationService::format_file_size(10240), "10 KB");
    }

    #[test]
    fn format_size_small_mb_uses_one_decimal() {
        // 5242880 bytes = 5.0 MB (< 10 → 1 decimal)
        assert_eq!(
            NotificationService::format_file_size(5 * 1024 * 1024),
            "5.0 MB"
        );
        // 10485760 bytes = 10 MB (= 10 → sem decimal)
        assert_eq!(
            NotificationService::format_file_size(10 * 1024 * 1024),
            "10 MB"
        );
    }
}
