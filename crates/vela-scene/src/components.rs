use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentKind {
    Transform,
    Mesh,
    Material,
    Camera,
    Light,
    Physics,
    Script,
    AudioSource,
}

impl ComponentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Transform => "Transform",
            Self::Mesh => "Mesh",
            Self::Material => "Material",
            Self::Camera => "Camera",
            Self::Light => "Light",
            Self::Physics => "Physics",
            Self::Script => "Script",
            Self::AudioSource => "AudioSource",
        }
    }
}

impl FromStr for ComponentKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Transform" => Ok(Self::Transform),
            "Mesh" => Ok(Self::Mesh),
            "Material" => Ok(Self::Material),
            "Camera" => Ok(Self::Camera),
            "Light" => Ok(Self::Light),
            "Physics" => Ok(Self::Physics),
            "Script" => Ok(Self::Script),
            "AudioSource" => Ok(Self::AudioSource),
            _ => Err(format!("unknown component type: {value}")),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "properties", rename_all = "camelCase")]
pub enum Component {
    Transform(Transform),
    Mesh(MeshComponent),
    Material(MaterialComponent),
    Camera(CameraComponent),
    Light(LightComponent),
    Physics(PhysicsComponent),
    Script(ScriptComponent),
    AudioSource(AudioSourceComponent),
}

impl Component {
    pub fn kind(&self) -> ComponentKind {
        match self {
            Self::Transform(_) => ComponentKind::Transform,
            Self::Mesh(_) => ComponentKind::Mesh,
            Self::Material(_) => ComponentKind::Material,
            Self::Camera(_) => ComponentKind::Camera,
            Self::Light(_) => ComponentKind::Light,
            Self::Physics(_) => ComponentKind::Physics,
            Self::Script(_) => ComponentKind::Script,
            Self::AudioSource(_) => ComponentKind::AudioSource,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn local_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshComponent {
    pub asset: Option<String>,
    pub primitive: PrimitiveKind,
    pub visible: bool,
    pub cast_shadow: bool,
}

impl Default for MeshComponent {
    fn default() -> Self {
        Self {
            asset: None,
            primitive: PrimitiveKind::default(),
            visible: true,
            cast_shadow: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrimitiveKind {
    #[default]
    Cube,
    Sphere,
    Plane,
    Cylinder,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaterialComponent {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive: [f32; 3],
    #[serde(default)]
    pub base_color_texture: Option<String>,
}

impl Default for MaterialComponent {
    fn default() -> Self {
        Self {
            base_color: [0.72, 0.76, 0.82, 1.0],
            metallic: 0.0,
            roughness: 0.65,
            emissive: [0.0; 3],
            base_color_texture: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraComponent {
    pub projection: Projection,
    pub vertical_fov_degrees: f32,
    pub near: f32,
    pub far: f32,
    pub clear_color: [f32; 4],
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self {
            projection: Projection::Perspective,
            vertical_fov_degrees: 60.0,
            near: 0.05,
            far: 500.0,
            clear_color: [0.035, 0.047, 0.065, 1.0],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Projection {
    #[default]
    Perspective,
    Orthographic,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightComponent {
    pub light_type: LightType,
    pub color: [f32; 3],
    pub intensity: f32,
    pub range: f32,
    pub cast_shadow: bool,
}

impl Default for LightComponent {
    fn default() -> Self {
        Self {
            light_type: LightType::Directional,
            color: [1.0, 0.92, 0.82],
            intensity: 3.0,
            range: 20.0,
            cast_shadow: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LightType {
    #[default]
    Directional,
    Point,
    Spot,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicsComponent {
    pub body_type: BodyType,
    pub mass: f32,
    pub collider: ColliderShape,
    pub friction: f32,
    pub restitution: f32,
    pub is_trigger: bool,
}

impl Default for PhysicsComponent {
    fn default() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            mass: 1.0,
            collider: ColliderShape::Box {
                half_extents: [0.5, 0.5, 0.5],
            },
            friction: 0.6,
            restitution: 0.1,
            is_trigger: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyType {
    Static,
    #[default]
    Dynamic,
    Kinematic,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", content = "parameters", rename_all = "camelCase")]
pub enum ColliderShape {
    Box { half_extents: [f32; 3] },
    Sphere { radius: f32 },
    Capsule { radius: f32, half_height: f32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptComponent {
    pub path: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSourceComponent {
    pub asset: Option<String>,
    pub volume: f32,
    pub spatial: bool,
    pub looping: bool,
}

impl Default for AudioSourceComponent {
    fn default() -> Self {
        Self {
            asset: None,
            volume: 1.0,
            spatial: false,
            looping: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meshes_are_visible_by_default() {
        let mesh = MeshComponent::default();
        assert!(mesh.visible);
        assert!(mesh.cast_shadow);
    }
}
