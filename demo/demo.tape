# FileView Demo Recording
# Usage: cd fileview && vhs demo/demo.tape

Output assets/demo.gif

Set FontFamily "FiraCode Nerd Font"
Set FontSize 18
Set Width 1400
Set Height 800
Set TypingSpeed 50ms
Set Theme "Catppuccin Mocha"
Set Framerate 30
Set PlaybackSpeed 1.5

# Setup: copy sample project to /tmp to avoid showing personal paths
Hide
Type "rm -rf /tmp/demo-project && cp -r demo/sample-project /tmp/demo-project && cd /tmp"
Enter
Sleep 1s
Show

# Start fileview
Type "fv demo-project"
Sleep 500ms
Enter
Sleep 3s

# Tree: demo-project > docs, src, tests, Cargo.toml, README.md
# Navigate down: demo-project -> docs -> src
Type "j"
Sleep 500ms
Type "j"
Sleep 500ms

# Expand src/ (cursor is on src)
Type "l"
Sleep 1s

# src children: utils/, lib.rs, main.rs
# Skip utils (collapsed), navigate to lib.rs
Type "j"
Sleep 400ms
Type "j"
Sleep 500ms

# Open preview panel to show lib.rs code
Type "P"
Sleep 2.5s

# Navigate down to main.rs - preview updates
Type "j"
Sleep 1.5s

# Navigate back up to lib.rs
Type "k"
Sleep 1.5s

# Navigate up to utils
Type "k"
Sleep 500ms

# Expand utils/
Type "l"
Sleep 1s

# Navigate to helper.rs - preview shows helper code
Type "j"
Sleep 2s

# Close preview
Type "P"
Sleep 800ms

# Go to top
Type "g"
Sleep 800ms

# Open fuzzy finder
Ctrl+P
Sleep 1.5s

# Search for a file
Type "main"
Sleep 2s

# Select result
Enter
Sleep 2s

# Quit - hide the exit to avoid showing shell prompt
Hide
Type "q"
Sleep 100ms
