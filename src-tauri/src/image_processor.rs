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
// SelectionRegion
// ============================================================

/// Região de seleção para captura de área (mode Area/Region).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

// ============================================================
// CaptureInput
// ============================================================

/// Entrada do pipeline de processamento de imagem.
pub struct CaptureInput {
    /// Imagem bruta capturada.
    pub image: RgbaImage,
    /// Modo de captura (Fullscreen, Area, Window).
    pub mode: CaptureModeName,
    /// Região selecionada — presente apenas para mode=Area.
    pub selection: Option<SelectionRegion>,
    /// Flag do detector de imagem preta.
    pub is_potentially_black: bool,
}

// ============================================================
// ProcessedCapture
// ============================================================

/// Resultado do processamento de imagem: bytes PNG + metadados para gravação em disco.
#[derive(Debug)]
pub struct ProcessedCapture {
    /// Bytes PNG codificados (sem I/O de disco — responsabilidade do StorageManager).
    pub png_bytes: Vec<u8>,
    /// Caminho completo de destino (diretório + filename).
    pub target_path: PathBuf,
    /// Nome do arquivo gerado (ex: `2026-02-23_14-35-22_region.png`).
    pub filename: String,
    /// Largura da imagem em pixels (pós-crop para mode Area).
    pub width: u32,
    /// Altura da imagem em pixels (pós-crop para mode Area).
    pub height: u32,
    /// `true` se a imagem foi detectada como possivelmente preta.
    pub is_black_warning: bool,
}

// ============================================================
// ImageProcessor
// ============================================================

/// Processa uma imagem capturada: crop condicional, naming, resolução de diretório e
/// encoding PNG.  Não realiza I/O de disco — essa responsabilidade é do `StorageManager`.
pub struct ImageProcessor;

impl ImageProcessor {
    /// Processa a imagem capturada e retorna um [`ProcessedCapture`] pronto para ser
    /// gravado pelo `StorageManager`.
    ///
    /// # Pipeline
    /// 1. Se `mode == Area`: valida bounds da [`SelectionRegion`] e aplica crop.
    /// 2. Resolve o diretório de capturas via [`PlatformDirResolver::resolve()`].
    /// 3. Gera o nome do arquivo via [`FileNameGenerator::generate()`].
    /// 4. Codifica a imagem (possivelmente recortada) para bytes PNG.
    /// 5. Retorna [`ProcessedCapture`] com todos os metadados.
    pub fn process(input: CaptureInput) -> Result<ProcessedCapture, ImageProcessError> {
        // 1. Crop condicional para mode Area
        let final_image: RgbaImage = match input.mode {
            CaptureModeName::Area => {
                let region = input.selection.ok_or_else(|| {
                    ImageProcessError::CropFailed(
                        "SelectionRegion is required for Area mode but was None".to_string(),
                    )
                })?;

                // Validar dimensões não-zero
                if region.width == 0 || region.height == 0 {
                    return Err(ImageProcessError::CropFailed(format!(
                        "SelectionRegion dimensions must be > 0, got width={} height={}",
                        region.width, region.height
                    )));
                }

                let img_width = input.image.width();
                let img_height = input.image.height();

                // Verificar bounds: right e bottom não podem ultrapassar as dimensões da imagem
                let right = region.x.checked_add(region.width).ok_or_else(|| {
                    ImageProcessError::CropFailed(
                        "SelectionRegion x + width overflow u32".to_string(),
                    )
                })?;
                let bottom = region.y.checked_add(region.height).ok_or_else(|| {
                    ImageProcessError::CropFailed(
                        "SelectionRegion y + height overflow u32".to_string(),
                    )
                })?;

                if right > img_width || bottom > img_height {
                    return Err(ImageProcessError::CropFailed(format!(
                        "SelectionRegion (x={}, y={}, {}x{}) is out of image bounds ({}x{})",
                        region.x, region.y, region.width, region.height, img_width, img_height,
                    )));
                }

                tracing::debug!(
                    x = region.x,
                    y = region.y,
                    width = region.width,
                    height = region.height,
                    "Applying crop for Area mode"
                );

                image::imageops::crop_imm(
                    &input.image,
                    region.x,
                    region.y,
                    region.width,
                    region.height,
                )
                .to_image()
            }
            // Fullscreen e Window: sem crop — usa imagem original
            CaptureModeName::Fullscreen | CaptureModeName::Window => input.image,
        };

        // 2. Resolver diretório de capturas por plataforma
        let target_dir = PlatformDirResolver::resolve()
            .map_err(|e| ImageProcessError::DirResolveFailed(e.to_string()))?;

        // 3. Gerar nome de arquivo seguindo a convenção YYYY-MM-DD_HH-MM-SS_mode.png
        let filename = FileNameGenerator::generate(&input.mode, &target_dir, "png")?;

        // 4. Codificar imagem para bytes PNG (sem I/O de disco)
        let width = final_image.width();
        let height = final_image.height();

        let mut png_bytes = Vec::new();
        image::DynamicImage::ImageRgba8(final_image)
            .write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            )
            .map_err(|e| ImageProcessError::EncodeFailed(e.to_string()))?;

