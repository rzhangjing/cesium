//! glTF 1.0 technique / material → glTF 2.0 PBR material 迁移。
//!
//! 涵盖 technique 渲染状态迁移、techniques/programs/shaders 打包进
//! `KHR_techniques_webgl` 扩展，以及两个遗留扩展（technique 与
//! `KHR_materials_common`）向 PBR material 的转换。
//!
//! 它们在结构化的 1.0 → 2.0 变换之后运行，因此顶层集合
//! 已是数组。一切都在原始 [`serde_json::Value`] 上操作。

use serde_json::{json, Map, Value};
use std::collections::HashMap;

use crate::gltf_upgrade_util::{
    add_extensions_required, add_extensions_used, add_to_array, for_each_material, is_texture,
    is_vec4, remove_extension, srgb_to_linear, webgl,
};

/// glTF 1.0 technique material 中指示 base color *texture* 的 uniform 名称。
pub(crate) const DEFAULT_BASE_COLOR_TEXTURE_NAMES: [&str; 4] =
    ["u_tex", "u_diffuse", "u_emission", "u_diffuse_tex"];

/// 指示 base color *factor*（一个 sRGB vec4）的 uniform 名称。
pub(crate) const DEFAULT_BASE_COLOR_FACTOR_NAMES: [&str; 2] = ["u_diffuse", "u_diffuse_mat"];

/// `KHR_blend` 接受的混合因子集合。
const SUPPORTED_BLEND_FACTORS: [u64; 10] = [
    webgl::ZERO,
    webgl::ONE,
    webgl::SRC_COLOR,
    webgl::ONE_MINUS_SRC_COLOR,
    webgl::SRC_ALPHA,
    webgl::ONE_MINUS_SRC_ALPHA,
    webgl::DST_ALPHA,
    webgl::ONE_MINUS_DST_ALPHA,
    webgl::DST_COLOR,
    webgl::ONE_MINUS_DST_COLOR,
];

/// 将 glTF 1.0 technique 渲染状态移动到
/// glTF 2.0 material 属性（`alphaMode` / `doubleSided`）和 `KHR_blend`
/// 扩展，然后删除 `technique.states`。
pub(crate) fn move_technique_render_states(gltf: &mut Value) {
    // 无 techniques 集合则本阶段无事可做
    if gltf.get("techniques").is_none() {
        return;
    }

    // 暂存每个 technique 的混合参数与目标 material 属性改写
    let mut blending_for_technique: HashMap<usize, Value> = HashMap::new();
    let mut material_props_for_technique: HashMap<usize, Map<String, Value>> = HashMap::new();

    // 遍历每个 technique，解析其 states 渲染状态
    if let Some(Value::Array(techniques)) = gltf.get_mut("techniques") {
        for (index, technique) in techniques.iter_mut().enumerate() {
            let Some(states) = technique.get("states").cloned() else {
                continue;
            };
            // 读取 enable 能力列表（GL 常量）
            let enable: Vec<u64> = states
                .get("enable")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_u64).collect())
                .unwrap_or_default();

            // 依据 enable 列表推导目标 material 属性
            let mut props = Map::new();
            if enable.contains(&webgl::BLEND) {
                // 启用了混合：设为 BLEND 模式并抽取混合方程/因子
                props.insert("alphaMode".to_string(), Value::String("BLEND".to_string()));
                if let Some(functions) = states.get("functions") {
                    // 仅当存在混合方程或因子时才记录混合参数
                    let has_equation = functions.get("blendEquationSeparate").is_some();
                    let has_func = functions.get("blendFuncSeparate").is_some();
                    if has_equation || has_func {
                        let blend_equation = functions
                            .get("blendEquationSeparate")
                            .cloned()
                            .unwrap_or_else(|| json!([webgl::FUNC_ADD, webgl::FUNC_ADD]));
                        let blend_factors =
                            supported_blend_factors(functions.get("blendFuncSeparate"));
                        blending_for_technique.insert(
                            index,
                            json!({ "blendEquation": blend_equation, "blendFactors": blend_factors }),
                        );
                    }
                }
            }
            // 未启用背面剔除即视为双面
            if !enable.contains(&webgl::CULL_FACE) {
                props.insert("doubleSided".to_string(), Value::Bool(true));
            }
            material_props_for_technique.insert(index, props);

            // 状态已转出，删除遗留 states
            if let Some(obj) = technique.as_object_mut() {
                obj.remove("states");
            }
        }
    }

    // 若任一 technique 带混合，则声明 KHR_blend 扩展
    if !blending_for_technique.is_empty() {
        // 先确保根 extensions 存在
        if let Some(obj) = gltf.as_object_mut() {
            obj.entry("extensions")
                .or_insert_with(|| Value::Object(Map::new()));
        }
        add_extensions_used(gltf, "KHR_blend");
    }

    // 将暂存的属性与混合扩展写回每个 material
    for_each_material(gltf, &mut |material| {
        // 按 material 引用的 technique 索引取回暂存数据
        let Some(tech_index) = material.get("technique").and_then(Value::as_u64) else {
            return;
        };
        let tech_index = tech_index as usize;
        if let Some(props) = material_props_for_technique.get(&tech_index) {
            if let Some(obj) = material.as_object_mut() {
                // 把推导出的 alphaMode/doubleSided 写回 material
                for (key, value) in props {
                    obj.insert(key.clone(), value.clone());
                }
            }
        }
        if let Some(blending) = blending_for_technique.get(&tech_index) {
            // 有混合参数则附加 KHR_blend 扩展
            if let Some(obj) = material.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    exts.insert("KHR_blend".to_string(), blending.clone());
                }
            }
        }
    });
}

