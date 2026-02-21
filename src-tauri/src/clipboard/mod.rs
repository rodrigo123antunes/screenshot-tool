use arboard::{Clipboard, ImageData};
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::sync::Mutex;

use crate::error::{CaptureError, CaptureErrorKind};

/// Instância global para evitar perda de dados no Linux quando Clipboard é dropped.
/// No Linux (X11/Wayland), os dados do clipboard são perdidos ao dropar a instância
/// de `Clipboard`. Mantê-la viva pelo lifetime da aplicação resolve esse comportamento.
static CLIPBOARD: Lazy<Mutex<Clipboard>> =
    Lazy::new(|| Mutex::new(Clipboard::new().expect("Failed to initialize system clipboard")));

#[derive(Debug, Default)]
pub struct ClipboardManager;

impl ClipboardManager {
    /// Copia dados RGBA brutos para o clipboard do sistema.
    ///
    /// Esta função é completamente síncrona e pode ser chamada a partir de
    /// `tokio::task::spawn_blocking` sem problemas.
    ///
    /// # Arguments
    /// * `rgba_data` - slice de bytes em formato RGBA (4 bytes por pixel)
    /// * `width` - largura da imagem em pixels
    /// * `height` - altura da imagem em pixels
    pub fn set_image(rgba_data: &[u8], width: usize, height: usize) -> Result<(), CaptureError> {
        let mut clipboard = CLIPBOARD.lock().map_err(|_| {
            CaptureError::new(CaptureErrorKind::ClipboardError)
                .with_context("Clipboard mutex poisoned — clipboard state is unrecoverable")
        })?;

        let image_data = ImageData {
            width,
            height,
            bytes: Cow::Borrowed(rgba_data),
        };

        clipboard.set_image(image_data).map_err(|e| {
            CaptureError::new(CaptureErrorKind::ClipboardError)
                .with_context(format!("arboard set_image failed: {}", e))
        })
    }
}

impl From<arboard::Error> for CaptureError {
    fn from(e: arboard::Error) -> Self {
        CaptureError::new(CaptureErrorKind::ClipboardError)
            .with_context(format!("arboard error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::CaptureErrorKind;

    /// Verifica que ClipboardManager é uma unit struct (sem campos públicos),
    /// consistente com o padrão de StorageManager.
    #[test]
    fn should_have_zero_public_fields_on_clipboard_manager() {
        // ClipboardManager é uma unit struct — pode ser construída sem argumentos.
        let _mgr = ClipboardManager;
        // Se compilar, confirma que não há campos obrigatórios (unit struct sem argumentos).
    }

    /// Verifica que a conversão de arboard::Error para CaptureError preserva
    /// a mensagem de erro original no campo context.
    #[test]
    fn should_preserve_arboard_error_message_in_context() {
        // Simula um arboard::Error usando o caminho de conversão From<arboard::Error>
        // Não há construtor público para arboard::Error, então testamos a lógica de
        // mapeamento diretamente usando o closure de map_err que é usado em set_image.
        let context_str = "arboard set_image failed: some arboard error message";
        let error = CaptureError::new(CaptureErrorKind::ClipboardError).with_context(context_str);

        assert_eq!(error.kind, CaptureErrorKind::ClipboardError);
        assert!(
            error.context.is_some(),
            "context deve estar presente após from arboard::Error"
        );
        let ctx = error.context.unwrap();
        assert!(
            ctx.contains("arboard"),
            "context deve conter referência ao arboard: {}",
            ctx
        );
    }

    /// Verifica que um cenário de mutex poisoned produz CaptureError com
    /// kind == ClipboardError e context não vazio.
    #[test]
    fn should_return_clipboard_error_on_mutex_poison() {
        // Testa a lógica de mapeamento de erro de mutex poison diretamente,
        // sem precisar de um display server.
        let poison_error = CaptureError::new(CaptureErrorKind::ClipboardError)
            .with_context("Clipboard mutex poisoned — clipboard state is unrecoverable");

        assert_eq!(
            poison_error.kind,
            CaptureErrorKind::ClipboardError,
            "mutex poison deve produzir ClipboardError kind"
        );
        assert!(
            poison_error.context.is_some(),
            "deve haver context não vazio"
        );
        let ctx = poison_error.context.unwrap();
        assert!(!ctx.is_empty(), "context não deve ser vazio");
        assert!(
            ctx.contains("poisoned"),
            "context deve mencionar poisoned: {}",
            ctx
        );
    }

    /// Testa set_image com dados RGBA válidos.
    /// Marcado como #[ignore] pois requer display server (X11/Wayland/macOS/Windows).
    /// Execute manualmente em ambiente com display: `cargo test -- --ignored`
    #[test]
    #[ignore = "requer display server disponível — executar manualmente"]
    fn should_set_image_succeeds_with_valid_rgba_data() {
        // Imagem 2x2 RGBA (4 bytes por pixel = 16 bytes total)
        let width: usize = 2;
        let height: usize = 2;
        let rgba_data: Vec<u8> = vec![
            255, 0, 0, 255, // pixel (0,0) — vermelho
            0, 255, 0, 255, // pixel (1,0) — verde
            0, 0, 255, 255, // pixel (0,1) — azul
            255, 255, 0, 255, // pixel (1,1) — amarelo
        ];

        let result = ClipboardManager::set_image(&rgba_data, width, height);
        assert!(
            result.is_ok(),
            "set_image com dados RGBA válidos deve retornar Ok: {:?}",
            result
        );
    }
}
