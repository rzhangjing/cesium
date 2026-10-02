//! Orbit 相机控制器 —— 鼠标拖拽旋转，滚轮缩放。
//!
//! 模仿 CesiumJS 默认 ScreenSpaceCameraController 的行为：
//! 左键拖拽绕地球轨道运行，滚轮放大/缩小。
//!
//! 地球处于 ECEF 朝向（北极在 +Z，赤道在 XY 平面），
//! 因此相机绕 Z（极地）轴轨道运行并以 Z 为“上”。
//!
//! ## M2.4 薄壳委派
//!
//! 旋转惯性与飞行插值委派给领域层
//! （`cesium_interaction::{InertiaController, InertiaSample, decay, CameraFlight,
//! compute_flight_duration, select_flight_easing}`）。抓地跟踪
//! 公式与缩放惯性滑行均逐字节保留自久经考验的
//! M0 实现。

use bevy::core_pipeline::bloom::Bloom;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::view::RenderLayers;
use cesium_plot_bevy::PlotInputCapture;
use glam::{DQuat, DVec2, DVec3};
use cesium_interaction::{
    CameraFlight, InertiaController, InertiaSample, InertiaState,
    INERTIA_MAX_CLICK_TIME_THRESHOLD, compute_flight_duration, select_flight_easing,
};

use crate::feature_flags::{postprocess_builtin_enabled, postprocess_enabled};
use crate::map2d::{map_is_3d, MapMode};

/// 相机垂直视场角（弧度）。在 spawn 的投影与拖拽数学之间保持同步，
/// 以使抓地跟踪精确。
pub const CAMERA_FOV_Y: f32 = std::f32::consts::FRAC_PI_3; // 60 度
/// 近平面 —— 保持在相机最近 `min_distance` 高度（≈100 m）远下方，
/// 因此当完全缩小以检视最细（亚米每 texel）瓦片时，正下方的地面仍可见。
/// Reversed-Z（wgpu 默认）能容忍由此产出的 ~1:6.7e8 近:远比而不产生
/// 表面 z-fighting（同层瓦片永不重叠；skirt 下降 + tuck 步长吸收了
/// 掠射角下剩余的深度精度）。
const CAMERA_NEAR: f32 = 0.0000005;
/// 远平面 —— 大到足以容纳星空（半径 ~50）。
const CAMERA_FAR: f32 = 200.0;
/// 地球（赤道）半径，以 render unit 计。
const GLOBE_RADIUS: f32 = 1.0;

/// WGS84 椭球半轴，以 render unit 计。瓦片渲染器在 WGS84 椭球上
/// 细分地面 —— 赤道 `ELLIPSOID_A`，极地 `ELLIPSOID_B`，极轴 = 局部 +Z
/// （参见 `tile_mesh::create_tile_mesh_uv`，其顶点 `z` 携带 `(1 - e²)` 因子）。
/// 抓地拖拽拾取必须与此 SAME 表面相交：在深缩放下相机仅位于
/// 地面之上 ~1e-3 render unit，因此单位球体与椭球之间 ~0.2% 的径向
/// 间隙（在中/高纬度最大）否则会膨胀为巨大的屏幕空间增益误差 ——
/// 即实测的 ~0.58× “不跟手”过冲。
const ELLIPSOID_A: f64 = 1.0; // = GLOBE_RADIUS (EARTH_RADIUS / METERS_PER_RENDER_UNIT)
const ELLIPSOID_B: f64 = 6356752.314245 / 6378137.0; // ≈ 0.99664719（极地 / 赤道）

/// 每拖拽帧的绝对地心角上限（弧度）—— 一个与缩放无关的仅兼底值。
/// 纯弧度上限在深缩放下太紧：在 `min_distance` 处 0.1 rad 对应 ≲ 20 px
/// 的屏幕平移，因此一个普通快速拖拽会被它限流，地图每帧都落后
/// 光标 10-20 px（即实测的“不跟手”）。真正的舒适度限制以每帧像素
/// 表达（`MAX_PAN_PX_PER_FRAME`）并转换为一个感知缩放的角；此弧度值
/// 仅保持两个上限有序（在最近的缩放处 px 上限 ≤ rad 上限）并防止
/// 原始运动路径失控。
const MAX_DRAG_STEP_RAD: f32 = 0.5;

/// 每个拖拽步骤的屏幕平移上限（像素 / 帧）—— 包括固定的抓地
/// 修正与原始运动回退，不论在窗口内与否。弧度上限与缩放无关 ——
/// 在最大缩放下 0.1 rad 仅 ~20 px 平移，这会在地表限流快速拖拽并使
/// 地图落后光标 —— 因此每帧的旋转额外被限定为从不平移超过这么多像素
/// （然后通过 `px · surface_dist / focal` 转换为感知缩放的角）。此值舒适地
/// 高于任何真正的单帧鼠标运动（一次极快的拂动 ≈ 150 px/帧），因此
/// 普通拖拽在所有缩放级别都保持精确 1:1；只有病态尖峰（合并的输入
/// 风暴、接近极点拾取导致经线收敛）会在几帧内被收敛而不是传送视图
/// 或将相机抛过地球。
const MAX_PAN_PX_PER_FRAME: f32 = 600.0;

/// 旋转惯性滑行的默认衰减系数（CesiumJS
/// `inertiaSpin` 默认 ≈ 0.9）。
const INERTIA_SPIN_COEFFICIENT: f64 = 0.9;

/// orbit 控制相机的标记组件。
#[derive(Component)]
pub struct OrbitCamera;

/// 保存 orbit 状态（围绕目标的球坐标）的资源。
#[derive(Resource)]
pub struct OrbitState {
    /// 方位角，以弧度计（绕地球 Z/极地轴的旋转）。
    pub heading: f32,
    /// 赤道（XY）平面上方的仰角，以弧度计。
    /// 正 = 赤道以北，负 = 以南。
    pub pitch: f32,
    /// 距目标的距离，以 render unit 计。
    pub distance: f32,
    /// 滚轮输入立即移动此值；`distance` 每帧向它滑行（指数缓动），
    /// 从而产生 CesiumJS 风格的缩放惯性，而非每格滚轮硬跳 30%。
    pub target_distance: f32,
    /// orbit 目标（世界空间，地球中心）。
    pub target: Vec3,
    /// 总体旋转灵敏度乘子（1.0 = 由相机几何导出的精确 1:1 表面
    /// 跟踪）。
    pub rotate_speed: f32,
    /// 缩放灵敏度：每单位滚轮的高度-相对-表面变化分数（0.3 = 每格
    /// 向表面靠近/远离 30%）。
    pub zoom_speed: f32,
    /// 最小缩放距离（略高于表面，以便你能检视细节）。
    pub min_distance: f32,
    /// 最大缩放距离。
    pub max_distance: f32,
}

impl Default for OrbitState {
    /// 默认视角：俯角 ~23°、距离 3 个渲染单位，旋转/缩放速度取交互手感值。
    fn default() -> Self {
        Self {
            heading: 0.0,
            pitch: 0.4, // ~23 度，赤道以北
            distance: 3.0,
            target_distance: 3.0,
            target: Vec3::ZERO,
            rotate_speed: 1.0, // 默认精确几何跟踪
            zoom_speed: 0.3,
            min_distance: 1.0000157, // 下降到 ~100 m 高度 -> level ~20 tiles
            max_distance: 20.0,
        }
    }
}

