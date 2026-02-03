-- Productivity Plugin for FileView
-- Copy this to ~/.config/fileview/plugins/

-- Custom command: Open file in default editor
fv.register_command("edit", function()
    local file = fv.current_file()
    if file then
        local editor = os.getenv("EDITOR") or "vim"
        os.execute(editor .. " " .. file)
    end
end)

-- Custom command: Open in VS Code
fv.register_command("code", function()
    local file = fv.current_file()
    if file then
        os.execute("code " .. file)
        fv.notify("Opened in VS Code")
    end
end)

-- Custom command: Create new file
fv.register_command("touch", function()
    local dir = fv.current_dir()
    if dir then
        -- This is a placeholder - actual implementation would need input
        fv.notify("Use 'a' key to create new file")
    end
end)

-- Custom command: Copy file path to clipboard
fv.register_command("yank-path", function()
    local file = fv.current_file()
    if file then
        fv.set_clipboard(file)
        fv.notify("Path copied")
    end
end)

-- Custom command: Copy file name only
fv.register_command("yank-name", function()
    local file = fv.current_file()
    if file then
        local name = file:match("([^/]+)$")
        fv.set_clipboard(name)
        fv.notify("Name copied: " .. name)
    end
end)

-- Custom command: Show file info
fv.register_command("info", function()
    local file = fv.current_file()
    if file then
        local handle = io.popen("ls -lh " .. file)
        if handle then
            local result = handle:read("*a")
            handle:close()
            fv.notify(result:gsub("\n", ""))
        end
    end
end)

fv.notify("Productivity: edit, code, yank-path, yank-name, info")
