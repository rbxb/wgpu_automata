use std::sync::Arc;
use wgpu::CommandBuffer;
use winit::{dpi::PhysicalSize, window::Window};
use rand;

const SIMULATION_WIDTH: u32 = 2048;
const SIMULATION_HEIGHT: u32 = 2048;

struct TextureResource {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    transition_bind_group: Option<wgpu::BindGroup>,
    display_bind_group: Option<wgpu::BindGroup>,
}

struct TextureSwapper {
    index: usize,
    texture_resources: [TextureResource; 2],
}

impl TextureSwapper {
    fn get_read_resource(&self) -> &TextureResource {
        &self.texture_resources[self.index]
    }

    fn get_write_resource(&self) -> &TextureResource {
        &self.texture_resources[(self.index + 1) % 2]
    }

    fn swap(&mut self) {
        self.index = (self.index + 1) % 2;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[{
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                }
            }],
        }
    }
}

const QUAD_VERTICES: &[Vertex] = &[
    Vertex { position: [0.0, 0.0, 0.0] },
    Vertex { position: [1.0, 0.0, 0.0] },
    Vertex { position: [0.0, 1.0, 0.0] },
    Vertex { position: [1.0, 1.0, 0.0] }
];
pub struct RenderState<'a> {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    texture_size: (u32, u32),

    texture_swapper: Option<TextureSwapper>,
    transition_pipeline: Option<wgpu::ComputePipeline>,
    display_pipeline: Option<wgpu::RenderPipeline>,
    vertex_buffer: Option<wgpu::Buffer>,
    surface_size_buffer: Option<wgpu::Buffer>,
    surface_size_bind_group: Option<wgpu::BindGroup>,
}

impl<'a> RenderState<'a> {
    pub async fn new(window: Arc<Window>) -> RenderState<'a> {
        // Create the instance
        let instance = wgpu::Instance::default();
        
        // Create the surface
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        // Create the adapter
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await.unwrap();