// ── M2.4 旋转惯性状态 ────────────────────────────────────

/// 跟踪 orbit 相机旋转惯性的资源。
///
/// 当一次左键拖拽在一次快速拂动（< [`INERTIA_MAX_CLICK_TIME_THRESHOLD`]
/// 秒）后释放时，上一帧的 heading/pitch 速度被捕获进领域
/// [`InertiaController`] 并以指数衰减每帧滑行。
#[derive(Resource)]
pub struct OrbitInertiaState {
    /// 领域惯性控制器（纯 f64 数学，无 Bevy 依赖）。
    pub controller: InertiaController,
    /// 上一帧左键是否按下。
    was_dragging: bool,
    /// 用于计算增量的上一帧方位角。
    prev_heading: f32,
    /// 用于计算增量的上一帧仰角。
    prev_pitch: f32,
    /// 经过的毫秒数（用于惯性计时的单调时钟）。
    now_ms: f64,
    /// 当前拖拽开始的时间戳（ms）。
    press_time_ms: f64,
    /// 当前拖拽释放的时间戳（ms）。
    release_time_ms: f64,
    /// 惯性滑行是否处于激活状态。
    coasting: bool,
    /// 释放时捕获的每弧度像素 heading 尺度。领域
    /// [`InertiaController`] 在像素空间滑行，因此滑行的像素增量用与
    /// 捕获时*相同*的尺度转回弧度，给出原始弧度速度的精确指数
    /// 衰减。
    capture_scale_h: f32,
    /// 释放时捕获的每弧度像素 pitch 尺度（见上）。
    capture_scale_p: f32,
}

impl Default for OrbitInertiaState {
    /// 默认惯性状态：新建控制器，历史角度与时间戳归零。
    fn default() -> Self {
        Self {
            controller: InertiaController::new(),
            was_dragging: false,
            prev_heading: 0.0,
            prev_pitch: 0.4,
            now_ms: 0.0,
            press_time_ms: 0.0,
            release_time_ms: 0.0,
            coasting: false,
            capture_scale_h: 1.0,
            capture_scale_p: 1.0,
        }
    }
}

// ── M2.4 飞行状态 ────────────────────────────────────────

/// 保存 orbit 相机一个活动的大圆航线相机飞行的资源。
///
/// 当飞行处于活动状态时，orbit 状态由领域 [`CameraFlight`] 的 slerp
/// 插值驱动，而非鼠标输入。
#[derive(Resource, Default)]
pub struct OrbitFlightState {
    /// 当前活动飞行（若有）。
    pub flight: Option<CameraFlight>,
}

/// 请求 orbit 相机飞向一个 ECEF 目的点（米）的事件。
///
/// 发送此事件以触发一次大圆航线飞行，其时长与缓动由领域的
/// [`compute_flight_duration`] 和 [`select_flight_easing`] 自动导出。
#[derive(Event)]
pub struct OrbitFlyToRequest {
    /// ECEF 米下的目标位置。
    pub destination_ecef: DVec3,
}

// ── FIX-ARCBALL：实时 trackball 朝向 ────────────────────────

/// 交互相机的实时 arcball（trackball）朝向。
///
/// `engaged` 在用户在窗口化会话中首次拖拽左键时翻为 `true`，
/// 然后保持 `true`，因此位姿 —— 包括 roll 与越极视图 —— 在帧之间
/// 持续存在。每个确定性捕获路径（`FIXED_CAMERA`、`--camera-script`、
/// M2.4 中性测试、v0 基线）从不馈入鼠标，因此 `engaged` 保持 `false`
/// 且 [`orbit_camera_system`] 继续通过纯 [`compute_camera_transform`] 球面
/// 路径驱动相机 → 逐字节不变。
#[derive(Resource)]
struct Arcball {
    /// 相机系架绕目标（地球中心）的旋转。以 f64 保存：在最大
    /// 缩放下每帧的抓地修正约 1e-7 rad，f32 会将其消灭（两个 O(1)
    /// 单位向量之间差异低于 ε 的灾难性抵消 → 一个无头探针显示整个
    /// 拖拽扫过 0.0 rad），使拖拽冻结。f64 保持那个微小残差有意义；
    /// 仅在渲染位姿时将其缩回 f32。
    orientation: DQuat,
    /// 一次真实拖拽是否已接管球面路径。
    engaged: bool,
    /// Geographic 锚点：当前拖拽中光标下抓取点的单位向量
    /// （target → surface）。每帧旋转系架以使该点粘住光标 → 在任何
    /// 抓取位置/缩放下精确跟踪指针。直到在光标下拾取到一个点之前为
    /// `None`。
    anchor: Option<DVec3>,
    /// 上一帧的左键状态，用于检测一次新按下。
    was_pressed: bool,
}

impl Default for Arcball {
    /// 默认单位四元数（IDENTITY）朝向，未吐合、无锚点。
    fn default() -> Self {
        Self {
            orientation: DQuat::IDENTITY,
            engaged: false,
            anchor: None,
            was_pressed: false,
        }
    }
}

// ── M0.1 从环境变量播种相机 ─────────────────────────
// 这是 M3.3 FIXED_CAMERA 的最小前置；M3.3 会用一个合适的配置结构体
// 与验证来正式化该接口。目前我们读取单个环境变量，使捕获 harness
// 能定位相机而无需碰 main.rs 或任何渲染逻辑。
//
// 支持的环境变量（全部可选；未设置 = 像素中性默认）：
//   CESIUM_CAM_LON      — 度数经度（相机位置方位角）
//   CESIUM_CAM_LAT      — 度数纬度（相机位置仰角）
//   CESIUM_CAM_HEIGHT   — render unit 下高于表面的高度（默认地球 R=1）
//   CESIUM_CAM_HEADING  — LON 的别名（若两者都设则优先）
//   CESIUM_CAM_PITCH    — LAT 的别名（若两者都设则优先）
//   CESIUM_CAM_DISTANCE — 直接给定的、距中心的 orbit 距离（覆盖 HEIGHT）
//
// 当这些均未设置时，返回的状态为 `OrbitState::default()`，
// 保证与未修改代码库二进制相同的输出。

/// 从环境变量读取相机播种。当无播种变量存在时返回
/// `OrbitState::default()`（像素中线路径）。
pub(crate) fn orbit_state_from_env() -> OrbitState {
    let mut state = OrbitState::default();
    let mut any_set = false;

    // 助手：从环境变量解析 f32
    let read_f32 = |name: &str| -> Option<f32> {
        std::env::var(name).ok().and_then(|v| v.trim().parse::<f32>().ok())
    };

    // heading：CESIUM_CAM_HEADING 优先于 CESIUM_CAM_LON
    if let Some(h) = read_f32("CESIUM_CAM_HEADING").or(read_f32("CESIUM_CAM_LON")) {
        state.heading = h.to_radians();
        any_set = true;
    }

    // pitch：CESIUM_CAM_PITCH 优先于 CESIUM_CAM_LAT
    if let Some(p) = read_f32("CESIUM_CAM_PITCH").or(read_f32("CESIUM_CAM_LAT")) {
        state.pitch = p.to_radians();
        any_set = true;
    }

    // distance：CESIUM_CAM_DISTANCE 覆盖 HEIGHT
    if let Some(d) = read_f32("CESIUM_CAM_DISTANCE") {
        state.distance = d;
        state.target_distance = d;
        any_set = true;
    } else if let Some(h) = read_f32("CESIUM_CAM_HEIGHT") {
        let d = GLOBE_RADIUS + h;
        state.distance = d;
        state.target_distance = d;
        any_set = true;
    }

    if any_set {
        info!(
            "[camera-seed] env override: heading={:.4} pitch={:.4} distance={:.4}",
            state.heading, state.pitch, state.distance
        );
    }
    state
}

