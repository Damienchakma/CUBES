use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackendPolicy {
    #[default]
    PreferVulkan,
    PreferOpenGl,
    AllHardware,
}

impl BackendPolicy {
    pub fn backends(self) -> wgpu::Backends {
        match self {
            Self::PreferVulkan => wgpu::Backends::VULKAN,
            Self::PreferOpenGl => wgpu::Backends::GL,
            Self::AllHardware => wgpu::Backends::PRIMARY,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterReport {
    pub name: String,
    pub vendor: String,
    pub architecture: String,
    pub device: String,
    pub driver: String,
    pub backend: String,
    pub device_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsCapabilities {
    pub max_texture_dimension_2d: u32,
    pub max_buffer_size: u64,
    pub max_uniform_buffer_binding_size: u64,
    pub max_storage_buffer_binding_size: u64,
    pub max_vertex_buffers: u32,
    pub supports_timestamp_query: bool,
}

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub report: AdapterReport,
    pub capabilities: GraphicsCapabilities,
}

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("no compatible hardware adapter was found")]
    NoAdapter,
    #[error("adapter request failed: {0}")]
    Adapter(#[from] wgpu::RequestAdapterError),
    #[error("device request failed: {0}")]
    Device(#[from] wgpu::RequestDeviceError),
    #[error("surface creation failed: {0}")]
    Surface(#[from] wgpu::CreateSurfaceError),
    #[error("surface has no supported format")]
    NoSurfaceFormat,
    #[error("glTF import failed: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("glTF asset has no usable mesh: {0}")]
    EmptyAsset(String),
    #[error("glTF asset is missing required vertex data: {0}")]
    MissingVertexData(String),
}

impl GpuContext {
    pub async fn create(policy: BackendPolicy) -> Result<Self, RendererError> {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = policy.backends();
        let instance = wgpu::Instance::new(descriptor);
        Self::from_instance(instance, policy).await
    }

    pub async fn from_instance(
        instance: wgpu::Instance,
        policy: BackendPolicy,
    ) -> Result<Self, RendererError> {
        Self::from_instance_for_surface(instance, policy, None).await
    }

    pub async fn from_instance_for_surface(
        instance: wgpu::Instance,
        _policy: BackendPolicy,
        compatible_surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, RendererError> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("vela-device"),
                required_features: wgpu::Features::empty(),
                required_limits: adapter.limits(),
                memory_hints: wgpu::MemoryHints::Performance,
                ..Default::default()
            })
            .await?;

        let info = adapter.get_info();
        let limits = adapter.limits();
        let report = AdapterReport {
            name: info.name,
            vendor: format!("0x{:04x}", info.vendor),
            architecture: std::env::consts::ARCH.to_owned(),
            device: format!("0x{:04x}", info.device),
            driver: info.driver,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
        };
        let capabilities = GraphicsCapabilities {
            max_texture_dimension_2d: limits.max_texture_dimension_2d,
            max_buffer_size: limits.max_buffer_size,
            max_uniform_buffer_binding_size: limits.max_uniform_buffer_binding_size,
            max_storage_buffer_binding_size: limits.max_storage_buffer_binding_size,
            max_vertex_buffers: limits.max_vertex_buffers,
            supports_timestamp_query: adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY),
        };

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            report,
            capabilities,
        })
    }

    pub fn backend(&self) -> wgpu::Backend {
        self.adapter.get_info().backend
    }
}

impl fmt::Display for AdapterReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({}, {})",
            self.name, self.backend, self.device_type
        )
    }
}
