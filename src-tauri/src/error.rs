use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl StructuredError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            context: None,
        }
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("INTERNAL_ERROR", message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureErrorKind {
    PermissionDenied,
    MonitorNotFound,
    CaptureFailure,
    ImageProcessingError,
    ClipboardError,
    StorageError,
    InvalidState,
    OverlayError,
    Cancelled,
    InvalidModel,
    InvalidCaptureMode,
    InternalError,
}

impl CaptureErrorKind {
    pub const fn code(self) -> &'static str {
        match self {
            Self::PermissionDenied => "PERMISSION_DENIED",
            Self::MonitorNotFound => "MONITOR_NOT_FOUND",
            Self::CaptureFailure => "CAPTURE_FAILURE",
            Self::ImageProcessingError => "IMAGE_PROCESSING_ERROR",
            Self::ClipboardError => "CLIPBOARD_ERROR",
            Self::StorageError => "STORAGE_ERROR",
            Self::InvalidState => "INVALID_STATE",
            Self::OverlayError => "OVERLAY_ERROR",
            Self::Cancelled => "CANCELLED",
            Self::InvalidModel => "INVALID_MODEL",
            Self::InvalidCaptureMode => "INVALID_CAPTURE_MODE",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }

    pub const fn default_message(self) -> &'static str {
        match self {
            Self::PermissionDenied => {
                "Permissao de captura de tela negada pelo sistema operacional."
            }
            Self::MonitorNotFound => "Nenhum monitor foi detectado na posicao ativa do cursor.",
            Self::CaptureFailure => "Falha ao capturar a imagem da tela.",
            Self::ImageProcessingError => "Falha ao processar a imagem capturada.",
            Self::ClipboardError => "Falha ao copiar a imagem para a area de transferencia.",
            Self::StorageError => "Falha ao salvar a imagem no armazenamento local.",
            Self::InvalidState => "Operacao invalida para o estado atual do pipeline de captura.",
            Self::OverlayError => "Falha ao preparar a janela de overlay da captura.",
            Self::Cancelled => "A captura foi cancelada pelo usuario.",
            Self::InvalidModel => "Payload de captura invalido.",
            Self::InvalidCaptureMode => "Modo de captura invalido.",
            Self::InternalError => "Erro interno nao esperado no backend de captura.",
        }
    }
}

impl From<CaptureErrorKind> for StructuredError {
    fn from(kind: CaptureErrorKind) -> Self {
        Self::new(kind.code(), kind.default_message())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureError {
    pub kind: CaptureErrorKind,
    pub message: String,
    pub context: Option<String>,
}

impl CaptureError {
    pub fn new(kind: CaptureErrorKind) -> Self {
        Self {
            kind,
            message: kind.default_message().to_string(),
            context: None,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

impl Display for CaptureError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(context) = &self.context {
            write!(f, "{} ({})", self.message, context)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for CaptureError {}

impl From<CaptureError> for StructuredError {
    fn from(value: CaptureError) -> Self {
        StructuredError {
            code: value.kind.code().to_string(),
            message: value.message,
            context: value.context,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_roundtrip_structured_error_with_context() {
        let error = StructuredError::new("INVALID_STATE", "Estado invalido")
            .with_context("state=Selecting");

        let json = serde_json::to_string(&error).expect("must serialize");
        let restored: StructuredError = serde_json::from_str(&json).expect("must deserialize");

        assert_eq!(error, restored);
    }

    #[test]
    fn should_omit_context_when_none() {
        let error = StructuredError::new("CAPTURE_FAILURE", "Falha ao capturar");
        let value = serde_json::to_value(&error).expect("must serialize");

        assert_eq!(value["code"], "CAPTURE_FAILURE");
        assert_eq!(value["message"], "Falha ao capturar");
        assert!(value.get("context").is_none());
    }

    #[test]
    fn should_map_error_kind_to_expected_codes() {
        let cases = [
            (CaptureErrorKind::PermissionDenied, "PERMISSION_DENIED"),
            (CaptureErrorKind::MonitorNotFound, "MONITOR_NOT_FOUND"),
            (CaptureErrorKind::CaptureFailure, "CAPTURE_FAILURE"),
            (
                CaptureErrorKind::ImageProcessingError,
                "IMAGE_PROCESSING_ERROR",
            ),
            (CaptureErrorKind::ClipboardError, "CLIPBOARD_ERROR"),
            (CaptureErrorKind::StorageError, "STORAGE_ERROR"),
            (CaptureErrorKind::InvalidState, "INVALID_STATE"),
            (CaptureErrorKind::OverlayError, "OVERLAY_ERROR"),
            (CaptureErrorKind::Cancelled, "CANCELLED"),
            (CaptureErrorKind::InvalidModel, "INVALID_MODEL"),
            (CaptureErrorKind::InvalidCaptureMode, "INVALID_CAPTURE_MODE"),
            (CaptureErrorKind::InternalError, "INTERNAL_ERROR"),
        ];

        for (kind, expected_code) in cases {
            let structured = StructuredError::from(kind);
            assert_eq!(structured.code, expected_code);
            assert!(!structured.message.is_empty());
            assert_eq!(structured.context, None);
        }
    }
}
