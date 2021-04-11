#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

use lodepng::{Bitmap, RGBA};
use nalgebra_glm as glm;
use rgb::ComponentBytes;
use shaderc::{CompileOptions, Compiler, SpirvVersion};
use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};
use wgpu::util::DeviceExt;
use winit::{
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

lazy_static! {
    static ref VERTICES: [Vertex; 4] = [
        Vertex {
            pos: glm::vec2(0.25, 0.25),
            color: glm::vec4(0.3, 0.5, 0.8, 1.0),
        },
        Vertex {
            pos: glm::vec2(0.75, 0.25),
            color: glm::vec4(0.3, 0.3, 0.4, 1.0),
        },
        Vertex {
            pos: glm::vec2(0.25, 0.75),
            color: glm::vec4(0.6, 0.1, 0.8, 1.0),
        },
        Vertex {
            pos: glm::vec2(0.75, 0.75),
            color: glm::vec4(0.3, 0.5, 0.6, 1.0),
        },
    ];
    static ref VERTICES2: [Vertex; 4] = [
        Vertex {
            pos: glm::vec2(25., 25.),
            color: glm::vec4(0.3, 0.5, 0.8, 1.0),
        },
        Vertex {
            pos: glm::vec2(750., 25.),
            color: glm::vec4(0.3, 0.3, 0.4, 1.0),
        },
        Vertex {
            pos: glm::vec2(25., 750.),
            color: glm::vec4(0.6, 0.1, 0.8, 1.0),
        },
        Vertex {
            pos: glm::vec2(750., 750.),
            color: glm::vec4(0.3, 0.5, 0.6, 1.0),
        },
    ];
    static ref INDICIES: [u16; 6] = [0, 1, 2, 2, 1, 3];

    #[rustfmt::skip]
    pub static ref OPENGL_TO_WGPU_MATRIX: glm::Mat4 =
        glm::mat4(
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0,
            0.0, 0.0, 0.5, 1.0,);
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    pub view_model: glm::Mat4,
}

impl Uniform {
    pub fn new() -> Self {
        Uniform {
            view_model: //glm::identity()
            (*OPENGL_TO_WGPU_MATRIX)
               * glm::ortho(0.0, 1280., 720., 0., -1., 1.)
                //* glm::look_at(
                 //   &glm::vec3(0., 1., 2.),
                  //  &glm::vec3(0., 0., 0.),
                   // &glm::vec3(0., 0., 1.),
                //),
                //  * glm::ortho(0.0, 1280., 720., 0., 0.01, 100.),  
                //* glm::scale(&glm::identity(), &glm::vec3(1., 1., 1.)),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    pub pos: glm::Vec2,
    pub color: glm::Vec4,
}

impl Vertex {
    fn impl_vertex<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<glm::Vec2>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}

pub struct GpuState {
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
    swap_chain: wgpu::SwapChain,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    shader: Shader,
    uniform_bind_group: wgpu::BindGroup,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
}

impl GpuState {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .unwrap();

        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: adapter.get_swap_chain_preferred_format(&surface),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

        let shader = Shader::new(&device, "src/fragment.glsl", "src/vertex.glsl");
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&*VERTICES2),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&*INDICIES),
            usage: wgpu::BufferUsage::INDEX,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[Uniform::new()]),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_SRC,
        });
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Global uniform bind group layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<Uniform>() as u64
                        ),
                    },
                    count: None,
                }],
            });
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader.vs_module,
                entry_point: "main",
                buffers: &[Vertex::impl_vertex()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader.fs_module,
                entry_point: "main",
                targets: &[wgpu::ColorTargetState {
                    format: swap_chain_desc.format,
                    alpha_blend: wgpu::BlendState::REPLACE,
                    color_blend: wgpu::BlendState::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                polygon_mode: wgpu::PolygonMode::Fill,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });

        GpuState {
            instance,
            surface,
            adapter,
            device,
            queue,
            swap_chain,
            swap_chain_desc,
            shader,
            pipeline_layout,
            render_pipeline,
            index_buffer,
            vertex_buffer,
            uniform_buffer,
            uniform_bind_group_layout,
            uniform_bind_group,
        }
    }

    pub fn draw(&mut self) {
        match self.swap_chain.get_current_frame() {
            Ok(frame) => {
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Global render pass"),
                        depth_stencil_attachment: None,
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.output.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        }],
                    });
                    render_pass.set_pipeline(&self.render_pipeline);
                    render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass
                        .set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..6 as u32, 0, 0..1);
                }

                self.queue.submit(std::iter::once(encoder.finish()));
            }
            Err(e) => {
                // info!("{}", e)
            }
        }
    }
}

