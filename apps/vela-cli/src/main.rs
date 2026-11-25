use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use vela_core::Project;
use vela_platform::probe;
use vela_renderer::{BackendPolicy, GpuContext};
use vela_scene::Scene;

#[derive(Debug, Parser)]
#[command(
    name = "cubes",
    bin_name = "cubes",
    version,
    about = "CUBES game creation tools"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect CPU, memory, thermal state, and the selected graphics adapter.
    Hardware {
        /// Graphics backend preference: vulkan, gl, or all.
        #[arg(long, default_value = "vulkan")]
        backend: String,
    },
    /// Create a project with a valid starter scene.
    NewProject {
        /// Directory in which the project will be created.
        path: PathBuf,
        /// Human-readable project name.
        name: String,
    },
    /// Print a project's manifest and scene summary.
    InspectProject { path: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Hardware { backend } => hardware(&backend),
        Command::NewProject { path, name } => new_project(&path, &name),
        Command::InspectProject { path } => inspect_project(&path),
    }
}

fn hardware(backend: &str) -> Result<()> {
    let report = probe().context("hardware probe failed")?;
    println!("{}", serde_json::to_string_pretty(&report)?);

    let policy = match backend {
        "vulkan" => BackendPolicy::PreferVulkan,
        "gl" | "opengl" => BackendPolicy::PreferOpenGl,
        "all" => BackendPolicy::AllHardware,
        _ => bail!("unknown backend '{backend}'; expected vulkan, gl, or all"),
    };
    let gpu =
        pollster::block_on(GpuContext::create(policy)).context("graphics adapter probe failed")?;
    println!("{}", serde_json::to_string_pretty(&gpu.report)?);
    Ok(())
}

fn new_project(path: &std::path::Path, name: &str) -> Result<()> {
    let project = Project::create(path, name).context("project creation failed")?;
    let scene_path = project
        .paths()
        .resolve_for_write(&project.manifest().main_scene)
        .context("main scene path is outside the project")?;
    let scene = Scene::starter_scene();
    scene
        .save(&scene_path)
        .context("starter scene creation failed")?;
    println!("Created '{}' at {}", name, project.paths().root().display());
    Ok(())
}

fn inspect_project(path: &std::path::Path) -> Result<()> {
    let project = Project::open(path).context("project could not be opened")?;
    println!("{}", serde_json::to_string_pretty(project.manifest())?);
    let scene_path = project
        .paths()
        .resolve_existing(&project.manifest().main_scene)
        .context("main scene is missing")?;
    let scene = Scene::load(scene_path).context("main scene could not be loaded")?;
    println!(
        "Scene '{}' contains {} entities",
        scene.name,
        scene.entities.len()
    );
    for entity in scene.entities.values() {
        println!(
            "  {} {} [{}]",
            entity.id,
            entity.name,
            entity
                .components
                .keys()
                .map(|kind| kind.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(())
}
