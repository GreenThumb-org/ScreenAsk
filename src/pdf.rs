use std::collections::HashMap;
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

#[derive(Clone)]
struct CachedPdf {
    modified: Option<SystemTime>,
    size: u64,
    text: String,
}

static PDF_CACHE: OnceLock<Mutex<HashMap<PathBuf, CachedPdf>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<PathBuf, CachedPdf>> {
    PDF_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Returns sorted `.pdf` filenames currently inside `folder`.
pub fn list_pdf_names(folder: &Path) -> Vec<String> {
    list_pdf_paths(folder)
        .into_iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}

/// Copies a `.pdf` file into `folder` and returns its destination filename.
pub fn copy_pdf_into_folder(src: &Path, folder: &Path) -> Result<String, String> {
    let is_pdf = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    if !is_pdf {
        return Err("Only .pdf files can be added as reference documents.".to_string());
    }
    let file_name = src
        .file_name()
        .ok_or_else(|| "Invalid PDF file path.".to_string())?
        .to_string_lossy()
        .into_owned();

    fs::create_dir_all(folder).map_err(|e| format!("Failed to create folder: {e}"))?;
    let dest = folder.join(&file_name);

    // Skip copying onto itself if the file is already inside the target folder.
    let same_file = fs::canonicalize(src)
        .ok()
        .zip(fs::canonicalize(&dest).ok())
        .map(|(a, b)| a == b)
        .unwrap_or(false);

    if !same_file {
        fs::copy(src, &dest).map_err(|e| format!("Failed to copy {file_name}: {e}"))?;
    }
    Ok(file_name)
}

/// Loads text from `.pdf` files in `folder`.
/// If `selected_pdfs` is `Some(list)`, only PDFs whose filenames are in `list` are included.
pub fn load_pdf_sources(folder: &Path, selected_pdfs: Option<&[String]>) -> String {
    let paths: Vec<PathBuf> = list_pdf_paths(folder)
        .into_iter()
        .filter(|p| match selected_pdfs {
            None => true,
            Some(allowed) => p
                .file_name()
                .map(|n| {
                    let name = n.to_string_lossy();
                    allowed.iter().any(|a| a.eq_ignore_ascii_case(&name))
                })
                .unwrap_or(false),
        })
        .collect();

    if paths.is_empty() {
        return String::new();
    }

    let mut sections = Vec::new();

    for path in paths {
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        let modified = meta.modified().ok();
        let size = meta.len();

        let cached_text = cache().lock().ok().and_then(|map| {
            map.get(&path)
                .filter(|c| c.modified == modified && c.size == size)
                .map(|c| c.text.clone())
        });

        let extracted = match cached_text {
            Some(t) => t,
            None => {
                let p_clone = path.clone();
                let res = panic::catch_unwind(move || pdf_extract::extract_text(&p_clone));
                let text = match res {
                    Ok(Ok(t)) => t.trim().to_string(),
                    Ok(Err(e)) => {
                        eprintln!("Failed to extract PDF {}: {e}", path.display());
                        String::new()
                    }
                    Err(_) => {
                        eprintln!("PDF extractor panicked on {}", path.display());
                        String::new()
                    }
                };
                if let Ok(mut map) = cache().lock() {
                    map.insert(
                        path.clone(),
                        CachedPdf {
                            modified,
                            size,
                            text: text.clone(),
                        },
                    );
                }
                text
            }
        };

        if !extracted.is_empty() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "document.pdf".to_string());
            sections.push(format!("--- Source: {name} ---\n{extracted}"));
        }
    }

    sections.join("\n\n")
}

fn list_pdf_paths(folder: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|entry| entry.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false)
        })
        .collect();
    paths.sort();
    paths
}