pub struct Shader {
    pub vs_module: wgpu::ShaderModule,
    pub fs_module: wgpu::ShaderModule,
}

impl Shader {
    pub fn new(device: &wgpu::Device, fs_path: &str, vs_path: &str) -> Self {
        let mut opts = CompileOptions::new().unwrap();
        opts.set_target_spirv(SpirvVersion::V1_5);
        opts.set_source_language(shaderc::SourceLanguage::GLSL);

        let mut compiler = Compiler::new().unwrap_or_else(|| panic!("Compiler create error"));
        let fs_source =
            fs::read_to_string(fs_path).unwrap_or_else(|_| panic!("Cannot  read file {}", fs_path));
        let vs_source =
            fs::read_to_string(vs_path).unwrap_or_else(|_| panic!("Cannot  read file {}", vs_path));
        let fs_file_name = Path::new(fs_path).file_name().unwrap();
        let vs_file_name = Path::new(vs_path).file_name().unwrap();

        let fs_artifact = compiler
            .compile_into_spirv(
                fs_source.as_str(),
                shaderc::ShaderKind::Fragment,
                fs_file_name.to_str().unwrap(),
                "main",
                Some(&opts),
            )
            .unwrap();

        let vs_artifact = compiler
            .compile_into_spirv(
                vs_source.as_str(),
                shaderc::ShaderKind::Vertex,
                vs_file_name.to_str().unwrap(),
                "main",
                Some(&opts),
            )
            .unwrap();
        debug!("VS WARNS {}", vs_artifact.get_warning_messages());
        debug!("FS WARNS {}", fs_artifact.get_warning_messages());
        let vs_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some(format!("GLSL vertex shader form {}", vs_path).as_str()),
            source: wgpu::util::make_spirv(vs_artifact.as_binary_u8()),
            flags: wgpu::ShaderFlags::VALIDATION,
        });

        let fs_module = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some(format!("GLSL fragment shader form {}", vs_path).as_str()),
            source: wgpu::util::make_spirv(fs_artifact.as_binary_u8()),
            flags: wgpu::ShaderFlags::VALIDATION,
        });

        Shader {
            vs_module,
            fs_module,
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    pub(crate) inner: wgpu::Texture,
    size: wgpu::Extent3d,
}

impl Texture {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bitmap: &Bitmap<RGBA>,
        format: wgpu::TextureFormat,
        dimension: wgpu::TextureDimension,
        usage: wgpu::TextureUsage,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: bitmap.width as u32,
            height: bitmap.height as u32,
            ..Default::default()
        };

        let inner = device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension,
                format,
                usage,
                label: None,
            },
            bitmap.buffer.as_bytes(),
        );

        Texture {
            inner,
            size: texture_size,
        }
    }

    pub fn empty(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        dimension: wgpu::TextureDimension,
        usage: wgpu::TextureUsage,
        size: glm::IVec2,
    ) -> Self {
        let texture_size = wgpu::Extent3d {
            width: size.x as u32,
            height: size.y as u32,
            ..Default::default()
        };
        let inner = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension,
            format,
            usage,
            label: None,
        });

        Texture {
            inner,
            size: texture_size,
        }
    }

    pub fn size(&self) -> glm::IVec2 {
        glm::vec2(self.size.width as i32, self.size.height as i32)
    }

    pub fn get_texel_size(&self) -> glm::Vec2 {
        glm::vec2(1.0 / self.size.width as f32, 1.0 / self.size.height as f32)
    }

    pub fn create_view(&self, desc: &wgpu::TextureViewDescriptor) -> wgpu::TextureView {
        self.inner.create_view(&desc)
    }

    pub fn write_all(&self, queue: &wgpu::Queue, data: &[u8]) {
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &self.inner,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            data,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * self.size.width,
                rows_per_image: self.size.height,
            },
            self.size,
        );
    }

    pub fn write_partially(
        &self,
        origin: glm::UVec2,
        size: glm::IVec2,
        queue: &wgpu::Queue,
        data: &[u8],
    ) {
        queue.write_texture(
            wgpu::TextureCopyView {
                texture: &self.inner,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.x as u32,
                    y: origin.y as u32,
                    z: 0,
                },
            },
            data,
            wgpu::TextureDataLayout {
                offset: 0,
                bytes_per_row: 4 * self.size.width,
                rows_per_image: self.size.height,
            },
            wgpu::Extent3d {
                width: size.x as u32,
                height: size.y as u32,
                depth: 0,
            },
        );
    }
}

fn main() {
    use futures::executor::block_on;
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720))
        .with_title("Game")
        .build(&event_loop)
        .unwrap();

    let mut gpu = block_on(GpuState::new(&window));

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(250));
        gpu.draw();
    });
}
