use crate::backend::RendererError;
use crate::mesh::{MeshData, Vertex};
use glam::Mat4;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ImportedTexture {
    pub key: String,
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub source_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct ImportedMesh {
    pub data: MeshData,
    pub base_color: [f32; 4],
    pub texture: Option<ImportedTexture>,
}

/// Loads the first usable mesh primitive from a glTF 2.0 file.
///
/// Node transforms are baked into the vertices. The first primitive's base-colour factor and texture
/// are returned for material setup. Skins, morph targets, animations, and multiple primitives are
/// not imported by this pipeline slice.
pub fn load_gltf_mesh(path: &Path) -> Result<ImportedMesh, RendererError> {
    let (document, buffers, _images) = gltf::import(path)?;
    let node_transform = document
        .nodes()
        .find(|node| node.mesh().is_some())
        .map(|node| Mat4::from_cols_array_2d(&node.transform().matrix()))
        .unwrap_or(Mat4::IDENTITY);

    let mesh = document
        .meshes()
        .find(|mesh| mesh.primitives().next().is_some())
        .ok_or_else(|| RendererError::EmptyAsset(path.display().to_string()))?;
    let primitive = mesh
        .primitives()
        .next()
        .ok_or_else(|| RendererError::EmptyAsset(path.display().to_string()))?;
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
    let positions = reader
        .read_positions()
        .ok_or_else(|| RendererError::MissingVertexData(path.display().to_string()))?;
    let normals = reader.read_normals();
    let tex_coords = reader
        .read_tex_coords(0)
        .map(|reader| reader.into_f32().collect::<Vec<_>>())
        .unwrap_or_default();
    let mut data = MeshData::default();

    for (index, position) in positions.enumerate() {
        let normal = normals
            .as_ref()
            .and_then(|reader| reader.clone().nth(index))
            .unwrap_or([0.0, 1.0, 0.0]);
        let uv = tex_coords.get(index).copied().unwrap_or([0.0, 0.0]);
        let world_position = node_transform.transform_point3(glam::Vec3::from_array(position));
        let world_normal = node_transform
            .transform_vector3(glam::Vec3::from_array(normal))
            .normalize_or_zero();
        data.vertices.push(Vertex {
            position: world_position.to_array(),
            normal: world_normal.to_array(),
            uv,
        });
    }

    if let Some(indices) = reader.read_indices() {
        data.indices.extend(indices.into_u32());
    } else {
        data.indices.extend(0..data.vertices.len() as u32);
    }
    if data.vertices.is_empty() || data.indices.is_empty() {
        return Err(RendererError::EmptyAsset(path.display().to_string()));
    }

    let material = primitive.material();
    let pbr = material.pbr_metallic_roughness();
    let base_color = pbr.base_color_factor();
    let texture = pbr.base_color_texture().and_then(|info| {
        let image = document.images().nth(info.texture().index())?;
        decode_image(image, &buffers, path.parent().unwrap_or(Path::new(".")))
    });

    Ok(ImportedMesh {
        data,
        base_color,
        texture,
    })
}

fn decode_image(
    image: gltf::Image<'_>,
    buffers: &[gltf::buffer::Data],
    base_dir: &Path,
) -> Option<ImportedTexture> {
    let (key, bytes, source_path) = match image.source() {
        gltf::image::Source::Uri { uri, .. } => {
            let source = base_dir.join(uri);
            let bytes = std::fs::read(&source).ok()?;
            (uri.to_owned(), bytes, Some(source))
        }
        gltf::image::Source::View { view, .. } => {
            let buffer = buffers.get(view.buffer().index())?;
            let start = view.offset();
            let end = start + view.length();
            let bytes = buffer.get(start..end)?.to_vec();
            (format!("embedded-{}.png", image.index()), bytes, None)
        }
    };
    let decoded = image::load_from_memory(&bytes).ok()?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(ImportedTexture {
        key,
        rgba: rgba.into_raw(),
        width,
        height,
        source_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn loads_a_minimal_gltf_triangle() {
        let root = std::env::temp_dir().join(format!(
            "vela-gltf-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut bin = Vec::new();
        for position in [[0.0_f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            bin.extend_from_slice(bytemuck::bytes_of(&position));
        }
        for _ in 0..3 {
            bin.extend_from_slice(bytemuck::bytes_of(&[0.0_f32, 0.0, 1.0]));
        }
        let indices = [0_u16, 1, 2]
            .into_iter()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        bin.extend_from_slice(&indices);
        for _ in 0..3 {
            bin.extend_from_slice(bytemuck::bytes_of(&[0.0_f32, 0.0]));
        }
        std::fs::write(root.join("tri.bin"), &bin).unwrap();

        let manifest = r#"{
  "asset": { "version": "2.0" },
  "scene": 0,
  "scenes": [ { "nodes": [0] } ],
  "nodes": [ { "mesh": 0 } ],
  "meshes": [ { "primitives": [ {
    "attributes": { "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 3 },
    "indices": 2,
    "mode": 4
  } ] } ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] },
    { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC3" },
    { "bufferView": 2, "componentType": 5123, "count": 3, "type": "SCALAR" },
    { "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC2" }
  ],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 72, "byteLength": 6 },
    { "buffer": 0, "byteOffset": 78, "byteLength": 24 }
  ],
  "buffers": [ { "uri": "tri.bin", "byteLength": 102 } ]
}"#;
        let path = root.join("tri.gltf");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(manifest.as_bytes()).unwrap();

        let mesh = load_gltf_mesh(&path).unwrap();
        assert_eq!(mesh.data.vertices.len(), 3);
        assert_eq!(mesh.data.indices, vec![0, 1, 2]);
        assert_eq!(mesh.data.vertices[0].normal, [0.0, 0.0, 1.0]);
        assert_eq!(mesh.base_color, [1.0, 1.0, 1.0, 1.0]);
        assert!(mesh.texture.is_none());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn loads_an_external_base_color_texture() {
        let root = std::env::temp_dir().join(format!(
            "vela-gltf-texture-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let rgba = [
            255_u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
        ];
        crate::write_rgba_png(&root.join("tex.png"), &rgba, 2, 2).unwrap();

        let bin = [
            0.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0,
        ]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>();
        std::fs::write(root.join("tri.bin"), &bin).unwrap();
        let manifest = r#"{
  "asset": { "version": "2.0" },
  "scene": 0,
  "scenes": [ { "nodes": [0] } ],
  "nodes": [ { "mesh": 0 } ],
  "meshes": [ { "primitives": [ {
    "attributes": { "POSITION": 0, "TEXCOORD_0": 1 },
    "material": 0,
    "mode": 4
  } ] } ],
  "materials": [ { "pbrMetallicRoughness": {
    "baseColorFactor": [0.5, 0.25, 0.75, 1.0],
    "baseColorTexture": { "index": 0 }
  } } ],
  "textures": [ { "sampler": 0, "source": 0 } ],
  "samplers": [ { "magFilter": 9729, "minFilter": 9729, "wrapS": 10497, "wrapT": 10497 } ],
  "images": [ { "uri": "tex.png" } ],
  "accessors": [
    { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0,0,0], "max": [1,1,0] },
    { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" }
  ],
  "bufferViews": [
    { "buffer": 0, "byteOffset": 0, "byteLength": 36 },
    { "buffer": 0, "byteOffset": 36, "byteLength": 24 }
  ],
  "buffers": [ { "uri": "tri.bin", "byteLength": 60 } ]
}"#;
        let path = root.join("tri.gltf");
        std::fs::write(&path, manifest).unwrap();

        let imported = load_gltf_mesh(&path).unwrap();
        assert_eq!(imported.base_color, [0.5, 0.25, 0.75, 1.0]);
        let texture = imported.texture.unwrap();
        assert_eq!(texture.key, "tex.png");
        assert_eq!((texture.width, texture.height), (2, 2));
        assert_eq!(texture.rgba.len(), 16);

        std::fs::remove_dir_all(root).unwrap();
    }
}
