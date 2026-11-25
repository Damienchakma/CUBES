//! Hardware discovery and thermal safety policy.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GraphicsApi {
    #[default]
    Unknown,
    Vulkan,
    OpenGlEs,
    OpenGl,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QualityPreset {
    #[default]
    Balanced,
    Performance,
    Quality,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub architecture: String,
    pub model: String,
    pub logical_cores: usize,
    pub features: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicsInfo {
    pub model: String,
    pub driver: String,
    pub preferred_api: GraphicsApi,
    pub vulkan_icd_present: bool,
    pub drm_render_node: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThermalInfo {
    pub temperature_celsius: Option<f32>,
    pub throttling_detected: bool,
    pub warning: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareReport {
    pub board: String,
    pub os: String,
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub graphics: GraphicsInfo,
    pub thermal: ThermalInfo,
    pub recommended_quality: QualityPreset,
}

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("failed to read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("required system file is missing: {0}")]
    MissingFile(PathBuf),
}

pub fn probe() -> Result<HardwareReport, ProbeError> {
    let board = read_trimmed(Path::new("/proc/device-tree/model"))
        .unwrap_or_else(|| "Unknown SBC".to_owned());
    let cpu = probe_cpu()?;
    let memory = probe_memory()?;
    let graphics = probe_graphics();
    let thermal = probe_thermal()?;
    let recommended_quality = recommend_quality(&board, &memory);

    Ok(HardwareReport {
        board,
        os: read_trimmed(Path::new("/etc/os-release"))
            .unwrap_or_else(|| "Unknown Linux distribution".to_owned()),
        cpu,
        memory,
        graphics,
        thermal,
        recommended_quality,
    })
}

fn probe_cpu() -> Result<CpuInfo, ProbeError> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").map_err(|source| ProbeError::Io {
        path: PathBuf::from("/proc/cpuinfo"),
        source,
    })?;
    let mut model = "Unknown ARM CPU".to_owned();
    let mut features = Vec::new();
    let mut logical_cores = 0;
    for line in cpuinfo.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if key == "processor" {
            logical_cores += 1;
        } else if key == "model name" && model == "Unknown ARM CPU" {
            model = value.to_owned();
        } else if key == "Features" && features.is_empty() {
            features = value
                .split_whitespace()
                .filter(|feature| {
                    matches!(
                        *feature,
                        "asimd"
                            | "asimdhp"
                            | "aes"
                            | "sha2"
                            | "crc32"
                            | "atomics"
                            | "fphp"
                            | "fp16"
                    )
                })
                .map(str::to_owned)
                .collect();
        }
    }
    Ok(CpuInfo {
        architecture: std::env::consts::ARCH.to_owned(),
        model,
        logical_cores,
        features,
    })
}

fn probe_memory() -> Result<MemoryInfo, ProbeError> {
    let meminfo = fs::read_to_string("/proc/meminfo").map_err(|source| ProbeError::Io {
        path: PathBuf::from("/proc/meminfo"),
        source,
    })?;
    let mut total_bytes = 0;
    let mut available_bytes = 0;
    for line in meminfo.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let Some(kib) = value
            .trim()
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
        else {
            continue;
        };
        match key {
            "MemTotal" => total_bytes = kib * 1024,
            "MemAvailable" => available_bytes = kib * 1024,
            _ => {}
        }
    }
    Ok(MemoryInfo {
        total_bytes,
        available_bytes,
    })
}

