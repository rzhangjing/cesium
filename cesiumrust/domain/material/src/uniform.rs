//! Fabric uniform 值与 GLSL 类型推断。
//!
//! 映射到 CesiumJS `Scene/Material.js` 中的 uniform 处理
//! （`getUniformType`、`createUniform`），以及 WebGL uniform 设置器
//! 所执行的 uniform 值强制转换。

use crate::error::MaterialError;
use serde_json::Value as JsonValue;

/// 默认纹理 uniform 值。
/// 映射到 `Material.DefaultImageId`。
pub const DEFAULT_IMAGE_ID: &str = "czm_defaultImage";

/// 默认立方体贴图纹理 uniform 值。
/// 映射到 `Material.DefaultCubeMapId`。
pub const DEFAULT_CUBEMAP_ID: &str = "czm_defaultCubeMap";

/// cube map uniform 的六个面图像。
/// 映射到 `samplerCube` uniform 所接受的 `{positiveX, negativeX, positiveY, negativeY, positiveZ,
/// negativeZ}` 对象。
#[derive(Debug, Clone, PartialEq)]
pub struct CubeMapFaces {
    pub positive_x: String,
    pub negative_x: String,
    pub positive_y: String,
    pub negative_y: String,
    pub positive_z: String,
    pub negative_z: String,
}

/// Fabric 材质的 uniform 值。
///
/// CesiumJS 将 uniform 值存储为原始 JS 值（数字、布尔、`Color`、
/// `Cartesian2`、图像 URL、通道字符串、以数组表示的矩阵、立方体贴图面
/// 对象），并在 `getUniformType` 中根据值的形状推断 GLSL uniform 类型。
/// 本 enum 捕获同一组形状，并将推断出的类型显式化。
#[derive(Debug, Clone, PartialEq)]
pub enum UniformValue {
    /// GLSL `float`（JS number）。
    Float(f64),
    /// GLSL `bool`（JS boolean）。
    Bool(bool),
    /// GLSL `vec2`（`Cartesian2`、`{x, y}`，或像 `fadeDirection: {x: true, y: true}`
    /// 这样的双通道布尔向量）。
    Vec2([f64; 2]),
    /// GLSL `vec3`（`Cartesian3` / `{x, y, z}`）。
    Vec3([f64; 3]),
    /// GLSL `vec4`（`Color` / `Cartesian4` / `{x, y, z, w}`）。
    Vec4([f64; 4]),
    /// GLSL `ivec3`（用于自动生成的 `<image>Dimensions` uniform）。
    IVec3([i64; 3]),
    /// GLSL `mat2`（4 个数字的列主序数组）。
    Mat2([f64; 4]),
    /// GLSL `mat3`（9 个数字的列主序数组）。
    Mat3([f64; 9]),
    /// GLSL `mat4`（16 个数字的列主序数组）。
    Mat4([f64; 16]),
    /// GLSL `sampler2D`（图像 URL，或 [`DEFAULT_IMAGE_ID`]）。
    Sampler2D(String),
    /// GLSL `samplerCube`。`None` 表示 [`DEFAULT_CUBEMAP_ID`]。
    SamplerCube(Option<CubeMapFaces>),
    /// 像 `"rgb"` 或 `"a"` 这样的通道 swizzle 字符串。在 CesiumJS 中这
    /// 并非真正的 uniform：该 token 会在着色器源码中被文本替换
    /// （`getUniformType` 中的 `channels` 类型）。
    Channels(String),
}

impl UniformValue {
    /// 该值的 GLSL uniform 类型名。
    /// 映射到 `getUniformType` 的返回值。
    pub fn glsl_type(&self) -> &'static str {
        match self {
            UniformValue::Float(_) => "float",
            UniformValue::Bool(_) => "bool",
            UniformValue::Vec2(_) => "vec2",
            UniformValue::Vec3(_) => "vec3",
            UniformValue::Vec4(_) => "vec4",
            UniformValue::IVec3(_) => "ivec3",
            UniformValue::Mat2(_) => "mat2",
            UniformValue::Mat3(_) => "mat3",
            UniformValue::Mat4(_) => "mat4",
            UniformValue::Sampler2D(_) => "sampler2D",
            UniformValue::SamplerCube(_) => "samplerCube",
            UniformValue::Channels(_) => "channels",
        }
    }

    /// 对于类颜色值返回 alpha 分量，对于 float 返回标量。
    /// 由半透明性求值使用（内置半透明函数中的
    /// `material.uniforms.color.alpha < 1.0` 和 `uniforms.cellAlpha < 1.0`）。
    pub fn alpha_or_scalar(&self) -> Option<f64> {
        match self {
            UniformValue::Float(f) => Some(*f),
            UniformValue::Vec4(v) => Some(v[3]),
            _ => None,
        }
    }
}