/// 当四个混合因子都受支持时返回该值，否则返回默认值。
fn supported_blend_factors(value: Option<&Value>) -> Value {
    // 默认混合因子：ONE, ZERO, ONE, ZERO
    let default = json!([webgl::ONE, webgl::ZERO, webgl::ONE, webgl::ZERO]);
    let Some(arr) = value.and_then(Value::as_array) else {
        return default;
    };
    // 少于四个分量则回退默认
    if arr.len() < 4 {
        return default;
    }
    // 四个因子必须全部在白名单内
    for item in arr.iter().take(4) {
        // 任一因子不在白名单即回退默认
        match item.as_u64() {
            Some(x) if SUPPORTED_BLEND_FACTORS.contains(&x) => {}
            _ => return default,
        }
    }
    // 全部通过则返回原始四因子
    value.cloned().unwrap_or(default)
}

/// 将 glTF 1.0 techniques / programs / shaders
/// 移入 `KHR_techniques_webgl` 扩展，并将每个 material 的
/// `technique` + `values` 重写为 `material.extensions.KHR_techniques_webgl`。
pub(crate) fn move_techniques_to_extension(gltf: &mut Value) {
    // techniqueId -> (parameterName -> uniformName)
    let mut mapped_uniforms: HashMap<usize, HashMap<String, String>> = HashMap::new();
    // 旧 technique 索引 -> 扩展内的新索引
    let mut updated_technique_indices: HashMap<usize, usize> = HashMap::new();
    // 旧 program 索引 -> 扩展内的新索引
    let mut seen_programs: HashMap<u64, usize> = HashMap::new();

    if gltf.get("techniques").is_some() {
        // 摘出临时保存的 GL 扩展字符串，稍后挂到每个 program 上
        let gl_extensions = gltf
            .as_object_mut()
            .and_then(|o| o.remove("glExtensionsUsed"));

        // 快照遗留集合，以便构建扩展时
        // 不借用根对象。下方 cloned 一次即与 gltf 解偶。
        let techniques = gltf.get("techniques").cloned().unwrap_or(Value::Null);
        let programs = gltf.get("programs").cloned().unwrap_or(Value::Null);
        let shaders = gltf.get("shaders").cloned().unwrap_or(Value::Null);

        let mut ext_programs: Vec<Value> = Vec::new();
        let mut ext_shaders: Vec<Value> = Vec::new();
        let mut ext_techniques: Vec<Value> = Vec::new();

        if let Value::Array(technique_list) = techniques {
            // 逐个 technique 重建为扩展内条目，并累积 program/shader 去重表
            for (technique_id, technique_legacy) in technique_list.iter().enumerate() {
                // parameters 表记录每个参数类型/语义，供 attributes 与 uniforms 查询
                let parameters = technique_legacy.get("parameters").cloned().unwrap_or(Value::Null);

                // 重建 attributes：把 attribute 名映射到其 parameter 的语义
                let mut new_attributes = Map::new();
                if let Some(attrs) = technique_legacy.get("attributes").and_then(Value::as_object) {
                    for (attribute_name, parameter_name) in attrs {
                        let Some(parameter_name) = parameter_name.as_str() else { continue };
                        // 从 parameters 表取出该参数的 semantic 字段
                        let semantic = parameters
                            .get(parameter_name)
                            .and_then(|p| p.get("semantic"))
                            .cloned();
                        let mut entry = Map::new();
                        // 有语义则写入 attribute 条目
                        if let Some(semantic) = semantic {
                            entry.insert("semantic".to_string(), semantic);
                        }
                        new_attributes
                            .insert(attribute_name.clone(), Value::Object(entry));
                    }
                }

                // 重建 uniforms，同时记录 parameter→uniform 名称映射，供后续改写 material.values
                let mut new_uniforms = Map::new();
                let mut param_to_uniform = HashMap::new();
                if let Some(uniforms) = technique_legacy.get("uniforms").and_then(Value::as_object) {
                    for (uniform_name, parameter_name) in uniforms {
                        let Some(parameter_name) = parameter_name.as_str() else { continue };
                        let parameter_legacy = parameters.get(parameter_name).cloned().unwrap_or(Value::Null);
                        let mut entry = Map::new();
                        // 仅搬运扩展保留的参数字段
                        // （未列出的遗留字段在此丢弃）
                        for field in ["count", "node", "type", "semantic", "value"] {
                            if let Some(v) = parameter_legacy.get(field) {
                                entry.insert(field.to_string(), v.clone());
                            }
                        }
                        new_uniforms.insert(uniform_name.clone(), Value::Object(entry));
                        param_to_uniform
                            .insert(parameter_name.to_string(), uniform_name.clone());
                    }
                }
                // 保存本 technique 的 parameter→uniform 映射
                mapped_uniforms.insert(technique_id, param_to_uniform);

                // 组装新的 technique 条目：名称 + attributes + uniforms
                let mut technique = Map::new();
                if let Some(name) = technique_legacy.get("name") {
                    technique.insert("name".to_string(), name.clone());
                }
                // 挂入重建后的 attributes 与 uniforms
                technique.insert("attributes".to_string(), Value::Object(new_attributes));
                technique.insert("uniforms".to_string(), Value::Object(new_uniforms));

                // 解析 program 索引：已见过的复用，未见过的去重建入扩展
                let legacy_program = technique_legacy.get("program").and_then(Value::as_u64);
                let program_index = match legacy_program {
                    Some(p) if seen_programs.contains_key(&p) => seen_programs[&p],
                    Some(p) => {
                        let program_legacy = index_value(&programs, p);
                        let mut program = Map::new();
                        if let Some(name) = program_legacy.get("name") {
                            program.insert("name".to_string(), name.clone());
                        }
                        // 把摘出的 GL 扩展附到该 program
                        if let Some(gl_ext) = &gl_extensions {
                            program.insert("glExtensions".to_string(), gl_ext.clone());
                        }
                        // 片元 shader：搬运并按内容去重加入扩展 shaders
                        let fs = program_legacy
                            .get("fragmentShader")
                            .and_then(Value::as_u64)
                            .map(|i| index_value(&shaders, i).clone())
                            .unwrap_or(Value::Null);
                        let vs = program_legacy
                            .get("vertexShader")
                            .and_then(Value::as_u64)
                            .map(|i| index_value(&shaders, i).clone())
                            .unwrap_or(Value::Null);
                        // true 表示按内容去重加入 shader 列表
                        let fs_index = add_to_array(&mut ext_shaders, fs, true);
                        let vs_index = add_to_array(&mut ext_shaders, vs, true);
                        // 用扩展内的新 shader 索引回填 program
                        program.insert("fragmentShader".to_string(), json!(fs_index));
                        program.insert("vertexShader".to_string(), json!(vs_index));
                        let new_program_index =
                            add_to_array(&mut ext_programs, Value::Object(program), false);
                        // 登记旧→新 program 索引，实现去重
                        seen_programs.insert(p, new_program_index);
                        new_program_index
                    }
                    None => 0,
                };
                technique.insert("program".to_string(), json!(program_index));

                // false 表示 technique 不去重，逐个追加
                let new_technique_index =
                    add_to_array(&mut ext_techniques, Value::Object(technique), false);
                // 登记旧→新 technique 索引，供 material 改写引用
                updated_technique_indices.insert(technique_id, new_technique_index);
            }
        }

        // 若收集到任何 technique，则打包成 KHR_techniques_webgl 扩展挂到根
        if !ext_techniques.is_empty() {
            let extension = json!({
                "programs": ext_programs,
                "shaders": ext_shaders,
                "techniques": ext_techniques,
            });
            // 确保根 extensions 对象存在后写入
            if let Some(obj) = gltf.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    exts.insert("KHR_techniques_webgl".to_string(), extension);
                }
            }
            add_extensions_used(gltf, "KHR_techniques_webgl");
            add_extensions_required(gltf, "KHR_techniques_webgl");
        }
    }

    // 重写 materials。
    for_each_material(gltf, &mut |material| {
        let tech_index = material.get("technique").and_then(Value::as_u64);
        if let Some(tech_index) = tech_index {
            // 构建 material 级 KHR_techniques_webgl 扩展：technique 新索引 + 改写后的 values
            let mut material_extension = Map::new();
            if let Some(new_index) = updated_technique_indices.get(&(tech_index as usize)) {
                material_extension.insert("technique".to_string(), json!(*new_index));
            }
            // 把 material.values 的参数名逐一经映射改写为 uniform 名
            let values = material.get("values").cloned().unwrap_or(Value::Null);
            if let Value::Object(values) = values {
                let mut new_values = Map::new();
                let param_map = mapped_uniforms.get(&(tech_index as usize));
                for (parameter_name, value) in values {
                    // 命中映射才搬运，未知参数名丢弃
                    if let Some(uniform_name) = param_map.and_then(|m| m.get(&parameter_name)) {
                        new_values.insert(uniform_name.clone(), value);
                    }
                }
                if !new_values.is_empty() {
                    material_extension.insert("values".to_string(), Value::Object(new_values));
                }
            }
            if let Some(obj) = material.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    exts.insert(
                        "KHR_techniques_webgl".to_string(),
                        Value::Object(material_extension),
                    );
                }
                // 顶层 technique/values 已入扩展，删除原字段
                obj.remove("technique");
                obj.remove("values");
            }
        }
    });

    // 顶层遗留集合已迁入扩展，删除之
    if let Some(obj) = gltf.as_object_mut() {
        obj.remove("techniques");
        obj.remove("programs");
        obj.remove("shaders");
    }
}

