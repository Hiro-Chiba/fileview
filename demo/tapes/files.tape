# Create, rename, mark, bulk rename, copy, and paste.
Output assets/demo-files.gif
Source demo/tapes/common.tape
Set PlaybackSpeed 1.3

Type "fv nebula"
Show
Enter
Sleep 1.5s

# New file in docs/
Type "jjjjj"
Sleep 300ms
Type "l"
Sleep 500ms
Type "a"
Sleep 600ms
Type "roadmap.md"
Sleep 500ms
Enter
Sleep 1s
Type "jjjjj"
Sleep 800ms

# Rename it
Type "r"
Sleep 600ms
Backspace 3
Type "-2027.md"
Sleep 500ms
Enter
Sleep 1.5s

# Mark the monthly archives in data/ and rename them together
Type "g"
Sleep 300ms
Type "jjjj"
Sleep 300ms
Type "l"
Sleep 500ms
Type "jj"
Sleep 300ms
Space
Sleep 400ms
Type "jj"
Sleep 300ms
Space
Sleep 800ms
Type "R"
Sleep 800ms
Type "2026-10"
Sleep 400ms
Tab
Sleep 300ms
Type "2026-11"
Sleep 1.5s
Enter
Sleep 2s

# Copy users.json and paste it into config/
Type "jj"
Sleep 600ms
Type "y"
Sleep 800ms
Type "g"
Sleep 300ms
Type "jj"
Sleep 300ms
Type "l"
Sleep 500ms
Type "p"
Sleep 2s

# Delete asks first
Type "jjj"
Sleep 600ms
Type "D"
Sleep 2s
Type "n"
Sleep 1s

Hide
Type "q"
Sleep 200ms
