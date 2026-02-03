-- Claude Code Integration Plugin for FileView
-- Copy this to ~/.config/fileview/plugins/

-- Notify on startup
fv.notify("Claude Code integration loaded")

-- Copy file path in Claude format when selecting .rs or .py files
fv.on("file_selected", function(path)
    if not path then return end

    -- Auto-notify for code files
    local ext = path:match("%.(%w+)$")
    if ext and (ext == "rs" or ext == "py" or ext == "ts" or ext == "js") then
        local name = path:match("([^/]+)$")
        fv.notify("Selected: " .. name)
    end
end)

-- Custom command: Copy path as Claude reference
fv.register_command("claude-ref", function()
    local file = fv.current_file()
    if file then
        -- Format: `path/to/file.rs:line`
        fv.set_clipboard(file)
        fv.notify("Claude ref: " .. file)
    end
end)

-- Custom command: Copy multiple files for context
fv.register_command("claude-context", function()
    local files = fv.marked_files()
    if #files == 0 then
        files = { fv.current_file() }
    end

    local result = {}
    for _, f in ipairs(files) do
        if f then
            table.insert(result, "- " .. f)
        end
    end

    if #result > 0 then
        fv.set_clipboard("Files for context:\n" .. table.concat(result, "\n"))
        fv.notify("Copied " .. #result .. " file(s) for Claude")
    end
end)