/// 设置 orbit 相机的插件。
pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    /// 插件装配入口：播种初始相机资源，挂载相机/惯性/飞行三段链式
    /// 更新系统（仅在 3D 地图模式下运行）。
    ///
    /// # 参数
    /// - `app`：Bevy 应用。
    fn build(&self, app: &mut App) {
        // M0.1：从环境变量播种初始相机（未设置时像素中性）
        let initial_state = orbit_state_from_env();
        app.insert_resource(initial_state)
            .init_resource::<MapMode>()
            .init_resource::<OrbitInertiaState>()
            .init_resource::<OrbitFlightState>()
            .init_resource::<Arcball>()
            .add_event::<OrbitFlyToRequest>()
            .add_systems(Startup, spawn_orbit_camera)
            .add_systems(
                Update,
                (orbit_camera_system, orbit_inertia_system, orbit_flight_system)
                    .chain()
                    // 根据当前地图模式门控整个 3D 路径。`MapMode`
                    // 默认为 ThreeD，因此在默认（及每个确定性）会话中这
                    // 些系统与以往完全一样运行；它们只在用户切到 2D 地图
                    // 后退让。
                    .run_if(map_is_3d),
            );
    }
}

/// 按当前轨道状态计算变换并 spawn 一个透视投影的 3D 相机实体。
fn spawn_orbit_camera(mut commands: Commands, state: Res<OrbitState>) {
    let transform = compute_camera_transform(&state);
    // 自定义透视投影：一个小近平面使相机能非常接近表面以检视影像
    // 细节，而远平面仍能触及星空。
    let projection = PerspectiveProjection {
        fov: CAMERA_FOV_Y,
        near: CAMERA_NEAR,
        far: CAMERA_FAR,
        ..default()
    };

    // M4.2：当内置后处理门控为 ON 时，启用 HDR 渲染，配合 ACES Fitted
    // 色调映射 + 自然 bloom。HDR 管线在线性空间计算光照，色调映射到
    // LDR，然后为显示作 sRGB 编码。当为 OFF（默认）时，Tonemapping::None
    // 精确保留 v0 基线（CesiumJS 原样显示影像；TonyMcMapFace 需要
    // `tonemapping_luts` feature，而它在本工作区被禁用）。
    //
    // M5-E1：FXAA 位于一个独立门控（CESIUM_ENABLE_POSTPROCESS）。当 ON 时，
    // 相机被标记 `CesiumFxaa`，使渲染图 FXAA 节点在色调映射后运行。
    // 两个门控相互独立：FXAA 可在启用或不启用 HDR/色调映射时开启（它
    // 作用于先前发它的任意 LDR 图像）。
    let mut cam = if postprocess_builtin_enabled() {
        commands.spawn((
            Camera3d::default(),
            Camera {
                hdr: true,
                ..default()
            },
            Tonemapping::AcesFitted,
            Bloom::NATURAL,
            OrbitCamera,
            // 图层 0 = 3D 地球场景，图层 2 = 共享 UI，图层 3 = 标绘
            // overlay。2D 地图相机拥有图层 1，因此两者从不渲染对方的
            // 世界；两只相机都拾取图层 3，因此标绘在两种模式下都显示。
            RenderLayers::from_layers(&[0, 2, 3]),
            Projection::Perspective(projection),
            transform,
        ))
    } else {
        commands.spawn((
            Camera3d::default(),
            Tonemapping::None,
            OrbitCamera,
            RenderLayers::from_layers(&[0, 2, 3]),
            Projection::Perspective(projection),
            transform,
        ))
    };

    // M5-E1：当后处理门控为 ON 时附加 FXAA 触发组件。
    // `fxaa_system`（adapters/bevy-render effects/post_process.rs）每帧将
    // `.enabled` 与 `PostProcessConfig.fxaa_enabled` 保持同步；该标记被提取到
    // 渲染世界并由 `FxaaNode::run` 读取。
    if postprocess_enabled() {
        cam.insert(cesium_bevy_render::effects::CesiumFxaa { enabled: true });
    }
}

