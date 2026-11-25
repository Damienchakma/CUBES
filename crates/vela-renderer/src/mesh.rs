use vela_scene::PrimitiveKind;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct Mesh {
    pub data: MeshData,
}

impl Mesh {
    pub fn primitive(kind: PrimitiveKind) -> Self {
        let data = match kind {
            PrimitiveKind::Cube => cube(),
            PrimitiveKind::Sphere => sphere(16, 10),
            PrimitiveKind::Plane => plane(),
            PrimitiveKind::Cylinder => cylinder(16),
        };
        Self { data }
    }
}

fn push_vertex(data: &mut MeshData, position: [f32; 3], normal: [f32; 3], uv: [f32; 2]) -> u32 {
    let index = data.vertices.len() as u32;
    data.vertices.push(Vertex {
        position,
        normal,
        uv,
    });
    index
}

fn quad(data: &mut MeshData, corners: [[f32; 3]; 4], normal: [f32; 3]) {
    let base = data.vertices.len() as u32;
    let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    for (corner, uv) in corners.into_iter().zip(uvs) {
        push_vertex(data, corner, normal, uv);
    }
    data.indices
        .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

fn cube() -> MeshData {
    let mut data = MeshData::default();
    quad(
        &mut data,
        [
            [-1.0, -1.0, 1.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
            [-1.0, 1.0, 1.0],
        ],
        [0.0, 0.0, 1.0],
    );
    quad(
        &mut data,
        [
            [1.0, -1.0, -1.0],
            [-1.0, -1.0, -1.0],
            [-1.0, 1.0, -1.0],
            [1.0, 1.0, -1.0],
        ],
        [0.0, 0.0, -1.0],
    );
    quad(
        &mut data,
        [
            [1.0, -1.0, 1.0],
            [1.0, -1.0, -1.0],
            [1.0, 1.0, -1.0],
            [1.0, 1.0, 1.0],
        ],
        [1.0, 0.0, 0.0],
    );
    quad(
        &mut data,
        [
            [-1.0, -1.0, -1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, 1.0, 1.0],
            [-1.0, 1.0, -1.0],
        ],
        [-1.0, 0.0, 0.0],
    );
    quad(
        &mut data,
        [
            [-1.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, -1.0],
            [-1.0, 1.0, -1.0],
        ],
        [0.0, 1.0, 0.0],
    );
    quad(
        &mut data,
        [
            [-1.0, -1.0, -1.0],
            [1.0, -1.0, -1.0],
            [1.0, -1.0, 1.0],
            [-1.0, -1.0, 1.0],
        ],
        [0.0, -1.0, 0.0],
    );
    data
}

fn plane() -> MeshData {
    let mut data = MeshData::default();
    quad(
        &mut data,
        [
            [-1.0, 0.0, -1.0],
            [-1.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, -1.0],
        ],
        [0.0, 1.0, 0.0],
    );
    data
}

fn sphere(segments: usize, rings: usize) -> MeshData {
    let mut data = MeshData::default();
    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;
        for segment in 0..=segments {
            let u = segment as f32 / segments as f32;
            let theta = u * std::f32::consts::TAU;
            let normal = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
            push_vertex(&mut data, normal, normal, [u, v]);
        }
    }
    let stride = segments + 1;
    for ring in 0..rings {
        for segment in 0..segments {
            let current = (ring * stride + segment) as u32;
            let next = current + stride as u32;
            data.indices.extend_from_slice(&[
                current,
                next,
                current + 1,
                current + 1,
                next,
                next + 1,
            ]);
        }
    }
    data
}

fn cylinder(segments: usize) -> MeshData {
    let mut data = MeshData::default();
    for segment in 0..=segments {
        let angle = segment as f32 / segments as f32 * std::f32::consts::TAU;
        let (x, z) = (angle.cos(), angle.sin());
        let u = segment as f32 / segments as f32;
        push_vertex(&mut data, [x, -1.0, z], [x, 0.0, z], [u, 1.0]);
        push_vertex(&mut data, [x, 1.0, z], [x, 0.0, z], [u, 0.0]);
    }
    for segment in 0..segments {
        let base = (segment * 2) as u32;
        data.indices
            .extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
    }
    let top_center = push_vertex(&mut data, [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.5, 0.5]);
    let top_start = data.vertices.len() as u32;
    for segment in 0..=segments {
        let angle = segment as f32 / segments as f32 * std::f32::consts::TAU;
        push_vertex(
            &mut data,
            [angle.cos(), 1.0, angle.sin()],
            [0.0, 1.0, 0.0],
            [0.5 + angle.cos() * 0.5, 0.5 + angle.sin() * 0.5],
        );
    }
    for segment in 0..segments {
        data.indices.extend_from_slice(&[
            top_center,
            top_start + segment as u32,
            top_start + segment as u32 + 1,
        ]);
    }
    let bottom_center = push_vertex(&mut data, [0.0, -1.0, 0.0], [0.0, -1.0, 0.0], [0.5, 0.5]);
    let bottom_start = data.vertices.len() as u32;
    for segment in 0..=segments {
        let angle = segment as f32 / segments as f32 * std::f32::consts::TAU;
        push_vertex(
            &mut data,
            [angle.cos(), -1.0, angle.sin()],
            [0.0, -1.0, 0.0],
            [0.5 + angle.cos() * 0.5, 0.5 + angle.sin() * 0.5],
        );
    }
    for segment in 0..segments {
        data.indices.extend_from_slice(&[
            bottom_center,
            bottom_start + segment as u32 + 1,
            bottom_start + segment as u32,
        ]);
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_winds_counter_clockwise_from_above() {
        let mesh = Mesh::primitive(PrimitiveKind::Plane);
        let indices = &mesh.data.indices[..3];
        let a = mesh.data.vertices[indices[0] as usize].position;
        let b = mesh.data.vertices[indices[1] as usize].position;
        let c = mesh.data.vertices[indices[2] as usize].position;
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let normal_y = ab[2] * ac[0] - ab[0] * ac[2];
        assert!(normal_y > 0.0);
    }

    #[test]
    fn primitives_have_valid_indices() {
        for kind in [
            PrimitiveKind::Cube,
            PrimitiveKind::Sphere,
            PrimitiveKind::Plane,
            PrimitiveKind::Cylinder,
        ] {
            let mesh = Mesh::primitive(kind);
            assert!(!mesh.data.vertices.is_empty());
            assert!(!mesh.data.indices.is_empty());
            assert!(
                mesh.data
                    .indices
                    .iter()
                    .all(|index| { (*index as usize) < mesh.data.vertices.len() })
            );
        }
    }
}
