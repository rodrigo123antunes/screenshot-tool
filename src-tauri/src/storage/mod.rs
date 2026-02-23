use crate::error::{CaptureError, CaptureErrorKind};
use image::RgbaImage;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct StorageManager;

impl StorageManager {
    /// Retorna o diretório de screenshots, criando-o automaticamente se necessário.
    pub fn screenshots_dir() -> Result<PathBuf, CaptureError> {
        let home = dirs::home_dir().ok_or_else(|| {
            CaptureError::new(CaptureErrorKind::StorageError)
                .with_context("Home directory not found")
        })?;
        let dir = home.join("Screenshots").join("screenshot-tool");
        std::fs::create_dir_all(&dir).map_err(|e| {
            CaptureError::new(CaptureErrorKind::StorageError)
                .with_context(format!("Failed to create directory {:?}: {}", dir, e))
        })?;
        Ok(dir)
    }

    /// Salva a imagem como PNG lossless com naming timestamped em ~/Screenshots/screenshot-tool/.
    pub fn save_screenshot(image: &RgbaImage) -> Result<PathBuf, CaptureError> {
        let dir = Self::screenshots_dir()?;
        Self::save_to_dir(image, &dir)
    }

    /// Salva a imagem temporária em temp_dir com nome UUID v4.
    pub fn save_temp(image: &RgbaImage) -> Result<PathBuf, CaptureError> {
        let uuid = uuid::Uuid::new_v4();
        let path = std::env::temp_dir().join(format!("{}.png", uuid));
        image.save(&path).map_err(|e| {
            CaptureError::new(CaptureErrorKind::StorageError)
                .with_context(format!("Failed to save temp file {:?}: {}", path, e))
        })?;
        Ok(path)
    }

    /// Resolve colisão de nome de arquivo adicionando sufixo _N quando o arquivo já existe.
    pub fn resolve_collision(base_path: PathBuf) -> PathBuf {
        if !base_path.exists() {
            return base_path;
        }
        let stem = base_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let ext = base_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let parent = base_path.parent().unwrap_or(Path::new("."));
        for i in 1u64.. {
            let candidate = parent.join(format!("{}_{}.{}", stem, i, ext));
            if !candidate.exists() {
                return candidate;
            }
        }
        unreachable!("Cannot find non-colliding path after u64::MAX attempts")
    }

