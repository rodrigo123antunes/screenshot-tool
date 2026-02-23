use crate::error::{CaptureError, CaptureErrorKind};
use crate::image_processor::ProcessedCapture;
use image::RgbaImage;
use std::fmt;
use std::path::PathBuf;

// ============================================================
// StorageError
// ============================================================

/// Erros que podem ocorrer durante operações de I/O do `StorageManager`.
#[derive(Debug)]
pub enum StorageError {
    /// Falha na escrita do arquivo (ex: diretório pai não existe, path inválido).
    WriteFailed(String),
    /// Erro de disco ou permissão (ex: sem permissão, disco cheio).
    DiskError(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WriteFailed(msg) => write!(f, "Failed to write file: {}", msg),
            Self::DiskError(msg) => write!(f, "Disk full or insufficient permissions: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}

// ============================================================
// SavedFile
// ============================================================

/// Resultado de uma operação de save: caminho do arquivo salvo e tamanho em bytes.
#[derive(Debug)]
pub struct SavedFile {
    /// Caminho completo do arquivo salvo no disco.
    pub path: PathBuf,
    /// Tamanho do arquivo em bytes (obtido via `std::fs::metadata` após a escrita).
    pub file_size: u64,
}

// ============================================================
// StorageManager
// ============================================================

#[derive(Debug, Default)]
pub struct StorageManager;

impl StorageManager {
    /// Escreve os bytes PNG de `processed` em `processed.target_path` e retorna um
    /// [`SavedFile`] com o caminho final e o tamanho do arquivo.
    ///
    /// É responsabilidade do chamador garantir que o diretório pai de `target_path`
    /// já existe (criado pelo [`crate::image_processor::PlatformDirResolver`]).
    pub fn save(processed: ProcessedCapture) -> Result<SavedFile, StorageError> {
        std::fs::write(&processed.target_path, &processed.png_bytes).map_err(|e| {
            let error = match e.kind() {
                std::io::ErrorKind::PermissionDenied => StorageError::DiskError(format!(
                    "Permission denied writing to {:?}: {}",
                    processed.target_path, e
                )),
                _ => StorageError::WriteFailed(format!(
                    "Failed to write {:?}: {}",
                    processed.target_path, e
                )),
            };
            tracing::error!(
                path = %processed.target_path.display(),
                error = %error,
                "I/O error saving file"
            );
            error
        })?;

        let file_size = std::fs::metadata(&processed.target_path)
            .map_err(|e| {
                let error = StorageError::DiskError(format!(
                    "Failed to read metadata for {:?}: {}",
                    processed.target_path, e
                ));
                tracing::error!(
                    path = %processed.target_path.display(),
                    error = %error,
                    "Failed to read file metadata after write"
                );
                error
            })?
            .len();

        tracing::info!(
            path = %processed.target_path.display(),
            file_size_bytes = file_size,
            "File saved successfully"
        );

        Ok(SavedFile {
            path: processed.target_path,
            file_size,
        })
    }

    /// Salva a imagem temporária em `temp_dir` com nome UUID v4.
    ///
    /// Preservado para o fluxo de freeze frame do overlay no `CaptureOrchestrator`.
    pub fn save_temp(image: &RgbaImage) -> Result<PathBuf, CaptureError> {
        let uuid = uuid::Uuid::new_v4();
        let path = std::env::temp_dir().join(format!("{}.png", uuid));
        image.save(&path).map_err(|e| {
            CaptureError::new(CaptureErrorKind::StorageError)
                .with_context(format!("Failed to save temp file {:?}: {}", path, e))
        })?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_processor::ProcessedCapture;
    use image::RgbaImage;

    // -------------------------
    // Funções auxiliares
    // -------------------------

    /// Gera bytes PNG válidos a partir de uma imagem 2x2 para uso nos testes.
    fn minimal_png_bytes() -> Vec<u8> {
        let image = RgbaImage::new(2, 2);
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .expect("falha ao codificar PNG de teste");
        bytes
    }

    /// Gera um path único em temp_dir para isolar testes.
    fn unique_temp_path(prefix: &str) -> PathBuf {
        let id = uuid::Uuid::new_v4().to_string().replace('-', "");
        std::env::temp_dir().join(format!("storage_test_{}_{}.png", prefix, id))
    }

    /// Cria um `ProcessedCapture` mínimo para os testes de `save()`.
    fn make_processed_capture(target_path: PathBuf) -> ProcessedCapture {
        let image = RgbaImage::new(2, 2);
        ProcessedCapture {
            png_bytes: minimal_png_bytes(),
            rgba_bytes: image.as_raw().to_vec(),
            target_path,
            filename: "test_2x2.png".to_string(),
            width: 2,
            height: 2,
            is_black_warning: false,
        }
    }

    // -------------------------
    // Testes do save()
    // -------------------------

    #[test]
    fn save_writes_file_and_returns_ok_saved_file() {
        let path = unique_temp_path("save_ok");
        let processed = make_processed_capture(path.clone());

        let result = StorageManager::save(processed);

        assert!(result.is_ok(), "save() deve ter sucesso: {:?}", result);
        let saved = result.unwrap();
        assert!(path.exists(), "arquivo deve existir após save()");
        let _ = std::fs::remove_file(&path);
        drop(saved);
    }

    #[test]
    fn save_saved_file_path_equals_target_path() {
        let path = unique_temp_path("save_path");
        let processed = make_processed_capture(path.clone());

        let saved = StorageManager::save(processed).expect("save() deve ter sucesso");

        assert_eq!(
            saved.path, path,
            "SavedFile.path deve ser igual ao target_path do ProcessedCapture"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_file_size_is_nonzero() {
        let path = unique_temp_path("save_fsize");
        let processed = make_processed_capture(path.clone());

        let saved = StorageManager::save(processed).expect("save() deve ter sucesso");

        assert!(
            saved.file_size > 0,
            "SavedFile.file_size deve ser > 0, obtido: {}",
            saved.file_size
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_file_size_matches_png_bytes_length() {
        let path = unique_temp_path("save_fsize_match");
        let png_bytes = minimal_png_bytes();
        let expected_size = png_bytes.len() as u64;
        let processed = ProcessedCapture {
            png_bytes,
            rgba_bytes: RgbaImage::new(2, 2).as_raw().to_vec(),
            target_path: path.clone(),
            filename: "test.png".to_string(),
            width: 2,
            height: 2,
            is_black_warning: false,
        };

        let saved = StorageManager::save(processed).expect("save() deve ter sucesso");

        assert_eq!(
            saved.file_size, expected_size,
            "SavedFile.file_size deve corresponder ao comprimento dos png_bytes gravados"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_file_content_matches_png_bytes() {
        let path = unique_temp_path("save_content");
        let png_bytes = minimal_png_bytes();
        let expected_bytes = png_bytes.clone();
        let processed = ProcessedCapture {
            png_bytes,
            rgba_bytes: RgbaImage::new(2, 2).as_raw().to_vec(),
            target_path: path.clone(),
            filename: "test.png".to_string(),
            width: 2,
            height: 2,
            is_black_warning: false,
        };

        StorageManager::save(processed).expect("save() deve ter sucesso");

        let written = std::fs::read(&path).expect("deve ser possível ler o arquivo gravado");
        assert_eq!(
            written, expected_bytes,
            "conteúdo do arquivo gravado deve ser igual aos png_bytes originais"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_fails_with_write_failed_on_nonexistent_directory() {
        let invalid_path =
            PathBuf::from("/nonexistent_xyz_storage_test_dir_abc/cannot_exist/test.png");
        let processed = make_processed_capture(invalid_path);

        let result = StorageManager::save(processed);

        assert!(
            result.is_err(),
            "save() com diretório pai inexistente deve falhar"
        );
        assert!(
            matches!(result.unwrap_err(), StorageError::WriteFailed(_)),
            "erro deve ser StorageError::WriteFailed para diretório inexistente"
        );
    }

    // -------------------------
    // Testes do StorageError Display
    // -------------------------

    #[test]
    fn storage_error_write_failed_display_contains_message() {
        let msg = "test write error detail";
        let error = StorageError::WriteFailed(msg.to_string());
        let display = format!("{}", error);
        assert!(
            display.contains(msg),
            "Display de WriteFailed deve conter o detalhe do erro: '{}'",
            display
        );
        assert!(
            display.contains("Failed to write file"),
            "Display de WriteFailed deve conter 'Failed to write file': '{}'",
            display
        );
    }

    #[test]
    fn storage_error_disk_error_display_contains_message() {
        let msg = "test disk error detail";
        let error = StorageError::DiskError(msg.to_string());
        let display = format!("{}", error);
        assert!(
            display.contains(msg),
            "Display de DiskError deve conter o detalhe do erro: '{}'",
            display
        );
        assert!(
            display.contains("Disk full or insufficient permissions"),
            "Display de DiskError deve conter 'Disk full or insufficient permissions': '{}'",
            display
        );
    }

    // -------------------------
    // Testes do save_temp (preservados)
    // -------------------------

    fn tiny_image() -> RgbaImage {
        RgbaImage::new(2, 2)
    }

    #[test]
    fn save_temp_returns_path_in_temp_dir_with_png_extension() {
        let image = tiny_image();
        let result = StorageManager::save_temp(&image);
        assert!(result.is_ok(), "save_temp deve ter sucesso: {:?}", result);
        let path = result.unwrap();
        assert!(
            path.starts_with(std::env::temp_dir()),
            "path deve estar em temp_dir: {:?}",
            path
        );
        assert_eq!(
            path.extension().and_then(|e| e.to_str()),
            Some("png"),
            "extensão deve ser .png"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_temp_name_is_valid_uuid_v4() {
        let image = tiny_image();
        let path = StorageManager::save_temp(&image).unwrap();
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let parsed = uuid::Uuid::parse_str(stem);
        assert!(
            parsed.is_ok(),
            "nome do arquivo deve ser UUID válido, recebeu: {}",
            stem
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_temp_creates_valid_readable_png() {
        let mut image = tiny_image();
        for pixel in image.pixels_mut() {
            *pixel = image::Rgba([255, 128, 64, 255]);
        }
        let path = StorageManager::save_temp(&image).unwrap();
        let loaded = image::open(&path).expect("PNG deve ser legível");
        assert_eq!(loaded.width(), 2);
        assert_eq!(loaded.height(), 2);
        let loaded_rgba = loaded.to_rgba8();
        assert_eq!(
            loaded_rgba.get_pixel(0, 0),
            &image::Rgba([255, 128, 64, 255]),
            "dados RGBA devem ser preservados lossless"
        );
        let _ = std::fs::remove_file(&path);
    }
}
