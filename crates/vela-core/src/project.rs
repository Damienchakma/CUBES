use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const PROJECT_FILE: &str = "project.vela.json";
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// On-disk project metadata. Unknown fields are retained by Serde defaults for forward compatibility.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub schema_version: u32,
    pub name: String,
    pub version: String,
    pub main_scene: PathBuf,
    pub target: String,
    pub render: RenderSettings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSettings {
    pub width: u32,
    pub height: u32,
    pub target_fps: u32,
    pub vsync: bool,
    pub msaa_samples: u32,
    pub shadow_resolution: u32,
    pub dynamic_resolution_scale: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            target_fps: 60,
            vsync: true,
            msaa_samples: 1,
            shadow_resolution: 1024,
            dynamic_resolution_scale: 1.0,
        }
    }
}

/// Canonical project paths and sandbox checks.
#[derive(Clone, Debug)]
pub struct ProjectPaths {
    root: PathBuf,
}

impl ProjectPaths {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let root = root.canonicalize().map_err(|source| Error::Io {
            path: root.clone(),
            source,
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join(PROJECT_FILE)
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn scenes_dir(&self) -> PathBuf {
        self.root.join("scenes")
    }

    pub fn scripts_dir(&self) -> PathBuf {
        self.root.join("scripts")
    }

    pub fn builds_dir(&self) -> PathBuf {
        self.root.join("builds")
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join(".vela")
    }

    /// Resolve an existing project-relative path and reject traversal or symlink escapes.
    pub fn resolve_existing(&self, relative: impl AsRef<Path>) -> Result<PathBuf> {
        let relative = relative.as_ref();
        validate_relative(relative)?;
        let candidate = self.root.join(relative);
        let canonical = candidate.canonicalize().map_err(|source| Error::Io {
            path: candidate.clone(),
            source,
        })?;
        if !canonical.starts_with(&self.root) {
            return Err(Error::SandboxViolation(relative.to_path_buf()));
        }
        Ok(canonical)
    }

    /// Resolve a path whose parent exists. The final component may not exist yet.
    pub fn resolve_for_write(&self, relative: impl AsRef<Path>) -> Result<PathBuf> {
        let relative = relative.as_ref();
        validate_relative(relative)?;
        let candidate = self.root.join(relative);
        let parent = candidate
            .parent()
            .ok_or_else(|| Error::SandboxViolation(relative.to_path_buf()))?;
        let canonical_parent = parent.canonicalize().map_err(|source| Error::Io {
            path: parent.to_path_buf(),
            source,
        })?;
        if !canonical_parent.starts_with(&self.root) {
            return Err(Error::SandboxViolation(relative.to_path_buf()));
        }
        Ok(canonical_parent.join(
            candidate
                .file_name()
                .ok_or_else(|| Error::SandboxViolation(relative.to_path_buf()))?,
        ))
    }
}

fn validate_relative(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(Error::SandboxViolation(path.to_path_buf()));
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            _ => return Err(Error::SandboxViolation(path.to_path_buf())),
        }
    }
    Ok(())
}

/// An opened CUBES project.
#[derive(Clone, Debug)]
pub struct Project {
    paths: ProjectPaths,
    manifest: ProjectManifest,
}

impl Project {
    pub fn create(root: impl AsRef<Path>, name: impl Into<String>) -> Result<Self> {
        let root = root.as_ref();
        fs::create_dir_all(root).map_err(|source| Error::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let paths = ProjectPaths::new(root)?;
        if paths.manifest_path().exists() {
            return Err(Error::ProjectExists(paths.root().to_path_buf()));
        }

        let paths = ProjectPaths::new(root)?;
        for directory in [
            paths.assets_dir(),
            paths.scenes_dir(),
            paths.scripts_dir(),
            paths.builds_dir(),
            paths.cache_dir(),
        ] {
            fs::create_dir_all(&directory).map_err(|source| Error::Io {
                path: directory.clone(),
                source,
            })?;
        }

        let manifest = ProjectManifest {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: name.into(),
            version: "0.1.0".to_owned(),
            main_scene: PathBuf::from("scenes/main.velascene"),
            target: "linux-arm64".to_owned(),
            render: RenderSettings::default(),
        };
        write_json_atomic(&paths.manifest_path(), &manifest)?;
        Ok(Self { paths, manifest })
    }

    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let paths = ProjectPaths::new(root)?;
        let bytes = fs::read(paths.manifest_path()).map_err(|source| Error::Io {
            path: paths.manifest_path(),
            source,
        })?;
        let manifest: ProjectManifest = serde_json::from_slice(&bytes)?;
        if manifest.schema_version > CURRENT_SCHEMA_VERSION {
            return Err(Error::InvalidManifest(format!(
                "schema version {} is newer than supported version {CURRENT_SCHEMA_VERSION}",
                manifest.schema_version
            )));
        }
        Ok(Self { paths, manifest })
    }

    pub fn paths(&self) -> &ProjectPaths {
        &self.paths
    }

    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    pub fn manifest_mut(&mut self) -> &mut ProjectManifest {
        &mut self.manifest
    }

    pub fn save_manifest(&self) -> Result<()> {
        write_json_atomic(&self.paths.manifest_path(), &self.manifest)
    }
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| Error::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(&temporary, bytes).map_err(|source| Error::Io {
        path: temporary.clone(),
        source,
    })?;
    fs::rename(&temporary, path).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_round_trip_and_sandbox() {
        let temporary = std::env::temp_dir().join(format!(
            "vela-project-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let project = Project::create(&temporary, "Test Game").unwrap();
        assert_eq!(project.manifest().name, "Test Game");
        assert!(project.paths().scenes_dir().is_dir());
        assert!(project.paths().resolve_existing("scenes").is_ok());
        assert!(matches!(
            project.paths().resolve_existing("../outside"),
            Err(Error::SandboxViolation(_))
        ));
        fs::remove_dir_all(temporary).unwrap();
    }
}