/// 对于数组形式的集合读取 `collection[index]`（`objectsToArrays` 之后）。
fn index_value(collection: &Value, index: u64) -> &Value {
    // 越界时回退 Null，调用方据此跳过
    collection
        .as_array()
        .and_then(|a| a.get(index as usize))
        .unwrap_or(&Value::Null)
}

/// 从常见的 glTF 1.0 technique uniform 名称构建 PBR base color，然后丢弃遗留扩展。
pub(crate) fn convert_techniques_to_pbr(
    gltf: &mut Value,
    base_color_texture_names: &[String],
    base_color_factor_names: &[String],
) {
    // 遍历每个 material 从其遗留 values 提取 base color
    for_each_material(gltf, &mut |material| {
        let values = collect_material_values(material);
        // 按 uniform 名匹配 base color 纹理/因子，命中则写入 PBR 字段
        for (name, value) in values {
            if base_color_texture_names.contains(&name) && is_texture(&value) {
                initialize_pbr_material(material);
                set_pbr_field(material, "baseColorTexture", value);
            } else if base_color_factor_names.contains(&name) && is_vec4(&value) {
                // base color 因子以 sRGB 存放，需转线性后再写入
                let linear: Vec<f64> = value
                    .as_array()
                    .map(|a| a.iter().filter_map(Value::as_f64).collect())
                    .unwrap_or_default();
                let linear = srgb_to_linear(&linear);
                // 转换后写入线性 baseColorFactor
                initialize_pbr_material(material);
                set_pbr_field(material, "baseColorFactor", json!(linear));
            }
        }
    });

    // 遗留扩展已消化，删除之
    remove_extension(gltf, "KHR_techniques_webgl");
    remove_extension(gltf, "KHR_blend");
}

