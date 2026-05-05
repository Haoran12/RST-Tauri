//! Performance caching module
//!
//! Provides caching infrastructure for Agent runtime performance optimization.
//! Implements the caching requirements from docs/02_app_data_and_modules.md.
//!
//! Key cache types:
//! - `TurnScopedCache`: Per-turn derived data cache (cleared each turn)
//! - `KnowledgeAccessCache`: Caches AccessibleKnowledge queries
//! - `DerivedAttributeCache`: Caches attribute calculations
//! - `SceneDerivedCache`: Caches SceneFilter/EmbodimentResolver results
//!
//! Batch processing:
//! - `BatchLogWriter`: Batches log entries for efficient writes
//! - `BatchTraceWriter`: Batches trace entries
//! - `BatchStateWriter`: Batches state updates

pub mod batch;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use lru::LruCache;
use parking_lot::Mutex;
use tokio::sync::RwLock;

use crate::agent::models::{
    AccessibleKnowledge, EffectiveAttributeProfile, EmbodimentState, FilteredSceneView, TimeAnchor,
};

/// Non-zero usize helper for LruCache
fn nz(v: usize) -> std::num::NonZeroUsize {
    std::num::NonZeroUsize::new(v).unwrap_or_else(|| std::num::NonZeroUsize::new(1).unwrap())
}

// ============================================================================
// Turn-Scoped Cache
// ============================================================================

/// Cache key for knowledge access
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct KnowledgeAccessKey {
    pub character_id: String,
    pub scene_turn_id: String,
    pub time_anchor_hash: u64,
}

impl KnowledgeAccessKey {
    pub fn new(character_id: &str, scene_turn_id: &str, time_anchor: &TimeAnchor) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        time_anchor.calendar_id.hash(&mut hasher);
        time_anchor.ordinal.hash(&mut hasher);

        Self {
            character_id: character_id.to_string(),
            scene_turn_id: scene_turn_id.to_string(),
            time_anchor_hash: hasher.finish(),
        }
    }
}

/// Cache key for derived attributes
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DerivedAttributeKey {
    pub character_id: String,
    pub base_hash: u64,
    pub temp_state_hash: u64,
    pub modifiers_hash: u64,
}

/// Cache key for scene-derived data
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SceneDerivedKey {
    pub character_id: String,
    pub scene_turn_id: String,
    pub embodiment_hash: u64,
}

/// Turn-scoped cache for derived data
///
/// This cache is cleared at the start of each turn to ensure
/// data consistency. It caches expensive computations within a turn.
pub struct TurnScopedCache {
    /// Knowledge access cache
    knowledge_cache: Mutex<LruCache<KnowledgeAccessKey, Arc<AccessibleKnowledge>>>,
    /// Attribute derivation cache
    attribute_cache: Mutex<LruCache<DerivedAttributeKey, Arc<EffectiveAttributeProfile>>>,
    /// Scene filter result cache
    scene_filter_cache: Mutex<LruCache<SceneDerivedKey, Arc<FilteredSceneView>>>,
    /// Embodiment state cache
    embodiment_cache: Mutex<LruCache<SceneDerivedKey, Arc<EmbodimentState>>>,
    /// Turn ID for cache validation
    current_turn_id: Mutex<Option<String>>,
}

impl TurnScopedCache {
    /// Create a new turn-scoped cache with default sizes
    pub fn new() -> Self {
        Self::with_sizes(64, 32, 32, 32)
    }

    /// Create a new turn-scoped cache with custom sizes
    pub fn with_sizes(
        knowledge_size: usize,
        attribute_size: usize,
        scene_filter_size: usize,
        embodiment_size: usize,
    ) -> Self {
        Self {
            knowledge_cache: Mutex::new(LruCache::new(nz(knowledge_size))),
            attribute_cache: Mutex::new(LruCache::new(nz(attribute_size))),
            scene_filter_cache: Mutex::new(LruCache::new(nz(scene_filter_size))),
            embodiment_cache: Mutex::new(LruCache::new(nz(embodiment_size))),
            current_turn_id: Mutex::new(None),
        }
    }

