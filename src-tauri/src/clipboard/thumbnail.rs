#[cfg(not(target_os = "macos"))]
use image::ImageFormat;
use sha2::{Digest, Sha256};
#[cfg(not(target_os = "macos"))]
use std::io::Cursor;
use std::{path::Path, time::UNIX_EPOCH};

// The list displays 40 × 28 CSS pixels. Keep enough detail for a 3× display.
const WIDTH: u32 = 120;
const HEIGHT: u32 = 84;

pub fn read_thumbnail(path: &Path) -> Result<Vec<u8>, String> {
    let cache = dirs::home_dir()
        .ok_or("Cannot locate home directory")?
        .join(".clipbrain/cache/thumbnails");
    thumbnail_png(path, &cache)
}

fn thumbnail_png(path: &Path, cache: &Path) -> Result<Vec<u8>, String> {
    let source = path
        .canonicalize()
        .map_err(|e| format!("Cannot locate image: {e}"))?;
    let metadata = source
        .metadata()
        .map_err(|e| format!("Cannot inspect image: {e}"))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|time| time.as_nanos())
        .unwrap_or_default();
    let key = format!(
        "v1:{}:{}:{modified}:{WIDTH}:{HEIGHT}",
        source.display(),
        metadata.len()
    );
    let cached_path = cache.join(format!("{:x}.png", Sha256::digest(key.as_bytes())));
    if let Ok(data) = std::fs::read(&cached_path) {
        if let Ok(image) = image::load_from_memory(&data) {
            if image.width() <= WIDTH && image.height() <= HEIGHT {
                return Ok(data);
            }
        }
    }

    let data = create_thumbnail_png(&source)?;
    // A failed disk cache must not prevent the preview from being displayed.
    if std::fs::create_dir_all(cache).is_ok() {
        let temporary = cached_path.with_extension(format!("{}.tmp", std::process::id()));
        if std::fs::write(&temporary, &data).is_ok() {
            let _ = std::fs::rename(&temporary, &cached_path);
        }
    }
    Ok(data)
}