/// 系统：读取鼠标输入并更新相机 transform。
///
/// 这是**原始 M0 系统** —— 抓地公式与缩放惯性滑行逐字节保留。
/// 旋转惯性滑行由 [`orbit_inertia_system`] 处理，它运行在此之后。
#[allow(clippy::too_many_arguments)] // Bevy 系统：每个资源/事件/查询一个参数
fn orbit_camera_system(
    mut state: ResMut<OrbitState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut motion_events: EventReader<MouseMotion>,
    mut wheel_events: EventReader<MouseWheel>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<OrbitCamera>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform), With<OrbitCamera>>,
    mut arcball: ResMut<Arcball>,
    capture: Option<Res<PlotInputCapture>>,
) {
    // 标绘 overlay 本帧拥有指针：排空已排队的运动/滚轮事件，以便它们
    // 不会在控制权交回时累积并猛拉相机，然后退让。用 `Option<Res>` 使本
    // 系统在桥接插件未注册（无头 / 标绘禁用）以及在最小单元测试 App 中
    // 仍能运行。
    if capture.is_some_and(|c| c.is_captured()) {
        motion_events.clear();
        wheel_events.clear();
        return;
    }
    // 旋转：左键拖拽 —— 光标固定的“抓地”。按下时我们拾取指针下的
    // Geographic 点（射线 → 球体），然后每帧绕目标旋转系架，使那个
    // 相同的点随着光标移动而粘住它。这能在任意抓取位置与缩放下精确
    // 跟踪指针 —— 之前的每像素切线增益只与单一屏幕中心点匹配，因此
    // 在其他位置抓取时两个轴上都显得脱离。当 OS 光标位置不可用时会运
    // 行一个几何回退，因此手感会优雅降级而非卡死。
    let pressed = mouse_buttons.pressed(MouseButton::Left);
    if pressed {
        // 新按下（新抓取）：重置锚点以便我们在光标下重新拾取，若
        // 这是历次首次拖拽则闭锁球面位姿。
        if !arcball.was_pressed {
            if !arcball.engaged {
                arcball.orientation = arcball_quat_from_spherical(&state).as_dquat();
                arcball.engaged = true;
            }
            arcball.anchor = None;
        }

        let win = windows.get_single().ok();
        let win_h = win.map(|w| w.height()).unwrap_or(720.0);
        let focal = (win_h * 0.5) / (CAMERA_FOV_Y * 0.5).tan();
        let surface_dist = (state.distance - GLOBE_RADIUS).max(0.001);

        // 穿过光标的世界空间拾取射线，然后它命中的表面方向。优先使用
        // 相机的*真实*投影（`viewport_to_world`，即 Bevy 实际的
        // `clip_from_view` + 渲染的 `GlobalTransform`），因此“光标下的点”按构造
        // 就是用户看到在光标下的点 —— 这消除了每个手工建模假设（视锥
        // 数学、transform 约定、上轴）。手工写的 `cursor_ray_world` 仅作为相机
        // 尚不可查询那些帧的回退存活。两者都在下游拓宽到 f64。
        let bevy_ray = win.and_then(|w| {
            cameras
                .get_single()
                .ok()
                .and_then(|(cam, ct)| cursor_ray_bevy(cam, ct, w))
        });
        let picked = bevy_ray
            .or_else(|| {
                win.and_then(|w| {
                    cursor_ray_world(w, arcball.orientation, state.distance, state.target)
                })
            })
            .and_then(|(o, d)| pick_surface_dir_ellipsoid(o, d, state.target.as_dvec3()));

        match (arcball.anchor, picked) {
            (Some(anchor), Some(bdir)) => {
                // 旋转系架使被抓的点重新回到光标下（精确 1:1）。增量无关
                // 紧要（绝对光标）。注意：故意不用 `Quat::from_rotation_arc` —— 当
                // `dot > 1 − ε` 它会提前返回 `IDENTITY`，而在最大缩放下锚点与光标
                // 表面方向收敛到 ~3e-5 rad 以内，因此它们的 f32 dot 舍入为 1.0 →
                // 拖拽冻结在 0 旋转。`rotation_from_unit_dir` 能恢复那个微小角度。
                //
                // 然后该修正每帧被钳位 —— 但上限是感知缩放的：
                // `MAX_PAN_PX_PER_FRAME` 像素的屏幕平移在当前高度转换为地心角
                // （`px · surface_dist / focal`），从不超过全局弧度兼底。一个固定
                // 弧度上限在整球缩放下看着安全，却将深缩放限流到地面：在
                // `min_distance` 处 0.1 rad ≈ 20 px/帧，因此每次快速拖拽都留下
                // 10-20 px 延迟（即“不跟手”）。普通拖拽在任何缩放从不触发像素
                // 上限（中球弦长 ≲ 0.03 rad，最大缩放步长 ≲ 1e-4 → 仍精确 1:1），
                // 而一个大的单帧光标增量（一次拂动 / 合并输入风暴）或一次
                // 接近极点的拾取 —— 经线收敛，一个水平鼠标移动映射为巨大的地心
                // 摆动 —— 会在几帧内被收敛，而不是传送视图或将相机抛过地球
                // （“地球没了”）。
                let step_cap =
                    (f64::from(MAX_PAN_PX_PER_FRAME) * f64::from(surface_dist / focal))
                        .min(f64::from(MAX_DRAG_STEP_RAD));
                // 通过绕其自身轴缩放抓地旋转来应用总体灵敏度乘子
                // `rotate_speed`（1.0 = 精确 1:1），然后每帧钳位。之前离球回退会
                // 消费此值；现在固定路径拥有它，因此该字段保持有效。
                let (grab_axis, grab_ang) = rotation_from_unit_dir(bdir, anchor).to_axis_angle();
                let r = clamp_rotation_angle(
                    DQuat::from_axis_angle(grab_axis, f64::from(state.rotate_speed) * grab_ang),
                    step_cap as f32,
                );
                arcball.orientation = (r * arcball.orientation).normalize();
                motion_events.clear();
            }
            (maybe_anchor, Some(bdir)) => {
                // 首个拾取帧（或光标重新进入地球）：将锚点闭锁到光标下的点，
                // 本帧不旋转。在此也应用几何增益会与下一帧的拾取修正相冲突
                // （它会把刚闭锁的点旋转回光标下），产生拖拽起始的传送。跟踪
                // 从下一帧开始干净地展开。
                let _ = maybe_anchor;
                arcball.anchor = Some(bdir);
                motion_events.clear();
            }
            (_, None) => {
                // 光标不再解析为地球上的一个点 —— 它被拖出了可见圆盘进入
                // 天空。根据明确的产品决策：完全不旋转，只冻结拖拽。重置锚点
                // 并排空鼠标运动积压，以便光标在球外时视图保持不动，且当光标
                // 返回时在下一个拾取帧重新干净地闭锁（无陈旧增量抛甩，无快速
                // 越边旋转）。
                arcball.anchor = None;
                motion_events.clear();
            }
        }

        // 重新导出 heading/pitch（故意丢弃 roll），使球面消费者 —— globe LOD
        // 子相机点、惯性捕获 —— 保持有效。
        let (h, p) = orbit_from_orientation(arcball.orientation);
        state.heading = h;
        state.pitch = p;
    } else {
        // 即使未拖拽也消费事件以避免累积
        motion_events.clear();
    }
    arcball.was_pressed = pressed;

    // 缩放：鼠标滚轮 —— 对高于表面的高度作乘性缩放，而非距中心的
    // 距离。接近地面时，距中心距离 ≈ R，因此它的固定比例是相对于表面
    // 上方小高度的巨大比例（一格就会撞进地面），而往回拉又显得迟钝。改为
    // 缩放高于表面的高度，可在任意高度给出一致的感知缩放：掠地时温和，
    // 从远处接近时快速。
    for ev in wheel_events.read() {
        let min_surf = state.min_distance - GLOBE_RADIUS;
        let max_surf = state.max_distance - GLOBE_RADIUS;
        let surface_dist = (state.target_distance - GLOBE_RADIUS).clamp(min_surf, max_surf);
        // ev.y > 0（向上滚）= 缩小 -> 缩小高于表面的高度。
        let zoom_factor = 1.0 - ev.y * state.zoom_speed;
        let new_surf = (surface_dist * zoom_factor).clamp(min_surf, max_surf);
        state.target_distance = GLOBE_RADIUS + new_surf;
    }

    // 缩放惯性：使 `distance` 向滚轮设定的目标滑行，使场景连续缩放
    // （CesiumJS 以同样方式缓动缩放；一个硬的一格一跳会读作瓦片“晃动”）。
    let k = 1.0 - (-10.0f32 * time.delta_secs()).exp();
    state.distance += (state.target_distance - state.distance) * k;
    if (state.target_distance - state.distance).abs() < 1.0e-5 {
        state.distance = state.target_distance;
    }

    // 应用 transform：engaged 时用 trackball 位姿，否则用纯球面北向上
    // 路径（与 arcball 前的基线字节相同）。
    if let Ok(mut transform) = query.get_single_mut() {
        *transform = if arcball.engaged {
            transform_from_arcball(arcball.orientation.as_quat(), state.distance, state.target)
        } else {
            compute_camera_transform(&state)
        };
    }
}

