//! Lua gameplay scripting runtime.

use mlua::{Lua, LuaOptions, StdLib};
use vela_core::EntityId;
use vela_scene::Transform;

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("Lua error: {0}")]
    Lua(#[from] mlua::Error),
    #[error("script error at line {line}: {message}")]
    Runtime { line: u32, message: String },
}

pub struct ScriptRuntime {
    lua: Lua,
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptRuntime {
    pub fn new() -> Self {
        let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::default()).expect("Lua runtime");
        Self { lua }
    }

    pub fn execute(&self, source: &str) -> Result<(), ScriptError> {
        self.lua.load(source).exec().map_err(ScriptError::from)
    }

    pub fn has_function(&self, name: &str) -> bool {
        self.lua
            .globals()
            .get::<mlua::Value>(name)
            .is_ok_and(|value| matches!(value, mlua::Value::Function(_)))
    }

    pub fn call_start(&self) -> Result<(), ScriptError> {
        if !self.has_function("on_start") {
            return Ok(());
        }
        let function: mlua::Function = self.lua.globals().get("on_start")?;
        function.call::<()>(()).map_err(ScriptError::from)
    }

    pub fn call_update(&self, entity: EntityId, delta: f32) -> Result<(), ScriptError> {
        let function: mlua::Function = self.lua.globals().get("on_update")?;
        function
            .call::<()>((entity.0, delta))
            .map_err(ScriptError::from)
    }
}

/// Minimal example used by the runtime smoke test.
pub fn example_script() -> &'static str {
    r#"
function on_start()
    print("Game started")
end

function on_update(entity, delta)
    _G.last_delta = delta
end
"#
}

pub fn transform_translation(transform: &Transform) -> glam::Vec3 {
    transform.translation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_script_exposes_update_hook() {
        let runtime = ScriptRuntime::new();
        runtime.execute(example_script()).unwrap();
        assert!(runtime.has_function("on_start"));
        assert!(runtime.has_function("on_update"));
        runtime.call_start().unwrap();
        runtime.call_update(EntityId(1), 1.0 / 60.0).unwrap();
    }
}
