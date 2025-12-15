use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

/// Metadata about processed local documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDocMetadata {
    pub file_path: String,
    pub file_type: DocFileType,
    pub last_modified: String,
    pub processed_content: String,
}

/// Supported documentation file types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocFileType {
    Pdf,
    Markdown,
    Text,
}

/// Local documentation processor
pub struct LocalDocsProcessor;

impl LocalDocsProcessor {
    /// Extract text content from a PDF file
    pub fn extract_pdf_text(pdf_path: &Path) -> Result<String> {
        let bytes = fs::read(pdf_path)
            .with_context(|| format!("Failed to read PDF file: {:?}", pdf_path))?;

        let text = pdf_extract::extract_text_from_mem(&bytes)
            .with_context(|| format!("Failed to extract text from PDF: {:?}", pdf_path))?;

        Ok(text)
    }

    /// Read markdown file content
    pub fn read_markdown(md_path: &Path) -> Result<String> {
        fs::read_to_string(md_path)
            .with_context(|| format!("Failed to read Markdown file: {:?}", md_path))
    }

    /// Read text file content
    pub fn read_text(txt_path: &Path) -> Result<String> {
        fs::read_to_string(txt_path)
            .with_context(|| format!("Failed to read text file: {:?}", txt_path))
    }

    /// Process a documentation file and return its metadata
    pub fn process_file(file_path: &Path) -> Result<LocalDocMetadata> {
        let file_type = Self::detect_file_type(file_path)?;
        
        let processed_content = match file_type {
            DocFileType::Pdf => Self::extract_pdf_text(file_path)?,
            DocFileType::Markdown => Self::read_markdown(file_path)?,
            DocFileType::Text => Self::read_text(file_path)?,
        };

        let metadata = fs::metadata(file_path)?;
        let last_modified = format!("{:?}", metadata.modified()?);

        Ok(LocalDocMetadata {
            file_path: file_path.to_string_lossy().to_string(),
            file_type,
            last_modified,
            processed_content,
        })
    }

    /// Detect file type from extension
    fn detect_file_type(file_path: &Path) -> Result<DocFileType> {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| anyhow::anyhow!("No file extension found"))?;

        match extension.to_lowercase().as_str() {
            "pdf" => Ok(DocFileType::Pdf),
            "md" | "markdown" => Ok(DocFileType::Markdown),
            "txt" | "text" => Ok(DocFileType::Text),
            _ => Err(anyhow::anyhow!("Unsupported file type: {}", extension)),
        }
    }

    /// Format documentation content for LLM consumption
    pub fn format_for_llm(docs: &[LocalDocMetadata]) -> String {
        let mut formatted = String::new();
        formatted.push_str("# Local Technical Documentation\n\n");

        for (idx, doc) in docs.iter().enumerate() {
            formatted.push_str(&format!("\n---\n\n## Document {} - {}\n\n", idx + 1, 
                Path::new(&doc.file_path).file_name().unwrap_or_default().to_string_lossy()));
            
            formatted.push_str(&format!("**Source:** {}\n", doc.file_path));
            formatted.push_str(&format!("**Type:** {:?}\n", doc.file_type));
            formatted.push_str(&format!("**Last Modified:** {}\n\n", doc.last_modified));
            
            formatted.push_str("**Content:**\n\n");
            formatted.push_str(&doc.processed_content);
            formatted.push_str("\n\n");
        }

        formatted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_file_type() {
        assert_eq!(
            LocalDocsProcessor::detect_file_type(Path::new("doc.pdf")).unwrap(),
            DocFileType::Pdf
        );
        assert_eq!(
            LocalDocsProcessor::detect_file_type(Path::new("readme.md")).unwrap(),
            DocFileType::Markdown
        );
        assert_eq!(
            LocalDocsProcessor::detect_file_type(Path::new("notes.txt")).unwrap(),
            DocFileType::Text
        );
    }
}