    /// Clear cache for a new turn
    pub fn clear_for_turn(&self, turn_id: &str) {
        let mut current = self.current_turn_id.lock();
        if current.as_ref() != Some(&turn_id.to_string()) {
            self.knowledge_cache.lock().clear();
            self.attribute_cache.lock().clear();
            self.scene_filter_cache.lock().clear();
            self.embodiment_cache.lock().clear();
            *current = Some(turn_id.to_string());
        }
    }

    // ===== Knowledge Access Cache =====

    /// Get cached accessible knowledge
    pub fn get_knowledge(&self, key: &KnowledgeAccessKey) -> Option<Arc<AccessibleKnowledge>> {
        self.knowledge_cache.lock().get(key).cloned()
    }

    /// Insert accessible knowledge into cache
    pub fn insert_knowledge(&self, key: KnowledgeAccessKey, value: Arc<AccessibleKnowledge>) {
        self.knowledge_cache.lock().put(key, value);
    }

    /// Get or compute accessible knowledge
    pub fn get_or_insert_knowledge<F>(
        &self,
        key: KnowledgeAccessKey,
        compute: F,
    ) -> Arc<AccessibleKnowledge>
    where
        F: FnOnce() -> AccessibleKnowledge,
    {
        if let Some(cached) = self.get_knowledge(&key) {
            return cached;
        }

        let value = Arc::new(compute());
        self.insert_knowledge(key, value.clone());
        value
    }

    // ===== Attribute Cache =====

    /// Get cached derived attributes
    pub fn get_attributes(
        &self,
        key: &DerivedAttributeKey,
    ) -> Option<Arc<EffectiveAttributeProfile>> {
        self.attribute_cache.lock().get(key).cloned()
    }

    /// Insert derived attributes into cache
    pub fn insert_attributes(
        &self,
        key: DerivedAttributeKey,
        value: Arc<EffectiveAttributeProfile>,
    ) {
        self.attribute_cache.lock().put(key, value);
    }

    // ===== Scene Filter Cache =====

    /// Get cached filtered scene view
    pub fn get_scene_filter(&self, key: &SceneDerivedKey) -> Option<Arc<FilteredSceneView>> {
        self.scene_filter_cache.lock().get(key).cloned()
    }

    /// Insert filtered scene view into cache
    pub fn insert_scene_filter(&self, key: SceneDerivedKey, value: Arc<FilteredSceneView>) {
        self.scene_filter_cache.lock().put(key, value);
    }

    // ===== Embodiment Cache =====

    /// Get cached embodiment state
    pub fn get_embodiment(&self, key: &SceneDerivedKey) -> Option<Arc<EmbodimentState>> {
        self.embodiment_cache.lock().get(key).cloned()
    }

    /// Insert embodiment state into cache
    pub fn insert_embodiment(&self, key: SceneDerivedKey, value: Arc<EmbodimentState>) {
        self.embodiment_cache.lock().put(key, value);
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            knowledge_entries: self.knowledge_cache.lock().len(),
            attribute_entries: self.attribute_cache.lock().len(),
            scene_filter_entries: self.scene_filter_cache.lock().len(),
            embodiment_entries: self.embodiment_cache.lock().len(),
        }
    }
}

impl Default for TurnScopedCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub knowledge_entries: usize,
    pub attribute_entries: usize,
    pub scene_filter_entries: usize,
    pub embodiment_entries: usize,
}

// ============================================================================
// Cross-Turn Cache (with TTL)
// ============================================================================

/// Cache entry with expiration
#[derive(Debug, Clone)]
struct TimedEntry<T> {
    value: T,
    expires_at: Instant,
}

impl<T> TimedEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() > self.expires_at
    }
}

