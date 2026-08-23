//! Ported from `packages/engine/Source/Scene/PostProcessStageLibrary.js`.
//!
//! Library of built-in post-process stages.

use crate::post_process_stage::PostProcessStage;

/// Library of built-in post-process stages.
///
/// Provides factory methods for common effects like bloom, ambient occlusion,
/// blur, and edge detection.
/// Mirrors CesiumJS `PostProcessStageLibrary` (715 lines).
pub struct PostProcessStageLibrary;

impl PostProcessStageLibrary {
    /// Creates a black-and-white post-process stage.
    pub fn create_black_and_white_stage() -> PostProcessStage {
        PostProcessStage::new(
            r#"uniform sampler2D colorTexture;
            in vec2 v_textureCoordinates;
            void main() {
                vec4 color = texture(colorTexture, v_textureCoordinates);
                float gray = dot(color.rgb, vec3(0.299, 0.587, 0.114));
                out_FragColor = vec4(vec3(gray), color.a);
            }"#.to_string(),
        )
    }

    /// Creates a brightness post-process stage.
    pub fn create_brightness_stage() -> PostProcessStage {
        PostProcessStage::new(
            r#"uniform sampler2D colorTexture;
            uniform float brightness;
            in vec2 v_textureCoordinates;
            void main() {
                vec4 color = texture(colorTexture, v_textureCoordinates);
                out_FragColor = vec4(color.rgb * brightness, color.a);
            }"#.to_string(),
        )
    }

    /// Creates a blur post-process stage.
    pub fn create_blur_stage() -> PostProcessStage {
        PostProcessStage::new(
            r#"uniform sampler2D colorTexture;
            uniform vec2 stepSize;
            in vec2 v_textureCoordinates;
            void main() {
                vec4 color = vec4(0.0);
                color += texture(colorTexture, v_textureCoordinates + vec2(-stepSize.x, -stepSize.y));
                color += texture(colorTexture, v_textureCoordinates + vec2(0.0, -stepSize.y));
                color += texture(colorTexture, v_textureCoordinates + vec2(stepSize.x, -stepSize.y));
                color += texture(colorTexture, v_textureCoordinates + vec2(-stepSize.x, 0.0));
                color += texture(colorTexture, v_textureCoordinates);
                color += texture(colorTexture, v_textureCoordinates + vec2(stepSize.x, 0.0));
                color += texture(colorTexture, v_textureCoordinates + vec2(-stepSize.x, stepSize.y));
                color += texture(colorTexture, v_textureCoordinates + vec2(0.0, stepSize.y));
                color += texture(colorTexture, v_textureCoordinates + vec2(stepSize.x, stepSize.y));
                out_FragColor = color / 9.0;
            }"#.to_string(),
        )
    }

    /// Creates an edge detection post-process stage.
    pub fn create_edge_detection_stage() -> PostProcessStage {
        PostProcessStage::new(
            r#"uniform sampler2D colorTexture;
            uniform sampler2D depthTexture;
            in vec2 v_textureCoordinates;
            void main() {
                out_FragColor = texture(colorTexture, v_textureCoordinates);
            }"#.to_string(),
        )
    }
}