/// M2.4：旋转惯性滑行系统（委派给领域 [`InertiaController`]）。
///
/// 在 [`orbit_camera_system`] 之后运行。跟踪帧之间的 heading/pitch 增量；
/// 在一次快速拂动释放时，捕获速度并以指数衰减滑行。当飞行处于活动
/// 状态时被抑制。
#[allow(clippy::too_many_arguments)] // Bevy system: one param per resource/event/query
fn orbit_inertia_system(
    mut state: ResMut<OrbitState>,
    mut inertia: ResMut<OrbitInertiaState>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    flight_state: Res<OrbitFlightState>,
    mut query: Query<&mut Transform, With<OrbitCamera>>,
    windows: Query<&Window>,
    mut arcball: ResMut<Arcball>,
) {
    // 推进惯性计时的单调时钟。
    inertia.now_ms += time.delta_secs() as f64 * 1000.0;

    let is_dragging = mouse_buttons.pressed(MouseButton::Left);
    let flight_active = flight_state.flight.is_some();

    // 用于弧度↔像素边界转换的窗口高度（领域 InertiaController 在像素
    // 空间滑行，参见 `inertia_pixel_scale`）。
    let win_h = windows
        .get_single()
        .map(|w| w.height())
        .unwrap_or(720.0);

    // ── 检测拖拽开始 ──────────────────────────────────────────────
    if is_dragging && !inertia.was_dragging {
        inertia.coasting = false;
        inertia.controller.deactivate(InertiaState::Spin);
        inertia.press_time_ms = inertia.now_ms;
    }

    // ── 检测拖拽释放 → 捕获惯性（弧度 → 像素）──────────
    if inertia.was_dragging && !is_dragging && !flight_active {
        inertia.release_time_ms = inertia.now_ms;
        let hold_secs = (inertia.release_time_ms - inertia.press_time_ms) / 1000.0;
        let heading_delta = (state.heading - inertia.prev_heading) as f64;
        let pitch_delta = (state.pitch - inertia.prev_pitch) as f64;

        if hold_secs < INERTIA_MAX_CLICK_TIME_THRESHOLD
            && (heading_delta.abs() > 1e-8 || pitch_delta.abs() > 1e-8)
        {
            // 将弧度速度转为像素空间：领域在像素中滑行，其
            // `INERTIA_STOP_DISTANCE` 守卫（0.5 px）对原始弧度（约 0.01）毫无
            // 意义。该尺度镜像抓地投影，因此往返精确。
            let (scale_h, scale_p) = inertia_pixel_scale(&state, win_h);
            inertia.capture_scale_h = scale_h;
            inertia.capture_scale_p = scale_p;
            // capture 存储 motion = (end - start) * 0.5，因此传入 end = 2×delta。
            // trackball engaged 时 `heading` 通过 atan2 重新导出，越极时可能环绕
            // ±π；将每帧增量归一化，以免一次环绕伪装成巨大速度（→ 失控滑行）。
            let motion_px = DVec2::new(
                wrap_pi(heading_delta as f32) as f64 * scale_h as f64,
                pitch_delta * scale_p as f64,
            );
            inertia
                .controller
                .capture(InertiaState::Spin, DVec2::ZERO, motion_px * 2.0);
            inertia.controller.activate(Some(InertiaState::Spin));
            inertia.coasting = true;
        } else {
            inertia.coasting = false;
        }
    }

    inertia.was_dragging = is_dragging;

    // ── 以指数衰减滑行（像素 → 弧度）───────────────────
    if inertia.coasting && !is_dragging && !flight_active {
        let sample = InertiaSample::new(
            INERTIA_SPIN_COEFFICIENT,
            inertia.press_time_ms,
            inertia.release_time_ms,
            inertia.now_ms,
        );
        // 在可变借用 `inertia` 之前快照捕获时的尺度。
        let scale_h = inertia.capture_scale_h as f64;
        let scale_p = inertia.capture_scale_p as f64;
        match inertia.controller.maintain(InertiaState::Spin, &sample) {
            Some(delta_px) => {
                let d_heading = (delta_px.x / scale_h) as f32;
                let d_pitch = (delta_px.y / scale_p) as f32;
                if arcball.engaged {
                    // 以拖拽驱动它相同的方式滑行实时 trackball：绕相机 up
                    // 偏航，绕相机 right 俯仰。
                    let up = (arcball.orientation * DVec3::Y).normalize();
                    let right = (arcball.orientation * DVec3::X).normalize();
                    let q = DQuat::from_axis_angle(up, f64::from(d_heading))
                        * DQuat::from_axis_angle(right, f64::from(d_pitch));
                    arcball.orientation = (q * arcball.orientation).normalize();
                    let (h, p) = orbit_from_orientation(arcball.orientation);
                    state.heading = h;
                    state.pitch = p;
                    if let Ok(mut transform) = query.get_single_mut() {
                        *transform = transform_from_arcball(
                            arcball.orientation.as_quat(),
                            state.distance,
                            state.target,
                        );
                    }
                } else {
                    state.heading += d_heading;
                    state.pitch = (state.pitch + d_pitch).clamp(-1.5, 1.5);
                    if let Ok(mut transform) = query.get_single_mut() {
                        *transform = compute_camera_transform(&state);
                    }
                }
            }
            None => {
                inertia.coasting = false;
            }
        }
    }

    // 存储当前 heading/pitch，用于下一帧的增量计算。
    inertia.prev_heading = state.heading;
    inertia.prev_pitch = state.pitch;
}

/// 在当前 orbit 状态下的每弧度像素尺度因子 `(heading, pitch)`。
///
/// 领域 [`InertiaController`] 在**像素空间**滑行 —— 其 `INERTIA_STOP_DISTANCE`
/// 守卫是 0.5 px —— 因此应用边界在捕获前将旋转增量从弧度转为像素，
/// 在 [`InertiaController::maintain`] 后再转回弧度。这些因子是几何抓地增益的
/// 精确逆：`focal = (H/2)/tan(fov/2)`，`surface_dist = distance - R`（与拖拽
/// 回退所用的同一逆）。捕获与滑行使用同一尺度使像素往返无损，因此
/// 滑行的运动是释放时弧度速度的干净指数衰减。
fn inertia_pixel_scale(state: &OrbitState, win_h: f32) -> (f32, f32) {
    let focal = (win_h * 0.5) / (CAMERA_FOV_Y * 0.5).tan();
    let surface_dist = (state.distance - GLOBE_RADIUS).max(0.001);
    let s = focal / surface_dist;
    (s, s)
}

/// 为 arcball 相机系架构建穿过 OS 光标的世界空间拾取射线（原点 +
/// 单位方向）。当光标位置不可用（例如指针在窗口外）时为 `None`。使用
/// 自定义视锥（`CAMERA_FOV_Y`）与窗口宽高比；相机沿其局部 -Z 看，位于
/// `target + orientation·(Ẑ · distance)`。
fn cursor_ray_world(
    window: &Window,
    orientation: DQuat,
    distance: f32,
    target: Vec3,
) -> Option<(DVec3, DVec3)> {
    let cursor = window.cursor_position()?;
    let w = window.width().max(1.0);
    let h = window.height().max(1.0);
    let aspect = w / h;
    let x_ndc = (cursor.x / w) * 2.0 - 1.0;
    let y_ndc = 1.0 - (cursor.y / h) * 2.0;
    let tan_y = (CAMERA_FOV_Y * 0.5).tan();
    let tan_x = tan_y * aspect;
    // 下游一切（锚点拾取 + 旋转）都以 f64 运行，使微小的最大缩放
    // 残差得以留存；像素/FOV 输入被精确拓宽。
    let dir_cam = DVec3::new(
        f64::from(x_ndc * tan_x),
        f64::from(y_ndc * tan_y),
        -1.0,
    )
    .normalize();
    let origin = target.as_dvec3() + orientation * (DVec3::Z * f64::from(distance));
    let dir = (orientation * dir_cam).normalize();
    Some((origin, dir))
}

/// 使用相机的真实投影构建穿过 OS 光标的世界空间拾取射线 ——
/// `Camera::viewport_to_world` 复合了实际的 `clip_from_view` 矩阵与渲染的
/// `GlobalTransform`，因此它返回的射线正是屏幕上那一帧绘制所沿的视线。
/// 这是 [`cursor_ray_world`] 的非循环替代：我们不再假设手工推导的视锥与
/// 渲染器匹配，而是去问渲染器。近平面 `origin` + 单位 `direction` 立即
/// 拓宽到 f64，使下游锚点旋转保持最大缩放下的微小角精度（f32 射线
/// 方向携带 ~1e-7 rad，远低于 ~1e-5 rad 的信号）。
fn cursor_ray_bevy(
    camera: &Camera,
    camera_transform: &GlobalTransform,
    window: &Window,
) -> Option<(DVec3, DVec3)> {
    let cursor = window.cursor_position()?;
    let ray = camera.viewport_to_world(camera_transform, cursor).ok()?;
    let origin = ray.origin.as_dvec3();
    let dir = Vec3::from(ray.direction).as_dvec3();
    Some((origin, dir))
}

