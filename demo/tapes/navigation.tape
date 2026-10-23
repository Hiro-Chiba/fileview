# Expand and collapse, sorting, hidden files, bookmarks, tabs, and help.
Output assets/demo-navigation.gif
Source demo/tapes/common.tape
Set PlaybackSpeed 1.3

Type "fv nebula"
Show
Enter
Sleep 1.5s

Type "L"
Sleep 2s
Type "jjjjjjjjjjjjjjjjjjjj"
Sleep 1s
Type "H"
Sleep 1.2s

# Hidden files
Type "."
Sleep 2s

# Sort by size, then by date, then by name
Type "S"
Sleep 1.8s
Type "S"
Sleep 1.8s
Type "S"
Sleep 1.2s

# Bookmark web/ and jump back to it
Type "G"
Sleep 300ms
Type "kkkkkkkkkkk"
Sleep 500ms
Type "m1"
Sleep 1.2s
Type "g"
Sleep 800ms
Type "'1"
Sleep 1.5s

# A second tab
Ctrl+T
Sleep 1.5s
Type "jjj"
Sleep 300ms
Type "l"
Sleep 1s
Alt+T
Sleep 1.5s
Alt+T
Sleep 1.5s

# Help
Type "?"
Sleep 3s
Escape
Sleep 500ms

Hide
Type "q"
Sleep 200ms