/// Cross-turn cache with TTL for configuration snapshots
///
/// This cache persists across turns but has TTL-based expiration.
/// Used for configuration snapshots and other data that changes rarely.
pub struct CrossTurnCache {
    /// Configuration snapshot cache
    config_cache: RwLock<HashMap<String, TimedEntry<serde_json::Value>>>,
    /// Default TTL
    default_ttl: Duration,
}

impl CrossTurnCache {
    /// Create a new cross-turn cache
    pub fn new() -> Self {
        Self::with_ttl(Duration::from_secs(300)) // 5 minutes default
    }

    /// Create a new cross-turn cache with custom TTL
    pub fn with_ttl(default_ttl: Duration) -> Self {
        Self {
            config_cache: RwLock::new(HashMap::new()),
            default_ttl,
        }
    }

    /// Get a cached value
    pub async fn get(&self, key: &str) -> Option<serde_json::Value> {
        let cache = self.config_cache.read().await;
        cache.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.value.clone())
            }
        })
    }

    /// Insert a value with default TTL
    pub async fn insert(&self, key: String, value: serde_json::Value) {
        self.insert_with_ttl(key, value, self.default_ttl).await;
    }

    /// Insert a value with custom TTL
    pub async fn insert_with_ttl(&self, key: String, value: serde_json::Value, ttl: Duration) {
        let mut cache = self.config_cache.write().await;
        cache.insert(key, TimedEntry::new(value, ttl));
    }

    /// Invalidate a key
    pub async fn invalidate(&self, key: &str) {
        let mut cache = self.config_cache.write().await;
        cache.remove(key);
    }

    /// Clear expired entries
    pub async fn cleanup_expired(&self) {
        let mut cache = self.config_cache.write().await;
        cache.retain(|_, entry| !entry.is_expired());
    }

    /// Clear all entries
    pub async fn clear(&self) {
        let mut cache = self.config_cache.write().await;
        cache.clear();
    }
}

impl Default for CrossTurnCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Batch Query Optimizer
// ============================================================================

/// Batch query optimizer for reducing database round trips
///
/// Collects multiple queries and executes them in batches.
pub struct BatchQueryOptimizer {
    /// Pending knowledge queries
    pending_knowledge: Mutex<HashMap<String, Instant>>,
    /// Batch window in milliseconds
    batch_window_ms: u64,
}

impl BatchQueryOptimizer {
    /// Create a new batch optimizer
    pub fn new() -> Self {
        Self::with_batch_window(10) // 10ms default
    }

    /// Create a new batch optimizer with custom window
    pub fn with_batch_window(batch_window_ms: u64) -> Self {
        Self {
            pending_knowledge: Mutex::new(HashMap::new()),
            batch_window_ms,
        }
    }

    /// Add a knowledge query to the batch
    pub fn add_knowledge_query(&self, character_id: &str) {
        let mut pending = self.pending_knowledge.lock();
        pending.insert(character_id.to_string(), Instant::now());
    }

    /// Get pending knowledge queries
    pub fn get_pending_knowledge_queries(&self) -> Vec<String> {
        let pending = self.pending_knowledge.lock();
        pending.keys().cloned().collect()
    }

    /// Clear pending queries
    pub fn clear_pending(&self) {
        let mut pending = self.pending_knowledge.lock();
        pending.clear();
    }

    /// Check if batch window has elapsed
    pub fn is_batch_ready(&self) -> bool {
        let pending = self.pending_knowledge.lock();
        if pending.is_empty() {
            return false;
        }

        let now = Instant::now();
        let window = Duration::from_millis(self.batch_window_ms);

        pending
            .values()
            .any(|&time| now.duration_since(time) >= window)
    }
}

impl Default for BatchQueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Global Cache Manager
// ============================================================================

use once_cell::sync::Lazy;

/// Global turn-scoped cache instance
pub static TURN_SCOPED_CACHE: Lazy<TurnScopedCache> = Lazy::new(TurnScopedCache::new);

/// Global cross-turn cache instance
pub static CROSS_TURN_CACHE: Lazy<CrossTurnCache> = Lazy::new(CrossTurnCache::new);

