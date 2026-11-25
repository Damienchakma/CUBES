//! Hardware-accelerated wgpu renderer for CUBES.

mod backend;
mod camera;
mod gltf;
mod mesh;
mod renderer;

pub use backend::{AdapterReport, BackendPolicy, GpuContext, GraphicsCapabilities, RendererError};
pub use camera::{Camera, CameraProjection};
pub use gltf::{ImportedMesh, ImportedTexture, load_gltf_mesh};
pub use mesh::{Mesh, MeshData, Vertex};
pub use renderer::{RenderInstance, RenderMetrics, SceneRenderer, write_rgba_png};