#[cfg(not(target_os = "macos"))]
fn create_thumbnail_png(source: &Path) -> Result<Vec<u8>, String> {
    let original = image::ImageReader::open(&source)
        .map_err(|e| format!("Cannot open image: {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("Cannot identify image: {e}"))?
        .decode()
        .map_err(|e| format!("Cannot decode image: {e}"))?;
    let thumbnail = original.thumbnail(WIDTH.min(original.width()), HEIGHT.min(original.height()));
    drop(original);
    let mut output = Cursor::new(Vec::new());
    thumbnail
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|e| format!("Cannot encode thumbnail: {e}"))?;
    Ok(output.into_inner())
}

// ImageIO downsampling avoids retaining full-resolution Rust decoding buffers.
#[cfg(target_os = "macos")]
fn create_thumbnail_png(path: &Path) -> Result<Vec<u8>, String> {
    use core_foundation::{
        base::{CFType, CFTypeRef, TCFType},
        boolean::CFBoolean,
        data::{CFData, CFDataCreateMutable, CFMutableDataRef},
        dictionary::{CFDictionary, CFDictionaryRef},
        number::CFNumber,
        string::{CFString, CFStringRef},
        url::{CFURLRef, CFURL},
    };
    #[link(name = "ImageIO", kind = "framework")]
    extern "C" {
        static kCGImageSourceShouldCache: CFStringRef;
        static kCGImageSourceCreateThumbnailFromImageAlways: CFStringRef;
        static kCGImageSourceThumbnailMaxPixelSize: CFStringRef;
        fn CGImageSourceCreateWithURL(url: CFURLRef, options: CFDictionaryRef) -> CFTypeRef;
        fn CGImageSourceCreateThumbnailAtIndex(
            source: CFTypeRef,
            index: usize,
            options: CFDictionaryRef,
        ) -> CFTypeRef;
        fn CGImageDestinationCreateWithData(
            data: CFMutableDataRef,
            kind: CFStringRef,
            count: usize,
            options: CFDictionaryRef,
        ) -> CFTypeRef;
        fn CGImageDestinationAddImage(
            destination: CFTypeRef,
            image: CFTypeRef,
            properties: CFDictionaryRef,
        );
        fn CGImageDestinationFinalize(destination: CFTypeRef) -> u8;
    }
    // PNG header inspection does not decode its full pixel buffer.
    let (width, height) = image::ImageReader::open(path)
        .map_err(|e| format!("Cannot open image: {e}"))?
        .with_guessed_format()
        .map_err(|e| format!("Cannot identify image: {e}"))?
        .into_dimensions()
        .map_err(|e| format!("Cannot inspect dimensions: {e}"))?;
    if width == 0 || height == 0 {
        return Err("Image has no pixels".into());
    }
    let scale = (WIDTH as f64 / width as f64)
        .min(HEIGHT as f64 / height as f64)
        .min(1.0);
    let maximum = ((width.max(height) as f64 * scale).floor() as i64).max(1);
    let url = CFURL::from_path(path, false).ok_or("Cannot create image URL")?;

    unsafe {
        let cache_key = CFString::wrap_under_get_rule(kCGImageSourceShouldCache);
        let options = CFDictionary::from_CFType_pairs(&[(
            cache_key.clone(),
            CFBoolean::false_value().as_CFType(),
        )]);
        let source =
            CGImageSourceCreateWithURL(url.as_concrete_TypeRef(), options.as_concrete_TypeRef());
        if source.is_null() {
            return Err("Cannot open image source".into());
        }
        let source = CFType::wrap_under_create_rule(source);
        let options = CFDictionary::from_CFType_pairs(&[
            (cache_key, CFBoolean::false_value().as_CFType()),
            (
                CFString::wrap_under_get_rule(kCGImageSourceCreateThumbnailFromImageAlways),
                CFBoolean::true_value().as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kCGImageSourceThumbnailMaxPixelSize),
                CFNumber::from(maximum).as_CFType(),
            ),
        ]);
        let thumbnail = CGImageSourceCreateThumbnailAtIndex(
            source.as_CFTypeRef(),
            0,
            options.as_concrete_TypeRef(),
        );
        if thumbnail.is_null() {
            return Err("Cannot create thumbnail".into());
        }
        let thumbnail = CFType::wrap_under_create_rule(thumbnail);
        let buffer = CFDataCreateMutable(std::ptr::null(), 0);
        if buffer.is_null() {
            return Err("Cannot allocate thumbnail data".into());
        }
        let data = CFData::wrap_under_create_rule(buffer);
        let png = CFString::new("public.png");
        let destination = CGImageDestinationCreateWithData(
            buffer,
            png.as_concrete_TypeRef(),
            1,
            std::ptr::null(),
        );
        if destination.is_null() {
            return Err("Cannot create thumbnail encoder".into());
        }
        let destination = CFType::wrap_under_create_rule(destination);
        CGImageDestinationAddImage(
            destination.as_CFTypeRef(),
            thumbnail.as_CFTypeRef(),
            std::ptr::null(),
        );
        if CGImageDestinationFinalize(destination.as_CFTypeRef()) == 0 {
            return Err("Cannot encode thumbnail".into());
        }
        Ok(data.bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_preserves_original_and_invalidates_changed_source() {
        let dir = std::env::temp_dir().join(format!(
            "clipbrain-thumbnail-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("original.png");
        let cache = dir.join("cache");
        let original = image::RgbaImage::from_fn(1600, 1200, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x * y) % 256) as u8, 255])
        });
        original.save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        let bytes = thumbnail_png(&path, &cache).unwrap();
        let thumb = image::load_from_memory(&bytes).unwrap();
        assert_eq!((thumb.width(), thumb.height()), (112, 84));
        assert!(bytes.len() < before.len() / 10);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(thumbnail_png(&path, &cache).unwrap(), bytes);

        image::RgbaImage::from_pixel(40, 28, image::Rgba([20, 30, 40, 255]))
            .save(&path)
            .unwrap();
        let changed = thumbnail_png(&path, &cache).unwrap();
        let small = image::load_from_memory(&changed).unwrap();
        assert_eq!((small.width(), small.height()), (40, 28));
        assert_ne!(changed, bytes);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
