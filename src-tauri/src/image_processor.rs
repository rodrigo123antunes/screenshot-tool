//! Módulo de processamento de imagem para o pipeline de captura.
//!
//! Fornece três sub-componentes:
//! - [`PlatformDirResolver`]: resolve o diretório de capturas por plataforma
//! - [`FileNameGenerator`]: gera nomes de arquivo no padrão `YYYY-MM-DD_HH-MM-SS_mode.ext`
//! - [`BlackImageDetector`]: detecta imagens pretas por amostragem estatística de pixels

use crate::capture::CaptureModeName;
use image::RgbaImage;
use rand::Rng;
use std::fmt;
use std::path::{Path, PathBuf};

// ============================================================
// Tipos de Erro
// ============================================================

/// Erros que podem ocorrer durante operações de processamento de imagem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageProcessError {
    /// Operação de crop falhou (ex: seleção fora dos limites).
    CropFailed(String),
    /// Encoding PNG falhou.
    EncodeFailed(String),
    /// Resolução do diretório da plataforma falhou.
    DirResolveFailed(String),
    /// Geração de nome de arquivo falhou (ex: muitas colisões).
    FileNameFailed(String),
}

impl fmt::Display for ImageProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CropFailed(msg) => write!(f, "Failed to crop image: {}", msg),
            Self::EncodeFailed(msg) => write!(f, "Failed to encode PNG: {}", msg),
            Self::DirResolveFailed(msg) => {
                write!(f, "Failed to resolve platform directory: {}", msg)
            }
            Self::FileNameFailed(msg) => write!(f, "Failed to generate filename: {}", msg),
        }
    }
}

impl std::error::Error for ImageProcessError {}

/// Erros específicos da resolução de diretório por plataforma.
#[derive(Debug)]
pub enum DirResolveError {
    /// Falha ao criar o diretório de capturas no disco.
    CreateFailed { path: String, reason: String },
    /// Nem `data_dir` nem `home_dir` estão disponíveis nesta plataforma.
    NoPlatformDir,
}

impl fmt::Display for DirResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFailed { path, reason } => {
                write!(f, "Failed to create directory '{}': {}", path, reason)
            }
            Self::NoPlatformDir => {
                write!(f, "No home or data directory available on this platform")
            }
        }
    }
}

impl std::error::Error for DirResolveError {}

// ============================================================
// BlackImageDetector
// ============================================================

/// Resultado da análise de detecção de imagem preta.
pub struct DetectionResult {
    /// Se a imagem é classificada como preta (>= 99% dos pixels amostrados são pretos).
    pub is_black: bool,
    /// Número de pixels que foram amostrados.
    pub sampled_pixels: usize,
    /// Proporção de pixels pretos entre os pixels amostrados (intervalo [0.0, 1.0]).
    pub black_pixel_ratio: f64,
}

/// Detecta imagens pretas via amostragem estatística de pixels.
pub struct BlackImageDetector;

impl BlackImageDetector {
    /// Amostra ~1000 pixels aleatórios distribuídos pela imagem.
    ///
    /// Um pixel é considerado "preto" se R + G + B < 30.
    /// A imagem é classificada como "preta" se >= 99% dos pixels amostrados são pretos.
    pub fn check(image: &RgbaImage) -> DetectionResult {
        let width = image.width() as usize;
        let height = image.height() as usize;
        let total_pixels = width * height;

        if total_pixels == 0 {
            return DetectionResult {
                is_black: false,
                sampled_pixels: 0,
                black_pixel_ratio: 0.0,
            };
        }

        let sample_count = total_pixels.min(1000);
        let mut rng = rand::thread_rng();
        let mut black_count = 0usize;

        for _ in 0..sample_count {
            let idx = rng.gen_range(0..total_pixels);
            let x = (idx % width) as u32;
            let y = (idx / width) as u32;
            let pixel = image.get_pixel(x, y);
            let r = pixel[0] as u32;
            let g = pixel[1] as u32;
            let b = pixel[2] as u32;
            if r + g + b < 30 {
                black_count += 1;
            }
        }

        let black_pixel_ratio = black_count as f64 / sample_count as f64;
        let is_black = black_pixel_ratio >= 0.99;

        DetectionResult {
            is_black,
            sampled_pixels: sample_count,
            black_pixel_ratio,
        }
    }
}

