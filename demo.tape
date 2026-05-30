# lampctl demo recording.
# Regenerate with:  cargo build --release && vhs demo.tape
Output docs/demo.gif

Set Shell bash
Set FontSize 18
Set Width 1000
Set Height 640
Set Padding 24
Set TypingSpeed 55ms

Hide
Type "./target/release/lampctl --demo"
Enter
Sleep 1.2s
Show

Sleep 900ms
# scrub the hue
Right@110ms 14
Sleep 700ms
# switch to saturation, pull toward white
Space
Sleep 400ms
Left@110ms 18
Sleep 900ms
Right@110ms 10
Sleep 300ms
# tweak brightness
Space
Sleep 300ms
Left@110ms 5
Right@110ms 8
Sleep 600ms
# show the animated modes
Down
Sleep 2.6s
Down
Sleep 2.6s
# back to static
Up
Up
Sleep 900ms
Type "q"
Sleep 600ms
