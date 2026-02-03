//! PDF preview using pdftoppm

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ratatui::{layout::Rect, widgets::Block, widgets::Borders, Frame};
use ratatui_image::{picker::Picker, FontSize, Resize, StatefulImage};
use tempfile::NamedTempFile;

use super::common::get_border_style;
use super::image::{calculate_centered_image_area, ImagePreview};

/// Cached pdftoppm path detection
static PDFTOPPM_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Cached pdfinfo path detection
static PDFINFO_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Find pdftoppm executable path (lazy detection with caching)
pub fn find_pdftoppm() -> Option<&'static PathBuf> {
    PDFTOPPM_PATH
        .get_or_init(|| {
            let candidates = [
                "/usr/bin/pdftoppm",
                "/usr/local/bin/pdftoppm",
                "/opt/homebrew/bin/pdftoppm",
            ];
            for path in candidates {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
            // fallback: which pdftoppm
            std::process::Command::new("which")
                .arg("pdftoppm")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        })
        .as_ref()
}

/// Find pdfinfo executable path (lazy detection with caching)
fn find_pdfinfo() -> Option<&'static PathBuf> {
    PDFINFO_PATH
        .get_or_init(|| {
            let candidates = [
                "/usr/bin/pdfinfo",
                "/usr/local/bin/pdfinfo",
                "/opt/homebrew/bin/pdfinfo",
            ];
            for path in candidates {
                let p = PathBuf::from(path);
                if p.exists() {
                    return Some(p);
                }
            }
            // fallback: which pdfinfo
            std::process::Command::new("which")
                .arg("pdfinfo")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        })
        .as_ref()
}

/// Get total page count from PDF using pdfinfo
fn get_pdf_page_count(path: &Path) -> anyhow::Result<usize> {
    let pdfinfo = find_pdfinfo().ok_or_else(|| anyhow::anyhow!("pdfinfo not found"))?;

    let output = std::process::Command::new(pdfinfo).arg(path).output()?;

    if !output.status.success() {
        anyhow::bail!("pdfinfo failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("Pages:") {
            let count_str = line.trim_start_matches("Pages:").trim();
            return count_str
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("Failed to parse page count"));
        }
    }

    anyhow::bail!("Pages not found in pdfinfo output")
}

/// PDF preview content
pub struct PdfPreview {
    /// Original PDF file path
    pub path: PathBuf,
    /// Current page number (1-indexed)
    pub current_page: usize,
    /// Total number of pages
    pub total_pages: usize,
    /// Rendered image preview
    pub image: ImagePreview,
    /// Temporary file holding the rendered page image (auto-cleanup on drop)
    _temp_file: NamedTempFile,
}

impl PdfPreview {
    /// Load PDF preview for a specific page
    pub fn load(path: &Path, page: usize, picker: &mut Picker) -> anyhow::Result<Self> {
        let pdftoppm = find_pdftoppm()
            .ok_or_else(|| anyhow::anyhow!("PDF preview requires pdftoppm (poppler-utils)"))?;

        // Get total page count
        let total_pages = get_pdf_page_count(path)?;

        // Clamp page to valid range
        let page = page.clamp(1, total_pages);

        // Create temporary file for the rendered image
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path();

        // Get the base path without extension for pdftoppm output
        let temp_base = temp_path.with_extension("");

        // Run pdftoppm to render the page
        // pdftoppm -png -f <page> -l <page> -singlefile -r 150 input.pdf output_prefix
        let status = std::process::Command::new(pdftoppm)
            .arg("-png")
            .arg("-f")
            .arg(page.to_string())
            .arg("-l")
            .arg(page.to_string())
            .arg("-singlefile")
            .arg("-r")
            .arg("150")
            .arg(path)
            .arg(&temp_base)
            .status()?;

        if !status.success() {
            anyhow::bail!("pdftoppm failed to render page");
        }

        // pdftoppm creates output_prefix.png
        let output_path = temp_base.with_extension("png");

        if !output_path.exists() {
            anyhow::bail!("pdftoppm did not create output image");
        }

        // Load the rendered image
        let image = ImagePreview::load(&output_path, picker)?;

        // Clean up the output file (we'll store it in the temp_file for auto-cleanup)
        // Actually, we need to keep the output file, so let's rename it to temp_path
        std::fs::rename(&output_path, temp_path)?;

        Ok(Self {
            path: path.to_path_buf(),
            current_page: page,
            total_pages,
            image,
            _temp_file: temp_file,
        })
    }