/// 字符串是否为通道 swizzle（`"r"`、`"rgb"`、`"rgba"` 等）。
/// 映射到 `getUniformType` 中的 `/^([rgba]){1,4}$/i` 测试。
pub fn is_channel_string(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 4
        && s.chars()
            .all(|c| matches!(c.to_ascii_lowercase(), 'r' | 'g' | 'b' | 'a'))
}

fn json_to_f64(v: &JsonValue) -> Option<f64> {
    match v {
        JsonValue::Number(n) => n.as_f64(),
        JsonValue::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn take_number(map: &serde_json::Map<String, JsonValue>, key: &str) -> Option<f64> {
    map.get(key).and_then(json_to_f64)
}

/// 将 Fabric JSON uniform 值解析为 [`UniformValue`]。
///
/// 对 CesiumJS `getUniformType` 中基于形状推断的忠实移植：
/// - number → `float`
/// - boolean → `bool`
/// - string → 当匹配 `^[rgba]{1,4}$` 时为 `channels`，当为
///   [`DEFAULT_CUBEMAP_ID`] 时为 `samplerCube`，否则为 `sampler2D`
/// - 4/9/16 个数字的数组 → `mat2`/`mat3`/`mat4`
/// - 含 `red/green/blue/alpha` 的对象 → `vec4`（一个 `Color`）
/// - 含六个立方体贴图面键的对象 → `samplerCube`
/// - 含 2/3/4 个属性的对象 → `vec2`/`vec3`/`vec4`（布尔强制转换为
///   1.0/0.0，与 WebGL uniform 设置器一致）
/// - 显式的 `"type"` 成员会覆盖推断出的类型名
pub fn uniform_value_from_json(value: &JsonValue) -> Result<UniformValue, MaterialError> {
    match value {
        JsonValue::Number(n) => Ok(UniformValue::Float(
            n.as_f64().ok_or(MaterialError::InvalidUniformValue {
                uniform: "<number>".to_string(),
                reason: "not a finite number".to_string(),
            })?,
        )),
        JsonValue::Bool(b) => Ok(UniformValue::Bool(*b)),
        JsonValue::String(s) => {
            if is_channel_string(s) {
                Ok(UniformValue::Channels(s.clone()))
            } else if s == DEFAULT_CUBEMAP_ID {
                Ok(UniformValue::SamplerCube(None))
            } else {
                Ok(UniformValue::Sampler2D(s.clone()))
            }
        }
        JsonValue::Array(arr) => {
            let nums: Option<Vec<f64>> = arr.iter().map(json_to_f64).collect();
            let nums = nums.ok_or(MaterialError::InvalidUniformValue {
                uniform: "<array>".to_string(),
                reason: "array elements must be numbers".to_string(),
            })?;
            match nums.len() {
                4 => Ok(UniformValue::Mat2([nums[0], nums[1], nums[2], nums[3]])),
                9 => {
                    let mut m = [0.0; 9];
                    m.copy_from_slice(&nums);
                    Ok(UniformValue::Mat3(m))
                }
                16 => {
                    let mut m = [0.0; 16];
                    m.copy_from_slice(&nums);
                    Ok(UniformValue::Mat4(m))
                }
                _ => Err(MaterialError::InvalidUniformValue {
                    uniform: "<array>".to_string(),
                    reason: format!(
                        "matrix arrays must have 4, 9 or 16 elements, got {}",
                        nums.len()
                    ),
                }),
            }
        }
        JsonValue::Object(map) => {
            // 显式类型标注（例如自动生成的
            // `{ type: "ivec3", x: 1, y: 1 }` 尺寸 uniform）。
            if let Some(JsonValue::String(type_name)) = map.get("type") {
                return uniform_value_with_explicit_type(type_name, map);
            }

            // Color：{ red, green, blue, alpha }
            if map.contains_key("red")
                && map.contains_key("green")
                && map.contains_key("blue")
                && map.contains_key("alpha")
            {
                return Ok(UniformValue::Vec4([
                    take_number(map, "red").unwrap_or(0.0),
                    take_number(map, "green").unwrap_or(0.0),
                    take_number(map, "blue").unwrap_or(0.0),
                    take_number(map, "alpha").unwrap_or(1.0),
                ]));
            }

            // 立方体贴图面：{ positiveX, negativeX, ..., negativeZ }
            if map.contains_key("positiveX")
                && map.contains_key("negativeX")
                && map.contains_key("positiveY")
                && map.contains_key("negativeY")
                && map.contains_key("positiveZ")
                && map.contains_key("negativeZ")
            {
                let face = |key: &str| -> String {
                    map.get(key)
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string()
                };
                return Ok(UniformValue::SamplerCube(Some(CubeMapFaces {
                    positive_x: face("positiveX"),
                    negative_x: face("negativeX"),
                    positive_y: face("positiveY"),
                    negative_y: face("negativeY"),
                    positive_z: face("positiveZ"),
                    negative_z: face("negativeZ"),
                })));
            }

            // 基于属性数量的向量推断（2..=4 个属性）。
            let num_attributes = map.len();
            let component = |key: &str| -> Option<f64> { take_number(map, key) };
            match num_attributes {
                2 => Ok(UniformValue::Vec2([
                    component("x").unwrap_or(0.0),
                    component("y").unwrap_or(0.0),
                ])),
                3 => Ok(UniformValue::Vec3([
                    component("x").unwrap_or(0.0),
                    component("y").unwrap_or(0.0),
                    component("z").unwrap_or(0.0),
                ])),
                4 => Ok(UniformValue::Vec4([
                    component("x").unwrap_or(0.0),
                    component("y").unwrap_or(0.0),
                    component("z").unwrap_or(0.0),
                    component("w").unwrap_or(0.0),
                ])),
                _ => Err(MaterialError::InvalidUniformValue {
                    uniform: "<object>".to_string(),
                    reason: format!(
                        "cannot infer uniform type from object with {} attributes",
                        num_attributes
                    ),
                }),
            }
        }
        JsonValue::Null => Err(MaterialError::InvalidUniformValue {
            uniform: "<null>".to_string(),
            reason: "null is not a valid uniform value".to_string(),
        }),
    }
}

fn uniform_value_with_explicit_type(
    type_name: &str,
    map: &serde_json::Map<String, JsonValue>,
) -> Result<UniformValue, MaterialError> {
    let num = |key: &str| take_number(map, key).unwrap_or(0.0);
    match type_name {
        "float" => Ok(UniformValue::Float(num("value"))),
        "bool" => Ok(UniformValue::Bool(
            map.get("value").and_then(|v| v.as_bool()).unwrap_or(false),
        )),
        "vec2" => Ok(UniformValue::Vec2([num("x"), num("y")])),
        "vec3" => Ok(UniformValue::Vec3([num("x"), num("y"), num("z")])),
        "vec4" => Ok(UniformValue::Vec4([num("x"), num("y"), num("z"), num("w")])),
        "ivec3" => Ok(UniformValue::IVec3([
            num("x") as i64,
            num("y") as i64,
            num("z") as i64,
        ])),
        "sampler2D" => Ok(UniformValue::Sampler2D(
            map.get("value")
                .and_then(|v| v.as_str())
                .unwrap_or(DEFAULT_IMAGE_ID)
                .to_string(),
        )),
        "samplerCube" => Ok(UniformValue::SamplerCube(None)),
        other => Err(MaterialError::InvalidUniformValue {
            uniform: format!("<typed:{other}>"),
            reason: format!("unsupported explicit uniform type '{other}'"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_number_is_float() {
        assert_eq!(
            uniform_value_from_json(&json!(0.8)).unwrap(),
            UniformValue::Float(0.8)
        );
        assert_eq!(
            uniform_value_from_json(&json!(0.8)).unwrap().glsl_type(),
            "float"
        );
    }

    #[test]
    fn test_bool() {
        assert_eq!(
            uniform_value_from_json(&json!(true)).unwrap(),
            UniformValue::Bool(true)
        );
        assert_eq!(
            uniform_value_from_json(&json!(false)).unwrap().glsl_type(),
            "bool"
        );
    }

    #[test]
    fn test_channel_strings() {
        assert_eq!(
            uniform_value_from_json(&json!("rgb")).unwrap(),
            UniformValue::Channels("rgb".to_string())
        );
        assert_eq!(
            uniform_value_from_json(&json!("a")).unwrap(),
            UniformValue::Channels("a".to_string())
        );
        assert!(is_channel_string("rgba"));
        assert!(!is_channel_string(""));
        assert!(!is_channel_string("rgbba"));
        assert!(!is_channel_string("qx"));
    }

    #[test]
    fn test_image_url_is_sampler2d() {
        assert_eq!(
            uniform_value_from_json(&json!("path/to/image.png")).unwrap(),
            UniformValue::Sampler2D("path/to/image.png".to_string())
        );
        assert_eq!(
            uniform_value_from_json(&json!("path/to/image.png"))
                .unwrap()
                .glsl_type(),
            "sampler2D"
        );
    }

    #[test]
    fn test_default_ids() {
        assert_eq!(
            uniform_value_from_json(&json!("czm_defaultImage")).unwrap(),
            UniformValue::Sampler2D(DEFAULT_IMAGE_ID.to_string())
        );
        assert_eq!(
            uniform_value_from_json(&json!("czm_defaultCubeMap")).unwrap(),
            UniformValue::SamplerCube(None)
        );
    }

    #[test]
    fn test_color_object_is_vec4() {
        let v = uniform_value_from_json(&json!({
            "red": 1.0, "green": 0.0, "blue": 0.0, "alpha": 0.5
        }))
        .unwrap();
        assert_eq!(v, UniformValue::Vec4([1.0, 0.0, 0.0, 0.5]));
        assert_eq!(v.glsl_type(), "vec4");
        assert_eq!(v.alpha_or_scalar(), Some(0.5));
    }

    #[test]
    fn test_cartesian2_object_is_vec2() {
        assert_eq!(
            uniform_value_from_json(&json!({"x": 8.0, "y": 8.0})).unwrap(),
            UniformValue::Vec2([8.0, 8.0])
        );
    }

    #[test]
    fn test_boolean_vector_coercion() {
        // fadeDirection：{ x: true, y: true } → vec2(1.0, 1.0)
        assert_eq!(
            uniform_value_from_json(&json!({"x": true, "y": false})).unwrap(),
            UniformValue::Vec2([1.0, 0.0])
        );
    }

    #[test]
    fn test_cubemap_faces() {
        let v = uniform_value_from_json(&json!({
            "positiveX": "px.png", "negativeX": "nx.png",
            "positiveY": "py.png", "negativeY": "ny.png",
            "positiveZ": "pz.png", "negativeZ": "nz.png"
        }))
        .unwrap();
        match v {
            UniformValue::SamplerCube(Some(faces)) => {
                assert_eq!(faces.positive_x, "px.png");
                assert_eq!(faces.negative_z, "nz.png");
            }
            _ => panic!("expected samplerCube"),
        }
    }

    #[test]
    fn test_matrix_arrays() {
        assert_eq!(
            uniform_value_from_json(&json!([1.0, 0.0, 0.0, 1.0])).unwrap(),
            UniformValue::Mat2([1.0, 0.0, 0.0, 1.0])
        );
        let mat3: Vec<f64> = vec![1.0; 9];
        assert!(matches!(
            uniform_value_from_json(&JsonValue::Array(
                mat3.iter().map(|v| json!(v)).collect()
            ))
            .unwrap(),
            UniformValue::Mat3(_)
        ));
        let mat4: Vec<f64> = vec![1.0; 16];
        assert!(matches!(
            uniform_value_from_json(&JsonValue::Array(
                mat4.iter().map(|v| json!(v)).collect()
            ))
            .unwrap(),
            UniformValue::Mat4(_)
        ));
        assert!(uniform_value_from_json(&json!([1.0, 2.0, 3.0])).is_err());
    }

    #[test]
    fn test_explicit_ivec3_type() {
        let v = uniform_value_from_json(&json!({"type": "ivec3", "x": 1, "y": 1})).unwrap();
        assert_eq!(v, UniformValue::IVec3([1, 1, 0]));
        assert_eq!(v.glsl_type(), "ivec3");
    }

    #[test]
    fn test_invalid_values() {
        assert!(uniform_value_from_json(&JsonValue::Null).is_err());
        // 5 个属性无法推断
        assert!(uniform_value_from_json(&json!({
            "a": 1, "b": 2, "c": 3, "d": 4, "e": 5
        }))
        .is_err());
    }
}
