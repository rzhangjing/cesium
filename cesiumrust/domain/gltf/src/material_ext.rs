//! 用于 glTF 2.0 的 PBR material 扩展。
//!
//! 描述 metallic-roughness 与各 KHR 扩展（含 specular/clearcoat/anisotropy
//! 等）的强类型结构，并提供扩展 JSON 的解析入口。
//!
//! 支持的 KHR 扩展：
//! - KHR_materials_pbrSpecularGlossiness
//! - KHR_materials_specular
//! - KHR_materials_clearcoat
//! - KHR_materials_anisotropy
//! - KHR_materials_transmission
//! - KHR_materials_ior
//! - KHR_materials_emissive_strength
//! - KHR_materials_unlit
//! - KHR_materials_sheen
//! - KHR_materials_volume
//! - KHR_texture_transform

use crate::gltf_model::TextureInfo;
use serde::{Deserialize, Serialize};

/// 扩展的 material 属性，将基础 PBR 与所有 KHR 扩展组合在一起。
///
/// 各扩展字段均可缺省，反序列化时按 KHR 名称逐一由 [`parse_material_extensions`] 填充。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedMaterial {
    /// 基础 PBR metallic-roughness（在 glTF 2.0 中始终存在）。
    #[serde(default)]
    pub metallic_roughness: MetallicRoughness,

    /// KHR_materials_pbrSpecularGlossiness 扩展。
    #[serde(default)]
    pub specular_glossiness: Option<SpecularGlossiness>,

    /// KHR_materials_specular 扩展。
    #[serde(default)]
    pub specular: Option<Specular>,

    /// KHR_materials_clearcoat 扩展。
    #[serde(default)]
    pub clearcoat: Option<Clearcoat>,

    /// KHR_materials_anisotropy 扩展。
    #[serde(default)]
    pub anisotropy: Option<Anisotropy>,

    /// KHR_materials_transmission 扩展。
    #[serde(default)]
    pub transmission: Option<Transmission>,

    /// KHR_materials_ior 扩展。
    #[serde(default)]
    pub ior: Option<Ior>,

    /// KHR_materials_emissive_strength 扩展。
    #[serde(default)]
    pub emissive_strength: Option<EmissiveStrength>,

    /// KHR_materials_sheen 扩展。
    #[serde(default)]
    pub sheen: Option<Sheen>,

    /// KHR_materials_volume 扩展。
    #[serde(default)]
    pub volume: Option<Volume>,

    /// 存在 KHR_materials_unlit 扩展。
    #[serde(default)]
    pub unlit: bool,
}

/// PBR metallic-roughness 明暗模型。
///
/// 以基础颜色/金属度/粗糙度三组因子与贴图描述常规金属-非金属表面。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetallicRoughness {
    /// 基础颜色因子 [r, g, b, a]。默认 [1,1,1,1]。
    #[serde(default = "default_base_color_factor")]
    pub base_color_factor: [f64; 4],

    /// 基础颜色贴图。
    #[serde(default)]
    pub base_color_texture: Option<TextureTransformInfo>,

    /// 金属度因子。默认 1.0。
    #[serde(default = "default_one")]
    pub metallic_factor: f64,

    /// 粗糙度因子。默认 1.0。
    #[serde(default = "default_one")]
    pub roughness_factor: f64,

    /// 金属度-粗糙度贴图（G=粗糙度，B=金属度）。
    #[serde(default)]
    pub metallic_roughness_texture: Option<TextureTransformInfo>,
}

impl Default for MetallicRoughness {
    /// 白基色、全金属全粗糙、无贴图的默认 PBR 参数。
    fn default() -> Self {
        Self {
            base_color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_texture: None,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            metallic_roughness_texture: None,
        }
    }
}