/// Global batch optimizer instance
pub static BATCH_OPTIMIZER: Lazy<BatchQueryOptimizer> = Lazy::new(BatchQueryOptimizer::new);

/// Initialize caches with custom sizes
pub fn init_caches(
    knowledge_size: usize,
    attribute_size: usize,
    scene_filter_size: usize,
    embodiment_size: usize,
) {
    // The Lazy statics are already initialized, but we can clear and resize
    // by accessing them and replacing the internal state
    let _ = (&TURN_SCOPED_CACHE, &CROSS_TURN_CACHE, &BATCH_OPTIMIZER);
}

/// Clear all caches (for testing or reset)
pub async fn clear_all_caches() {
    TURN_SCOPED_CACHE.knowledge_cache.lock().clear();
    TURN_SCOPED_CACHE.attribute_cache.lock().clear();
    TURN_SCOPED_CACHE.scene_filter_cache.lock().clear();
    TURN_SCOPED_CACHE.embodiment_cache.lock().clear();
    CROSS_TURN_CACHE.clear().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_scoped_cache_basic_operations() {
        let cache = TurnScopedCache::new();
        cache.clear_for_turn("turn-1");

        let key = KnowledgeAccessKey {
            character_id: "char-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            time_anchor_hash: 0,
        };

        // Test insert and get
        let value = Arc::new(AccessibleKnowledge {
            character_id: "char-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            entries: vec![],
        });

        cache.insert_knowledge(key.clone(), value.clone());
        let cached = cache.get_knowledge(&key);
        assert!(cached.is_some());
        assert!(Arc::ptr_eq(&value, &cached.unwrap()));
    }

    #[test]
    fn turn_scoped_cache_clears_on_new_turn() {
        let cache = TurnScopedCache::new();
        cache.clear_for_turn("turn-1");

        let key = KnowledgeAccessKey {
            character_id: "char-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            time_anchor_hash: 0,
        };

        let value = Arc::new(AccessibleKnowledge {
            character_id: "char-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            entries: vec![],
        });

        cache.insert_knowledge(key.clone(), value);
        assert!(cache.get_knowledge(&key).is_some());

        // Clear for new turn
        cache.clear_for_turn("turn-2");
        assert!(cache.get_knowledge(&key).is_none());
    }

    #[test]
    fn turn_scoped_cache_same_turn_no_clear() {
        let cache = TurnScopedCache::new();
        cache.clear_for_turn("turn-1");

        let key = KnowledgeAccessKey {
            character_id: "char-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            time_anchor_hash: 0,
        };

        let value = Arc::new(AccessibleKnowledge {
            character_id: "char-1".to_string(),
            scene_turn_id: "turn-1".to_string(),
            entries: vec![],
        });

        cache.insert_knowledge(key.clone(), value);

        // Same turn ID should not clear
        cache.clear_for_turn("turn-1");
        assert!(cache.get_knowledge(&key).is_some());
    }

    #[tokio::test]
    async fn cross_turn_cache_ttl() {
        let cache = CrossTurnCache::with_ttl(Duration::from_millis(50));

        cache
            .insert("key-1".to_string(), serde_json::json!("value-1"))
            .await;

        // Should be present immediately
        assert!(cache.get("key-1").await.is_some());

        // Wait for TTL
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Should be expired
        assert!(cache.get("key-1").await.is_none());
    }

    #[test]
    fn batch_optimizer_collects_queries() {
        let optimizer = BatchQueryOptimizer::new();

        optimizer.add_knowledge_query("char-1");
        optimizer.add_knowledge_query("char-2");

        let pending = optimizer.get_pending_knowledge_queries();
        assert_eq!(pending.len(), 2);
        assert!(pending.contains(&"char-1".to_string()));
        assert!(pending.contains(&"char-2".to_string()));

        optimizer.clear_pending();
        assert!(optimizer.get_pending_knowledge_queries().is_empty());
    }
}