/// 将一条世界射线与 WGS84 椭球（以 `center` 为中心，x/y 为赤道半径
/// `ELLIPSOID_A`，z 为极地半径 `ELLIPSOID_B` —— 与瓦片网格细分所在的 SAME
/// 表面）相交，返回从中心到就近命中点的单位方向。未命中或唯一交点在
/// 相机后方时为 `None`。这取代了旧的单位球拾取，用于抓地拖拽，使
/// 锚点是用户在地面上实际看到的那个点，而非其上方漂浮的球体 ——
/// 两者之间的径向间隙正是使深缩放拖拽过冲的原因。
fn pick_surface_dir_ellipsoid(origin: DVec3, dir: DVec3, center: DVec3) -> Option<DVec3> {
    // 各向异性缩放到单位球空间，解 |o + t·d|² = 1，将就近命中点缩回
    // 世界。`dir` 在世界中是单位向量，但缩放后不是，因此保留一般二次式
    // （a ≠ 1）。
    let s = DVec3::new(1.0 / ELLIPSOID_A, 1.0 / ELLIPSOID_A, 1.0 / ELLIPSOID_B);
    let os = (origin - center) * s;
    let ds = dir * s;
    let a = ds.dot(ds);
    let half_b = os.dot(ds);
    let c = os.dot(os) - 1.0;
    let disc = half_b * half_b - a * c;
    if disc <= 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let t0 = (-half_b - sq) / a;
    let t = if t0 > 0.0 { t0 } else { (-half_b + sq) / a };
    if t <= 0.0 {
        return None;
    }
    let hit = center + (os + ds * t) / s;
    (hit - center).try_normalize()
}

/// 将单位方向 `from` 变到单位方向 `to` 的最短弧旋转，计算为
/// `axis = from × to`，`angle = atan2(|axis|, from·to)`。
///
/// 这取代了 `Quat::from_rotation_arc`，它对抓地锚点不可用：只要 `dot > 1 − ε`
/// 它就退回 `IDENTITY`。在最大缩放（相机位于表面上方 ~765 m）下，两个
/// 地心表面方向 —— 闭锁的锚点与当前光标拾取 —— 仅相差 ~3e-5 rad，因此它们
/// 的 f32 dot 舍入为 1.0 且固定旋转退化为单位，使拖拽冻结在 0 px。叉积
/// 量级（~3e-5）仍高出 f32 次正规 floor 数个量级，因此 `atan2` 恢复真实
/// 微小角度，抓取点一路到地表都 1:1 跟踪光标 —— 同时同一绝对锚点数学在
/// 宽视野/整球缩放下保持精确（从不过旋），不像一个随相机高度缩放的
/// 每像素增益。
fn rotation_from_unit_dir(from: DVec3, to: DVec3) -> DQuat {
    let axis = from.cross(to);
    let sin = axis.length();
    let cos = from.dot(to);
    if sin < 1.0e-12 {
        // （反）平行：无有意义的旋转轴。
        return if cos < 0.0 {
            // 绕任一垂直于 `from` 的单位轴旋转 180°。
            let perp = if from.x.abs() < from.y.abs() { DVec3::X } else { DVec3::Y };
            from.cross(perp)
                .try_normalize()
                .map(|a| DQuat::from_axis_angle(a, std::f64::consts::PI))
                .unwrap_or(DQuat::IDENTITY)
        } else {
            DQuat::IDENTITY
        };
    }
    DQuat::from_axis_angle(axis / sin, sin.atan2(cos))
}

/// 将旋转的角软上限到至多 `max_angle` 弧度，保留其轴。用于限定每
/// 帧的“抓地”修正：一个巨大的单帧残差（快速拂动、合并输入风暴，或
/// 经线收敛的近极拾取）否则会传送视图；钳位使其能在几帧内平滑收敛。
/// 一个近单位或已经很小的旋转会原样返回，因此普通拖拽保持字节相同
/// （精确 1:1）—— 只有过大的猛拉会被软化。
fn clamp_rotation_angle(q: DQuat, max_angle: f32) -> DQuat {
    let (axis, angle) = q.to_axis_angle();
    let max_angle = f64::from(max_angle);
    if angle <= max_angle || axis.length_squared() < 1.0e-12 {
        return q;
    }
    DQuat::from_axis_angle(axis, max_angle)
}

/// 将一个角归一化到 (-π, π] 区间。防止 trackball 穿越极点时 ±π 方位角
/// 环绕对惯性捕获造成影响。
fn wrap_pi(a: f32) -> f32 {
    let two_pi = 2.0 * std::f32::consts::PI;
    let mut x = (a + std::f32::consts::PI) % two_pi;
    if x < 0.0 {
        x += two_pi;
    }
    x - std::f32::consts::PI
}

/// M2.4：大圆航线飞行系统（委派给领域 [`CameraFlight`]）。
///
/// 最后运行，使飞行对 orbit 状态有最终决定权。使用领域的大圆 slerp
/// 插值，配合从 [`compute_flight_duration`] 与 [`select_flight_easing`] 导出的
/// 自动时长/缓动。
fn orbit_flight_system(
    mut flight_state: ResMut<OrbitFlightState>,
    mut state: ResMut<OrbitState>,
    time: Res<Time>,
    mut query: Query<&mut Transform, With<OrbitCamera>>,
    mut fly_requests: EventReader<OrbitFlyToRequest>,
    mut arcball: ResMut<Arcball>,
) {
    // 处理新的 fly-to 请求。一次 fly-to 将控制权交回确定性的北向上球面
    // 路径，因此丢弃实时 trackball。
    for request in fly_requests.read() {
        arcball.engaged = false;
        arcball.anchor = None;
        orbit_fly_to(&state, request.destination_ecef, &mut flight_state);
    }

    let flight = match flight_state.flight.as_mut() {
        Some(f) if !f.complete => f,
        _ => return,
    };

    let dt = time.delta_secs() as f64;
    if let Some((position, _direction, _up)) = flight.update(dt) {
        let meters_per_render_unit = 6378137.0_f64;
        let (heading, pitch, distance) = ecef_to_orbit(position, meters_per_render_unit);
        state.heading = heading;
        state.pitch = pitch.clamp(-1.5, 1.5);
        state.distance = distance;
        state.target_distance = distance;
        if let Ok(mut transform) = query.get_single_mut() {
            *transform = compute_camera_transform(&state);
        }
    }

    if flight.complete {
        flight_state.flight = None;
    }
}

/// 发起一次到给定 ECEF 目的点（米）的大圆航线飞行。
///
/// 时长与缓动使用领域的 [`compute_flight_duration`] 与 [`select_flight_easing`]
/// 从距离自动导出。
pub(crate) fn orbit_fly_to(
    state: &OrbitState,
    destination_ecef: DVec3,
    flight_state: &mut OrbitFlightState,
) {
    let meters_per_render_unit = 6378137.0_f64;
    let position = orbit_position_to_ecef(state, meters_per_render_unit);
    let direction = -position.normalize();
    let cam = cesium_camera::Camera::new(position, direction, DVec3::Z);

    let distance = (destination_ecef - position).length();
    let duration = compute_flight_duration(distance);
    let mut flight = CameraFlight::fly_to(&cam, destination_ecef, None, None, duration);
    flight.easing = select_flight_easing(distance);
    flight_state.flight = Some(flight);
}

