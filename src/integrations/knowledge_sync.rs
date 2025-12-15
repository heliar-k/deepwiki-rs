use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;
use chrono::{DateTime, Utc};

use crate::config::{Config, LocalDocsConfig};
use crate::integrations::local_docs::{LocalDocsProcessor, LocalDocMetadata};

/// Metadata about synced knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeMetadata {
    pub last_synced: DateTime<Utc>,
    pub local_docs: Vec<LocalDocMetadata>,
}

/// Syncs external knowledge sources to local cache
pub struct KnowledgeSyncer {
    config: Config,
}

impl KnowledgeSyncer {
    /// Create a new knowledge syncer
    pub fn new(config: Config) -> Result<Self> {
        Ok(Self { config })
    }

    /// Sync all configured knowledge sources
    pub async fn sync_all(&self) -> Result<()> {
        let target_lang = self.config.target_language.display_name();
        println!("🔄 Syncing external knowledge sources (target language: {})...", target_lang);

        let mut synced_any = false;

        if let Some(ref local_docs_config) = self.config.knowledge.local_docs {
            if local_docs_config.enabled {
                self.sync_local_docs(local_docs_config).await?;
                synced_any = true;
            } else {
                println!("ℹ️  Local docs integration is disabled");
            }
        }

        if !synced_any {
            println!("ℹ️  No knowledge sources are configured");
        }

        println!("✅ Knowledge sync completed");
        Ok(())
    }

    /// Sync local documentation files
    async fn sync_local_docs(&self, config: &LocalDocsConfig) -> Result<()> {
        println!("\n📄 Processing local documentation files...");

        let cache_dir = config
            .cache_dir
            .clone()
            .unwrap_or_else(|| {
                self.config
                    .internal_path
                    .join("knowledge")
                    .join("local_docs")
            });

        fs::create_dir_all(&cache_dir).context("Failed to create local docs cache directory")?;

        let mut all_docs = Vec::new();
        let mut processed_count = 0;

        // Process PDF files
        for pdf_path in &config.pdf_paths {
            let path = PathBuf::from(pdf_path);
            match LocalDocsProcessor::process_file(&path) {
                Ok(doc_meta) => {
                    println!("  ✓ Processed PDF: {}", pdf_path);
                    all_docs.push(doc_meta);
                    processed_count += 1;
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to process {}: {}", pdf_path, e);
                }
            }
        }

        // Process Markdown files
        for md_path in &config.markdown_paths {
            let path = PathBuf::from(md_path);
            match LocalDocsProcessor::process_file(&path) {
                Ok(doc_meta) => {
                    println!("  ✓ Processed Markdown: {}", md_path);
                    all_docs.push(doc_meta);
                    processed_count += 1;
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to process {}: {}", md_path, e);
                }
            }
        }

        // Process text files
        for txt_path in &config.text_paths {
            let path = PathBuf::from(txt_path);
            match LocalDocsProcessor::process_file(&path) {
                Ok(doc_meta) => {
                    println!("  ✓ Processed text file: {}", txt_path);
                    all_docs.push(doc_meta);
                    processed_count += 1;
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to process {}: {}", txt_path, e);
                }
            }
        }

        // Save metadata
        let metadata = KnowledgeMetadata {
            last_synced: Utc::now(),
            local_docs: all_docs,
        };

        let metadata_file = cache_dir.join("_metadata.json");
        let metadata_json =
            serde_json::to_string_pretty(&metadata).context("Failed to serialize metadata")?;
        fs::write(&metadata_file, metadata_json).context("Failed to write metadata")?;

        println!("✅ Processed {} local documentation files", processed_count);
        Ok(())
    }

    /// Check if knowledge needs to be re-synced
    pub fn should_sync(&self) -> Result<bool> {
        // Check if local docs need syncing
        if let Some(ref local_docs_config) = self.config.knowledge.local_docs {
            if !local_docs_config.enabled {
                return Ok(false);
            }

            let cache_dir = local_docs_config
                .cache_dir
                .clone()
                .unwrap_or_else(|| {
                    self.config
                        .internal_path
                        .join("knowledge")
                        .join("local_docs")
                });

            let metadata_file = cache_dir.join("_metadata.json");

            // Always sync local docs if cache doesn't exist or if watch_for_changes is true
            if !metadata_file.exists() {
                return Ok(true);
            }

            if local_docs_config.watch_for_changes {
                // Check if any source file has been modified since last sync
                let metadata_content = fs::read_to_string(&metadata_file)?;
                let metadata: KnowledgeMetadata = serde_json::from_str(&metadata_content)?;
                
                // Check if any source file has been modified
                for doc in &metadata.local_docs {
                    let source_path = PathBuf::from(&doc.file_path);
                    if source_path.exists() {
                        if let Ok(file_metadata) = fs::metadata(&source_path) {
                            if let Ok(modified) = file_metadata.modified() {
                                // Convert SystemTime to DateTime<Utc>
                                let modified_datetime: DateTime<Utc> = modified.into();
                                // Compare with cached modification time
                                if modified_datetime > metadata.last_synced {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
                return Ok(false);
            }
        }

        Ok(false)
    }

    /// Load all cached knowledge content
    pub fn load_cached_knowledge(&self) -> Result<Option<String>> {
        // Load local documentation content if available
        self.load_local_docs_cache()
    }
    
    /// Load cached local documentation content
    fn load_local_docs_cache(&self) -> Result<Option<String>> {
        let local_docs_config = match &self.config.knowledge.local_docs {
            Some(cfg) if cfg.enabled => cfg,
            _ => return Ok(None),
        };

        let cache_dir = local_docs_config
            .cache_dir
            .clone()
            .unwrap_or_else(|| {
                self.config
                    .internal_path
                    .join("knowledge")
                    .join("local_docs")
            });

        let metadata_file = cache_dir.join("_metadata.json");
        if !metadata_file.exists() {
            return Ok(None);
        }

        let metadata_content = fs::read_to_string(&metadata_file)?;
        let metadata: KnowledgeMetadata = serde_json::from_str(&metadata_content)?;

        if metadata.local_docs.is_empty() {
            return Ok(None);
        }

        let target_lang = self.config.target_language.display_name();
        let mut combined_content = String::new();
        combined_content.push_str(&format!("# Local Documentation ({})\n\n", target_lang));
        combined_content.push_str(&format!(
            "Last processed: {}\nTotal documents: {}\n\n",
            metadata.last_synced.format("%Y-%m-%d %H:%M:%S UTC"),
            metadata.local_docs.len()
        ));

        for doc in &metadata.local_docs {
            combined_content.push_str(&format!("\n---\n\n# {}\n\n", doc.file_path));
            combined_content.push_str(&format!("Type: {:?}\n", doc.file_type));
            combined_content.push_str(&format!("Last Modified: {}\n\n", doc.last_modified));
            combined_content.push_str(&doc.processed_content);
            combined_content.push_str("\n\n");
        }

        Ok(Some(combined_content))
    }
}