        // 5. Compor caminho de destino completo
        let target_path = target_dir.join(&filename);

        let crop_applied = matches!(input.mode, CaptureModeName::Area);
        tracing::info!(
            filename = %filename,
            width = width,
            height = height,
            crop_applied = crop_applied,
            "Image processing complete"
        );
        tracing::debug!(
            crop_applied = crop_applied,
            "Crop applied flag for processed capture"
        );

        Ok(ProcessedCapture {
            png_bytes,
            target_path,
            filename,
            width,
            height,
            is_black_warning: input.is_potentially_black,
        })
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

    // -------------------------
    // Testes do ImageProcessor::process()
    // -------------------------

    #[test]
    fn process_region_mode_crops_to_selection_dimensions() {
        let image = all_white_image(100, 100);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Area,
            selection: Some(SelectionRegion {
                x: 10,
                y: 10,
                width: 50,
                height: 40,
            }),
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input).expect("deve processar sem erro");
        assert_eq!(result.width, 50, "Largura deve ser 50 após crop");
        assert_eq!(result.height, 40, "Altura deve ser 40 após crop");
    }

    #[test]
    fn process_fullscreen_mode_preserves_original_dimensions() {
        let image = all_white_image(200, 150);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Fullscreen,
            selection: None,
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input).expect("deve processar sem erro");
        assert_eq!(result.width, 200, "Largura deve ser 200 sem crop");
        assert_eq!(result.height, 150, "Altura deve ser 150 sem crop");
    }

    #[test]
    fn process_window_mode_preserves_original_dimensions() {
        let image = all_white_image(200, 150);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Window,
            selection: None,
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input).expect("deve processar sem erro");
        assert_eq!(
            result.width, 200,
            "Largura deve ser 200 sem crop (Window mode)"
        );
        assert_eq!(
            result.height, 150,
            "Altura deve ser 150 sem crop (Window mode)"
        );
    }

    #[test]
    fn process_region_out_of_bounds_returns_crop_failed() {
        // Imagem 100x100, seleção x=90, y=90, width=50, height=50 → right=140, bottom=140 → fora dos bounds
        let image = all_white_image(100, 100);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Area,
            selection: Some(SelectionRegion {
                x: 90,
                y: 90,
                width: 50,
                height: 50,
            }),
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input);
        assert!(
            matches!(result, Err(ImageProcessError::CropFailed(_))),
            "Seleção fora dos bounds deve retornar CropFailed, obtido: {:?}",
            result
        );
    }

    #[test]
    fn process_region_width_zero_returns_crop_failed() {
        let image = all_white_image(100, 100);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Area,
            selection: Some(SelectionRegion {
                x: 0,
                y: 0,
                width: 0,
                height: 50,
            }),
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input);
        assert!(
            matches!(result, Err(ImageProcessError::CropFailed(_))),
            "width=0 deve retornar CropFailed, obtido: {:?}",
            result
        );
    }

    #[test]
    fn process_png_bytes_are_valid_png() {
        let image = all_white_image(50, 50);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Fullscreen,
            selection: None,
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input).expect("deve processar sem erro");
        let decoded = image::load_from_memory(&result.png_bytes);
        assert!(
            decoded.is_ok(),
            "png_bytes deve ser um PNG válido: {:?}",
            decoded.err()
        );
    }

    #[test]
    fn process_is_black_warning_propagated() {
        let image = all_black_image(50, 50);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Fullscreen,
            selection: None,
            is_potentially_black: true,
        };
        let result = ImageProcessor::process(input).expect("deve processar sem erro");
        assert!(
            result.is_black_warning,
            "is_black_warning deve ser true quando is_potentially_black=true"
        );
    }

    #[test]
    fn process_is_black_warning_false_when_not_set() {
        let image = all_white_image(50, 50);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Fullscreen,
            selection: None,
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input).expect("deve processar sem erro");
        assert!(
            !result.is_black_warning,
            "is_black_warning deve ser false quando is_potentially_black=false"
        );
    }

    #[test]
    fn process_filename_follows_naming_convention() {
        // Area mode → deve conter "_region.png"
        let image_area = all_white_image(80, 80);
        let input_area = CaptureInput {
            image: image_area,
            mode: CaptureModeName::Area,
            selection: Some(SelectionRegion {
                x: 0,
                y: 0,
                width: 80,
                height: 80,
            }),
            is_potentially_black: false,
        };
        let result_area = ImageProcessor::process(input_area).expect("deve processar Area");
        assert!(
            result_area.filename.contains("_region.png"),
            "Filename para Area mode deve conter '_region.png', obtido: {}",
            result_area.filename
        );

        // Fullscreen mode → deve conter "_fullscreen.png"
        let image_full = all_white_image(80, 80);
        let input_full = CaptureInput {
            image: image_full,
            mode: CaptureModeName::Fullscreen,
            selection: None,
            is_potentially_black: false,
        };
        let result_full = ImageProcessor::process(input_full).expect("deve processar Fullscreen");
        assert!(
            result_full.filename.contains("_fullscreen.png"),
            "Filename para Fullscreen mode deve conter '_fullscreen.png', obtido: {}",
            result_full.filename
        );
    }

    #[test]
    fn process_target_path_equals_dir_plus_filename() {
        let image = all_white_image(60, 60);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Window,
            selection: None,
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input).expect("deve processar sem erro");

        // target_path deve ser igual ao diretório pai concatenado com o filename
        let expected = result
            .target_path
            .parent()
            .expect("target_path deve ter diretório pai")
            .join(&result.filename);

        assert_eq!(
            result.target_path, expected,
            "target_path deve ser igual a parent_dir.join(filename)"
        );
    }

    #[test]
    fn process_region_height_zero_returns_crop_failed() {
        let image = all_white_image(100, 100);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Area,
            selection: Some(SelectionRegion {
                x: 0,
                y: 0,
                width: 50,
                height: 0,
            }),
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input);
        assert!(
            matches!(result, Err(ImageProcessError::CropFailed(_))),
            "height=0 deve retornar CropFailed, obtido: {:?}",
            result
        );
    }

    #[test]
    fn process_region_mode_with_no_selection_returns_crop_failed() {
        // Area mode com selection=None deve retornar CropFailed (seleção obrigatória)
        let image = all_white_image(100, 100);
        let input = CaptureInput {
            image,
            mode: CaptureModeName::Area,
            selection: None,
            is_potentially_black: false,
        };
        let result = ImageProcessor::process(input);
        assert!(
            matches!(result, Err(ImageProcessError::CropFailed(_))),
            "Area mode com selection=None deve retornar CropFailed, obtido: {:?}",
            result
        );
    }
}