/// 将 orbit 状态转为米下的 ECEF 位置。
fn orbit_position_to_ecef(state: &OrbitState, meters_per_render_unit: f64) -> DVec3 {
    let d = state.distance as f64 * meters_per_render_unit;
    let cos_pitch = (state.pitch as f64).cos();
    let sin_pitch = (state.pitch as f64).sin();
    let heading = state.heading as f64;
    DVec3::new(
        d * cos_pitch * heading.cos(),
        d * cos_pitch * heading.sin(),
        d * sin_pitch,
    )
}

/// 将一个 ECEF 位置（米）转回 orbit 球坐标。
fn ecef_to_orbit(position: DVec3, meters_per_render_unit: f64) -> (f32, f32, f32) {
    let r = position.length();
    let distance = (r / meters_per_render_unit) as f32;
    let pitch =
        position.z.atan2((position.x * position.x + position.y * position.y).sqrt()) as f32;
    let heading = position.y.atan2(position.x) as f32;
    (heading, pitch, distance)
}

/// 从球面 orbit 状态计算相机 Transform。
///
/// 地球是 ECEF：北极在 +Z，赤道在 XY 平面。相机位置以围绕 Z（极地）
/// 轴的球坐标表达：
///   x = distance * cos(pitch) * cos(heading)
///   y = distance * cos(pitch) * sin(heading)
///   z = distance * sin(pitch)
/// 且相机的“上”是地球的 +Z 轴，因此北总是朝上。
fn compute_camera_transform(state: &OrbitState) -> Transform {
    let cos_pitch = state.pitch.cos();
    let sin_pitch = state.pitch.sin();

    let offset = Vec3::new(
        state.distance * cos_pitch * state.heading.cos(),
        state.distance * cos_pitch * state.heading.sin(),
        state.distance * sin_pitch,
    );

    let position = state.target + offset;
    Transform::from_translation(position).looking_at(state.target, Vec3::Z)
}

// ── FIX-ARCBALL 助手 ────────────────────────────────────

/// 构建一个复现当前球面状态旧式北向上极地位姿的 arcball 朝向。用于
/// 在拖拽开始的那一刻将 trackball 闭锁到现有视图，因此接管是无缝的。
fn arcball_quat_from_spherical(state: &OrbitState) -> Quat {
    compute_camera_transform(state).rotation
}

/// 由 arcball 朝向得到的相机 `Transform`：相机位于
/// `target + orientation·(Ẑ · distance)` 并沿 `-orientation·Ẑ` 回看。对于由
/// [`arcball_quat_from_spherical`] 产生的朝向，它与 [`compute_camera_transform`]
/// 相同（系架的 +Z 轴从 target 指向相机，因此 -Z —— Bevy 的相机前方 —— 目
/// 标向 target）。
fn transform_from_arcball(orientation: Quat, distance: f32, target: Vec3) -> Transform {
    let position = target + orientation * (Vec3::Z * distance);
    Transform::from_translation(position).with_rotation(orientation)
}