/// KHR_materials_pbrSpecularGlossiness 扩展。
///
/// 采用旧式 specular-glossiness 工作流，与 metallic-roughness 互斥备选。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpecularGlossiness {
    /// 漫反射因子 [r, g, b, a]。默认 [1,1,1,1]。
    #[serde(default = "default_base_color_factor")]
    pub diffuse_factor: [f64; 4],

    /// 漫反射贴图。
    #[serde(default)]
    pub diffuse_texture: Option<TextureTransformInfo>,

    /// 高光因子 [r, g, b]。默认 [1,1,1]。
    #[serde(default = "default_specular_factor")]
    pub specular_factor: [f64; 3],

    /// 光滑度因子。默认 1.0。
    #[serde(default = "default_one")]
    pub glossiness_factor: f64,

    /// 高光-光滑度贴图。
    #[serde(default)]
    pub specular_glossiness_texture: Option<TextureTransformInfo>,
}

impl Default for SpecularGlossiness {
    /// 漫反射白、高光全白、光泽度 1.0、无贴图的默认参数。
    fn default() -> Self {
        Self {
            diffuse_factor: [1.0, 1.0, 1.0, 1.0],
            diffuse_texture: None,
            specular_factor: [1.0, 1.0, 1.0],
            glossiness_factor: 1.0,
            specular_glossiness_texture: None,
        }
    }
}

/// KHR_materials_specular 扩展。
///
/// 在 metallic-roughness 之上叠加独立的高光强度与颜色控制。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Specular {
    /// 高光因子。默认 1.0。
    #[serde(default = "default_one")]
    pub specular_factor: f64,

    /// 高光贴图。
    #[serde(default)]
    pub specular_texture: Option<TextureTransformInfo>,

    /// 高光颜色因子 [r, g, b]。默认 [1,1,1]。
    #[serde(default = "default_specular_factor")]
    pub specular_color_factor: [f64; 3],

    /// 高光颜色贴图。
    #[serde(default)]
    pub specular_color_texture: Option<TextureTransformInfo>,
}

impl Default for Specular {
    /// 高光因子与颜色均为 1.0、无贴图的默认 specular 参数。
    fn default() -> Self {
        Self {
            specular_factor: 1.0,
            specular_texture: None,
            specular_color_factor: [1.0, 1.0, 1.0],
            specular_color_texture: None,
        }
    }
}

/// KHR_materials_clearcoat 扩展。
///
/// 模拟车漆类材质表面的一层额外清漆层及其法线细节。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clearcoat {
    /// clearcoat 层强度。默认 0.0。
    #[serde(default)]
    pub clearcoat_factor: f64,

    /// clearcoat 强度贴图。
    #[serde(default)]
    pub clearcoat_texture: Option<TextureTransformInfo>,

    /// clearcoat 粗糙度。默认 0.0。
    #[serde(default)]
    pub clearcoat_roughness_factor: f64,

    /// clearcoat 粗糙度贴图。
    #[serde(default)]
    pub clearcoat_roughness_texture: Option<TextureTransformInfo>,

    /// clearcoat 法线贴图。
    #[serde(default)]
    pub clearcoat_normal_texture: Option<NormalTextureInfo>,
}

impl Default for Clearcoat {
    /// 清漆强度/粗糙度均为 0.0、无贴图的默认 clearcoat 参数。
    fn default() -> Self {
        Self {
            clearcoat_factor: 0.0,
            clearcoat_texture: None,
            clearcoat_roughness_factor: 0.0,
            clearcoat_roughness_texture: None,
            clearcoat_normal_texture: None,
        }
    }
}

/// KHR_materials_anisotropy 扩展。
///
/// 描述沿切线方向拉伸的高光（如拉丝金属、头发）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anisotropy {
    /// 各向异性强度。默认 0.0。
    #[serde(default)]
    pub anisotropy_strength: f64,

    /// 各向异性旋转（弧度）。默认 0.0。
    #[serde(default)]
    pub anisotropy_rotation: f64,

    /// 各向异性贴图。
    #[serde(default)]
    pub anisotropy_texture: Option<TextureTransformInfo>,
}

impl Default for Anisotropy {
    /// 强度与旋转均为 0.0、无贴图的默认各向异性参数。
    fn default() -> Self {
        Self {
            anisotropy_strength: 0.0,
            anisotropy_rotation: 0.0,
            anisotropy_texture: None,
        }
    }
}

