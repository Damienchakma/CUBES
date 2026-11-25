use crate::camera::Camera;
use crate::mesh::{Mesh, MeshData, Vertex};
use glam::{Mat4, Vec3};
use std::collections::HashMap;
use vela_scene::PrimitiveKind;
use wgpu::util::DeviceExt;

const GRID_SIZE: i32 = 10;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    view_projection: [[f32; 4]; 4],
    camera_position: [f32; 4],
    light_direction: [f32; 4],
    light_color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceVertex {
    model: [[f32; 4]; 4],
    base_color: [f32; 4],
    material: [f32; 4],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MeshKey {
    Primitive(PrimitiveKind),
    Asset(String),
}

impl MeshKey {
    fn label(&self) -> String {
        match self {
            Self::Primitive(kind) => format!("{kind:?}"),
            Self::Asset(path) => path.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RenderInstance {
    pub primitive: PrimitiveKind,
    pub asset: Option<String>,
    pub texture: Option<String>,
    pub model: Mat4,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: f32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct BatchKey {
    mesh: MeshKey,
    texture: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RenderMetrics {
    pub draw_calls: u32,
    pub triangles: u32,
    pub instances: u32,
}

struct MeshGpu {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

pub struct SceneRenderer {
    pipeline: wgpu::RenderPipeline,
    grid_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    default_texture_bind_group: wgpu::BindGroup,
    texture_bind_groups: HashMap<String, wgpu::BindGroup>,
    textures: HashMap<String, wgpu::Texture>,
    globals_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    grid_instance_buffer: wgpu::Buffer,
    meshes: HashMap<MeshKey, MeshGpu>,
    grid: MeshGpu,
}

impl SceneRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vela-scene-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/scene.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vela-globals-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Globals>() as u64),
                },
                count: None,
            }],
        });
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("vela-texture-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vela-scene-pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout), Some(&texture_bind_group_layout)],
            immediate_size: 0,
        });

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vela-globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vela-globals-bind-group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("vela-material-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });
        let default_texture = create_texture_gpu(
            device,
            queue,
            "vela-white-texture",
            &[255, 255, 255, 255],
            1,
            1,
        );
        let default_texture_view =
            default_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let default_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vela-default-texture-bind-group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&default_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x3,
                1 => Float32x3,
                2 => Float32x2,
            ],
        };
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![
                3 => Float32x4,
                4 => Float32x4,
                5 => Float32x4,
                6 => Float32x4,
                7 => Float32x4,
                8 => Float32x4,
            ],
        };

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vela-scene-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(vertex_layout.clone()), Some(instance_layout.clone())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let grid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vela-grid-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(vertex_layout), Some(instance_layout)],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let meshes = [
            PrimitiveKind::Cube,
            PrimitiveKind::Sphere,
            PrimitiveKind::Plane,
            PrimitiveKind::Cylinder,
        ]
        .into_iter()
        .map(|kind| {
            (
                MeshKey::Primitive(kind),
                upload_mesh(device, &Mesh::primitive(kind).data, "mesh"),
            )
        })
        .collect();

        let grid = upload_mesh(device, &grid_mesh(), "grid");
        let instance_capacity = 256;
        let instance_buffer = create_instance_buffer(device, instance_capacity);
        let grid_instance = InstanceVertex {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            base_color: [0.22, 0.25, 0.31, 1.0],
            material: [0.0, 1.0, 0.0, 0.0],
        };
        let grid_instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grid-instance"),
            contents: bytemuck::bytes_of(&grid_instance),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            pipeline,
            grid_pipeline,
            bind_group,
            texture_bind_group_layout,
            sampler,
            default_texture_bind_group,
            texture_bind_groups: HashMap::new(),
            textures: HashMap::new(),
            globals_buffer,
            instance_buffer,
            instance_capacity,
            grid_instance_buffer,
            meshes,
            grid,
        }
    }

    /// Uploads an imported mesh under a stable project-relative key.
    pub fn load_asset_mesh(
        &mut self,
        device: &wgpu::Device,
        key: impl Into<String>,
        data: &MeshData,
    ) {
        let key = MeshKey::Asset(key.into());
        self.meshes
            .insert(key, upload_mesh(device, data, "asset-mesh"));
    }

    pub fn has_asset_mesh(&self, key: &str) -> bool {
        self.meshes.contains_key(&MeshKey::Asset(key.to_owned()))
    }

    /// Uploads a decoded RGBA8 texture under a stable project-relative key.
    pub fn load_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: impl Into<String>,
        rgba: &[u8],
        width: u32,
        height: u32,
    ) {
        let key = key.into();
        if self.texture_bind_groups.contains_key(&key) {
            return;
        }
        let texture = create_texture_gpu(device, queue, "asset-texture", rgba, width, height);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vela-asset-texture-bind-group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.textures.insert(key.clone(), texture);
        self.texture_bind_groups.insert(key, bind_group);
    }

    pub fn has_asset_texture(&self, key: &str) -> bool {
        self.texture_bind_groups.contains_key(key)
    }

    pub fn resize_instance_buffer(&mut self, device: &wgpu::Device, required: usize) {
        let required = required.max(1);
        if required > self.instance_capacity {
            self.instance_capacity = required.next_power_of_two();
            self.instance_buffer = create_instance_buffer(device, self.instance_capacity);
        }
    }

    pub fn render(
        &mut self,
        render_pass: &mut wgpu::RenderPass<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera: &Camera,
        aspect_ratio: f32,
        instances: &[RenderInstance],
        light_direction: Vec3,
        light_color: [f32; 3],
        light_intensity: f32,
    ) -> RenderMetrics {
        let globals = Globals {
            view_projection: camera.view_projection(aspect_ratio).to_cols_array_2d(),
            camera_position: camera.position.extend(1.0).to_array(),
            light_direction: light_direction.normalize_or_zero().extend(0.0).to_array(),
            light_color: [
                light_color[0] * light_intensity,
                light_color[1] * light_intensity,
                light_color[2] * light_intensity,
                1.0,
            ],
        };
        queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        let mut grouped: HashMap<BatchKey, Vec<InstanceVertex>> = HashMap::new();
        for instance in instances {
            let mesh = instance
                .asset
                .as_ref()
                .map(|path| MeshKey::Asset(path.clone()))
                .unwrap_or(MeshKey::Primitive(instance.primitive));
            let key = BatchKey {
                mesh,
                texture: instance.texture.clone(),
            };
            grouped.entry(key).or_default().push(InstanceVertex {
                model: instance.model.to_cols_array_2d(),
                base_color: instance.base_color,
                material: [
                    instance.metallic,
                    instance.roughness.clamp(0.04, 1.0),
                    instance.emissive,
                    0.0,
                ],
            });
        }

        let mut keys = grouped.keys().cloned().collect::<Vec<_>>();
        keys.sort_by_key(|key| (key.mesh.label(), key.texture.clone().unwrap_or_default()));
        let mut upload = Vec::with_capacity(instances.len());
        let mut batches = Vec::new();
        for key in keys {
            let batch = grouped.remove(&key).expect("key came from grouped");
            let first_instance = upload.len() as u32;
            let count = batch.len() as u32;
            upload.extend_from_slice(&batch);
            batches.push((key, first_instance, count));
        }
        self.resize_instance_buffer(device, upload.len());
        if !upload.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&upload));
        }

        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.default_texture_bind_group, &[]);
        render_pass.set_pipeline(&self.grid_pipeline);
        render_pass.set_vertex_buffer(0, self.grid.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.grid.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.set_vertex_buffer(1, self.grid_instance_buffer.slice(..));
        render_pass.draw_indexed(0..self.grid.index_count, 0, 0..1);

        render_pass.set_pipeline(&self.pipeline);
        let mut metrics = RenderMetrics {
            draw_calls: 1,
            ..RenderMetrics::default()
        };
        for (key, first_instance, count) in batches {
            let Some(mesh) = self.meshes.get(&key.mesh) else {
                continue;
            };
            let texture_bind_group = key
                .texture
                .as_ref()
                .and_then(|texture| self.texture_bind_groups.get(texture))
                .unwrap_or(&self.default_texture_bind_group);
            render_pass.set_bind_group(1, texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.draw_indexed(
                0..mesh.index_count,
                0,
                first_instance..first_instance + count,
            );
            metrics.draw_calls += 1;
            metrics.triangles += mesh.index_count / 3 * count;
            metrics.instances += count;
        }
        metrics
    }
}