fn probe_graphics() -> GraphicsInfo {
    let model = read_trimmed(Path::new("/proc/device-tree/gpu/companion_name"))
        .or_else(|| read_trimmed(Path::new("/sys/class/drm/card0/device/product_name")))
        .unwrap_or_else(|| "Integrated GPU".to_owned());
    let driver = if Path::new("/sys/module/v3d").exists() {
        "vc4/v3d"
    } else if Path::new("/sys/module/panfrost").exists() {
        "panfrost"
    } else if Path::new("/sys/module/amdgpu").exists() {
        "amdgpu"
    } else if Path::new("/sys/module/i915").exists() {
        "i915"
    } else {
        "unknown"
    };
    let vulkan_icd_present = Path::new("/usr/share/vulkan/icd.d/broadcom_icd.json").exists()
        || fs::read_dir("/usr/share/vulkan/icd.d")
            .ok()
            .into_iter()
            .flatten()
            .any(|entry| {
                entry.is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
            });
    let preferred_api = if vulkan_icd_present {
        GraphicsApi::Vulkan
    } else {
        GraphicsApi::OpenGlEs
    };
    let drm_render_node = fs::read_dir("/dev/dri")
        .ok()
        .into_iter()
        .flatten()
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().contains("renderD"));
    GraphicsInfo {
        model,
        driver: driver.to_owned(),
        preferred_api,
        vulkan_icd_present,
        drm_render_node,
    }
}

fn probe_thermal() -> Result<ThermalInfo, ProbeError> {
    let mut temperature_celsius = None;
    let mut throttling_detected = false;
    let mut warning = None;

    for zone in fs::read_dir("/sys/class/thermal")
        .map_err(|source| ProbeError::Io {
            path: PathBuf::from("/sys/class/thermal"),
            source,
        })?
        .flatten()
    {
        let path = zone.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(millicelsius) =
            read_trimmed(&path.join("temp")).and_then(|value| value.parse::<i64>().ok())
        {
            let celsius = millicelsius as f32 / 1000.0;
            temperature_celsius =
                Some(temperature_celsius.map_or(celsius, |old: f32| old.max(celsius)));
        }
        if let Some(millicelsius) =
            read_trimmed(&path.join("temp")).and_then(|value| value.parse::<i64>().ok())
        {
            for entry in fs::read_dir(&path).into_iter().flatten().flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !name.starts_with("trip_point_") {
                    continue;
                }
                let trip_type = read_trimmed(&entry.path().join("type")).unwrap_or_default();
                if !(trip_type.eq_ignore_ascii_case("critical")
                    || trip_type.eq_ignore_ascii_case("hot"))
                {
                    continue;
                }
                let trip_millicelsius = read_trimmed(&entry.path().join("temp"))
                    .and_then(|value| value.parse::<i64>().ok());
                if trip_millicelsius.is_some_and(|limit| millicelsius >= limit) {
                    throttling_detected = true;
                    warning = Some(format!(
                        "CPU reached the {trip_type} thermal limit of {:.1}°C",
                        trip_millicelsius.unwrap_or(millicelsius) as f32 / 1000.0
                    ));
                }
            }
        }
    }

    for cooling in fs::read_dir("/sys/class/thermal")
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("cooling_device")
        })
    {
        let state = read_trimmed(&cooling.path().join("cur_state"))
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0);
        let kind = read_trimmed(&cooling.path().join("type")).unwrap_or_default();
        if state > 0 && kind.to_ascii_lowercase().contains("cpu") {
            throttling_detected = true;
            warning = Some(format!(
                "CPU thermal throttling is active at cooling state {state}"
            ));
        }
    }

    if let Some(temperature) = temperature_celsius
        && temperature >= 80.0
    {
        warning = Some(format!(
            "CPU temperature is {temperature:.1}°C; sustained rendering should reduce quality"
        ));
        throttling_detected = true;
    }

    Ok(ThermalInfo {
        temperature_celsius,
        throttling_detected,
        warning,
    })
}

fn recommend_quality(board: &str, memory: &MemoryInfo) -> QualityPreset {
    let board = board.to_ascii_lowercase();
    if board.contains("raspberry pi 5") || board.contains("compute module 5") {
        QualityPreset::Balanced
    } else if memory.total_bytes >= 12 * 1024 * 1024 * 1024 {
        QualityPreset::Quality
    } else {
        QualityPreset::Performance
    }
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim_end_matches('\0').trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_policy_distinguishes_pi5() {
        let memory = MemoryInfo {
            total_bytes: 8 * 1024 * 1024 * 1024,
            available_bytes: 4 * 1024 * 1024 * 1024,
        };
        assert_eq!(
            recommend_quality("Raspberry Pi 5 Model B Rev 1.0", &memory),
            QualityPreset::Balanced
        );
    }
}
