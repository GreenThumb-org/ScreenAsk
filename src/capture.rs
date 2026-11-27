use crate::win_util::MonitorRect;
use chrono::Local;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, RgbImage};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

#[cfg(windows)]
pub fn capture_monitor(rect: MonitorRect) -> Result<RgbImage, String> {
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        SRCCOPY,
    };

    let width = rect.width();
    let height = rect.height();

    unsafe {
        let screen_dc = GetDC(std::ptr::null_mut());
        if screen_dc.is_null() {
            return Err("GetDC failed".to_string());
        }

        let mem_dc = CreateCompatibleDC(screen_dc);
        if mem_dc.is_null() {
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err("CreateCompatibleDC failed".to_string());
        }

        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_null() {
            DeleteDC(mem_dc);
            ReleaseDC(std::ptr::null_mut(), screen_dc);
            return Err("CreateCompatibleBitmap failed".to_string());
        }

        let old_obj = SelectObject(mem_dc, bitmap);
        let blt_ok = BitBlt(
            mem_dc,
            0,
            0,
            width,
            height,
            screen_dc,
            rect.left,
            rect.top,
            SRCCOPY,
        );

        let result = if blt_ok == 0 {
            Err("BitBlt screen capture failed".to_string())
        } else {
            let mut bmi: BITMAPINFO = std::mem::zeroed();
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -height; // top-down DIB
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;

            let pixel_count = (width as usize) * (height as usize);
            let mut bgra = vec![0u8; pixel_count * 4];

            let lines = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                bgra.as_mut_ptr() as *mut _,
                &mut bmi,
                DIB_RGB_COLORS,
            );

            if lines == 0 {
                Err("GetDIBits failed".to_string())
            } else {
                let mut rgb = Vec::with_capacity(pixel_count * 3);
                for chunk in bgra.chunks_exact(4) {
                    rgb.push(chunk[2]); // R
                    rgb.push(chunk[1]); // G
                    rgb.push(chunk[0]); // B
                }
                RgbImage::from_raw(width as u32, height as u32, rgb)
                    .ok_or_else(|| "Failed to construct RGB image".to_string())
            }
        };

        SelectObject(mem_dc, old_obj);
        DeleteObject(bitmap);
        DeleteDC(mem_dc);
        ReleaseDC(std::ptr::null_mut(), screen_dc);

        result
    }
}

#[cfg(not(windows))]
pub fn capture_monitor(_rect: MonitorRect) -> Result<RgbImage, String> {
    Err("Screen capture is only supported on Windows".to_string())
}

pub fn save_screenshot_png(img: &RgbImage, folder: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(folder)
        .map_err(|e| format!("Failed to create screenshot folder: {e}"))?;
    let filename = format!("screenshot_{}.png", Local::now().format("%Y%m%d_%H%M%S"));
    let path = folder.join(filename);
    img.save(&path)
        .map_err(|e| format!("Failed to save screenshot PNG: {e}"))?;
    Ok(path)
}

pub fn encode_for_gemini(img: &RgbImage) -> Result<Vec<u8>, String> {
    const MAX_DIM: u32 = 2560;
    let dyn_img = if img.width() > MAX_DIM || img.height() > MAX_DIM {
        DynamicImage::ImageRgb8(img.clone()).resize(MAX_DIM, MAX_DIM, FilterType::Triangle)
    } else {
        DynamicImage::ImageRgb8(img.clone())
    };

    let mut out = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut out, 90);
    encoder
        .encode_image(&dyn_img)
        .map_err(|e| format!("Failed to encode JPEG: {e}"))?;
    Ok(out.into_inner())
}
