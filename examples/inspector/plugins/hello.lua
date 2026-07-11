-- An example BaseUI plugin.
--
-- Note what is NOT here: no widgets, no layout, no painting. Scripts extend what
-- the app *does* — commands, shortcuts, events, status — while widgets stay in
-- Rust. Every command registered here shows up in the Command Palette (F1)
-- automatically, because the command registry is the single source of truth for
-- the palette, menus, and shortcuts alike.

local ui = baseui

ui.log.info("hello.lua loaded")

-- A command. Appears in the palette under the "Plugin" category, with an icon,
-- a color, and a keyboard shortcut — no Rust changes, no recompile.
ui.commands.register {
  id = "plugin.greet",
  title = "Say Hello (from Lua)",
  category = "Plugin",
  icon = "glyph:star",
  color = "#e0a44e",
  shortcut = "Ctrl+H",
  run = function()
    ui.log.info("Hello from Lua!")
    -- Scripts can publish on the same event bus Rust uses.
    ui.bus.emit("plugin.greeted", { from = "hello.lua" })
  end,
}

-- Commands can drive framework state too.
ui.commands.register {
  id = "plugin.big_text",
  title = "Text Size 150% (from Lua)",
  category = "Plugin",
  icon = "gis:compass",
  color = "#4d9cf5",
  run = function() ui.text.set_scale(1.5) end,
}

-- React to app events. The tree publishes "selection.changed"; the script has no
-- reference to the tree, and the tree has no idea the script exists.
local selections = 0
local last = "none"

ui.bus.on("selection.changed", function(payload)
  selections = selections + 1
  last = payload.name
  ui.log.info("lua saw selection: " .. tostring(payload.name))
end)

-- Contribute a status-bar item. Its text is a function, so it re-evaluates every
-- frame and stays live.
ui.status.add {
  text = function()
    return "Lua: " .. last .. " (" .. selections .. ")"
  end,
  icon = "glyph:check",
  color = "#5cc97a",
  right = true,
}