    /// Navigate to a different page
    pub fn go_to_page(&mut self, page: usize, picker: &mut Picker) -> anyhow::Result<()> {
        let page = page.clamp(1, self.total_pages);

        if page == self.current_page {
            return Ok(());
        }

        // Create a new PdfPreview for the target page and update self
        let new_preview = PdfPreview::load(&self.path, page, picker)?;
        *self = new_preview;

        Ok(())
    }

    /// Go to the previous page
    pub fn prev_page(&mut self, picker: &mut Picker) -> anyhow::Result<()> {
        if self.current_page > 1 {
            self.go_to_page(self.current_page - 1, picker)
        } else {
            Ok(())
        }
    }

    /// Go to the next page
    pub fn next_page(&mut self, picker: &mut Picker) -> anyhow::Result<()> {
        if self.current_page < self.total_pages {
            self.go_to_page(self.current_page + 1, picker)
        } else {
            Ok(())
        }
    }
}

/// Render PDF preview
pub fn render_pdf_preview(
    frame: &mut Frame,
    pdf: &mut PdfPreview,
    area: Rect,
    title: &str,
    focused: bool,
    font_size: FontSize,
) {
    // Title with page info and navigation hint
    let full_title = format!(
        " {} ({}/{}) [/] prev/next ",
        title, pdf.current_page, pdf.total_pages
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title(full_title)
        .border_style(get_border_style(focused));

    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    // Calculate centered area for the image
    let centered_area =
        calculate_centered_image_area(inner_area, pdf.image.width, pdf.image.height, font_size);

    // Render image using ratatui-image's StatefulImage widget
    let image_widget = StatefulImage::default().resize(Resize::Scale(None));
    frame.render_stateful_widget(image_widget, centered_area, &mut pdf.image.protocol);
}

/// Check if a file is a PDF
pub fn is_pdf_file(path: &std::path::Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    matches!(ext.as_deref(), Some("pdf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pdf_file() {
        assert!(is_pdf_file(Path::new("document.pdf")));
        assert!(is_pdf_file(Path::new("DOCUMENT.PDF")));
        assert!(is_pdf_file(Path::new("Document.Pdf")));
        assert!(is_pdf_file(Path::new("/path/to/file.pdf")));
    }

    #[test]
    fn test_is_pdf_file_non_pdf() {
        assert!(!is_pdf_file(Path::new("document.txt")));
        assert!(!is_pdf_file(Path::new("document.doc")));
        assert!(!is_pdf_file(Path::new("document.docx")));
        assert!(!is_pdf_file(Path::new("image.png")));
        assert!(!is_pdf_file(Path::new("no_extension")));
    }

    #[test]
    fn test_find_pdftoppm_returns_consistent() {
        let result1 = find_pdftoppm();
        let result2 = find_pdftoppm();

        assert_eq!(result1.is_some(), result2.is_some());
        if let (Some(p1), Some(p2)) = (result1, result2) {
            assert_eq!(p1, p2);
        }
    }

    #[test]
    fn test_find_pdftoppm_path_exists_if_found() {
        if let Some(path) = find_pdftoppm() {
            assert!(path.exists(), "pdftoppm path should exist");
            assert!(
                path.to_string_lossy().contains("pdftoppm"),
                "Path should contain pdftoppm"
            );
        }
    }
}