/// 从 arcball 朝向导出 `(heading, pitch)` —— 仅视图方向，故意丢弃 roll ——
/// 使球面消费者（globe LOD 子相机点、惯性）在 trackball 存活时保持填充。
fn orbit_from_orientation(orientation: DQuat) -> (f32, f32) {
    let dir = orientation * DVec3::Z; // normalize(position - target)
    (
        dir.y.atan2(dir.x) as f32,
        dir.z.clamp(-1.0, 1.0).asin() as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_state_default_is_pixel_neutral() {
        let state = OrbitState::default();
        assert_eq!(state.heading, 0.0);
        assert!((state.pitch - 0.4).abs() < 1e-6);
        assert!((state.distance - 3.0).abs() < 1e-6);
    }

    #[test]
    fn clamp_rotation_angle_bounds_large_steps_only() {
        // 一个小的旋转（普通拖拽步长）原样通过，字节相同。
        let small = DQuat::from_axis_angle(DVec3::Z, 0.02);
        assert_eq!(clamp_rotation_angle(small, MAX_DRAG_STEP_RAD), small);

        // 一个巨大旋转（传送大小的猛拉）被钳位到上限同时保留其轴，
        // 因此视图滑行而非跳跃。
        let big = DQuat::from_axis_angle(DVec3::X, 1.2);
        let (axis, angle) = clamp_rotation_angle(big, MAX_DRAG_STEP_RAD).to_axis_angle();
        assert!((angle - f64::from(MAX_DRAG_STEP_RAD)).abs() < 1e-5);
        assert!((axis.abs() - DVec3::X.abs()).length() < 1e-5);
    }

    #[test]
    fn clamp_rotation_angle_preserves_1to1_tracking_scale() {
        // 在整球缩放下，一个普通快速拖拽步长（~0.03 rad）必须不被钳位，
        // 因此抓取点仍精确落在光标下。
        let typical = DQuat::from_axis_angle(DVec3::Y, 0.03);
        assert_eq!(clamp_rotation_angle(typical, MAX_DRAG_STEP_RAD), typical);
    }

    #[test]
    fn compute_transform_produces_correct_position() {
        let state = OrbitState {
            heading: 0.0,
            pitch: 0.0,
            distance: 3.0,
            ..Default::default()
        };
        let t = compute_camera_transform(&state);
        // heading=0、pitch=0 时：position = (3, 0, 0)
        assert!((t.translation.x - 3.0).abs() < 1e-5);
        assert!((t.translation.y).abs() < 1e-5);
        assert!((t.translation.z).abs() < 1e-5);
    }

    #[test]
    fn ecef_to_orbit_roundtrip() {
        let mpru = 6378137.0_f64;
        let state = OrbitState {
            heading: 0.5,
            pitch: 0.3,
            distance: 3.0,
            ..Default::default()
        };
        let ecef = orbit_position_to_ecef(&state, mpru);
        let (h, p, d) = ecef_to_orbit(ecef, mpru);
        assert!((h - state.heading).abs() < 1e-5);
        assert!((p - state.pitch).abs() < 1e-5);
        assert!((d - state.distance).abs() < 1e-4);
    }

    #[test]
    fn flight_duration_and_easing_from_domain() {
        // 短跳 -> quintic，最少 1s。
        assert!((compute_flight_duration(500_000.0) - 1.0).abs() < 1e-12);
        assert_eq!(
            select_flight_easing(500_000.0),
            cesium_camera::EasingFunction::QuinticInOut
        );
        // 长跳 -> cubic，上限 5s。
        assert!((compute_flight_duration(10_000_000.0) - 5.0).abs() < 1e-12);
        assert_eq!(
            select_flight_easing(2_000_000.0),
            cesium_camera::EasingFunction::CubicInOut
        );
    }

    #[test]
    fn orbit_fly_to_creates_valid_flight() {
        let state = OrbitState::default();
        let mut fs = OrbitFlightState::default();
        let dest = DVec3::new(6378137.0 * 2.0, 0.0, 0.0);
        orbit_fly_to(&state, dest, &mut fs);
        assert!(fs.flight.is_some());
        let f = fs.flight.as_ref().unwrap();
        assert!(!f.complete);
        assert!(f.duration >= 1.0 && f.duration <= 5.0);
    }

    #[test]
    fn inertia_coasts_in_pixel_space_via_boundary_conversion() {
        // 领域 InertiaController 在像素空间滑行（0.5 px 停止守卫），因此
        // 应用在捕获时将弧度速度→像素，在 maintain 时再转回弧度。馈入原始
        // 弧度（~0.01）会直接穿过停止守卫且永不滑行。
        let state = OrbitState {
            heading: 0.0,
            pitch: 0.4,
            distance: 3.0,
            ..Default::default()
        };
        let (scale_h, scale_p) = inertia_pixel_scale(&state, 720.0);
        // 在 distance=3（surface_dist=2）、focal≈623.5 时：尺度 >>1 px/rad，
        // 因此一次 0.02 rad 拂动是多像素运动，能越过守卫。
        assert!(scale_h > 100.0 && scale_p > 100.0);

        // 一次真实拂动：一帧内 ~0.02 rad heading / 0.01 rad pitch。
        let heading_delta = 0.02_f64;
        let pitch_delta = 0.01_f64;
        let motion_px = DVec2::new(heading_delta * scale_h as f64, pitch_delta * scale_p as f64);

        let mut ctrl = InertiaController::new();
        ctrl.capture(InertiaState::Spin, DVec2::ZERO, motion_px * 2.0);
        ctrl.activate(Some(InertiaState::Spin));

        // 首个滑行帧（释放后 16 ms）：仍高于守卫。
        let sample = InertiaSample::new(INERTIA_SPIN_COEFFICIENT, 0.0, 0.0, 16.0);
        let delta_px = ctrl.maintain(InertiaState::Spin, &sample).expect("coasting");

        // 转回弧度：decay(0.016s, 0.9) = exp(-2.5*0.016) ≈ 0.9608，因此滑行的
        // 弧度速度略低于释放速度。
        let heading_back = delta_px.x / scale_h as f64;
        let pitch_back = delta_px.y / scale_p as f64;
        assert!(heading_back > 0.0 && heading_back <= heading_delta);
        assert!(
            (heading_back - heading_delta * 0.9608).abs() < 1e-3,
            "heading coast {heading_back} should ≈ {}",
            heading_delta * 0.9608
        );
        assert!(pitch_back > 0.0 && pitch_back <= pitch_delta);
    }

    #[test]
    fn inertia_pixel_scale_matches_grab_the_globe_gain() {
        // 尺度必须是抓地增益的精确逆，以使弧度→像素→弧度的往返无损。
        // 两轴都使用简单几何增益（focal/surface_dist）；旧的 1/cos(pitch)
        // 经线收敛因子属于旧式 ECEF-绕-Z 模型，在 arcball（相机相关）跟踪接管
        // 时已被丢弃。
        let state = OrbitState {
            pitch: 0.3,
            distance: 4.0,
            ..Default::default()
        };
        let win_h = 900.0_f32;
        let (scale_h, scale_p) = inertia_pixel_scale(&state, win_h);
        let focal = (win_h * 0.5) / (CAMERA_FOV_Y * 0.5).tan();
        let surface_dist = state.distance - GLOBE_RADIUS;
        assert!((scale_h - focal / surface_dist).abs() < 1e-3);
        assert!((scale_p - focal / surface_dist).abs() < 1e-3);

        // 往返：一个弧度增量 → 像素 → 弧度是恒等。
        let d_heading = 0.05_f64;
        let px = d_heading * scale_h as f64;
        assert!((px / scale_h as f64 - d_heading).abs() < 1e-9);
    }

    #[test]
    fn inertia_stops_after_threshold() {
        let mut ctrl = InertiaController::new();
        ctrl.capture(InertiaState::Spin, DVec2::ZERO, DVec2::new(0.02, 0.0));
        // 保持 0.5s ≥ INERTIA_MAX_CLICK_TIME_THRESHOLD → 无滑行。
        let sample = InertiaSample::new(INERTIA_SPIN_COEFFICIENT, 0.0, 500.0, 516.0);
        assert!(ctrl.maintain(InertiaState::Spin, &sample).is_none());
    }

    /// M2.4 验证门：无头关键帧播放中性。
    ///
    /// 精确复现 `--headless --camera-script` 路径：`camera_script_system` 每帧
    /// 覆写 `OrbitState` 且**无**鼠标/滚轮/fly-to 输入，因此委派的旋转惯性与
    /// 大圆航线飞行系统必须贡献恰好为零。所得的 `Transform` 因此等于脚本状态
    /// 的纯抓地 transform —— 与旧式 M0 构建位相同（pos/quat 差
    /// `0.0 < 1e-4` render unit）对每个脚本位姿都成立。
    ///
    /// 十个不同的脚本位姿代替代十个手势脚本；中性论证是逐帧且与脚本无关的，
    /// 因此这覆盖了整个系列。在 `MinimalPlugins`（无 GPU / 渲染后端）上运行。
    #[test]
    fn headless_keyframe_playback_matches_legacy_within_1e4() {
        // 一个 10 位姿的脚本轨迹（heading 扫过一整圈，pitch 与 distance 变化）
        // —— 捕获脚本的确定性等价物。
        let scripted: Vec<(f32, f32, f32)> = (0..10)
            .map(|i| {
                let t = i as f32 / 9.0;
                (
                    t * std::f32::consts::TAU,
                    0.4 + 0.15 * t,
                    3.0 - 0.75 * t,
                )
            })
            .collect();

        let mut max_pos_err = 0.0_f32;
        let mut max_quat_err = 0.0_f32;

        for &(heading, pitch, distance) in &scripted {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .add_event::<MouseMotion>()
                .add_event::<MouseWheel>()
                .add_event::<OrbitFlyToRequest>()
                .init_resource::<ButtonInput<MouseButton>>()
                .init_resource::<OrbitInertiaState>()
                .init_resource::<OrbitFlightState>()
                .init_resource::<Arcball>()
                .insert_resource(OrbitState {
                    heading,
                    pitch,
                    distance,
                    target_distance: distance,
                    ..Default::default()
                })
                .add_systems(
                    Update,
                    (orbit_camera_system, orbit_inertia_system, orbit_flight_system).chain(),
                );
            let cam = app.world_mut().spawn((OrbitCamera, Transform::IDENTITY)).id();

            // 一个无输入事件的无头帧（镜像播放）。
            app.update();

            let got = *app.world().get::<Transform>(cam).expect("camera transform");
            // 旧式 transform 是脚本状态的纯函数（抓地主体逐字节保留；
            // 因 target_distance == distance 而滑行是空操作）。
            let expected = compute_camera_transform(&OrbitState {
                heading,
                pitch,
                distance,
                target_distance: distance,
                ..Default::default()
            });

            max_pos_err = max_pos_err.max(got.translation.distance(expected.translation));
            let quat_err = got
                .rotation
                .to_array()
                .iter()
                .zip(expected.rotation.to_array().iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f32, f32::max);
            max_quat_err = max_quat_err.max(quat_err);
        }

        assert!(
            max_pos_err < 1e-4,
            "playback pos drift {max_pos_err} render units must be < 1e-4"
        );
        assert!(
            max_quat_err < 1e-4,
            "playback quat drift {max_quat_err} must be < 1e-4"
        );
    }
}