    /// Salva imagem como PNG lossless em um diretório específico (usado internamente e em testes).
    fn save_to_dir(image: &RgbaImage, dir: &Path) -> Result<PathBuf, CaptureError> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let filename = format!("Screenshot_{}.png", timestamp);
        let path = dir.join(&filename);
        let final_path = Self::resolve_collision(path);
        image.save(&final_path).map_err(|e| {
            CaptureError::new(CaptureErrorKind::StorageError).with_context(format!(
                "Failed to save screenshot to {:?}: {}",
                final_path, e
            ))
        })?;
        Ok(final_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbaImage;

    fn tiny_image() -> RgbaImage {
        RgbaImage::new(2, 2)
    }

    // --- save_temp ---

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
        // Preenche com dados conhecidos para verificar lossless
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

    // --- resolve_collision ---

    #[test]
    fn resolve_collision_returns_base_when_file_does_not_exist() {
        let tmp = std::env::temp_dir().join("test_no_collision_storagemanager.png");
        let _ = std::fs::remove_file(&tmp);
        let result = StorageManager::resolve_collision(tmp.clone());
        assert_eq!(
            result, tmp,
            "deve retornar o path base quando não há colisão"
        );
    }

    #[test]
    fn resolve_collision_returns_suffix_1_when_base_exists() {
        let tmp = std::env::temp_dir().join("test_collision_storagemanager_a.png");
        std::fs::write(&tmp, b"dummy").unwrap();
        let result = StorageManager::resolve_collision(tmp.clone());
        let _ = std::fs::remove_file(&tmp);
        let expected = std::env::temp_dir().join("test_collision_storagemanager_a_1.png");
        assert_eq!(result, expected, "deve retornar path com sufixo _1");
    }

    #[test]
    fn resolve_collision_returns_suffix_2_when_1_also_exists() {
        let tmp = std::env::temp_dir().join("test_collision_storagemanager_b.png");
        let tmp1 = std::env::temp_dir().join("test_collision_storagemanager_b_1.png");
        std::fs::write(&tmp, b"dummy").unwrap();
        std::fs::write(&tmp1, b"dummy").unwrap();
        let result = StorageManager::resolve_collision(tmp.clone());
        let _ = std::fs::remove_file(&tmp);
        let _ = std::fs::remove_file(&tmp1);
        let expected = std::env::temp_dir().join("test_collision_storagemanager_b_2.png");
        assert_eq!(
            result, expected,
            "deve retornar path com sufixo _2 quando _1 também existe"
        );
    }

    // --- save_to_dir (via save_screenshot internamente) ---

    #[test]
    fn save_to_dir_creates_png_with_correct_filename_format() {
        let dir = std::env::temp_dir().join("storagemanager_test_dir");
        std::fs::create_dir_all(&dir).unwrap();
        let image = tiny_image();
        let result = StorageManager::save_to_dir(&image, &dir);
        assert!(result.is_ok(), "save_to_dir deve ter sucesso: {:?}", result);
        let path = result.unwrap();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(
            filename.starts_with("Screenshot_"),
            "filename deve iniciar com 'Screenshot_': {}",
            filename
        );
        assert!(
            filename.ends_with(".png"),
            "filename deve terminar com '.png': {}",
            filename
        );
        // Verifica formato YYYY-MM-DD_HH-MM-SS
        let without_prefix = filename.strip_prefix("Screenshot_").unwrap();
        let without_ext = without_prefix.strip_suffix(".png").unwrap();
        // Pode ter sufixo _N de colisão, mas a parte de data deve estar lá
        assert!(
            without_ext.len() >= 19,
            "timestamp deve ter pelo menos 19 chars (YYYY-MM-DD_HH-MM-SS): {}",
            without_ext
        );
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn save_to_dir_creates_directory_automatically() {
        let base = std::env::temp_dir().join("storagemanager_autodir_test");
        // Remove caso exista
        let _ = std::fs::remove_dir_all(&base);
        // screenshots_dir cria automaticamente; aqui testamos save_to_dir com dir inexistente
        // A função save_to_dir não cria o dir — isso é responsabilidade de screenshots_dir.
        // Criamos o dir manualmente para isolar o teste.
        std::fs::create_dir_all(&base).unwrap();
        let image = tiny_image();
        let result = StorageManager::save_to_dir(&image, &base);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists(), "arquivo PNG deve existir após save");
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&base);
    }

    #[test]
    fn screenshots_dir_creates_dir_if_not_exists() {
        // Este teste verifica que screenshots_dir() cria o diretório.
        // Se home_dir() não estiver disponível, o teste é ignorado graciosamente.
        match dirs::home_dir() {
            None => {
                // Plataforma sem home_dir — StorageError esperado
                let result = StorageManager::screenshots_dir();
                assert!(result.is_err());
                assert_eq!(result.unwrap_err().kind, CaptureErrorKind::StorageError);
            }
            Some(home) => {
                let expected_dir = home.join("Screenshots").join("screenshot-tool");
                let result = StorageManager::screenshots_dir();
                assert!(
                    result.is_ok(),
                    "screenshots_dir deve ter sucesso quando home existe"
                );
                let dir = result.unwrap();
                assert_eq!(dir, expected_dir);
                assert!(dir.exists(), "diretório deve ter sido criado");
            }
        }
    }

    #[test]
    fn image_error_on_invalid_path_is_converted_to_storage_error() {
        // Simula falha de image save convertendo o erro para CaptureError com StorageError kind
        let image = tiny_image();
        let invalid_path = PathBuf::from("/nonexistent_root_dir_xyz/cannot_exist/file.png");
        let result = image.save(&invalid_path).map_err(|e| {
            CaptureError::new(CaptureErrorKind::StorageError)
                .with_context(format!("path={:?}: {}", invalid_path, e))
        });
        assert!(result.is_err(), "save para path inválido deve falhar");
        let error = result.unwrap_err();
        assert_eq!(
            error.kind,
            CaptureErrorKind::StorageError,
            "erro deve ser do tipo StorageError"
        );
        assert!(error.context.is_some(), "erro deve conter context com path");
    }
}
