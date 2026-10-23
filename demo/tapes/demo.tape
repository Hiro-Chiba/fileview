# Overview used at the top of the README.
Output assets/demo.gif
Source demo/tapes/common.tape
Set PlaybackSpeed 1.3

Type "fv nebula"
Show
Enter
Sleep 2s

# The whole tree at once, then back
Type "L"
Sleep 2s
Type "H"
Sleep 800ms

# crates/core/src/scheduler.rs with a Git diff in the preview
Type "jjj"
Sleep 300ms
Type "l"
Sleep 400ms
Type "jjl"
Sleep 400ms
Type "jl"
Sleep 400ms
Type "jjjjj"
Sleep 300ms
Type "P"
Sleep 2.5s
Type "k"
Sleep 2s
Type "k"
Sleep 2s

# Jump anywhere with the fuzzy finder
Ctrl+P
Sleep 600ms
Type "readme"
Sleep 1s
Enter
Sleep 3s

Ctrl+P
Sleep 500ms
Type "screenshot"
Sleep 800ms
Enter
Sleep 2.5s

Ctrl+P
Sleep 500ms
Type "icon.svg"
Sleep 800ms
Enter
Sleep 2.5s

Ctrl+P
Sleep 500ms
Type "intro"
Sleep 800ms
Enter
Sleep 2.5s

Ctrl+P
Sleep 500ms
Type "whitepaper"
Sleep 800ms
Enter
Sleep 2.5s
Type "]"
Sleep 2s

Ctrl+P
Sleep 500ms
Type "users.json"
Sleep 800ms
Enter
Sleep 2.5s

Hide
Type "q"
Sleep 200ms
