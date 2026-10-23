# Git status in the tree, diff previews, and staging.
Output assets/demo-git.gif
Source demo/tapes/common.tape
Set PlaybackSpeed 1.3

Type "fv nebula"
Show
Enter
Sleep 2s
Type "P"
Sleep 500ms

# Every changed file, straight from the fuzzy finder
Ctrl+P
Sleep 500ms
Type "git:changed type:file"
Sleep 2.5s
Enter
Sleep 3s

# Stage it, then unstage it again
Type "s"
Sleep 2s
Type "u"
Sleep 2s

Ctrl+P
Sleep 500ms
Type "git:changed type:file"
Sleep 1s
Down
Sleep 400ms
Enter
Sleep 3s

Ctrl+P
Sleep 500ms
Type "git:changed type:file"
Sleep 1s
Down
Sleep 400ms
Down
Sleep 400ms
Enter
Sleep 3s

Hide
Type "q"
Sleep 200ms