/// KHR_materials_transmission 扩展。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transmission {
    /// 透射因子。默认 0.0。
    #[serde(default)]
    pub transmission_factor: f64,

    /// 透射贴图。
    #[serde(default)]
    pub transmission_texture: Option<TextureTransformInfo>,
}

impl Default for Transmission {
    /// 透射因子 0.0（完全不透射）的默认参数。
    fn default() -> Self {
        Self {
            transmission_factor: 0.0,
            transmission_texture: None,
        }
    }
}

/// KHR_materials_ior 扩展。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ior {
    /// 折射率。默认 1.5。
    #[serde(default = "default_ior")]
    pub ior: f64,
}

impl Default for Ior {
    /// 折射率 1.5（常见介质）的默认参数。
    fn default() -> Self {
        Self { ior: 1.5 }
    }
}

/// KHR_materials_emissive_strength 扩展。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmissiveStrength {
    /// 自发光强度乘数。默认 1.0。
    #[serde(default = "default_one")]
    pub emissive_strength: f64,
}

impl Default for EmissiveStrength {
    /// 自发光强度乘数为 1.0（不额外增强）的默认参数。
    fn default() -> Self {
        Self {
            emissive_strength: 1.0,
        }
    }
}

/// KHR_materials_sheen 扩展。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sheen {
    /// 包边颜色因子 [r, g, b]。默认 [0,0,0]。
    #[serde(default)]
    pub sheen_color_factor: [f64; 3],

    /// 包边颜色贴图。
    #[serde(default)]
    pub sheen_color_texture: Option<TextureTransformInfo>,

    /// 包边粗糙度因子。默认 0.0。
    #[serde(default)]
    pub sheen_roughness_factor: f64,

    /// 包边粗糙度贴图。
    #[serde(default)]
    pub sheen_roughness_texture: Option<TextureTransformInfo>,
}

impl Default for Sheen {
    /// 包边颜色为黑、粗糙度 0.0、无贴图的默认 sheen 参数。
    fn default() -> Self {
        Self {
            sheen_color_factor: [0.0, 0.0, 0.0],
            sheen_color_texture: None,
            sheen_roughness_factor: 0.0,
            sheen_roughness_texture: None,
        }
    }
}

/// KHR_materials_volume 扩展。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    /// 厚度因子。默认 0.0。
    #[serde(default)]
    pub thickness_factor: f64,

    /// 厚度贴图。
    #[serde(default)]
    pub thickness_texture: Option<TextureTransformInfo>,

    /// 衰减距离。默认 +无穷大。
    #[serde(default = "default_attenuation_distance")]
    pub attenuation_distance: f64,

    /// 衰减颜色 [r, g, b]。默认 [1,1,1]。
    #[serde(default = "default_specular_factor")]
    pub attenuation_color: [f64; 3],
}

impl Default for Volume {
    /// 厚度 0.0、衰减距离无穷大（无吸收）的默认体积参数。
    fn default() -> Self {
        Self {
            thickness_factor: 0.0,
            thickness_texture: None,
            attenuation_distance: f64::INFINITY,
            attenuation_color: [1.0, 1.0, 1.0],
        }
    }
}

/// 支持 KHR_texture_transform 扩展的 texture info。
///
/// 在基础 index/texCoord 之外附带可选的 UV 变换与法线缩放。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureTransformInfo {
    /// texture 的索引。
    pub index: usize,

    /// texture 坐标集。
    #[serde(default)]
    pub tex_coord: usize,

    /// KHR_texture_transform 扩展。
    #[serde(default)]
    pub extensions: Option<TextureTransformExtensions>,

    /// 法线贴图缩放（仅用于法线贴图）。
    #[serde(default)]
    pub scale: Option<f64>,
}

/// texture transform 扩展的容器。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureTransformExtensions {
    /// KHR_texture_transform 数据。
    #[serde(default, rename = "KHR_texture_transform")]
    pub texture_transform: Option<TextureTransform>,
}