// ============================================================
// FileNameGenerator
// ============================================================

/// Gera nomes de arquivo para capturas seguindo a convenção do projeto.
pub struct FileNameGenerator;

impl FileNameGenerator {
    /// Gera um nome de arquivo no formato `YYYY-MM-DD_HH-MM-SS_mode.ext`.
    ///
    /// Se um arquivo com o nome gerado já existir em `target_dir`,
    /// adiciona um sufixo numérico `_2`, `_3`, etc. (até 100 tentativas).
    ///
    /// Retorna `Err(ImageProcessError::FileNameFailed)` se 100 tentativas forem esgotadas.
    pub fn generate(
        mode: &CaptureModeName,
        target_dir: &Path,
        extension: &str,
    ) -> Result<String, ImageProcessError> {
        let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let mode_label = Self::mode_label(mode);
        let base_name = format!("{timestamp}_{mode_label}");

        // Verifica nome base (sem colisão)
        let base_filename = format!("{}.{}", base_name, extension);
        if std::fs::metadata(target_dir.join(&base_filename)).is_err() {
            return Ok(base_filename);
        }

        // Resolve colisão com sufixo _2, _3, ... até 100
        for i in 2u32..=100 {
            let candidate = format!("{}_{}.{}", base_name, i, extension);
            if std::fs::metadata(target_dir.join(&candidate)).is_err() {
                tracing::debug!(
                    original = %base_filename,
                    resolved = %candidate,
                    suffix = i,
                    "Filename collision resolved with numeric suffix"
                );
                return Ok(candidate);
            }
        }

        Err(ImageProcessError::FileNameFailed(format!(
            "Could not find non-colliding filename after 100 attempts for base '{}'",
            base_name
        )))
    }

    fn mode_label(mode: &CaptureModeName) -> &'static str {
        match mode {
            CaptureModeName::Fullscreen => "fullscreen",
            CaptureModeName::Area => "region",
            CaptureModeName::Window => "window",
        }
    }
}

// ============================================================
// PlatformDirResolver
// ============================================================

/// Resolve o diretório de capturas específico da plataforma e o auto-cria.
pub struct PlatformDirResolver;

impl PlatformDirResolver {
    /// Retorna o diretório de capturas para a plataforma atual.
    ///
    /// - Linux: `~/.local/share/screenshot-tool/captures/`
    /// - macOS: `~/Library/Application Support/screenshot-tool/captures/`
    /// - Windows: `%APPDATA%/screenshot-tool/captures/`
    ///
    /// Faz fallback para `home_dir()/screenshot-tool/captures/` se `data_dir()` retornar `None`.
    /// Auto-cria o diretório via `std::fs::create_dir_all` se não existir.
    pub fn resolve() -> Result<PathBuf, DirResolveError> {
        Self::resolve_from(dirs::data_dir(), dirs::home_dir())
    }

    fn resolve_from(
        data_dir: Option<PathBuf>,
        home_dir: Option<PathBuf>,
    ) -> Result<PathBuf, DirResolveError> {
        let base = match data_dir {
            Some(dir) => dir,
            None => {
                tracing::warn!("dirs::data_dir() returned None, falling back to dirs::home_dir()");
                home_dir.ok_or(DirResolveError::NoPlatformDir)?
            }
        };

        let captures_dir = base.join("screenshot-tool").join("captures");

        std::fs::create_dir_all(&captures_dir).map_err(|e| DirResolveError::CreateFailed {
            path: captures_dir.display().to_string(),
            reason: e.to_string(),
        })?;

        tracing::info!(
            path = %captures_dir.display(),
            "Captures directory resolved"
        );

        Ok(captures_dir)
    }
}

