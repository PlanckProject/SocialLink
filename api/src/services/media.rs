use std::sync::Arc;

use anyhow::{Result, ensure};
use bytes::Bytes;

use crate::error::ErrorKind;
use crate::providers::storage::{ObjectStorage, StoredObject};

#[derive(Debug, Clone, Copy)]
pub enum MediaKind {
    Image,
    Favicon,
    /// A link icon image: center-cropped to a square and downscaled to at
    /// most [`MAX_LINK_ICON_DIM`]×[`MAX_LINK_ICON_DIM`], re-encoded as PNG.
    LinkIcon,
}

/// Maximum side length (px) for a stored link icon. Larger uploads are
/// center-cropped to a square and downscaled to this size.
pub const MAX_LINK_ICON_DIM: u32 = 1024;

#[derive(Clone)]
pub struct MediaService<S> {
    storage: Arc<S>,
    max_bytes: usize,
    route_prefix: String,
}

impl<S> MediaService<S>
where
    S: ObjectStorage,
{
    pub fn new(storage: Arc<S>, max_bytes: usize, route_prefix: String) -> Result<Self> {
        ensure!(max_bytes > 0, "media upload limit must be positive");
        ensure!(
            route_prefix.starts_with('/'),
            "media route prefix must start with '/'"
        );
        Ok(Self {
            storage,
            max_bytes,
            route_prefix: route_prefix.trim_end_matches('/').to_string(),
        })
    }

    pub async fn store(
        &self,
        bytes: Bytes,
        content_type: Option<&str>,
        file_name: Option<&str>,
        kind: MediaKind,
    ) -> Result<String> {
        if bytes.len() > self.max_bytes {
            return Err(ErrorKind::BadRequest(
                "uploaded file exceeds configured size limit".into(),
            )
            .into());
        }
        let (extension, content_type, bytes) = match kind {
            MediaKind::LinkIcon => {
                if !is_supported_link_icon(content_type, file_name) {
                    return Err(ErrorKind::BadRequest(
                        "unsupported link icon image type (use PNG, JPEG, WEBP or GIF)".into(),
                    )
                    .into());
                }
                let processed = process_link_icon(&bytes)?;
                ("png", "image/png", processed)
            }
            _ => {
                let (extension, content_type) = media_type(content_type, file_name, kind)
                    .ok_or_else(|| {
                        ErrorKind::BadRequest("unsupported uploaded file type".into())
                    })?;
                (extension, content_type, bytes)
            }
        };
        let key = format!("{}.{}", uuid::Uuid::new_v4(), extension);
        self.storage
            .put(&key, bytes, content_type)
            .await
            .map_err(|error| error.context("store uploaded media"))?;
        Ok(format!("{}/{}", self.route_prefix, key))
    }

    pub async fn load(&self, key: &str) -> Result<Option<StoredObject>> {
        self.storage
            .get(key)
            .await
            .map_err(|error| error.context("load uploaded media"))
    }

    pub async fn delete(&self, public_path: &str) -> Result<bool> {
        let prefix = format!("{}/", self.route_prefix);
        let Some(key) = public_path
            .strip_prefix(&prefix)
            .filter(|key| !key.is_empty())
        else {
            return Ok(false);
        };
        self.storage
            .delete(key)
            .await
            .map_err(|error| error.context("delete uploaded media"))?;
        Ok(true)
    }
}

fn media_type(
    content_type: Option<&str>,
    file_name: Option<&str>,
    kind: MediaKind,
) -> Option<(&'static str, &'static str)> {
    if matches!(kind, MediaKind::Favicon) {
        if matches!(
            content_type,
            Some(
                "image/x-icon"
                    | "image/vnd.microsoft.icon"
                    | "image/ico"
                    | "image/icon"
                    | "image/x-ico"
            )
        ) || extension(file_name).as_deref() == Some("ico")
        {
            return Some(("ico", "image/x-icon"));
        }
        return None;
    }

    match content_type {
        Some("image/png") => Some(("png", "image/png")),
        Some("image/jpeg") => Some(("jpg", "image/jpeg")),
        Some("image/webp") => Some(("webp", "image/webp")),
        Some("image/gif") => Some(("gif", "image/gif")),
        Some("image/avif") => Some(("avif", "image/avif")),
        _ => match extension(file_name).as_deref() {
            Some("png") => Some(("png", "image/png")),
            Some("jpg" | "jpeg") => Some(("jpg", "image/jpeg")),
            Some("webp") => Some(("webp", "image/webp")),
            Some("gif") => Some(("gif", "image/gif")),
            Some("avif") => Some(("avif", "image/avif")),
            _ => None,
        },
    }
}

fn extension(file_name: Option<&str>) -> Option<String> {
    file_name
        .and_then(|name| name.rsplit('.').next())
        .map(str::to_ascii_lowercase)
}

/// Whether an upload is a raster type we can safely center-crop for a link
/// icon. Accepts PNG/JPEG/WEBP/GIF; rejects AVIF (heavy decode) and vector
/// or unknown types.
fn is_supported_link_icon(content_type: Option<&str>, file_name: Option<&str>) -> bool {
    match media_type(content_type, file_name, MediaKind::Image) {
        Some((extension, _)) => extension != "avif",
        None => false,
    }
}

