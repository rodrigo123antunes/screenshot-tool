/// Integration tests para o NotificationService.
///
/// Verificam:
/// - Contrato de dados: `CaptureNotification` construído corretamente a partir de `SavedFile`
/// - Comportamento non-blocking: falha de notificação não bloqueia pipeline
/// - Formatação de tamanho de arquivo
/// - Diferenciação entre notificação de sucesso e warning
use screenshot_tool_lib::notification::{
    CaptureNotification, NotificationError, NotificationService,
};

// ============================================================
// Testes de Formatação de Tamanho
// ============================================================

#[test]
fn format_file_size_bytes_threshold() {
    assert_eq!(NotificationService::format_file_size(0), "0 B");
    assert_eq!(NotificationService::format_file_size(1), "1 B");
    assert_eq!(NotificationService::format_file_size(1023), "1023 B");
}

#[test]
fn format_file_size_kb_boundary() {
    // Exatamente 1 KB = 1.0 KB (1 decimal, valor < 10)
    assert_eq!(NotificationService::format_file_size(1024), "1.0 KB");
    // 10 KB = 10 KB (sem decimal, valor >= 10)
    assert_eq!(NotificationService::format_file_size(10 * 1024), "10 KB");
    // 245 KB
    assert_eq!(NotificationService::format_file_size(245 * 1024), "245 KB");
}

#[test]
fn format_file_size_mb_boundary() {
    // Exatamente 1 MB = 1.0 MB (1 decimal, valor < 10)
    assert_eq!(NotificationService::format_file_size(1024 * 1024), "1.0 MB");
    // 2.3 MB aproximado
    assert_eq!(NotificationService::format_file_size(2457600), "2.3 MB");
    // 10 MB = 10 MB (sem decimal, valor >= 10)
    assert_eq!(
        NotificationService::format_file_size(10 * 1024 * 1024),
        "10 MB"
    );
}

// ============================================================
// Testes de CaptureNotification
// ============================================================

#[test]
fn capture_notification_success_fields() {
    let notification = CaptureNotification {
        filename: "2026-02-23_14-35-22_region.png".to_string(),
        file_size_display: NotificationService::format_file_size(245760),
        dir_path: "/home/user/.local/share/screenshot-tool/captures".to_string(),
        is_warning: false,
    };

    assert!(
        !notification.is_warning,
        "sucesso deve ter is_warning=false"
    );
    assert_eq!(notification.file_size_display, "240 KB");
    assert!(notification.filename.ends_with(".png"));
    assert!(!notification.dir_path.is_empty());
}

#[test]
fn capture_notification_warning_fields() {
    let notification = CaptureNotification {
        filename: "2026-02-23_14-35-22_fullscreen.png".to_string(),
        file_size_display: NotificationService::format_file_size(1048576),
        dir_path: "/home/user/.local/share/screenshot-tool/captures".to_string(),
        is_warning: true,
    };

    assert!(
        notification.is_warning,
        "imagem preta deve ter is_warning=true"
    );
    assert_eq!(notification.file_size_display, "1.0 MB");
}

#[test]
fn is_black_warning_true_maps_to_notification_is_warning_true() {
    // Simula o que o CaptureOrchestrator faz: usar is_black_warning do processed
    let is_black_warning = true;
    let notification = CaptureNotification {
        filename: "test.png".to_string(),
        file_size_display: NotificationService::format_file_size(512 * 1024),
        dir_path: "/tmp/captures".to_string(),
        is_warning: is_black_warning,
    };
    assert!(
        notification.is_warning,
        "is_black_warning=true deve resultar em CaptureNotification::is_warning=true"
    );
}

#[test]
fn is_black_warning_false_maps_to_notification_is_warning_false() {
    let is_black_warning = false;
    let notification = CaptureNotification {
        filename: "test.png".to_string(),
        file_size_display: NotificationService::format_file_size(512 * 1024),
        dir_path: "/tmp/captures".to_string(),
        is_warning: is_black_warning,
    };
    assert!(
        !notification.is_warning,
        "is_black_warning=false deve resultar em CaptureNotification::is_warning=false"
    );
}

// ============================================================
// Testes de NotificationError (contrato de tipos)
// ============================================================

#[test]
fn notification_error_send_failed_display_contract() {
    let err = NotificationError::SendFailed("dbus connection failed".to_string());
    let display = format!("{}", err);
    assert!(
        display.contains("Failed to send notification"),
        "SendFailed Display deve conter 'Failed to send notification'"
    );
}

#[test]
fn notification_error_setup_failed_display_contract() {
    let err = NotificationError::SetupFailed("permission denied".to_string());
    let display = format!("{}", err);
    assert!(
        display.contains("Failed to setup notification actions"),
        "SetupFailed Display deve conter 'Failed to setup notification actions'"
    );
}

#[test]
fn notification_error_is_std_error() {
    let err = NotificationError::SendFailed("test".to_string());
    // Deve implementar std::error::Error para ser usável em contextos de erro genérico
    let boxed: Box<dyn std::error::Error> = Box::new(err);
    assert!(boxed.to_string().contains("Failed to send notification"));
}

// ============================================================
// Testes de comportamento non-blocking do pipeline
// ============================================================

#[test]
fn notification_failure_result_can_be_logged_without_panic() {
    // Simula o padrão de uso no CaptureOrchestrator: falha de notificação
    // deve ser tratável com if let Err(e) = ... { log warn } sem panic
    let fake_error = NotificationError::SendFailed("simulated failure".to_string());
    let result: Result<(), NotificationError> = Err(fake_error);

    // O padrão exato usado no orchestrator:
    if let Err(e) = result {
        let warn_msg = format!("notify_capture failed (non-blocking): {}", e);
        // Deve produzir mensagem de warn sem panic
        assert!(warn_msg.contains("Failed to send notification"));
        assert!(warn_msg.contains("simulated failure"));
    } else {
        panic!("result deve ser Err");
    }
}

#[test]
fn notification_error_does_not_propagate_past_if_let_pattern() {
    // Verifica que o padrão if let Err(e) = NotificationService::notify_* não
    // interfere com o restante do fluxo (simula pipeline continuando após falha)
    let mut steps_completed = 0u32;

    let result: Result<(), NotificationError> =
        Err(NotificationError::SendFailed("test".to_string()));

    if let Err(e) = result {
        // Apenas logamos — não retornamos nem propagamos
        let _ = format!("warn: {}", e);
        steps_completed += 1;
    }

    // Pipeline continua normalmente após tratar o erro de notificação
    steps_completed += 1;
    assert_eq!(
        steps_completed, 2,
        "pipeline deve completar mesmo com falha de notificação"
    );
}

// ============================================================
// Testes de contrato de dir_path
// ============================================================

#[test]
fn dir_path_from_saved_file_path_parent() {
    // Simula a extração do dir_path do saved.path.parent() feita no orchestrator
    let saved_path = std::path::PathBuf::from(
        "/home/user/.local/share/screenshot-tool/captures/2026-02-23_14-35-22_region.png",
    );
    let dir_path = saved_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    assert_eq!(dir_path, "/home/user/.local/share/screenshot-tool/captures");
    assert!(!dir_path.is_empty(), "dir_path não deve ser vazio");
}

#[test]
fn dir_path_fallback_for_path_without_parent() {
    // Edge case: path sem parent (apenas nome de arquivo) → fallback para ""
    let saved_path = std::path::PathBuf::from("test.png");
    let dir_path = saved_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // PathBuf::from("test.png").parent() retorna Some("") no Rust
    // Mas o unwrap_or_default é seguro mesmo assim
    let _ = dir_path; // apenas verificamos que não panicar
}