// ============================================================
// Testes
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    // -------------------------
    // Funções auxiliares
    // -------------------------

    fn all_black_image(width: u32, height: u32) -> RgbaImage {
        ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 255]))
    }

    fn all_white_image(width: u32, height: u32) -> RgbaImage {
        ImageBuffer::from_pixel(width, height, Rgba([255, 255, 255, 255]))
    }

    fn uniform_image(width: u32, height: u32, r: u8, g: u8, b: u8) -> RgbaImage {
        ImageBuffer::from_pixel(width, height, Rgba([r, g, b, 255]))
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        let id = uuid::Uuid::new_v4().to_string().replace('-', "");
        let dir = std::env::temp_dir().join(format!("test_imgproc_{}_{}", prefix, id));
        std::fs::create_dir_all(&dir).expect("failed to create unique test dir");
        dir
    }

    // -------------------------
    // Testes do BlackImageDetector
    // -------------------------

    #[test]
    fn black_detector_all_black_image_is_black() {
        let image = all_black_image(100, 100);
        let result = BlackImageDetector::check(&image);
        assert!(
            result.is_black,
            "Imagem totalmente preta deve ser classificada como preta"
        );
    }

    #[test]
    fn black_detector_all_white_image_is_not_black() {
        let image = all_white_image(100, 100);
        let result = BlackImageDetector::check(&image);
        assert!(
            !result.is_black,
            "Imagem totalmente branca NÃO deve ser classificada como preta"
        );
    }

    #[test]
    fn black_detector_near_threshold_rgb_29_is_black() {
        // R+G+B = 9+10+10 = 29 → abaixo do threshold 30 → pixel é "preto"
        let image = uniform_image(100, 100, 9, 10, 10);
        let result = BlackImageDetector::check(&image);
        assert!(
            result.is_black,
            "R+G+B=29 deve ser classificado como preto, ratio={}",
            result.black_pixel_ratio
        );
    }

    #[test]
    fn black_detector_above_threshold_rgb_31_is_not_black() {
        // R+G+B = 10+11+10 = 31 → acima do threshold 30 → pixel NÃO é "preto"
        let image = uniform_image(100, 100, 10, 11, 10);
        let result = BlackImageDetector::check(&image);
        assert!(
            !result.is_black,
            "R+G+B=31 NÃO deve ser classificado como preto, ratio={}",
            result.black_pixel_ratio
        );
    }

    #[test]
    fn black_detector_single_pixel_image_no_panic() {
        let image = all_black_image(1, 1);
        let result = BlackImageDetector::check(&image);
        assert!(
            result.sampled_pixels >= 1,
            "Deve amostrar pelo menos 1 pixel"
        );
        assert!(
            result.sampled_pixels <= 1,
            "Não pode amostrar mais de 1 pixel de imagem 1x1"
        );
        assert!(
            result.black_pixel_ratio >= 0.0 && result.black_pixel_ratio <= 1.0,
            "black_pixel_ratio deve estar em [0.0, 1.0]"
        );
    }

    #[test]
    fn black_detector_result_has_valid_stats() {
        let image = all_black_image(200, 200);
        let result = BlackImageDetector::check(&image);
        assert!(result.sampled_pixels > 0, "sampled_pixels deve ser > 0");
        assert!(
            result.black_pixel_ratio >= 0.0 && result.black_pixel_ratio <= 1.0,
            "black_pixel_ratio deve estar em [0.0, 1.0], obtido: {}",
            result.black_pixel_ratio
        );
    }

    // -------------------------
    // Testes do FileNameGenerator
    // -------------------------

    #[test]
    fn file_name_generator_format_fullscreen_png() {
        let dir = unique_test_dir("fname_format");
        let filename =
            FileNameGenerator::generate(&CaptureModeName::Fullscreen, &dir, "png").unwrap();
        // Deve terminar com _fullscreen.png
        assert!(
            filename.ends_with("_fullscreen.png"),
            "Filename fullscreen deve terminar com '_fullscreen.png', obtido: {}",
            filename
        );
        // A parte de timestamp deve ter exatamente 19 chars (YYYY-MM-DD_HH-MM-SS)
        let ts_part = filename
            .strip_suffix("_fullscreen.png")
            .expect("deve ter sufixo _fullscreen.png");
        assert_eq!(
            ts_part.len(),
            19,
            "Timestamp deve ter exatamente 19 chars, obtido: '{}'",
            ts_part
        );
    }

    #[test]
    fn file_name_generator_mode_label_fullscreen() {
        let dir = unique_test_dir("mode_fullscreen");
        let f = FileNameGenerator::generate(&CaptureModeName::Fullscreen, &dir, "png").unwrap();
        assert!(
            f.contains("_fullscreen.png"),
            "Modo Fullscreen deve produzir label 'fullscreen': {}",
            f
        );
    }

    #[test]
    fn file_name_generator_mode_label_area_maps_to_region() {
        let dir = unique_test_dir("mode_area");
        let f = FileNameGenerator::generate(&CaptureModeName::Area, &dir, "png").unwrap();
        assert!(
            f.contains("_region.png"),
            "Modo Area deve produzir label 'region' (não 'area'): {}",
            f
        );
    }

    #[test]
    fn file_name_generator_mode_label_window() {
        let dir = unique_test_dir("mode_window");
        let f = FileNameGenerator::generate(&CaptureModeName::Window, &dir, "png").unwrap();
        assert!(
            f.contains("_window.png"),
            "Modo Window deve produzir label 'window': {}",
            f
        );
    }

    #[test]
    fn file_name_generator_no_collision_returns_base_name() {
        let dir = unique_test_dir("no_collision");
        let filename =
            FileNameGenerator::generate(&CaptureModeName::Fullscreen, &dir, "png").unwrap();
        // Nome base termina diretamente com o label do modo (sem sufixo _N)
        assert!(
            filename.ends_with("_fullscreen.png"),
            "Sem colisão: deve retornar nome base terminando com label do modo: {}",
            filename
        );
        assert!(
            !filename.contains("_2.png"),
            "Sem colisão: não deve ter sufixo _2: {}",
            filename
        );
    }

    #[test]
    fn file_name_generator_collision_returns_suffix_2() {
        let dir = unique_test_dir("collision_2");

        // Obtém o nome base
        let first = FileNameGenerator::generate(&CaptureModeName::Area, &dir, "png")
            .expect("primeiro generate");

        // Cria o arquivo para simular colisão
        std::fs::write(dir.join(&first), b"dummy").expect("criar arquivo de colisão");

        // Gera novamente — deve produzir nome diferente
        let second = FileNameGenerator::generate(&CaptureModeName::Area, &dir, "png")
            .expect("segundo generate após colisão");

        // Segundo nome deve diferir do primeiro
        assert_ne!(
            first, second,
            "Segundo filename deve diferir do primeiro após colisão"
        );

        // O arquivo gerado não deve existir ainda
        assert!(
            !dir.join(&second).exists(),
            "Filename gerado não deve existir ainda: {}",
            second
        );

        // Se mesmo segundo (caso mais comum): deve ter sufixo _2
        // Se segundo diferente (raro): timestamp novo também é válido
        let first_base = first.strip_suffix(".png").expect("deve ter extensão .png");
        let expected_2 = format!("{}_2.png", first_base);
        let is_suffix_2 = second == expected_2;
        let is_fresh_base = second.ends_with("_region.png") && !second.contains("_2");

        assert!(
            is_suffix_2 || is_fresh_base,
            "Esperado sufixo _2 ou timestamp novo, obtido: first={}, second={}",
            first,
            second
        );
    }

    #[test]
    fn file_name_generator_multiple_collisions_returns_suffix_3() {
        let dir = unique_test_dir("collision_3");

        // Cria primeiro arquivo
        let first = FileNameGenerator::generate(&CaptureModeName::Window, &dir, "png").unwrap();
        std::fs::write(dir.join(&first), b"dummy").unwrap();

        // Cria segundo arquivo
        let second = FileNameGenerator::generate(&CaptureModeName::Window, &dir, "png").unwrap();

        let first_base = first.strip_suffix(".png").expect("deve ter .png");
        let expected_2 = format!("{}_2.png", first_base);

        // Se segundo diferente (mudança de segundo): teste válido mas não testa colisão múltipla
        if second != expected_2 {
            assert!(
                !dir.join(&second).exists(),
                "Segundo nome não deve existir: {}",
                second
            );
            return;
        }

        // Mesmo segundo: cria segundo arquivo e obtém terceiro
        std::fs::write(dir.join(&second), b"dummy").unwrap();
        let third = FileNameGenerator::generate(&CaptureModeName::Window, &dir, "png").unwrap();

        let expected_3 = format!("{}_3.png", first_base);

        let is_suffix_3 = third == expected_3;
        let is_fresh =
            third.ends_with("_window.png") && !third.contains("_2") && !third.contains("_3");

        assert!(
            is_suffix_3 || is_fresh,
            "Com 2 colisões, esperado sufixo _3 ou timestamp novo, obtido: {}",
            third
        );
    }

    #[test]
    fn file_name_generator_accepts_custom_extension() {
        let dir = unique_test_dir("custom_ext");
        let filename =
            FileNameGenerator::generate(&CaptureModeName::Fullscreen, &dir, "jpg").unwrap();
        assert!(
            filename.ends_with(".jpg"),
            "Filename deve usar extensão 'jpg' especificada: {}",
            filename
        );
    }

    // -------------------------
    // Testes do PlatformDirResolver
    // -------------------------

    #[test]
    fn platform_dir_resolver_path_ends_with_captures() {
        let tmp = unique_test_dir("dir_resolver_path");
        let result = PlatformDirResolver::resolve_from(Some(tmp.clone()), None)
            .expect("deve resolver com sucesso");
        let path_str = result.to_string_lossy();
        assert!(
            path_str.ends_with("screenshot-tool/captures")
                || path_str.ends_with("screenshot-tool\\captures"),
            "Path deve terminar com screenshot-tool/captures, obtido: {}",
            path_str
        );
    }

    #[test]
    fn platform_dir_resolver_creates_directory_automatically() {
        let tmp = unique_test_dir("dir_resolver_create");
        let result = PlatformDirResolver::resolve_from(Some(tmp.clone()), None)
            .expect("deve resolver e criar diretório");
        assert!(
            result.exists(),
            "Diretório deve ter sido criado automaticamente: {}",
            result.display()
        );
    }

    #[test]
    fn platform_dir_resolver_is_idempotent() {
        let tmp = unique_test_dir("dir_resolver_idem");
        let first =
            PlatformDirResolver::resolve_from(Some(tmp.clone()), None).expect("primeiro resolve");
        let second = PlatformDirResolver::resolve_from(Some(tmp.clone()), None)
            .expect("segundo resolve (idempotente)");
        assert_eq!(
            first, second,
            "Ambas as chamadas de resolve() devem retornar o mesmo path"
        );
    }

    #[test]
    fn platform_dir_resolver_falls_back_to_home_dir() {
        let tmp = unique_test_dir("dir_resolver_fallback");
        // data_dir = None → deve fazer fallback para home_dir
        let result = PlatformDirResolver::resolve_from(None, Some(tmp.clone()))
            .expect("deve usar fallback para home_dir");
        let path_str = result.to_string_lossy();
        assert!(
            path_str.contains("screenshot-tool"),
            "Path de fallback deve conter 'screenshot-tool': {}",
            path_str
        );
    }

    #[test]
    fn platform_dir_resolver_error_when_both_dirs_none() {
        let result = PlatformDirResolver::resolve_from(None, None);
        assert!(
            matches!(result, Err(DirResolveError::NoPlatformDir)),
            "Deve retornar DirResolveError::NoPlatformDir quando ambos os dirs são None"
        );
    }
}