/// KHR_texture_transform 扩展数据。
///
/// 提供 UV 变换：偏移、旋转、缩放以及 texCoord 覆盖。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureTransform {
    /// UV 偏移 [u, v]。默认 [0, 0]。
    #[serde(default)]
    pub offset: [f64; 2],

    /// 旋转（弧度，逆时针）。默认 0。
    #[serde(default)]
    pub rotation: f64,

    /// UV 缩放 [u, v]。默认 [1, 1]。
    #[serde(default = "default_uv_scale")]
    pub scale: [f64; 2],

    /// 覆盖的 texCoord 集索引。
    #[serde(default)]
    pub tex_coord: Option<usize>,
}

impl Default for TextureTransform {
    /// 零偏移、零旋转、单位缩放的恒等 UV 变换。
    fn default() -> Self {
        Self {
            offset: [0.0, 0.0],
            rotation: 0.0,
            scale: [1.0, 1.0],
            tex_coord: None,
        }
    }
}

impl TextureTransform {
    /// 计算 3x3 的 UV 变换矩阵。
    ///
    /// 变换顺序为：T(offset) * R(rotation) * S(scale)，返回列主序存储。
    pub fn compute_matrix(&self) -> [f64; 9] {
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        // 列主序 3x3：T * R * S
        // T = [1 0 ox; 0 1 oy; 0 0 1]
        // R = [cos -sin 0; sin cos 0; 0 0 1]
        // S = [sx 0 0; 0 sy 0; 0 0 1]
        let sx = self.scale[0];
        let sy = self.scale[1];
        let ox = self.offset[0];
        let oy = self.offset[1];

        // 合并：T * R * S（为清晰起见行主序，存储为列主序）
        // 行 0：[cos*sx, -sin*sy, ox]
        // 行 1：[sin*sx,  cos*sy, oy]
        // 行 2：[0,       0,      1 ]
        [
            cos_r * sx,
            sin_r * sx,
            0.0, // 列 0
            -sin_r * sy,
            cos_r * sy,
            0.0, // 列 1
            ox,
            oy,
            1.0, // 列 2
        ]
    }

    /// 使用本变换对一个 UV 坐标进行变换。
    pub fn transform_uv(&self, u: f64, v: f64) -> [f64; 2] {
        // 旋转的三角值，与 compute_matrix 保持一致的 T*R*S 语义
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();

        // 先按 scale 缩放 UV
        let su = u * self.scale[0];
        let sv = v * self.scale[1];

        // 再绕原点旋转
        let ru = cos_r * su - sin_r * sv;
        let rv = sin_r * su + cos_r * sv;

        // 最后平移 offset，得到最终 UV
        [ru + self.offset[0], rv + self.offset[1]]
    }
}

/// 带缩放的法线贴图 texture info。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalTextureInfo {
    /// texture 的索引。
    pub index: usize,

    /// texture 坐标集。
    #[serde(default)]
    pub tex_coord: usize,

    /// 法线贴图缩放。默认 1.0。
    #[serde(default = "default_one")]
    pub scale: f64,

    /// KHR_texture_transform 扩展。
    #[serde(default)]
    pub extensions: Option<TextureTransformExtensions>,
}

impl TextureTransformInfo {
    /// 由一个基础的 TextureInfo 创建。
    pub fn from_texture_info(info: &TextureInfo) -> Self {
        // 拷贝 index/texCoord，扩展与法线缩放置空
        Self {
            index: info.index,
            tex_coord: info.tex_coord,
            extensions: None,
            scale: None,
        }
    }

    /// 获取生效的 texCoord（考虑 KHR_texture_transform 覆盖）。
    pub fn effective_tex_coord(&self) -> usize {
        // 优先取 KHR_texture_transform 的覆盖，否则回落结构体 tex_coord
        self.extensions
            .as_ref()
            .and_then(|e| e.texture_transform.as_ref())
            .and_then(|t| t.tex_coord)
            .unwrap_or(self.tex_coord)
    }

