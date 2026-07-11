//! # baseui-lua
//!
//! Optional Lua (mlua) scripting for BaseUI — the **plugin / glue layer**.
//!
//! ## What this deliberately is *not*
//!
//! It does **not** expose the [`Widget`](baseui::Widget) trait. Scripts do not
//! implement `layout`/`paint`/`event`: that would mean crossing the FFI boundary
//! for every widget, every frame, and handing out `&mut Scene`. Custom widgets
//! are written in **Rust**.
//!
//! ## What it *is*
//!
//! Everything an extension actually needs in order to change what the app *does*
//! without recompiling it:
//!
//! - **commands** — register an id/title/category/icon/color/shortcut plus a
//!   handler. Registered commands immediately appear in the **Command Palette**
//!   and can be bound to keys and menus, because BaseUI's command registry is
//!   the single source of truth for all of them.
//! - **shortcuts** — bind a chord to a command id.
//! - **events** — subscribe/publish on the named channel
//!   ([`bus::on_named`](baseui::bus::on_named)), so scripts and Rust see the
//!   same traffic.
//! - **status items** — contribute to the status bar.
//! - **text scale** and **logging**.
//!
//! This works because those systems are already string-keyed and global; Lua is
//! a thin adapter over the *same* public API a Rust consumer uses, so there is
//! exactly one source of truth.
//!
//! ## Errors
//!
//! A failing Lua handler never unwinds into the renderer: errors are caught and
//! logged, and the app keeps running.
//!
//! ```no_run
//! let engine = baseui_lua::LuaEngine::new().unwrap();
//! engine.load_dir("plugins"); // every *.lua in the directory
//! ```
//!
//! ```lua
//! -- plugins/hello.lua
//! baseui.commands.register{
//!   id = "plugin.greet", title = "Say Hello", category = "Plugin",
//!   icon = "glyph:star", shortcut = "Ctrl+H",
//!   run = function() baseui.log.info("hi from Lua") end,
//! }
//! ```

use std::path::Path;

use baseui::command::{self, CommandMeta};
use baseui::widget::{StatusItem, statusbar};
use baseui::{Color, bus, icon};
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};

/// A Lua state with the BaseUI API installed.
///
/// Scripts should be loaded **before** the widget tree is built, so that
/// contributed commands, shortcuts, and status items exist when the UI is
/// assembled.
pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    /// Create a Lua state and install the `baseui` global table.
    pub fn new() -> mlua::Result<Self> {
        let lua = Lua::new();
        install(&lua)?;
        Ok(LuaEngine { lua })
    }

    /// Evaluate a chunk of Lua source. `name` is used in error messages.
    pub fn eval(&self, name: &str, source: &str) -> mlua::Result<()> {
        self.lua.load(source).set_name(name).exec()
    }

    /// Load and run a single `.lua` file.
    pub fn load_file(&self, path: impl AsRef<Path>) -> mlua::Result<()> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path)
            .map_err(|e| mlua::Error::external(format!("{}: {e}", path.display())))?;
        self.eval(&path.display().to_string(), &source)
    }

    /// Load every `*.lua` file in `dir` (sorted). Failing scripts are logged and
    /// skipped rather than aborting the app. Returns how many loaded cleanly.
    pub fn load_dir(&self, dir: impl AsRef<Path>) -> usize {
        let dir = dir.as_ref();
        let Ok(entries) = std::fs::read_dir(dir) else {
            log::warn!("lua: no plugin directory at {}", dir.display());
            return 0;
        };
        let mut paths: Vec<_> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "lua"))
            .collect();
        paths.sort();

        let mut loaded = 0;
        for path in paths {
            match self.load_file(&path) {
                Ok(()) => {
                    log::info!("lua: loaded {}", path.display());
                    loaded += 1;
                }
                Err(e) => log::error!("lua: failed to load {}: {e}", path.display()),
            }
        }
        loaded
    }

    /// The underlying Lua state, for applications that want to extend the API.
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

