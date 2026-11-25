//! Asynchronous asset import and dependency tracking.

use sha2::Digest;
use std::path::Path;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAsset {
    pub source_path: String,
    pub kind: AssetKind,
    pub imported_path: String,
    pub checksum: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Texture,
    Model,
    Audio,
    Scene,
    Script,
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("unsupported asset extension: {0}")]
    UnsupportedExtension(String),
    #[error("asset is missing: {0}")]
    Missing(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid asset file name: {0}")]
    InvalidName(String),
    #[error("image import failed: {0}")]
    Image(String),
    #[error("glTF import failed: {0}")]
    Gltf(String),
}

pub fn import_asset(source: &Path) -> Result<ImportedAsset, AssetError> {
    if !source.exists() {
        return Err(AssetError::Missing(source.display().to_string()));
    }
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let kind = match extension.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "ktx2" => AssetKind::Texture,
        "gltf" | "glb" | "obj" => AssetKind::Model,
        "wav" | "ogg" | "mp3" => AssetKind::Audio,
        "velascene" => AssetKind::Scene,
        "lua" => AssetKind::Script,
        _ => return Err(AssetError::UnsupportedExtension(extension)),
    };
    let bytes = std::fs::read(source)?;
    let checksum = format!("{:x}", sha2::Sha256::digest(&bytes));
    Ok(ImportedAsset {
        source_path: source.display().to_string(),
        kind,
        imported_path: source.display().to_string(),
        checksum,
    })
}

/// Copies a validated asset into a project-owned directory.
pub fn import_asset_into(
    source: &Path,
    destination_dir: &Path,
) -> Result<ImportedAsset, AssetError> {
    let mut asset = import_asset(source)?;
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AssetError::InvalidName(source.display().to_string()))?;
    if Path::new(name).file_name().and_then(|value| value.to_str()) != Some(name)
        || name == "."
        || name == ".."
    {
        return Err(AssetError::InvalidName(name.to_owned()));
    }
    std::fs::create_dir_all(destination_dir)?;
    let destination = destination_dir.join(name);
    std::fs::copy(source, &destination)?;
    asset.imported_path = destination.display().to_string();
    Ok(asset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_copies_asset_and_preserves_checksum() {
        let root = std::env::temp_dir().join(format!(
            "vela-assets-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("source")).unwrap();
        let source = root.join("source/level.lua");
        std::fs::write(&source, b"function on_update() end").unwrap();

        let asset = import_asset_into(&source, &root.join("assets")).unwrap();
        assert_eq!(asset.kind, AssetKind::Script);
        assert_eq!(
            asset.imported_path,
            root.join("assets/level.lua").display().to_string()
        );
        assert_eq!(
            import_asset(&root.join("assets/level.lua"))
                .unwrap()
                .checksum,
            asset.checksum
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}
