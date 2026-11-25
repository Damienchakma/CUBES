use crate::components::{Component, ComponentKind, Transform};
use glam::Mat4;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use vela_core::{EntityId, project::write_json_atomic};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRecord {
    pub id: EntityId,
    pub name: String,
    pub parent: Option<EntityId>,
    pub visible: bool,
    pub locked: bool,
    pub components: BTreeMap<ComponentKind, Component>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scene {
    pub schema_version: u32,
    pub name: String,
    pub entities: BTreeMap<EntityId, EntityRecord>,
    pub next_entity_id: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SceneError {
    #[error("entity {0} does not exist")]
    MissingEntity(EntityId),
    #[error("parenting {child} to {parent} would create a hierarchy cycle")]
    HierarchyCycle { child: EntityId, parent: EntityId },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            schema_version: 1,
            name: "Main Scene".to_owned(),
            entities: BTreeMap::new(),
            next_entity_id: 1,
        }
    }
}

impl Scene {
    /// Creates a small, valid starter scene used by new projects and tutorials.
    pub fn starter_scene() -> Self {
        let mut scene = Self::default();

        let camera = scene.create_entity("Main Camera");
        if let Ok(entity) = scene.entity_mut(camera) {
            entity.components.insert(
                ComponentKind::Transform,
                Component::Transform(Transform {
                    translation: glam::Vec3::new(0.0, 2.5, 7.0),
                    ..Transform::default()
                }),
            );
            entity.components.insert(
                ComponentKind::Camera,
                Component::Camera(crate::components::CameraComponent::default()),
            );
        }

        let light = scene.create_entity("Sun");
        if let Ok(entity) = scene.entity_mut(light) {
            entity.components.insert(
                ComponentKind::Transform,
                Component::Transform(Transform {
                    translation: glam::Vec3::new(4.0, 8.0, 3.0),
                    ..Transform::default()
                }),
            );
            entity.components.insert(
                ComponentKind::Light,
                Component::Light(crate::components::LightComponent::default()),
            );
        }

        let cube = scene.create_entity("Player Cube");
        if let Ok(entity) = scene.entity_mut(cube) {
            entity.components.insert(
                ComponentKind::Mesh,
                Component::Mesh(crate::components::MeshComponent::default()),
            );
            entity.components.insert(
                ComponentKind::Material,
                Component::Material(crate::components::MaterialComponent {
                    base_color: [0.18, 0.62, 0.92, 1.0],
                    metallic: 0.05,
                    roughness: 0.42,
                    emissive: [0.0; 3],
                    base_color_texture: None,
                }),
            );
            entity.components.insert(
                ComponentKind::Physics,
                Component::Physics(crate::components::PhysicsComponent::default()),
            );
        }

        let ground = scene.create_entity("Ground");
        if let Ok(entity) = scene.entity_mut(ground) {
            entity.components.insert(
                ComponentKind::Transform,
                Component::Transform(Transform {
                    translation: glam::Vec3::new(0.0, -1.0, 0.0),
                    scale: glam::Vec3::new(8.0, 0.1, 8.0),
                    ..Transform::default()
                }),
            );
            entity.components.insert(
                ComponentKind::Mesh,
                Component::Mesh(crate::components::MeshComponent {
                    primitive: crate::components::PrimitiveKind::Plane,
                    ..crate::components::MeshComponent::default()
                }),
            );
            entity.components.insert(
                ComponentKind::Material,
                Component::Material(crate::components::MaterialComponent {
                    base_color: [0.16, 0.19, 0.23, 1.0],
                    metallic: 0.0,
                    roughness: 0.9,
                    emissive: [0.0; 3],
                    base_color_texture: None,
                }),
            );
        }

        scene
    }