/// Optional `icon = "gis:compass"` / `color = "#e0a44e"` fields.
fn optional_icon(table: &Table, key: &str) -> Option<icon::Icon> {
    table
        .get::<Option<String>>(key)
        .ok()
        .flatten()
        .and_then(|spec| {
            let parsed = icon::parse(&spec);
            if parsed.is_none() {
                log::warn!("lua: unknown icon {spec:?}");
            }
            parsed
        })
}

fn optional_color(table: &Table, key: &str) -> Option<Color> {
    table
        .get::<Option<String>>(key)
        .ok()
        .flatten()
        .and_then(|hex| {
            let parsed = Color::from_hex(&hex);
            if parsed.is_none() {
                log::warn!("lua: bad color {hex:?}");
            }
            parsed
        })
}

/// Install the `baseui` global table.
fn install(lua: &Lua) -> mlua::Result<()> {
    let root = lua.create_table()?;

    // -- baseui.commands ----------------------------------------------------
    let commands = lua.create_table()?;
    commands.set(
        "register",
        lua.create_function(|_, spec: Table| {
            let id: String = spec.get("id")?;
            let title: String = spec.get("title")?;
            let run: Function = spec.get("run")?;

            let mut meta = CommandMeta::new(id.clone(), title);
            if let Some(category) = spec.get::<Option<String>>("category")? {
                meta = meta.category(category);
            }
            if let Some(shortcut) = spec.get::<Option<String>>("shortcut")? {
                meta = meta.shortcut(shortcut);
            }
            if let Some(icon) = optional_icon(&spec, "icon") {
                meta = meta.icon(icon);
            }
            if let Some(color) = optional_color(&spec, "color") {
                meta = meta.color(color);
            }

            // A failing handler must not unwind into the renderer.
            command::register(meta, move || {
                if let Err(e) = run.call::<()>(()) {
                    log::error!("lua: command {id} failed: {e}");
                }
            });
            Ok(())
        })?,
    )?;
    commands.set(
        "run",
        lua.create_function(|_, id: String| {
            command::run(&id);
            Ok(())
        })?,
    )?;
    root.set("commands", commands)?;

    // -- baseui.shortcuts ---------------------------------------------------
    let shortcuts = lua.create_table()?;
    shortcuts.set(
        "bind",
        lua.create_function(|_, (chord, id): (String, String)| {
            command::bind_shortcut(&chord, &id);
            Ok(())
        })?,
    )?;
    root.set("shortcuts", shortcuts)?;

    // -- baseui.bus ---------------------------------------------------------
    let bus_table = lua.create_table()?;
    bus_table.set(
        "on",
        lua.create_function(|lua, (name, handler): (String, Function)| {
            let lua = lua.clone();
            let event = name.clone();
            // Subscriptions live for the process (plugins are loaded once).
            bus::on_named(&name, move |payload| match lua.to_value(payload) {
                Ok(value) => {
                    if let Err(e) = handler.call::<()>(value) {
                        log::error!("lua: handler for {event} failed: {e}");
                    }
                }
                Err(e) => log::error!("lua: bad payload for {event}: {e}"),
            })
            .leak();
            Ok(())
        })?,
    )?;
    bus_table.set(
        "emit",
        lua.create_function(|lua, (name, payload): (String, Value)| {
            let json: serde_json::Value = lua.from_value(payload)?;
            bus::publish_named(&name, json);
            Ok(())
        })?,
    )?;
    root.set("bus", bus_table)?;

    // -- baseui.status ------------------------------------------------------
    let status = lua.create_table()?;
    status.set(
        "add",
        lua.create_function(|_, spec: Table| {
            // `text` may be a string or a function (re-evaluated each frame).
            let mut item = match spec.get::<Value>("text")? {
                Value::Function(f) => StatusItem::dynamic(move || match f.call::<String>(()) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("lua: status item failed: {e}");
                        String::new()
                    }
                }),
                Value::String(s) => StatusItem::new(s.to_string_lossy()),
                other => {
                    return Err(mlua::Error::external(format!(
                        "status.add: `text` must be a string or function, got {}",
                        other.type_name()
                    )));
                }
            };
            if let Some(icon) = optional_icon(&spec, "icon") {
                item = item.icon(icon);
            }
            if let Some(color) = optional_color(&spec, "color") {
                item = item.color(color);
            }
            if spec.get::<Option<bool>>("right")?.unwrap_or(false) {
                item = item.right();
            }
            statusbar::contribute(item);
            Ok(())
        })?,
    )?;
    root.set("status", status)?;

    // -- baseui.text --------------------------------------------------------
    let text = lua.create_table()?;
    text.set(
        "set_scale",
        lua.create_function(|_, scale: f32| {
            baseui::text::set_scale(scale);
            Ok(())
        })?,
    )?;
    text.set(
        "scale",
        lua.create_function(|_, ()| Ok(baseui::text::scale()))?,
    )?;
    root.set("text", text)?;

    // -- baseui.log ---------------------------------------------------------
    let log_table = lua.create_table()?;
    log_table.set(
        "info",
        lua.create_function(|_, msg: String| {
            log::info!("lua: {msg}");
            Ok(())
        })?,
    )?;
    log_table.set(
        "warn",
        lua.create_function(|_, msg: String| {
            log::warn!("lua: {msg}");
            Ok(())
        })?,
    )?;
    log_table.set(
        "error",
        lua.create_function(|_, msg: String| {
            log::error!("lua: {msg}");
            Ok(())
        })?,
    )?;
    root.set("log", log_table)?;

    lua.globals().set("baseui", root)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_registers_a_command_that_runs() {
        let engine = LuaEngine::new().unwrap();
        engine
            .eval(
                "test",
                r##"
                ran = 0
                baseui.commands.register{
                  id = "test.lua.cmd",
                  title = "Lua Test Command",
                  category = "Test",
                  icon = "glyph:star",
                  color = "#5cc97a",
                  run = function() ran = ran + 1 end,
                }
                "##,
            )
            .unwrap();

        // It is now a first-class command: searchable, and runnable from Rust.
        let hits = command::search("Lua Test");
        assert!(hits.iter().any(|m| m.id == "test.lua.cmd"));
        assert!(
            hits.iter()
                .any(|m| m.category == "Test" && m.icon.is_some())
        );

        command::run("test.lua.cmd");
        let ran: i64 = engine.lua().globals().get("ran").unwrap();
        assert_eq!(ran, 1);
    }

    #[test]
    fn named_events_round_trip_between_rust_and_lua() {
        let engine = LuaEngine::new().unwrap();
        engine
            .eval(
                "test",
                r#"
                seen = "none"
                baseui.bus.on("test.sel", function(p) seen = p.name end)
                "#,
            )
            .unwrap();

        // Published from Rust, observed in Lua.
        bus::publish_named("test.sel", serde_json::json!({ "name": "Cube" }));
        let seen: String = engine.lua().globals().get("seen").unwrap();
        assert_eq!(seen, "Cube");

        // ...and emitted from Lua, observed in Rust.
        let got = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
        let g2 = got.clone();
        let _sub = bus::on_named("test.from_lua", move |p| {
            *g2.borrow_mut() = p["msg"].as_str().unwrap_or_default().to_string();
        });
        engine
            .eval(
                "test",
                r#"baseui.bus.emit("test.from_lua", { msg = "hi" })"#,
            )
            .unwrap();
        assert_eq!(got.borrow().as_str(), "hi");
    }

    #[test]
    fn failing_handler_is_reported_not_panicking() {
        let engine = LuaEngine::new().unwrap();
        engine
            .eval(
                "test",
                r#"
                baseui.commands.register{
                  id = "test.boom", title = "Boom",
                  run = function() error("kaboom") end,
                }
                "#,
            )
            .unwrap();
        // Must not panic/unwind — the error is logged and the app survives.
        command::run("test.boom");
    }
}
