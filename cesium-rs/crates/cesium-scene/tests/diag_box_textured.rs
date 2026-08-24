//! diagnostic 2
use cesium_scene::gltf_pipeline::parse_glb::parse_glb;
use cesium_scene::model::model::Model;
use cesium_scene::frame_state::FrameState;
use cesium_renderer::context::Context;

fn try_gpu() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("diag2"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::default(),
        trace: wgpu::Trace::Off,
    })).ok()?;
    Some((device, queue))
}

#[test]
#[ignore = "诊断用，非功能测试"]
fn diag_dump_vertex_array() {
    let Some((device, queue)) = try_gpu() else { return; };
    let path = cesium_specs::data_path("Models/glTF-2.0/BoxTextured/glTF-Binary/BoxTextured.glb");
    let glb = std::fs::read(path).unwrap();
    let gltf = parse_glb(&glb).unwrap();
    let mut model = Model::from_gltf(gltf);
    let mut context = Context::new(device.clone(), queue.clone(), 64, 64, None);
    model.update(&FrameState::new(), &mut context);
    println!("ready={} primitives={}", model.ready, model.runtime_primitives().len());
    for prim in model.runtime_primitives() {
        let va = prim.vertex_array.as_ref().unwrap();
        for attr in va.attributes() {
            println!("ATTR loc={} buf={} comps={} fmt={:?} stride={} offset={} bufsize={}",
                attr.index, attr.buffer.id(), attr.components_per_attribute,
                attr.component_datatype, attr.stride_in_bytes, attr.offset_in_bytes,
                attr.buffer.size_in_bytes());
        }
        println!("index: {:?}", va.index_buffer().map(|ib| ib.number_of_indices()));
        for (i, l) in va.buffer_layouts().iter().enumerate() {
            println!("LAYOUT slot={i} stride={} attrs={:?}", l.array_stride, l.attributes.iter().map(|a| (a.shader_location, a.offset, format!("{:?}", a.format))).collect::<Vec<_>>());
        }
    }
    panic!("dump done");
}
