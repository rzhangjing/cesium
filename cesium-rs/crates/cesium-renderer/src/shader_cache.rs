//! Ported from `packages/engine/Source/Renderer/ShaderCache.js`.
//!
//! Caches compiled shader programs to avoid redundant compilation.
//! In CesiumJS, this is keyed by the combined shader source string hash.

use std::collections::HashMap;
use crate::shader_program::ShaderProgram;
use crate::shader_source::ShaderSource;

/// Caches compiled shader programs keyed by source hash.
///
/// Mirrors the JS `ShaderCache` which avoids redundant shader compilation
/// by caching programs keyed by their combined source + defines.
pub struct ShaderCache {
    cache: HashMap<String, ShaderProgram>,
    /// Shaders that were released this frame and should be destroyed
    /// at the end of the frame (after all draw calls using them complete).
    _shaders_to_destroy: Vec<ShaderProgram>,
}

impl ShaderCache {
    /// Creates a new shader cache.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            _shaders_to_destroy: Vec::new(),
        }
    }

    /// Returns the number of cached programs.
    pub fn len(&self) -> usize { self.cache.len() }

    /// Returns whether the cache is empty.
    pub fn is_empty(&self) -> bool { self.cache.is_empty() }

    /// Clears the cache, destroying all cached programs.
    pub fn clear(&mut self) {
        for (_key, program) in self.cache.drain() {
            drop(program);
        }
    }

    /// Gets or creates a cached shader program for the given shader source.
    ///
    /// If a program with the same cache key already exists, returns a reference
    /// to it. Otherwise, creates a new program, caches it, and returns a reference.
    ///
    /// Mirrors `ShaderCache.prototype.getShaderProgram()`.
    pub fn get_or_create(
        &mut self,
        shader_source: &ShaderSource,
        factory: impl FnOnce() -> ShaderProgram,
    ) -> &ShaderProgram {
        let cache_key = shader_source.get_cache_key();

        if !self.cache.contains_key(&cache_key) {
            let program = factory();
            self.cache.insert(cache_key.clone(), program);
        }

        self.cache.get(&cache_key).unwrap()
    }

    /// Gets a cached shader program by cache key.
    pub fn get(&self, cache_key: &str) -> Option<&ShaderProgram> {
        self.cache.get(cache_key)
    }

    /// Releases a shader program from the cache (marks for destruction).
    ///
    /// Mirrors `ShaderCache.prototype.releaseShaderProgram()`.
    pub fn release(&mut self, cache_key: &str) {
        if let Some(program) = self.cache.remove(cache_key) {
            self._shaders_to_destroy.push(program);
        }
    }

    /// Destroys all released shaders that are no longer in use.
    ///
    /// Mirrors `ShaderCache.prototype.destroyReleasedShaderPrograms()`.
    pub fn destroy_released(&mut self) {
        self._shaders_to_destroy.clear();
    }

    /// Returns an iterator over all cached cache keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.cache.keys().map(|k| k.as_str())
    }
}

impl Default for ShaderCache {
    fn default() -> Self { Self::new() }
}