/// Center-crops an uploaded image to the largest centered square, downscales
/// it to at most [`MAX_LINK_ICON_DIM`] per side, and re-encodes as PNG.
fn process_link_icon(bytes: &[u8]) -> Result<Bytes> {
    use image::imageops::FilterType;

    let image = image::load_from_memory(bytes).map_err(|_| {
        ErrorKind::BadRequest("could not decode uploaded link icon image".into())
    })?;

    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return Err(ErrorKind::BadRequest("uploaded link icon image is empty".into()).into());
    }

    let side = width.min(height);
    let x = (width - side) / 2;
    let y = (height - side) / 2;
    let mut square = image.crop_imm(x, y, side, side);
    if side > MAX_LINK_ICON_DIM {
        square = square.resize_exact(MAX_LINK_ICON_DIM, MAX_LINK_ICON_DIM, FilterType::Lanczos3);
    }

    let mut buffer = std::io::Cursor::new(Vec::new());
    square.write_to(&mut buffer, image::ImageFormat::Png).map_err(|_| {
        ErrorKind::BadRequest("could not encode processed link icon image".into())
    })?;
    Ok(Bytes::from(buffer.into_inner()))
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::providers::storage::LocalStorage;

    use super::*;

    #[test]
    fn validates_media_types() {
        assert_eq!(
            media_type(Some("image/png"), None, MediaKind::Image),
            Some(("png", "image/png"))
        );
        assert_eq!(
            media_type(Some("image/svg+xml"), Some("image.svg"), MediaKind::Image),
            None
        );
        assert_eq!(media_type(None, Some("image.svg"), MediaKind::Image), None);
        assert_eq!(
            media_type(None, Some("favicon.ico"), MediaKind::Favicon),
            Some(("ico", "image/x-icon"))
        );
        assert_eq!(
            media_type(Some("image/png"), Some("favicon.png"), MediaKind::Favicon),
            None
        );
    }

    fn encode_png(image: image::DynamicImage) -> Vec<u8> {
        let mut buffer = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut buffer, image::ImageFormat::Png)
            .expect("encode png");
        buffer.into_inner()
    }

    #[test]
    fn link_icon_support_matches_raster_types_only() {
        assert!(is_supported_link_icon(Some("image/png"), None));
        assert!(is_supported_link_icon(Some("image/jpeg"), None));
        assert!(is_supported_link_icon(Some("image/webp"), None));
        assert!(is_supported_link_icon(None, Some("logo.gif")));
        assert!(!is_supported_link_icon(Some("image/avif"), None));
        assert!(!is_supported_link_icon(Some("image/svg+xml"), Some("logo.svg")));
        assert!(!is_supported_link_icon(None, Some("logo.svg")));
    }

    #[test]
    fn link_icon_is_center_cropped_and_downscaled_to_square() {
        let source = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            2000,
            1500,
            image::Rgb([10, 20, 30]),
        ));
        let processed = process_link_icon(&encode_png(source)).expect("process");
        let decoded = image::load_from_memory(&processed).expect("decode processed");
        assert_eq!(decoded.width(), MAX_LINK_ICON_DIM);
        assert_eq!(decoded.height(), MAX_LINK_ICON_DIM);
    }

    #[test]
    fn link_icon_crops_to_square_without_upscaling() {
        let source = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            2000,
            800,
            image::Rgb([10, 20, 30]),
        ));
        let processed = process_link_icon(&encode_png(source)).expect("process");
        let decoded = image::load_from_memory(&processed).expect("decode processed");
        assert_eq!(decoded.width(), 800);
        assert_eq!(decoded.height(), 800);
    }

    #[test]
    fn link_icon_small_square_keeps_dimensions() {
        let source = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            64,
            64,
            image::Rgb([1, 2, 3]),
        ));
        let processed = process_link_icon(&encode_png(source)).expect("process");
        let decoded = image::load_from_memory(&processed).expect("decode processed");
        assert_eq!(decoded.width(), 64);
        assert_eq!(decoded.height(), 64);
    }

    #[test]
    fn link_icon_rejects_non_image_bytes() {
        assert!(process_link_icon(b"definitely not an image").is_err());
    }

    #[tokio::test]
    async fn stores_loads_and_deletes_through_object_storage() {
        let base = std::env::temp_dir().join(format!(
            "social-link-media-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        let storage = Arc::new(LocalStorage::new(base.clone()).await.expect("storage"));
        let media = MediaService::new(storage, 1024, "/uploads".to_string()).expect("media");

        let public_path = media
            .store(
                Bytes::from_static(b"image"),
                Some("image/png"),
                None,
                MediaKind::Image,
            )
            .await
            .expect("store");
        let key = public_path.strip_prefix("/uploads/").expect("managed path");
        assert!(media.load(key).await.expect("load").is_some());
        assert!(media.delete(&public_path).await.expect("delete"));
        assert!(media.load(key).await.expect("load after delete").is_none());
        assert!(
            !media
                .delete("https://cdn.example.com/avatar.png")
                .await
                .expect("ignore external")
        );

        tokio::fs::remove_dir_all(base).await.expect("cleanup");
    }
}
