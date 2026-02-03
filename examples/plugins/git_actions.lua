-- Git Actions Plugin for FileView
-- Copy this to ~/.config/fileview/plugins/

-- Custom command: Stage current file
fv.register_command("git-add", function()
    local file = fv.current_file()
    if file then
        os.execute("git add " .. file)
        fv.notify("Staged: " .. file:match("([^/]+)$"))
    end
end)

-- Custom command: Unstage current file
fv.register_command("git-reset", function()
    local file = fv.current_file()
    if file then
        os.execute("git reset HEAD " .. file)
        fv.notify("Unstaged: " .. file:match("([^/]+)$"))
    end
end)

-- Custom command: Show git diff for current file
fv.register_command("git-diff", function()
    local file = fv.current_file()
    if file then
        os.execute("git diff " .. file .. " | less")
    end
end)

-- Custom command: Show git log for current file
fv.register_command("git-log", function()
    local file = fv.current_file()
    if file then
        os.execute("git log --oneline -10 " .. file .. " | less")
    end
end)

-- Custom command: Stage all marked files
fv.register_command("git-add-marked", function()
    local files = fv.marked_files()
    if #files > 0 then
        for _, f in ipairs(files) do
            os.execute("git add " .. f)
        end
        fv.notify("Staged " .. #files .. " file(s)")
    else
        fv.notify("No files marked")
    end
end)

fv.notify("Git actions: git-add, git-reset, git-diff, git-log")
