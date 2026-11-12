//! Media file storage and preview/sample generation.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use image::imageops::FilterType;
use image::GenericImageView;

pub const PREVIEW_MAX: u32 = 150;
pub const SAMPLE_MAX: u32 = 850;

#[derive(Debug, Clone)]
pub struct MediaProcessor {
    pub data_dir: PathBuf,
}

#[derive(Debug, Default)]
pub struct MediaInfo {
    pub width: i64,
    pub height: i64,
    pub has_sample: bool,
    pub preview_width: i64,
    pub preview_height: i64,
    pub sample_width: i64,
    pub sample_height: i64,
}

impl MediaProcessor {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn in_memory() -> Self {
        Self {
            data_dir: std::env::temp_dir().join(format!("self26-media-{}", uuid::Uuid::new_v4())),
        }
    }

    fn dir(&self, kind: &str, md5: &str) -> PathBuf {
        self.data_dir.join(kind).join(&md5[..2])
    }

    pub fn remove(&self, md5: &str) {
        for kind in ["original", "preview", "sample"] {
            let dir = self.dir(kind, md5);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut others = false;
            for e in entries.flatten() {
                if e.file_name().to_string_lossy().starts_with(md5) {
                    let _ = std::fs::remove_file(e.path());
                } else {
                    others = true;
                }
            }
            if !others {
                let _ = std::fs::remove_dir(&dir);
            }
        }
    }

    pub fn path(&self, kind: &str, md5: &str, ext: &str) -> PathBuf {
        self.dir(kind, md5).join(format!("{md5}.{ext}"))
    }

    /// Stores the original file and derives preview/sample images when the
    /// file is a decodable image.
    pub fn store(&self, bytes: &[u8], md5: &str, ext: &str) -> Result<MediaInfo> {
        let dir = self.dir("original", md5);
        std::fs::create_dir_all(&dir).context("create media dir")?;
        std::fs::write(dir.join(format!("{md5}.{ext}")), bytes).context("write original")?;

        let mut info = MediaInfo::default();
        let Ok(reader) = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()
        else {
            return Ok(info); // video or unsupported: stored but no previews
        };
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(16_384);
        limits.max_image_height = Some(16_384);
        let mut reader = reader;
        reader.limits(limits);
        let Ok(img) = reader.decode() else {
            return Ok(info);
        };
        let (w, h) = img.dimensions();
        info.width = w as i64;
        info.height = h as i64;

        let preview = img.resize(PREVIEW_MAX, PREVIEW_MAX, FilterType::Lanczos3);
        let (pw, ph) = preview.dimensions();
        info.preview_width = pw as i64;
        info.preview_height = ph as i64;
        let pdir = self.dir("preview", md5);
        std::fs::create_dir_all(&pdir)?;
        preview
            .save_with_format(pdir.join(format!("{md5}.jpg")), image::ImageFormat::Jpeg)
            .context("write preview")?;
        save_webp(&preview, &pdir.join(format!("{md5}.webp")));

        if w > SAMPLE_MAX || h > SAMPLE_MAX {
            let sample = img.resize(SAMPLE_MAX, SAMPLE_MAX, FilterType::Lanczos3);
            let (sw, sh) = sample.dimensions();
            info.has_sample = true;
            info.sample_width = sw as i64;
            info.sample_height = sh as i64;
            let sdir = self.dir("sample", md5);
            std::fs::create_dir_all(&sdir)?;
            sample
                .save_with_format(sdir.join(format!("{md5}.jpg")), image::ImageFormat::Jpeg)
                .context("write sample")?;
            save_webp(&sample, &sdir.join(format!("{md5}.webp")));
        } else {
            info.sample_width = w as i64;
            info.sample_height = h as i64;
        }
        Ok(info)
    }
}

fn save_webp(img: &image::DynamicImage, path: &Path) {
    // Lossy-enough webp for previews; ignore failures since jpg is canonical.
    let _ = img.save_with_format(path, image::ImageFormat::WebP);
}

pub fn file_md5(bytes: &[u8]) -> String {
    hex::encode(md5::compute(bytes).0)
}

pub fn file_ext(bytes: &[u8], filename: &str) -> String {
    let sniffed = if bytes.starts_with(b"\x89PNG") {
        Some("png")
    } else if bytes.starts_with(b"\xFF\xD8") {
        Some("jpg")
    } else if bytes.starts_with(b"GIF8") {
        Some("gif")
    } else if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        Some("mp4")
    } else if bytes.starts_with(b"\x1A\x45\xDF\xA3") {
        Some("webm")
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else if bytes.starts_with(b"FWS") || bytes.starts_with(b"CWS") || bytes.starts_with(b"ZWS") {
        Some("swf")
    } else if bytes.starts_with(b"%PDF") {
        Some("pdf")
    } else {
        None
    };
    if let Some(e) = sniffed {
        return e.into();
    }
    match Path::new(filename)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
    {
        Some("jpeg") => "jpg".into(),
        Some(e @ ("png" | "jpg" | "gif" | "webp" | "webm" | "mp4")) => e.into(),
        _ => "bin".into(),
    }
}