    pub fn create_entity(&mut self, name: impl Into<String>) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;
        self.entities.insert(
            id,
            EntityRecord {
                id,
                name: name.into(),
                parent: None,
                visible: true,
                locked: false,
                components: BTreeMap::from([(
                    ComponentKind::Transform,
                    Component::Transform(Transform::default()),
                )]),
            },
        );
        id
    }

    pub fn create_entity_with_parent(
        &mut self,
        name: impl Into<String>,
        parent: EntityId,
    ) -> Result<EntityId, SceneError> {
        if !self.entities.contains_key(&parent) {
            return Err(SceneError::MissingEntity(parent));
        }
        let id = self.create_entity(name);
        self.set_parent(id, Some(parent))?;
        Ok(id)
    }

    pub fn delete_entity(&mut self, id: EntityId) -> Result<(), SceneError> {
        if !self.entities.contains_key(&id) {
            return Err(SceneError::MissingEntity(id));
        }
        let children: Vec<_> = self
            .entities
            .values()
            .filter(|entity| entity.parent == Some(id))
            .map(|entity| entity.id)
            .collect();
        for child in children {
            self.delete_entity(child)?;
        }
        self.entities.remove(&id);
        Ok(())
    }

    pub fn entity(&self, id: EntityId) -> Result<&EntityRecord, SceneError> {
        self.entities.get(&id).ok_or(SceneError::MissingEntity(id))
    }

    pub fn entity_mut(&mut self, id: EntityId) -> Result<&mut EntityRecord, SceneError> {
        self.entities
            .get_mut(&id)
            .ok_or(SceneError::MissingEntity(id))
    }

    pub fn add_component(&mut self, id: EntityId, component: Component) -> Result<(), SceneError> {
        let entity = self.entity_mut(id)?;
        entity.components.insert(component.kind(), component);
        Ok(())
    }

    pub fn remove_component(
        &mut self,
        id: EntityId,
        kind: ComponentKind,
    ) -> Result<(), SceneError> {
        if kind == ComponentKind::Transform {
            return Ok(());
        }
        let entity = self.entity_mut(id)?;
        entity.components.remove(&kind);
        Ok(())
    }

    pub fn component(&self, id: EntityId, kind: ComponentKind) -> Result<&Component, SceneError> {
        self.entity(id)?
            .components
            .get(&kind)
            .ok_or(SceneError::MissingEntity(id))
    }

    pub fn component_mut(
        &mut self,
        id: EntityId,
        kind: ComponentKind,
    ) -> Result<&mut Component, SceneError> {
        self.entity_mut(id)?
            .components
            .get_mut(&kind)
            .ok_or(SceneError::MissingEntity(id))
    }

    pub fn set_parent(&mut self, id: EntityId, parent: Option<EntityId>) -> Result<(), SceneError> {
        if !self.entities.contains_key(&id) {
            return Err(SceneError::MissingEntity(id));
        }
        if let Some(parent) = parent {
            if parent == id || self.is_descendant(parent, id) {
                return Err(SceneError::HierarchyCycle { child: id, parent });
            }
        }
        self.entity_mut(id)?.parent = parent;
        Ok(())
    }

    fn is_descendant(&self, candidate: EntityId, ancestor: EntityId) -> bool {
        let mut current = Some(candidate);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.entities.get(&id).and_then(|entity| entity.parent);
        }
        false
    }

    pub fn world_transform(&self, id: EntityId) -> Result<Mat4, SceneError> {
        let entity = self.entity(id)?;
        let local = entity
            .components
            .get(&ComponentKind::Transform)
            .and_then(|component| match component {
                Component::Transform(transform) => Some(transform.local_matrix()),
                _ => None,
            })
            .unwrap_or_else(|| Transform::default().local_matrix());
        let parent = entity.parent;
        match parent {
            Some(parent) => Ok(self.world_transform(parent)? * local),
            None => Ok(local),
        }
    }

    pub fn roots(&self) -> Vec<EntityId> {
        self.entities
            .values()
            .filter(|entity| entity.parent.is_none())
            .map(|entity| entity.id)
            .collect()
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SceneError> {
        write_json_atomic(path.as_ref(), self).map_err(|error| match error {
            vela_core::Error::Io { source, .. } => SceneError::Io(source),
            vela_core::Error::Serialization(error) => SceneError::Serialization(error),
            other => SceneError::Io(std::io::Error::other(other.to_string())),
        })
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, SceneError> {
        let bytes = std::fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchy_cycle_is_rejected() {
        let mut scene = Scene::default();
        let parent = scene.create_entity("Parent");
        let child = scene.create_entity_with_parent("Child", parent).unwrap();
        assert!(matches!(
            scene.set_parent(parent, Some(child)),
            Err(SceneError::HierarchyCycle { .. })
        ));
    }

    #[test]
    fn deleting_parent_deletes_children() {
        let mut scene = Scene::default();
        let parent = scene.create_entity("Parent");
        let child = scene.create_entity_with_parent("Child", parent).unwrap();
        scene.delete_entity(parent).unwrap();
        assert!(scene.entity(child).is_err());
    }
}
