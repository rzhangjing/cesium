//! Renderer/BufferSpec.js, BufferUsageSpec.js, VertexArraySpec.js
//! → Rust integration tests

use cesium_scene::{GpuBuffer, BufferUsage};

// === BufferUsage ===

#[test]
fn test_buffer_usage_default() {
    assert_eq!(BufferUsage::default(), BufferUsage::StaticDraw);
}

#[test]
fn test_buffer_usage_variants() {
    assert_ne!(BufferUsage::StaticDraw, BufferUsage::DynamicDraw);
    assert_ne!(BufferUsage::DynamicDraw, BufferUsage::StreamDraw);
}

// === GpuBuffer ===

#[test]
fn test_gpu_buffer_new() {
    let buf = GpuBuffer::new(0, 1024, BufferUsage::StaticDraw);
    assert_eq!(buf.id, 0);
    assert_eq!(buf.size_in_bytes, 1024);
    assert_eq!(buf.usage, BufferUsage::StaticDraw);
}

#[test]
fn test_gpu_buffer_dynamic() {
    let buf = GpuBuffer::new(1, 2048, BufferUsage::DynamicDraw);
    assert_eq!(buf.usage, BufferUsage::DynamicDraw);
    assert_eq!(buf.size_in_bytes, 2048);
}

#[test]
fn test_gpu_buffer_stream() {
    let buf = GpuBuffer::new(2, 512, BufferUsage::StreamDraw);
    assert_eq!(buf.usage, BufferUsage::StreamDraw);
}
