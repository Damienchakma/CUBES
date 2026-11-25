mod app;
mod state;
mod ui;

use anyhow::{Context, Result};
use app::EditorApplication;
use std::path::PathBuf;
use winit::event_loop::EventLoop;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let event_loop = EventLoop::new().context("winit event loop could not be created")?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let project_path = parse_project_argument(std::env::args().skip(1));
    let mut application = EditorApplication::new(project_path);
    event_loop.run_app(&mut application)?;
    if let Some(error) = application.take_error() {
        return Err(error);
    }
    Ok(())
}

fn parse_project_argument(arguments: impl Iterator<Item = String>) -> Option<PathBuf> {
    let mut arguments = arguments.peekable();
    while let Some(argument) = arguments.next() {
        if argument == "--project" || argument == "-p" {
            return arguments.next().map(PathBuf::from);
        }
        if let Some(path) = argument.strip_prefix("--project=") {
            return Some(PathBuf::from(path));
        }
    }
    None
}
