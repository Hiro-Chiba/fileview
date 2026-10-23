# Shared settings for every demo tape. Run tapes from the repository root,
# after `cargo build --release`, for example `vhs demo/tapes/demo.tape`.

Set FontFamily "FiraCode Nerd Font"
Set FontSize 18
Set Width 1800
Set Height 900
Set TypingSpeed 70ms
Set Theme "Catppuccin Mocha"
Set Framerate 30
Set Padding 20

# Halfblocks is the image protocol that renders inside the VHS terminal.
Hide
Type "export PATH=$PWD/target/release:$PATH FILEVIEW_IMAGE_PROTOCOL=halfblocks && demo/setup.sh > /dev/null && cd /tmp && clear"
Enter
Sleep 8s
