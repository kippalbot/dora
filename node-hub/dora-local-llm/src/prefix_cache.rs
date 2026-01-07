//! Prefix-based KV cache for multi-tenant LLM serving
//!
//! Implements RadixAttention-like prefix caching where KV caches are keyed
//! by the hash of message prefixes, enabling automatic cache sharing across
//! requests with common prefixes (e.g., same system prompt).

use std::collections::HashMap;
use std::time::Instant;

use outfox_openai::spec::ChatCompletionRequestMessage;
use sha2::{Digest, Sha256};

/// Entry in the prefix cache
pub struct CacheEntry {
    /// Hash of the message prefix (kept for debugging)
    #[allow(dead_code)]
    prefix_hash: String,
    /// Number of tokens cached
    token_count: usize,
    /// Last access time for LRU eviction
    last_access: Instant,
    // TODO: Add actual KV cache data from mlx-rs
    // kv_cache: mlx_rs::KvCache,
}

/// Prefix-based KV cache with LRU eviction
pub struct PrefixCache {
    /// Cache entries keyed by prefix hash
    entries: HashMap<String, CacheEntry>,
    /// Maximum number of cache entries
    max_entries: usize,
    /// Statistics
    hits: u64,
    misses: u64,
}

impl PrefixCache {
    /// Create a new prefix cache with given capacity
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            hits: 0,
            misses: 0,
        }
    }

    /// Extract role name from a ChatCompletionRequestMessage enum variant
    fn get_role(msg: &ChatCompletionRequestMessage) -> &'static str {
        match msg {
            ChatCompletionRequestMessage::System(_) => "system",
            ChatCompletionRequestMessage::User(_) => "user",
            ChatCompletionRequestMessage::Assistant(_) => "assistant",
            ChatCompletionRequestMessage::Tool(_) => "tool",
            ChatCompletionRequestMessage::Function(_) => "function",
            ChatCompletionRequestMessage::Developer(_) => "developer",
        }
    }

    /// Compute a hash of the message prefix for cache lookup
    pub fn compute_prefix_hash(&self, messages: &[ChatCompletionRequestMessage]) -> String {
        let mut hasher = Sha256::new();

        for msg in messages {
            // Hash role and content
            let role = Self::get_role(msg);
            hasher.update(role.as_bytes());
            for text in msg.to_texts() {
                hasher.update(text.as_bytes());
            }
            hasher.update(b"|"); // Separator between messages
        }

        hex::encode(hasher.finalize())
    }

    /// Get cached prefix or create a new entry
    ///
    /// Returns (number of cached tokens, KV cache reference)
    pub fn get_or_create(&mut self, prefix_hash: &str, _total_tokens: usize) -> (usize, Option<()>) {
        // Try to find the longest matching prefix
        // For now, we do exact prefix matching
        // TODO: Implement radix tree for prefix matching

        if let Some(entry) = self.entries.get_mut(prefix_hash) {
            // Cache hit!
            entry.last_access = Instant::now();
            self.hits += 1;
            tracing::debug!(
                "Cache HIT for prefix {}: {} tokens cached",
                &prefix_hash[..16],
                entry.token_count
            );
            return (entry.token_count, Some(()));
        }

        // Cache miss - will need to create entry
        self.misses += 1;
        tracing::debug!("Cache MISS for prefix {}", &prefix_hash[..16]);

        // Evict if at capacity
        if self.entries.len() >= self.max_entries {
            self.evict_lru();
        }

        (0, None)
    }

    /// Update cache with new prefix entry
    pub fn update(&mut self, prefix_hash: &str, token_count: usize) {
        // Only update if not already cached or if we have more tokens
        let should_update = self
            .entries
            .get(prefix_hash)
            .map(|e| e.token_count < token_count)
            .unwrap_or(true);

        if should_update {
            self.entries.insert(
                prefix_hash.to_string(),
                CacheEntry {
                    prefix_hash: prefix_hash.to_string(),
                    token_count,
                    last_access: Instant::now(),
                },
            );
            tracing::debug!(
                "Cache UPDATE for prefix {}: {} tokens",
                &prefix_hash[..16],
                token_count
            );
        }
    }

    /// Evict least recently used cache entry
    fn evict_lru(&mut self) {
        if let Some((oldest_key, _)) = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(k, v)| (k.clone(), v.last_access))
        {
            tracing::debug!("Evicting LRU cache entry: {}", &oldest_key[..16]);
            self.entries.remove(&oldest_key);
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> serde_json::Value {
        let total = self.hits + self.misses;
        let hit_rate = if total > 0 {
            (self.hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        serde_json::json!({
            "entries": self.entries.len(),
            "max_entries": self.max_entries,
            "hits": self.hits,
            "misses": self.misses,
            "hit_rate_percent": format!("{:.1}", hit_rate),
            "cached_tokens": self.entries.values().map(|e| e.token_count).sum::<usize>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_hash() {
        let cache = PrefixCache::new(10);

        // Same messages should produce same hash
        let msgs1 = vec![];
        let msgs2 = vec![];
        assert_eq!(
            cache.compute_prefix_hash(&msgs1),
            cache.compute_prefix_hash(&msgs2)
        );
    }

    #[test]
    fn test_cache_hit_miss() {
        let mut cache = PrefixCache::new(10);

        let hash = "test_hash_123".to_string();

        // First access should be a miss
        let (cached, _) = cache.get_or_create(&hash, 100);
        assert_eq!(cached, 0);
        assert_eq!(cache.misses, 1);
        assert_eq!(cache.hits, 0);

        // Update cache
        cache.update(&hash, 100);

        // Second access should be a hit
        let (cached, _) = cache.get_or_create(&hash, 100);
        assert_eq!(cached, 100);
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = PrefixCache::new(2);

        // Fill cache
        cache.update("hash1", 50);
        std::thread::sleep(std::time::Duration::from_millis(10));
        cache.update("hash2", 60);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Access hash1 to make it more recent
        cache.get_or_create("hash1", 50);

        // Add new entry, should evict hash2 (LRU)
        cache.get_or_create("hash3", 70);
        cache.update("hash3", 70);

        // hash1 should still exist, hash2 should be evicted
        assert!(cache.entries.contains_key("hash1"));
        assert!(!cache.entries.contains_key("hash2"));
        assert!(cache.entries.contains_key("hash3"));
    }
}