    /// 若存在则获取 texture transform。
    pub fn get_transform(&self) -> Option<&TextureTransform> {
        // 逐层解包 Option，仅当扩展链完整时返回变换引用
        self.extensions
            .as_ref()
            .and_then(|e| e.texture_transform.as_ref())
    }
}

/// 从 glTF material 的扩展 JSON 解析扩展的 material。
///
/// 逐个探测常见 KHR 扩展键，将命中的子对象反序列化进对应的可选字段。
pub fn parse_material_extensions(
    extensions: &serde_json::Value,
) -> ExtendedMaterial {
    // 先取默认材质，再按存在的扩展键逐一覆盖对应字段
    let mut mat = ExtendedMaterial::default();

    if let Some(obj) = extensions.as_object() {
        // KHR_materials_pbrSpecularGlossiness
        if let Some(sg) = obj.get("KHR_materials_pbrSpecularGlossiness") {
            mat.specular_glossiness =
                serde_json::from_value(sg.clone()).ok();
        }

        // KHR_materials_specular
        if let Some(sp) = obj.get("KHR_materials_specular") {
            mat.specular = serde_json::from_value(sp.clone()).ok();
        }

        // KHR_materials_clearcoat
        if let Some(cc) = obj.get("KHR_materials_clearcoat") {
            mat.clearcoat = serde_json::from_value(cc.clone()).ok();
        }

        // KHR_materials_anisotropy
        if let Some(an) = obj.get("KHR_materials_anisotropy") {
            mat.anisotropy = serde_json::from_value(an.clone()).ok();
        }

        // KHR_materials_transmission
        if let Some(tr) = obj.get("KHR_materials_transmission") {
            mat.transmission = serde_json::from_value(tr.clone()).ok();
        }

        // KHR_materials_ior
        if let Some(ior) = obj.get("KHR_materials_ior") {
            mat.ior = serde_json::from_value(ior.clone()).ok();
        }

        // KHR_materials_emissive_strength
        if let Some(es) = obj.get("KHR_materials_emissive_strength") {
            mat.emissive_strength = serde_json::from_value(es.clone()).ok();
        }

        // KHR_materials_sheen
        if let Some(sh) = obj.get("KHR_materials_sheen") {
            mat.sheen = serde_json::from_value(sh.clone()).ok();
        }

        // KHR_materials_volume
        if let Some(vol) = obj.get("KHR_materials_volume") {
            mat.volume = serde_json::from_value(vol.clone()).ok();
        }

        // KHR_materials_unlit
        if obj.contains_key("KHR_materials_unlit") {
            mat.unlit = true;
        }
    }

    mat
}

// serde 使用的默认值函数
/// metallic/roughness/specular 因子共用的单位默认值 1.0。
fn default_one() -> f64 {
    1.0
}

/// RGBA 基础颜色因子的默认不透明白色。
fn default_base_color_factor() -> [f64; 4] {
    [1.0, 1.0, 1.0, 1.0]
}

/// RGB 三元因子的默认全白 [1,1,1]（高光色/衰减色共用）。
fn default_specular_factor() -> [f64; 3] {
    [1.0, 1.0, 1.0]
}

/// 折射率的默认值 1.5。
fn default_ior() -> f64 {
    1.5
}

/// 衰减距离的默认无穷大（相当于无吸收）。
fn default_attenuation_distance() -> f64 {
    f64::INFINITY
}

/// UV 缩放的默认单位 [1,1]。
fn default_uv_scale() -> [f64; 2] {
    [1.0, 1.0]
}

