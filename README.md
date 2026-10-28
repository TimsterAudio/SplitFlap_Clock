# Split-Flap Clock

A desktop clock that looks like the split-flap departure boards of old
airports and railway stations. Each digit is a card that folds over when it
changes, and the board can list other time zones below the local time, like
departures.

- Big split-flap display of the local time, with a small date line under it
- Yellow or white digits on black
- 12- or 24-hour time, with seconds if you want them
- As many other places as you like, each with its own name, time and day, from its time zone or set by hand
- Group places into regions, with a heading each, in the order you choose
- Colours of your own for the main time, the date and each place
- Day and night: a card showing morning, day, evening or night at each place, from its own sunrise and sunset
- The board refreshes now and then: every card spins once round and lands back, like a real board (every hour by default, every 15 or 30 minutes, or at random within a range you choose)
- Keep on top, show on all desktops, and an optional frameless window to sit on the desktop
- Alarms, announced by the board with an airport chime, by your time or any place's, with a 9-minute snooze
- Can start when you log in
- Optional sound: the clatter of the cards at start-up and on refreshes, with the clock itself kept quiet
- One file: no installer, no runtime, nothing downloaded while it runs

## Using it

Download it from [Releases](https://github.com/TimsterAudio/SplitFlap_Clock/releases/latest):
`splitflap-clock.exe` for Windows, or `splitflap-clock-linux-x86_64` for Linux
(make it executable with `chmod +x`). Then run it.

**New to it?** Open **`Split-Flap Clock User Guide.html`** ("Split-Flap Clock
for Dummies"): everything on the board and in Settings explained, with
itineraries, troubleshooting and a fair amount of 1970s airport cheek, with
dark or light pages. It comes as PDFs too, one dark and one light. Each
release carries all three: `splitflap-clock-user-guide.html`,
`splitflap-clock-user-guide-dark.pdf` and `splitflap-clock-user-guide-light.pdf`.
The guide is built from `docs/guide`.

- **Right-click** the clock for the menu, or **double-click** it (or use the
  ⚙ in the corner) to open Settings.
- **Drag** the face to move the window. Resize the window, or use the Size
  slider in Settings; the board keeps its shape.
- Add places in Settings by searching for a city, such as `Tokyo` or
  `New York`, and pressing **Add**. Edit the name shown on the board, give it
  a region and a colour, set its time by hand if you like, and move places
  and regions up and down.

**Start when I log in** adds the clock to the Windows Run list (it shows in
Task Manager's Startup apps) or writes `~/.config/autostart/splitflap-clock.desktop`
on Linux. **Show on all desktops** works on Windows 10 and 11 virtual desktops,
where it also hides the taskbar button, and on Linux desktops with workspaces.

Settings are saved when the clock closes: in `%APPDATA%\splitflap-clock` on
Windows and `~/.local/share/splitflap-clock` on Linux.

On Linux the program needs an X11 or Wayland desktop with OpenGL, and ALSA
for sound (PipeWire and PulseAudio systems have it too), which every desktop
has. It runs as an X11 window wherever X11 is there (on
Wayland desktops through XWayland), since Wayland doesn't let apps stay on
top or on every workspace. It loads `libxkbcommon` (X11) or the Wayland
libraries at run time.

## Releasing

Bump `version` in `Cargo.toml` and add its section to `CHANGELOG.md`, then
push to `main`. The Release workflow creates the `vX.Y.Z` tag and the release
with both downloads.

## Building

```sh
cargo build --release                                     # Linux (needs libasound2-dev)
cargo build --release --target x86_64-pc-windows-gnu      # Windows, from Linux with MinGW
```

On Windows itself, `cargo build --release` builds the exe. The binary is in
`target/release/` (or `target/<target>/release/`).

Checks:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Credits

The board lettering is Barlow Condensed SemiBold by The Barlow Project
Authors, under the SIL Open Font License (`assets/OFL.txt`). It is built
into the binary, and into the user guide with Barlow Semi Condensed.

Thanks to Oscar, who asked for unlimited places, hand-set times, colours and
regions, then tested them all and found the colour you couldn't paste.
