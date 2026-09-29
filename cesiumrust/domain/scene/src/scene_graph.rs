//! 场景图节点结构与遍历。
//!
//! 映射到 CesiumJS `Scene/Scene.js` 与 `Scene/Primitive.js`

use cesium_geospatial::bounding::BoundingSphere;
use std::collections::HashMap;

/// 场景节点的唯一标识符。
pub type NodeId = u64;

/// 场景图中的一个节点。
///
/// 映射到 CesiumJS 场景图元与模型节点。
#[derive(Debug, Clone)]
pub struct SceneNode {
    /// 唯一标识符。
    pub id: NodeId,

    /// 用于调试的可选名称。
    pub name: Option<String>,

    /// 相对于父节点的局部变换。
    pub local_transform: glam::DMat4,

    /// 世界变换（在遍历时计算）。
    pub world_transform: glam::DMat4,

    /// 局部空间的包围体。
    pub bounding_volume: Option<BoundingSphere>,

    /// 该节点是否可见。
    pub visible: bool,

    /// 该节点是否投射阴影。
    pub shadows_enabled: bool,

    /// 子节点 ID。
    pub children: Vec<NodeId>,

    /// 父节点 ID（根节点为 None）。
    pub parent: Option<NodeId>,

    /// 可渲染内容（若有）。
    pub renderable: Option<RenderableContent>,

    /// 用户自定义元数据。
    pub metadata: HashMap<String, String>,
}

impl SceneNode {
    /// 使用给定 ID 创建一个新的场景节点。
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            name: None,
            local_transform: glam::DMat4::IDENTITY,
            world_transform: glam::DMat4::IDENTITY,
            bounding_volume: None,
            visible: true,
            shadows_enabled: true,
            children: Vec::new(),
            parent: None,
            renderable: None,
            metadata: HashMap::new(),
        }
    }

    /// 创建一个带名称的节点。
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// 设置局部变换。
    pub fn with_transform(mut self, transform: glam::DMat4) -> Self {
        self.local_transform = transform;
        self
    }

    /// 设置包围体。
    pub fn with_bounding_volume(mut self, bv: BoundingSphere) -> Self {
        self.bounding_volume = Some(bv);
        self
    }

    /// 设置可渲染内容。
    pub fn with_renderable(mut self, renderable: RenderableContent) -> Self {
        self.renderable = Some(renderable);
        self
    }

    /// 计算世界空间的包围球。
    pub fn world_bounding_sphere(&self) -> Option<BoundingSphere> {
        self.bounding_volume.map(|bv| {
            let center = self.world_transform.transform_point3(bv.center);
            // 按最大缩放因子缩放半径
            let scale = self.world_transform.x_axis.truncate().length()
                .max(self.world_transform.y_axis.truncate().length())
                .max(self.world_transform.z_axis.truncate().length());
            BoundingSphere::new(center, bv.radius * scale)
        })
    }
}

/// 可渲染内容类型。
#[derive(Debug, Clone)]
pub enum RenderableContent {
    /// 带材质的网格。
    Mesh {
        /// 网格资产 ID。
        mesh_id: u64,
        /// 材质 ID。
        material_id: u64,
    },

    /// 一个模型（glTF）。
    Model {
        /// 模型资产 ID。
        model_id: u64,
    },

    /// 一个点云。
    PointCloud {
        /// 点云资产 ID。
        point_cloud_id: u64,
        /// 点的数量。
        point_count: usize,
    },

    /// 一个线框包围体（用于调试）。
    DebugWireframe {
        /// 颜色 [r, g, b, a]。
        color: [f32; 4],
    },
}

/// 包含所有节点的场景图。
#[derive(Debug, Default)]
pub struct SceneGraph {
    /// 场景中的所有节点。
    nodes: HashMap<NodeId, SceneNode>,

    /// 根节点 ID。
    roots: Vec<NodeId>,

    /// 下一个可用的节点 ID。
    next_id: NodeId,
}