        // Pick a device
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None, // Trace path
        ))
        .unwrap();
        println!("Using device {}", adapter.get_info().name);

        // Configure the surface
        let window_size = window.inner_size();
        let surface_config = surface.get_default_config(
            &adapter, 
            window_size.width, 
            window_size.height).unwrap();
        surface.configure(&device, &surface_config);

        let texture_size = (SIMULATION_WIDTH, SIMULATION_HEIGHT);   

        Self {
            _instance: instance,
            surface,
            device,
            queue,
            surface_config,
            texture_size,
            texture_swapper: None,
            transition_pipeline: None,
            display_pipeline: None,
            vertex_buffer: None,
            surface_size_buffer: None,
            surface_size_bind_group: None,
        }
    }

    pub fn create_pipelines(&mut self) {
        // Create bind group layout for transition
        let transition_bind_group_layout =
            self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture { 
                            access: wgpu::StorageTextureAccess::WriteOnly, 
                            format: wgpu::TextureFormat::Rg32Float, 
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
                label: Some("transition_bind_group_layout"),
            });

        // Create pipeline layout for transition
        let transition_pipeline_layout =
            self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("transition_pipeline_layout"),
                bind_group_layouts: &[&transition_bind_group_layout],
                push_constant_ranges: &[],
            });

        // Create the transition pipeline
        let transition_shader_module = self.device.create_shader_module(wgpu::include_wgsl!("transition.wgsl"));
        self.transition_pipeline = Some(self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("compute_pipeline"),
            layout: Some(&transition_pipeline_layout),
            module: &transition_shader_module,
            entry_point: "main",
            compilation_options: Default::default(),
        }));

        // Create bind group layout for display
        let display_bind_group_layout =
            self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
                label: Some("display_bind_group_layout"),
            });

        // Create surface size uniform
        self.surface_size_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("surface_size_buffer"),
            size: (std::mem::size_of::<u32>() * 2) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        // Upload the surface size
        self.queue.write_buffer(
            &self.surface_size_buffer.as_ref().unwrap(), 
            0, 
            bytemuck::cast_slice(&[self.surface_config.width, self.surface_config.height])
        );

        // Create surface size bind group layout
        let surface_size_bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("surface_size_uniform_layout"),
        });

        // Create surface size bind group
        self.surface_size_bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &surface_size_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: self.surface_size_buffer.as_ref().unwrap(),
                        offset: 0,
                        size: None,
                    }),
                }
            ],
            label: Some("camera_bind_group"),
        }));

        // Create pipeline layout for display
        let display_pipeline_layout =
            self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("display_pipeline_layout"),
                bind_group_layouts: &[&display_bind_group_layout, &surface_size_bind_group_layout],
                push_constant_ranges: &[],
            });
        
        // Create the display pipeline
        let display_shader_module = self.device.create_shader_module(wgpu::include_wgsl!("display.wgsl"));
        self.display_pipeline = Some(self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("display_pipeline"),
            layout: Some(&display_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &display_shader_module,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::layout(),
                ],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        }));

        // Create vertex buffer
        self.vertex_buffer = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("quad_vertex_buffer"),
            size: (std::mem::size_of::<Vertex>() * QUAD_VERTICES.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));

        // Upload the vertices
        self.queue.write_buffer(&self.vertex_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&QUAD_VERTICES));

        // Create the texture resources
        let mut tex_a = self.create_texture_resource();
        let mut tex_b = self.create_texture_resource();

        // Create bindings for reading and writing to textures
        self.create_transition_bind_group(&mut tex_a, &mut tex_b, &transition_bind_group_layout);
        self.create_transition_bind_group(&mut tex_b, &mut tex_a, &transition_bind_group_layout);

        // Create bindings for displaying textures
        self.create_display_bind_group(&mut tex_a, &display_bind_group_layout);
        self.create_display_bind_group(&mut tex_b, &display_bind_group_layout);

        // Create the texture swapper
        self.texture_swapper = Some(TextureSwapper {
            index: 0,
            texture_resources: [
                tex_a,
                tex_b,
            ],
        });
    }

    fn create_texture_resource(&self) -> TextureResource {
        let texture = self.device.create_texture(
            &wgpu::TextureDescriptor{
                label: Some("texture"),
                size: wgpu::Extent3d {
                    width: self.texture_size.0,
                    height: self.texture_size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rg32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });

        TextureResource {
            texture,
            view,
            sampler,
            transition_bind_group: None,
            display_bind_group: None,
        }
    }

    fn create_transition_bind_group(
        &self, 
        read: &mut TextureResource, 
        write: &mut TextureResource, 
        layout: &wgpu::BindGroupLayout
    ) {
        read.transition_bind_group = Some(self.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&read.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&write.view),
                    },
                ],
                label: Some("transition_bind_group"),
            }
        ));
    }

    fn create_display_bind_group(
        &self, 
        resource: &mut TextureResource, 
        layout: &wgpu::BindGroupLayout
    ) {
        resource.display_bind_group = Some(self.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&resource.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&resource.sampler),
                    },
                ],
                label: Some("display_bind_group"),
            }
        ));
    }

    fn compute_work_group_count(
        (width, height): (u32, u32),
        (workgroup_width, workgroup_height): (u32, u32),
    ) -> (u32, u32) {
        let x = (width + workgroup_width - 1) / workgroup_width;
        let y = (height + workgroup_height - 1) / workgroup_height;
    
        (x, y)
    }

    pub fn transition(&mut self) -> CommandBuffer {
        let texture_resource = self.texture_swapper.as_ref().unwrap().get_read_resource();
        let texture_size = texture_resource.texture.size();

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("transition_encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor{
                label: Some("transition_compute_pass"),
                timestamp_writes: None,
            });

            let (dispatch_with, dispatch_height) = RenderState::compute_work_group_count(
                (texture_size.width, texture_size.height), 
                (14, 14)
            );
            // The work groups are actually 16x16 but we use 14x14 because the threads around the edges
            // are only used for populating the shared memory.

            compute_pass.set_pipeline(self.transition_pipeline.as_ref().unwrap());
            compute_pass.set_bind_group(0, texture_resource.transition_bind_group.as_ref().unwrap(), &[]);
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);
        }

        self.texture_swapper.as_mut().unwrap().swap();
        encoder.finish()
    }

    pub fn draw(&mut self) {
        let output = self.surface.get_current_texture().unwrap();
        let output_view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let input_bind_group = &self.texture_swapper.as_ref().unwrap().get_read_resource().display_bind_group;
        let surface_size_bind_group = self.surface_size_bind_group.as_ref().unwrap();

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("display_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("display_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0, // Pick any color you want here
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.display_pipeline.as_ref().unwrap());
            render_pass.set_bind_group(0, input_bind_group.as_ref().unwrap(), &[]);
            render_pass.set_bind_group(1, surface_size_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.as_ref().unwrap().slice(..));
            render_pass.draw(0..4, 0..1);
        }

        let transition_command_buffer = self.transition();

        self.queue.submit([transition_command_buffer, encoder.finish()].into_iter());
        output.present();
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);

            // Upload the surface size
            if self.surface_size_buffer.is_some() {
                self.queue.write_buffer(
                    &self.surface_size_buffer.as_ref().unwrap(), 
                    0, 
                    bytemuck::cast_slice(&[self.surface_config.width, self.surface_config.height])
                );
            }
        }
    }

    pub fn set_texture(&mut self, data: &[u8]) {
        let texture = &self.texture_swapper.as_ref().unwrap().get_write_resource().texture;

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.texture_size.0 * 2 * std::mem::size_of::<f32>() as u32),
                rows_per_image: Some(self.texture_size.1),
            },
            wgpu::Extent3d {
                width: self.texture_size.0,
                height: self.texture_size.1,
                depth_or_array_layers: 1,
            },
        );

        self.texture_swapper.as_mut().unwrap().swap();
    }

    pub fn randomize(&mut self) {
        let width = self.texture_size.0 as usize;
        let height = self.texture_size.1 as usize;
        let capacity = width * height * 2;
        let mut data = vec![0f32; capacity];
        
        for y in 0..(height / 2) {
            for x in 0..(width / 2) {
                let random_value: f32 = match rand::random::<bool>() {
                    true => 1.0,
                    _ => -1.0,
                };

                if x < (width/4) || y < (height/4) {
                    continue;
                }

                if (width / 2 - x) < (width/8) && (height / 2 - y) < (height/8) {
                    continue;
                }

                data[(y * width + x) * 2] = random_value;
                data[(y * width + (width - x - 1)) * 2] = random_value;
                data[((height - y - 1) * width + x) * 2] = random_value;
                data[((height - y - 1) * width + (width - x - 1)) * 2] = random_value;
            }
        }

        self.set_texture(bytemuck::cast_slice(data.as_slice()));
    }
}