/// material 的值要么位于
/// `material.values`（glTF 1.0），要么位于 `material.extensions.KHR_techniques_webgl.values`。
fn collect_material_values(material: &Value) -> Vec<(String, Value)> {
    // 优先取扩展内 values，回退到 glTF 1.0 顶层 values
    let values = material
        .pointer("/extensions/KHR_techniques_webgl/values")
        .or_else(|| material.get("values"));
    match values.and_then(Value::as_object) {
        // 命中对象则克隆为 (名,值) 列表，否则空
        Some(obj) => obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        None => Vec::new(),
    }
}

/// 确保 `pbrMetallicRoughness`
/// 存在且 `roughnessFactor = 1.0`、`metallicFactor = 0.0`。
fn initialize_pbr_material(material: &mut Value) {
    let Some(obj) = material.as_object_mut() else { return };
    // 缺省 pbr 块时新建，再写入完全粗糙、非金属的默认值
    let pbr = obj
        .entry("pbrMetallicRoughness")
        .or_insert_with(|| Value::Object(Map::new()));
    if let Some(pbr) = pbr.as_object_mut() {
        // 默认完全粗糙、非金属
        pbr.insert("roughnessFactor".to_string(), json!(1.0));
        pbr.insert("metallicFactor".to_string(), json!(0.0));
    }
}