impl SceneGraph {
    /// 创建一个空的场景图。
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            roots: Vec::new(),
            next_id: 1,
        }
    }

    /// 向场景图添加一个节点。
    ///
    /// 返回被分配的节点 ID。
    pub fn add_node(&mut self, mut node: SceneNode) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        node.id = id;

        if node.parent.is_none() {
            self.roots.push(id);
        }

        self.nodes.insert(id, node);
        id
    }

    /// 向父节点添加一个子节点。
    pub fn add_child(&mut self, parent_id: NodeId, mut child: SceneNode) -> Option<NodeId> {
        if !self.nodes.contains_key(&parent_id) {
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;
        child.id = id;
        child.parent = Some(parent_id);

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(id);
        }

        self.nodes.insert(id, child);
        Some(id)
    }

    /// 移除一个节点及其所有后代。
    pub fn remove_node(&mut self, id: NodeId) -> Option<SceneNode> {
        let node = self.nodes.remove(&id)?;

        // 从父节点的子列表中移除
        if let Some(parent_id) = node.parent {
            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                parent.children.retain(|&c| c != id);
            }
        }

        // 若为根节点，则从 roots 中移除
        self.roots.retain(|&r| r != id);

        // 移除所有后代
        for child_id in &node.children {
            self.remove_node_recursive(*child_id);
        }

        Some(node)
    }

    /// 递归移除一个节点及其后代。
    fn remove_node_recursive(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.remove(&id) {
            for child_id in node.children {
                self.remove_node_recursive(child_id);
            }
        }
    }

    /// 按 ID 获取节点。
    pub fn get(&self, id: NodeId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    /// 按 ID 获取可变节点。
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    /// 返回根节点 ID。
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    /// 返回节点总数。
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// 更新所有节点的世界变换。
    pub fn update_world_transforms(&mut self) {
        let roots: Vec<NodeId> = self.roots.clone();
        for root_id in roots {
            self.update_node_transform(root_id, glam::DMat4::IDENTITY);
        }
    }

    /// 递归更新一个节点的世界变换。
    fn update_node_transform(&mut self, id: NodeId, parent_world: glam::DMat4) {
        let (world_transform, children) = if let Some(node) = self.nodes.get_mut(&id) {
            node.world_transform = parent_world * node.local_transform;
            (node.world_transform, node.children.clone())
        } else {
            return;
        };

        for child_id in children {
            self.update_node_transform(child_id, world_transform);
        }
    }

    /// 遍历场景图，对每个可见节点调用访问器。
    pub fn traverse<F>(&self, mut visitor: F)
    where
        F: FnMut(&SceneNode),
    {
        for root_id in &self.roots {
            self.traverse_node(*root_id, &mut visitor);
        }
    }

    /// 递归遍历一个节点及其后代。
    fn traverse_node<F>(&self, id: NodeId, visitor: &mut F)
    where
        F: FnMut(&SceneNode),
    {
        if let Some(node) = self.nodes.get(&id) {
            if !node.visible {
                return;
            }
            visitor(node);
            for child_id in &node.children {
                self.traverse_node(*child_id, visitor);
            }
        }
    }

    /// 收集所有可渲染节点的 ID。
    pub fn collect_renderable_ids(&self) -> Vec<NodeId> {
        let mut renderables = Vec::new();
        self.traverse(|node| {
            if node.renderable.is_some() {
                renderables.push(node.id);
            }
        });
        renderables
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;

    #[test]
    fn test_add_node() {
        let mut scene = SceneGraph::new();
        let node = SceneNode::new(0).with_name("TestNode");
        let id = scene.add_node(node);

        assert_eq!(scene.node_count(), 1);
        assert!(scene.get(id).is_some());
        assert_eq!(scene.get(id).unwrap().name, Some("TestNode".to_string()));
    }

    #[test]
    fn test_add_child() {
        let mut scene = SceneGraph::new();
        let parent = SceneNode::new(0).with_name("Parent");
        let parent_id = scene.add_node(parent);

        let child = SceneNode::new(0).with_name("Child");
        let child_id = scene.add_child(parent_id, child).unwrap();

        assert_eq!(scene.node_count(), 2);
        assert_eq!(scene.get(child_id).unwrap().parent, Some(parent_id));
        assert!(scene.get(parent_id).unwrap().children.contains(&child_id));
    }

    #[test]
    fn test_remove_node() {
        let mut scene = SceneGraph::new();
        let parent = SceneNode::new(0);
        let parent_id = scene.add_node(parent);

        let child = SceneNode::new(0);
        let child_id = scene.add_child(parent_id, child).unwrap();

        scene.remove_node(child_id);

        assert_eq!(scene.node_count(), 1);
        assert!(scene.get(child_id).is_none());
        assert!(!scene.get(parent_id).unwrap().children.contains(&child_id));
    }

    #[test]
    fn test_remove_node_with_descendants() {
        let mut scene = SceneGraph::new();
        let root = SceneNode::new(0);
        let root_id = scene.add_node(root);

        let child = SceneNode::new(0);
        let child_id = scene.add_child(root_id, child).unwrap();

        let grandchild = SceneNode::new(0);
        let grandchild_id = scene.add_child(child_id, grandchild).unwrap();

        scene.remove_node(child_id);

        assert_eq!(scene.node_count(), 1);
        assert!(scene.get(child_id).is_none());
        assert!(scene.get(grandchild_id).is_none());
    }

    #[test]
    fn test_update_world_transforms() {
        let mut scene = SceneGraph::new();

        let parent = SceneNode::new(0)
            .with_transform(glam::DMat4::from_translation(DVec3::new(10.0, 0.0, 0.0)));
        let parent_id = scene.add_node(parent);

        let child = SceneNode::new(0)
            .with_transform(glam::DMat4::from_translation(DVec3::new(5.0, 0.0, 0.0)));
        scene.add_child(parent_id, child);

        scene.update_world_transforms();

        // 父节点世界 = identity * local = translation(10, 0, 0)
        let parent_node = scene.get(parent_id).unwrap();
        let parent_pos = parent_node.world_transform.w_axis.truncate();
        assert!((parent_pos.x - 10.0).abs() < 1e-10);

        // 子节点世界 = parent_world * child_local = translation(15, 0, 0)
        let child_id = parent_node.children[0];
        let child_node = scene.get(child_id).unwrap();
        let child_pos = child_node.world_transform.w_axis.truncate();
        assert!((child_pos.x - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_traverse_visible_only() {
        let mut scene = SceneGraph::new();

        let mut root = SceneNode::new(0).with_name("Root");
        root.visible = true;
        let root_id = scene.add_node(root);

        let mut visible_child = SceneNode::new(0).with_name("Visible");
        visible_child.visible = true;
        scene.add_child(root_id, visible_child);

        let mut hidden_child = SceneNode::new(0).with_name("Hidden");
        hidden_child.visible = false;
        scene.add_child(root_id, hidden_child);

        let mut visited = Vec::new();
        scene.traverse(|node| {
            visited.push(node.name.clone());
        });

        assert_eq!(visited.len(), 2);
        assert!(visited.contains(&Some("Root".to_string())));
        assert!(visited.contains(&Some("Visible".to_string())));
        assert!(!visited.contains(&Some("Hidden".to_string())));
    }

    #[test]
    fn test_collect_renderables() {
        let mut scene = SceneGraph::new();

        let node_with_mesh = SceneNode::new(0).with_renderable(RenderableContent::Mesh {
            mesh_id: 1,
            material_id: 1,
        });
        scene.add_node(node_with_mesh);

        let node_without_mesh = SceneNode::new(0);
        scene.add_node(node_without_mesh);

        let renderables = scene.collect_renderable_ids();
        assert_eq!(renderables.len(), 1);
    }

    #[test]
    fn test_world_bounding_sphere() {
        let node = SceneNode::new(0)
            .with_transform(glam::DMat4::from_translation(DVec3::new(100.0, 0.0, 0.0)))
            .with_bounding_volume(BoundingSphere::new(DVec3::ZERO, 10.0));

        // 本测试中手动设置世界变换
        let mut node = node;
        node.world_transform = node.local_transform;

        let world_bv = node.world_bounding_sphere().unwrap();
        assert!((world_bv.center.x - 100.0).abs() < 1e-10);
        assert!((world_bv.radius - 10.0).abs() < 1e-10);
    }
}
