use image::{imageops::FilterType, DynamicImage, GenericImageView, ImageFormat};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tracing::{debug, error};

pub struct ImageResizer {
    cache_dir: PathBuf,
}

impl ImageResizer {
    pub fn new(cache_dir: PathBuf) -> Result<Self, ImageResizerError> {
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    pub fn resize_image(
        &self,
        source_path: &Path,
        width: Option<u32>,
        height: Option<u32>,
        quality: Option<u32>,
    ) -> Result<PathBuf, ImageResizerError> {
        // If no resize parameters, return original path
        if width.is_none() && height.is_none() && quality.is_none() {
            return Ok(source_path.to_path_buf());
        }

        let cache_key = self.generate_cache_key(source_path, width, height, quality);
        let cache_path = self.cache_dir.join(&cache_key);

        if cache_path.exists() {
            debug!("Serving cached image: {}", cache_key);
            return Ok(cache_path);
        }

        debug!("Resizing image: {:?}", source_path);

        // Read file bytes for format detection
        let file_bytes = match fs::read(source_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("Failed to read image {:?}: {}", source_path, e);
                return Ok(source_path.to_path_buf());
            }
        };

        // Detect format from file content
        let format = match image::guess_format(&file_bytes) {
            Ok(fmt) => fmt,
            Err(e) => {
                error!("Failed to detect format for {:?}: {}", source_path, e);
                return Ok(source_path.to_path_buf());
            }
        };

        // Load the image
        let img = match image::load_from_memory(&file_bytes) {
            Ok(img) => img,
            Err(e) => {
                error!("Failed to load image {:?}: {}", source_path, e);
                return Ok(source_path.to_path_buf());
            }
        };

        let (orig_width, orig_height) = img.dimensions();

        let (target_width, target_height) =
            self.calculate_dimensions(orig_width, orig_height, width, height);

        let resized = if target_width == orig_width && target_height == orig_height {
            img
        } else {
            img.resize(target_width, target_height, FilterType::Lanczos3)
        };

        let encoded = match self.encode_image(resized, format, quality) {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to encode image {:?}: {}", source_path, e);
                return Ok(source_path.to_path_buf());
            }
        };

        if let Err(e) = fs::write(&cache_path, &encoded) {
            error!("Failed to write cache file {}: {}", cache_key, e);
            return Ok(source_path.to_path_buf());
        }

        // Copy mtime of original file.
        if let Ok(meta) = fs::metadata(&cache_path) {
            let times = fs::FileTimes::new()
                .set_accessed(meta.accessed()?)
                .set_modified(meta.modified()?);
            if let Ok(dest) = fs::File::open(&cache_path) {
                let _ = dest.set_times(times);
            }
        }

        Ok(cache_path)
    }

    fn calculate_dimensions(
        &self,
        orig_width: u32,
        orig_height: u32,
        width: Option<u32>,
        height: Option<u32>,
    ) -> (u32, u32) {
        match (width, height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => {
                let aspect_ratio = orig_height as f32 / orig_width as f32;
                let h = (w as f32 * aspect_ratio).round() as u32;
                (w, h)
            }
            (None, Some(h)) => {
                let aspect_ratio = orig_width as f32 / orig_height as f32;
                let w = (h as f32 * aspect_ratio).round() as u32;
                (w, h)
            }
            (None, None) => (orig_width, orig_height),
        }
    }

    fn encode_image(
        &self,
        img: DynamicImage,
        format: ImageFormat,
        quality: Option<u32>,
    ) -> Result<Vec<u8>, ImageResizerError> {
        let mut buffer = Cursor::new(Vec::new());

        match format {
            ImageFormat::Jpeg => {
                let quality = quality.unwrap_or(90).clamp(1, 100);
                let encoder =
                    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, quality as u8);
                img.write_with_encoder(encoder)?;
            }
            _ => {
                img.write_to(&mut buffer, format)?;
            }
        }

        Ok(buffer.into_inner())
    }

    fn generate_cache_key(
        &self,
        source_path: &Path,
        width: Option<u32>,
        height: Option<u32>,
        quality: Option<u32>,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source_path.to_string_lossy().as_bytes());
        hasher.update(width.unwrap_or(0).to_le_bytes());
        hasher.update(height.unwrap_or(0).to_le_bytes());
        hasher.update(quality.unwrap_or(0).to_le_bytes());

        if let Ok(metadata) = fs::metadata(source_path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    hasher.update(duration.as_secs().to_le_bytes());
                }
            }
        }

        let result = hasher.finalize();
        let hash = hex::encode(result);

        let extension = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");

        format!("{}.{}", hash, extension)
    }

    pub fn clear_cache(&self) -> Result<(), ImageResizerError> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)?;
            fs::create_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }

    pub fn get_cache_stats(&self) -> Result<CacheStats, ImageResizerError> {
        let mut stats = CacheStats {
            total_files: 0,
            total_size: 0,
            oldest_file: None,
            newest_file: None,
        };

        if !self.cache_dir.exists() {
            return Ok(stats);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                stats.total_files += 1;
                stats.total_size += metadata.len();

                if let Ok(modified) = metadata.modified() {
                    if stats.oldest_file.is_none() || Some(modified) < stats.oldest_file {
                        stats.oldest_file = Some(modified);
                    }
                    if stats.newest_file.is_none() || Some(modified) > stats.newest_file {
                        stats.newest_file = Some(modified);
                    }
                }
            }
        }

        Ok(stats)
    }

    pub fn cleanup_old_cache(&self, max_age_days: u64) -> Result<usize, ImageResizerError> {
        let mut removed = 0;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let now = SystemTime::now();
        let max_age = Duration::from_secs(max_age_days * 24 * 60 * 60);

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age > max_age {
                            fs::remove_file(entry.path())?;
                            removed += 1;
                        }
                    }
                }
            }
        }

        Ok(removed)
    }

    pub fn get_cache_size(&self) -> Result<u64, ImageResizerError> {
        let mut total_size = 0u64;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_files: usize,
    pub total_size: u64,
    pub oldest_file: Option<std::time::SystemTime>,
    pub newest_file: Option<std::time::SystemTime>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImageResizerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
}
