-- FileView plugin example
-- Copy to ~/.config/fileview/plugins/init.lua, or create the file with
-- `fv plugin init` and paste the parts you want.

fv.on("start", function()
  fv.notify("FileView " .. fv.version() .. " plugin loaded")
end)

-- Warn when an environment file gets focus.
fv.on("file_selected", function(path)
  if path and path:match("%.env$") then
    fv.notify("Warning: environment file")
  end
end)

-- Report how many files are marked.
fv.on("selection_changed", function()
  fv.notify(#fv.selected_files() .. " file(s) marked")
end)
