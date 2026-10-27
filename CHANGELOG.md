# Changelog

## Unreleased

## 0.5.1-powernct – 2026-10-27 (a build for one user, not released)

- Colours can be pasted: next to every colour button is a box showing the colour as `#RRGGBB`. Type or paste a colour into it as `#RRGGBB`, `#RGB`, or `R, G, B`, which is what the picker's copy button gives.

## 0.5.0-powernct – 2026-10-27 (a build for one user, not released)

- **No limit on places**, and an **Add** button: search for a time zone, pick it from the list (or take the best match) and press Add or Enter. The window never grows past the screen; the board shrinks to fit.
- **Set a place's time by hand**: choose "Set by hand" and enter the time it is there; the clock keeps that offset from UTC (without summer time changes).
- **Colours**: the main time, the date and each place can have their own colour, picked in Settings; Reset goes back to the board's yellow or white.
- **Regions**: give places a region and the board groups them under a heading per region; places with no region go under OTHER unless they come first. Move regions up and down the board in Settings.

## 0.4.0 – 2026-10-27

- **Sound**: the clatter of the cards when the board starts up and when it refreshes, made as it plays. Every flap that falls clicks, bigger cards sound deeper, each card sounds from its place on the board, and the last flap of each card lands with a knock. Turn it on in Settings (or from the right-click menu) and set the volume there.
- The clock itself stays quiet: the time changing makes no sound unless you also tick **Also when the time changes**.
- If the sound output isn't ready, as when the clock starts at login, the clock keeps trying for it.

## 0.3.1 – 2026-10-27

- The clock starts up the same way a refresh looks: the cards set off a moment apart, each at its own speed, and settle in any order instead of sweeping across the board. Cards that appear later, such as for a newly added time zone, do the same.

## 0.3.0 – 2026-10-27

- **At random** refreshes: the board refreshes after a random wait, between two times you choose in Settings (10 to 60 minutes apart to start with).
- A refresh looks more like the real thing. Every card starts a moment apart and runs at its own speed, so the board scrambles and settles card by card instead of sweeping from top left to bottom right.

## 0.2.0 – 2026-10-27

- The board refreshes: every hour on the hour, each card spins once round its drum and lands back where it was, rippling across the board like a real departure board. Choose every 15 or 30 minutes, every hour or never in Settings, or pick **Refresh the board** from the right-click menu.
- **Start when I log in**, in Settings. On Windows it adds the clock to your startup programs; on Linux it adds an entry to `~/.config/autostart`. If you move the exe, the entry follows it the next time the clock runs.
- **Show on all desktops**, in Settings and the right-click menu, keeps the clock on every virtual desktop or workspace. On Windows the clock has no taskbar button while this is on.
- On Linux the clock now runs as an X11 window wherever X11 is available (XWayland included), so Keep on top and Show on all desktops work on Wayland desktops too.

## 0.1.0 – 2026-10-26

The first release.

- A split-flap display of the local time, with cards that fold over when they change and a small date line under it.
- Yellow or white digits on black; 12- or 24-hour time; seconds if you want them.
- Up to ten other time zones listed below, departure-board style, each with its own name, time and day.
- Keep on top, a frameless window option, and a Size control. Right-click the clock for the menu; drag it to move it.
- One file for Windows and one for Linux, with nothing to install.