/// 在 material 的 `pbrMetallicRoughness` 对象上写入一个字段（仅当该对象存在）。
fn set_pbr_field(material: &mut Value, field: &str, value: Value) {
    // pbr 块存在才写入，避免误建空结构
    if let Some(pbr) = material
        .as_object_mut()
        .and_then(|o| o.get_mut("pbrMetallicRoughness"))
        .and_then(Value::as_object_mut)
    {
        pbr.insert(field.to_string(), value);
    }
}

/// 将
/// `KHR_materials_common` 扩展转换为 PBR material（为 `CONSTANT` technique
/// 添加 `KHR_materials_unlit`），然后丢弃该扩展。
pub(crate) fn convert_materials_common_to_pbr(gltf: &mut Value) {
    // 记录是否出现过 CONSTANT technique，决定是否声明 unlit 扩展
    let mut used_unlit = false;

    for_each_material(gltf, &mut |material| {
        let common = material
            .pointer("/extensions/KHR_materials_common")
            .cloned();
        let Some(common) = common else { return };

        // 抽取遗留 common 的 ambient/diffuse/emission 及各开关
        let values = common.get("values").cloned().unwrap_or(Value::Null);
        let ambient = values.get("ambient").cloned();
        let diffuse = values.get("diffuse").cloned();
        let emission = values.get("emission").cloned();
        let transparency = values.get("transparency").and_then(Value::as_f64);
        let double_sided = common.get("doubleSided").and_then(Value::as_bool);
        let transparent = common.get("transparent").and_then(Value::as_bool);
        let technique = common.get("technique").and_then(Value::as_str).map(str::to_string);

        initialize_pbr_material(material);

        // CONSTANT technique 无光照：走 unlit，把 emission/ambient 当 base color
        if technique.as_deref() == Some("CONSTANT") {
            used_unlit = true;
            // 确保 material.extensions 存在后注入 unlit 标记
            if let Some(obj) = material.as_object_mut() {
                let exts = obj
                    .entry("extensions")
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Some(exts) = exts.as_object_mut() {
                    // 标记该 material 为 unlit
                    exts.insert(
                        "KHR_materials_unlit".to_string(),
                        Value::Object(Map::new()),
                    );
                }
            }
            assign_as_base_color(material, emission.as_ref());
            // 依次尝试 emission、ambient 作为 base color 来源
            assign_as_base_color(material, ambient.as_ref());
        } else {
            // 其余（BLINN/PHONG）：diffuse 作 base color，ambient/emission 累加为 emissive
            assign_as_base_color(material, diffuse.as_ref());
            assign_as_emissive(material, ambient.as_ref());
            assign_as_emissive(material, emission.as_ref());
        }

        if let Some(obj) = material.as_object_mut() {
            // doubleSided 直接透传
            if let Some(double_sided) = double_sided {
                obj.insert("doubleSided".to_string(), Value::Bool(double_sided));
            }
            // transparency 乘入 baseColorFactor 的 alpha 通道
            if let Some(transparency) = transparency {
                // 读取已存在的 baseColorFactor 以便合并 alpha
                let existing = obj
                    .get("pbrMetallicRoughness")
                    .and_then(|p| p.get("baseColorFactor"))
                    .and_then(Value::as_array)
                    .cloned();
                // 已有 vec4 因子则缩放其 alpha，否则以 transparency 新建
                let factor = match existing {
                    Some(mut f) if f.len() == 4 => {
                        let alpha = f[3].as_f64().unwrap_or(1.0) * transparency;
                        f[3] = json!(alpha);
                        f
                    }
                    _ => vec![json!(1.0), json!(1.0), json!(1.0), json!(transparency)],
                };
                if let Some(pbr) = obj
                    .get_mut("pbrMetallicRoughness")
                    .and_then(Value::as_object_mut)
                {
                    pbr.insert("baseColorFactor".to_string(), Value::Array(factor));
                }
            }
            // transparent 开关映射为 BLEND/OPAQUE
            if let Some(transparent) = transparent {
                obj.insert(
                    "alphaMode".to_string(),
                    Value::String(if transparent { "BLEND" } else { "OPAQUE" }.to_string()),
                );
            }
        }
    });

    if used_unlit {
        add_extensions_used(gltf, "KHR_materials_unlit");
    }
    // 遗留 common 扩展已消化，删除之
    remove_extension(gltf, "KHR_materials_common");
}

