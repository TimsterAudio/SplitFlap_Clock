# Changelog

## Unreleased

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
