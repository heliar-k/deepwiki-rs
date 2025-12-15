use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::{
    cache::CacheManager, 
    config::Config, 
    llm::client::LLMClient, 
    memory::Memory,
};

#[derive(Clone)]
pub struct GeneratorContext {
    /// LLM client for communicating with AI.
    pub llm_client: LLMClient,
    /// Configuration
    pub config: Config,
    /// Cache manager
    pub cache_manager: Arc<RwLock<CacheManager>>,
    /// Generator memory
    pub memory: Arc<RwLock<Memory>>,
}

impl GeneratorContext {
    /// Store data to Memory
    pub async fn store_to_memory<T>(&self, scope: &str, key: &str, data: T) -> Result<()>
    where
        T: Serialize + Send + Sync,
    {
        let mut memory = self.memory.write().await;
        memory.store(scope, key, data)
    }

    /// Get data from Memory
    pub async fn get_from_memory<T>(&self, scope: &str, key: &str) -> Option<T>
    where
        T: for<'a> Deserialize<'a> + Send + Sync,
    {
        let mut memory = self.memory.write().await;
        memory.get(scope, key)
    }

    /// Check if data exists in Memory
    pub async fn has_memory_data(&self, scope: &str, key: &str) -> bool {
        let memory = self.memory.read().await;
        memory.has_data(scope, key)
    }

    /// Get all data keys within a scope
    pub async fn list_memory_keys(&self, scope: &str) -> Vec<String> {
        let memory = self.memory.read().await;
        memory.list_keys(scope)
    }

    /// Get Memory usage statistics
    pub async fn get_memory_stats(&self) -> HashMap<String, usize> {
        let memory = self.memory.read().await;
        memory.get_usage_stats()
    }

    /// Load external knowledge (Confluence, Jira, etc.)
    pub async fn load_external_knowledge(&self) -> Option<String> {
        use crate::integrations::KnowledgeSyncer;
        
        match KnowledgeSyncer::new(self.config.clone()) {
            Ok(syncer) => {
                match syncer.load_cached_knowledge() {
                    Ok(Some(knowledge)) => {
                        let lang = self.config.target_language.display_name();
                        println!("📚 Loaded external knowledge base ({})", lang);
                        Some(knowledge)
                    }
                    Ok(None) => {
                        let lang = self.config.target_language.display_name();
                        println!("ℹ️  No external knowledge cache found for language: {}", lang);
                        None
                    }
                    Err(e) => {
                        eprintln!("⚠️  Failed to load external knowledge: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  Failed to create knowledge syncer: {}", e);
                None
            }
        }
    }
}