/// 将一个 vec4 或 texture 值作为 PBR baseColorFactor / baseColorTexture 赋给 material。
fn assign_as_base_color(material: &mut Value, base_color: Option<&Value>) {
    let Some(base_color) = base_color else { return };
    // vec4：sRGB 转线性后作 baseColorFactor
    if is_vec4(base_color) {
        let rgba: Vec<f64> = base_color
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_f64).collect())
            .unwrap_or_default();
        let linear = srgb_to_linear(&rgba);
        set_pbr_field(material, "baseColorFactor", json!(linear));
    } else if is_texture(base_color) {
        // texture 引用：原样作 baseColorTexture
        set_pbr_field(material, "baseColorTexture", base_color.clone());
    }
}

/// 将一个 vec4 值经 sRGB→线性转换后作为 emissiveFactor 赋给 material。
fn assign_as_emissive(material: &mut Value, emissive: Option<&Value>) {
    let Some(emissive) = emissive else { return };
    // 取 vec4 前三分量作 emissiveFactor
    if is_vec4(emissive) {
        let rgb: Vec<f64> = emissive
            .as_array()
            .map(|a| a.iter().take(3).filter_map(Value::as_f64).collect())
            .unwrap_or_default();
        if let Some(obj) = material.as_object_mut() {
            obj.insert("emissiveFactor".to_string(), json!(rgb));
        }
    } else if is_texture(emissive) {
        // texture 形式的 emissive 作 emissiveTexture
        if let Some(obj) = material.as_object_mut() {
            obj.insert("emissiveTexture".to_string(), emissive.clone());
        }
    }
}