fn create_texture_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    label: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> wgpu::Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    texture
}

/// Persists decoded RGBA pixels as a PNG so imported textures survive project reloads.
pub fn write_rgba_png(
    path: &std::path::Path,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8)
        .map_err(|error| error.to_string())
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("vela-instances"),
        size: (capacity * std::mem::size_of::<InstanceVertex>()) as u64,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn upload_mesh(device: &wgpu::Device, mesh: &MeshData, label: &str) -> MeshGpu {
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{label}-vertices")),
        contents: bytemuck::cast_slice(&mesh.vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{label}-indices")),
        contents: bytemuck::cast_slice(&mesh.indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    MeshGpu {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
    }
}

fn grid_mesh() -> MeshData {
    let mut data = MeshData::default();
    for coordinate in -GRID_SIZE..=GRID_SIZE {
        let coordinate = coordinate as f32;
        data.vertices.push(Vertex {
            position: [coordinate, 0.0, -GRID_SIZE as f32],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        });
        data.vertices.push(Vertex {
            position: [coordinate, 0.0, GRID_SIZE as f32],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        });
        data.vertices.push(Vertex {
            position: [-GRID_SIZE as f32, 0.0, coordinate],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        });
        data.vertices.push(Vertex {
            position: [GRID_SIZE as f32, 0.0, coordinate],
            normal: [0.0, 1.0, 0.0],
            uv: [0.0, 0.0],
        });
    }
    data.indices = (0..data.vertices.len() as u32).collect();
    data
}
