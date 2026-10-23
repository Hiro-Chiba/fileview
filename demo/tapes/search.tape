# Fuzzy finder with filters, incremental search, and glob filters.
Output assets/demo-search.gif
Source demo/tapes/common.tape
Set PlaybackSpeed 1.3

Type "fv nebula"
Show
Enter
Sleep 1.5s
Type "P"
Sleep 800ms

# Fuzzy filename search
Ctrl+P
Sleep 600ms
Type "router"
Sleep 1.5s
Down
Sleep 600ms
Enter
Sleep 2.5s

# Filter by extension
Ctrl+P
Sleep 500ms
Type "ext:py"
Sleep 2s
Down
Sleep 500ms
Enter
Sleep 2.5s

# Only files with Git changes
Ctrl+P
Sleep 500ms
Type "git:changed"
Sleep 2.5s
Enter
Sleep 2.5s

# Incremental search in the tree
Type "/"
Sleep 300ms
Type "test"
Sleep 1s
Enter
Sleep 1.5s
Type "n"
Sleep 1.5s
Type "n"
Sleep 1.5s

# Glob filter, shown across the whole tree
Type "P"
Sleep 300ms
Type "F"
Sleep 300ms
Type "*.rs"
Sleep 600ms
Enter
Sleep 500ms
Type "L"
Sleep 3s
Type "F"
Sleep 300ms
Type "H"
Sleep 1.5s

Hide
Type "q"
Sleep 200ms