// 单元测试：覆盖各扩展默认值、JSON 解析与 UV 变换/矩阵。
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_metallic_roughness() {
        // 验证 MetallicRoughness 默认值
        let mr = MetallicRoughness::default();
        assert_eq!(mr.base_color_factor, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(mr.metallic_factor, 1.0);
        assert_eq!(mr.roughness_factor, 1.0);
        assert!(mr.base_color_texture.is_none());
    }

    #[test]
    fn test_specular_glossiness_defaults() {
        // 验证 SpecularGlossiness 默认因子
        let sg = SpecularGlossiness::default();
        assert_eq!(sg.diffuse_factor, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(sg.specular_factor, [1.0, 1.0, 1.0]);
        assert_eq!(sg.glossiness_factor, 1.0);
    }

    #[test]
    fn test_clearcoat_defaults() {
        // 验证 Clearcoat 默认强度为 0
        let cc = Clearcoat::default();
        assert_eq!(cc.clearcoat_factor, 0.0);
        assert_eq!(cc.clearcoat_roughness_factor, 0.0);
    }

    #[test]
    fn test_anisotropy_defaults() {
        // 验证 Anisotropy 默认强度与旋转为 0
        let an = Anisotropy::default();
        assert_eq!(an.anisotropy_strength, 0.0);
        assert_eq!(an.anisotropy_rotation, 0.0);
    }

    #[test]
    fn test_ior_default() {
        // 验证 Ior 默认折射率 1.5
        let ior = Ior::default();
        assert!((ior.ior - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_specular_glossiness_extension() {
        // 从 JSON 解析 pbrSpecularGlossiness 扩展字段
        let json = serde_json::json!({
            "KHR_materials_pbrSpecularGlossiness": {
                "diffuseFactor": [0.8, 0.2, 0.1, 1.0],
                "specularFactor": [0.5, 0.5, 0.5],
                "glossinessFactor": 0.9
            }
        });

        let mat = parse_material_extensions(&json);
        let sg = mat.specular_glossiness.unwrap();
        assert_eq!(sg.diffuse_factor, [0.8, 0.2, 0.1, 1.0]);
        assert_eq!(sg.specular_factor, [0.5, 0.5, 0.5]);
        assert!((sg.glossiness_factor - 0.9).abs() < 1e-10);
    }

    #[test]
    fn test_parse_clearcoat_extension() {
        // 从 JSON 解析 clearcoat 扩展因子
        let json = serde_json::json!({
            "KHR_materials_clearcoat": {
                "clearcoatFactor": 0.8,
                "clearcoatRoughnessFactor": 0.2
            }
        });

        let mat = parse_material_extensions(&json);
        let cc = mat.clearcoat.unwrap();
        assert!((cc.clearcoat_factor - 0.8).abs() < 1e-10);
        assert!((cc.clearcoat_roughness_factor - 0.2).abs() < 1e-10);
    }

    #[test]
    fn test_parse_unlit_extension() {
        // 存在 unlit 键即将 unlit 置为 true
        let json = serde_json::json!({
            "KHR_materials_unlit": {}
        });

        let mat = parse_material_extensions(&json);
        assert!(mat.unlit);
    }

    #[test]
    fn test_parse_transmission_extension() {
        // 从 JSON 解析 transmission 因子
        let json = serde_json::json!({
            "KHR_materials_transmission": {
                "transmissionFactor": 0.7
            }
        });

        let mat = parse_material_extensions(&json);
        let tr = mat.transmission.unwrap();
        assert!((tr.transmission_factor - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_parse_multiple_extensions() {
        // 多个扩展同时存在时各自归位，未提供的保持 None
        let json = serde_json::json!({
            "KHR_materials_clearcoat": { "clearcoatFactor": 1.0 },
            "KHR_materials_ior": { "ior": 1.45 },
            "KHR_materials_unlit": {}
        });

        let mat = parse_material_extensions(&json);
        assert!(mat.clearcoat.is_some());
        assert!(mat.ior.is_some());
        assert!(mat.unlit);
        assert!(mat.specular_glossiness.is_none());
    }

    #[test]
    fn test_texture_transform_identity() {
        // 恒等变换不改变 UV
        let transform = TextureTransform::default();
        let uv = transform.transform_uv(0.5, 0.5);
        assert!((uv[0] - 0.5).abs() < 1e-10);
        assert!((uv[1] - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_offset() {
        // 仅平移时 UV 逐轴加上 offset
        let transform = TextureTransform {
            offset: [0.1, 0.2],
            ..Default::default()
        };
        let uv = transform.transform_uv(0.5, 0.5);
        assert!((uv[0] - 0.6).abs() < 1e-10);
        assert!((uv[1] - 0.7).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_scale() {
        // 仅缩放时 UV 逐轴乘以 scale
        let transform = TextureTransform {
            scale: [2.0, 3.0],
            ..Default::default()
        };
        let uv = transform.transform_uv(0.5, 0.5);
        assert!((uv[0] - 1.0).abs() < 1e-10);
        assert!((uv[1] - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_rotation_90() {
        // 旋转 90° 时 (1,0) 应变为 (0,1)
        let transform = TextureTransform {
            rotation: std::f64::consts::FRAC_PI_2,
            ..Default::default()
        };
        let uv = transform.transform_uv(1.0, 0.0);
        // cos(90°)=0, sin(90°)=1 → (0*1 - 1*0, 1*1 + 0*0) = (0, 1)
        assert!(uv[0].abs() < 1e-10);
        assert!((uv[1] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_matrix_identity() {
        // 默认变换的矩阵应为列主序单位阵
        let transform = TextureTransform::default();
        let m = transform.compute_matrix();
        // 列主序单位矩阵
        assert!((m[0] - 1.0).abs() < 1e-10);
        assert!((m[4] - 1.0).abs() < 1e-10);
        assert!((m[8] - 1.0).abs() < 1e-10);
        assert!(m[1].abs() < 1e-10);
        assert!(m[3].abs() < 1e-10);
    }

    #[test]
    fn test_texture_transform_info_effective_tex_coord() {
        // 存在 transform 覆盖时取覆盖的 texCoord
        let info = TextureTransformInfo {
            index: 0,
            tex_coord: 0,
            extensions: Some(TextureTransformExtensions {
                texture_transform: Some(TextureTransform {
                    tex_coord: Some(1),
                    ..Default::default()
                }),
            }),
            scale: None,
        };
        assert_eq!(info.effective_tex_coord(), 1);
    }

    #[test]
    fn test_texture_transform_info_no_override() {
        // 无覆盖时回落到结构体自身 tex_coord
        let info = TextureTransformInfo {
            index: 0,
            tex_coord: 2,
            extensions: None,
            scale: None,
        };
        assert_eq!(info.effective_tex_coord(), 2);
    }

    #[test]
    fn test_extended_material_serde_roundtrip() {
        // ExtendedMaterial 序列化后再反序列化应保持字段一致
        let mat = ExtendedMaterial {
            clearcoat: Some(Clearcoat {
                clearcoat_factor: 0.5,
                ..Default::default()
            }),
            unlit: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&mat).unwrap();
        let parsed: ExtendedMaterial = serde_json::from_str(&json).unwrap();
        assert!(parsed.unlit);
        assert!((parsed.clearcoat.unwrap().clearcoat_factor - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_parse_sheen_extension() {
        // 从 JSON 解析 sheen 颜色与粗糙度
        let json = serde_json::json!({
            "KHR_materials_sheen": {
                "sheenColorFactor": [0.5, 0.3, 0.1],
                "sheenRoughnessFactor": 0.8
            }
        });

        let mat = parse_material_extensions(&json);
        let sheen = mat.sheen.unwrap();
        assert_eq!(sheen.sheen_color_factor, [0.5, 0.3, 0.1]);
        assert!((sheen.sheen_roughness_factor - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_parse_volume_extension() {
        // 从 JSON 解析 volume 厚度/衰减距离/衰减色
        let json = serde_json::json!({
            "KHR_materials_volume": {
                "thicknessFactor": 2.0,
                "attenuationDistance": 5.0,
                "attenuationColor": [0.9, 0.8, 0.7]
            }
        });

        let mat = parse_material_extensions(&json);
        let vol = mat.volume.unwrap();
        assert!((vol.thickness_factor - 2.0).abs() < 1e-10);
        assert!((vol.attenuation_distance - 5.0).abs() < 1e-10);
        assert_eq!(vol.attenuation_color, [0.9, 0.8, 0.7]);
    }
}
