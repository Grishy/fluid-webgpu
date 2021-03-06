use super::RenderNode;
use crate::{FlowType, ParticleUniform};
use idroid::buffer::{BufferObj, MVPUniform};
use idroid::geometry::plane::Plane;
use idroid::math::ViewSize;
use idroid::node::BindingGroupSettingNode;
use idroid::vertex::{Pos, PosTex};
use zerocopy::AsBytes;

pub struct PigmentDiffuseRenderNode {
    setting_node: BindingGroupSettingNode,
    pipeline: wgpu::RenderPipeline,

    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    index_count: usize,

    depth_texture_view: wgpu::TextureView,
}

impl PigmentDiffuseRenderNode {
    pub fn new(
        sc_desc: &wgpu::SwapChainDescriptor, device: &mut wgpu::Device, fluid_buffer: &BufferObj,
        diffuse_buffer: &BufferObj, _flow_type: FlowType, lattice: wgpu::Extent3d,
        particle: wgpu::Extent3d,
    ) -> Self {
        let _view_size = ViewSize { width: sc_desc.width as f32, height: sc_desc.height as f32 };

        let uniform_buf = BufferObj::create_uniform_buffer(
            device,
            &ParticleUniform {
                lattice_size: [2.0 / lattice.width as f32, 2.0 / lattice.height as f32],
                lattice_num: [lattice.width, lattice.height],
                particle_num: [particle.width, particle.height],
                canvas_size: [sc_desc.width, sc_desc.height],
                pixel_distance: [2.0 / sc_desc.width as f32, 2.0 / sc_desc.height as f32],
            },
        );

        let uniform0_buf = BufferObj::create_uniform_buffer(
            device,
            &MVPUniform { mvp_matrix: idroid::utils::matrix_helper::fullscreen_mvp(sc_desc) },
        );

        let setting_node = BindingGroupSettingNode::new(
            device,
            vec![&uniform0_buf, &uniform_buf],
            vec![fluid_buffer, diffuse_buffer],
            vec![],
            vec![],
            vec![
                wgpu::ShaderStage::VERTEX,
                wgpu::ShaderStage::FRAGMENT,
                wgpu::ShaderStage::FRAGMENT,
                wgpu::ShaderStage::FRAGMENT,
            ],
        );

        // Create the vertex and index buffers
        let (vertex_data, index_data) = Plane::new(1, 1).generate_vertices();
        let vertex_buf =
            device.create_buffer_with_data(&vertex_data.as_bytes(), wgpu::BufferUsage::VERTEX);

        let index_buf =
            device.create_buffer_with_data(&index_data.as_bytes(), wgpu::BufferUsage::INDEX);

        // Create the render pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&setting_node.bind_group_layout],
        });
        let shader = idroid::shader::Shader::new(
            "particle/pigment_diffuse",
            device,
            env!("CARGO_MANIFEST_DIR"),
        );
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: &pipeline_layout,
            vertex_stage: shader.vertex_stage(),
            fragment_stage: shader.fragment_stage(),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::None,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: sc_desc.format,
                color_blend: idroid::utils::color_blend(),
                alpha_blend: idroid::utils::alpha_blend(),
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: Some(idroid::depth_stencil::create_state_descriptor()),
            index_format: wgpu::IndexFormat::Uint32,
            vertex_buffers: &[wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<PosTex>() as wgpu::BufferAddress,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &PosTex::attri_descriptor(0),
            }],
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });
        let depth_texture_view = idroid::depth_stencil::create_depth_texture_view(sc_desc, device);

        PigmentDiffuseRenderNode {
            setting_node,
            pipeline,
            depth_texture_view,
            vertex_buf,
            index_buf,
            index_count: index_data.len(),
        }
    }
}

impl RenderNode for PigmentDiffuseRenderNode {
    fn dispatch(&mut self, _cpass: &mut wgpu::ComputePass) {}

    fn begin_render_pass(
        &mut self, _device: &mut wgpu::Device, frame: &wgpu::SwapChainOutput,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                attachment: &frame.view,
                resolve_target: None,
                load_op: wgpu::LoadOp::Clear,
                store_op: wgpu::StoreOp::Store,
                clear_color: wgpu::Color { r: 0.1, g: 0.15, b: 0.17, a: 1.0 },
            }],
            depth_stencil_attachment: Some(idroid::depth_stencil::create_attachment_descriptor(
                &self.depth_texture_view,
            )),
        });
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.setting_node.bind_group, &[]);
        rpass.set_index_buffer(&self.index_buf, 0);
        rpass.set_vertex_buffers(0, &[(&self.vertex_buf, 0)]);
        rpass.draw_indexed(0..self.index_count as u32, 0, 0..1);
    }
}
