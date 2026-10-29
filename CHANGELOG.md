# Changelog

## Unreleased

- Each alarm can have its own **Colour**, in Settings next to its reason: its row on the board, and its SNOOZE and STOP cards, ring in that colour. **Reset** goes back to the board's.

- **Card brightness**, in Settings: lighter cards with brighter pins, so the split and the edges show up better against the board (on white cards, a darker split and pins). **Classic** goes back to the usual look.

- The user guide covers everything in 1.1: day and night (a new Gate 09), alarm reasons, lit days and dates, the SNOOZE and STOP cards, white cards, light or dark menus, flip speed, the version line and beer money.

- **Flip speed**, in Settings: how fast the flaps fall, from a quarter as fast (a stately board) to four times as fast (a frantic one), with **Normal** to go back. It sets everything: each flap's fall, the spin through the alphabet, and the ripple at start-up and on refreshes.

- When an alarm rings, **SNOOZE** and **STOP** flap cards appear under it, to press. They light up under the pointer. (A click anywhere else on the board still snoozes, and Esc still stops.)
- An alarm's days stand out: the days it rings on are lit yellow. An alarm can be set **On a date** instead, to ring once on that day.
- Alarms only ring for times that come round while the clock is running: starting the clock just after an alarm's time, or setting or changing an alarm for a time just gone, no longer rings it straight away. After the computer has been asleep, an alarm missed by less than 10 minutes still rings.
- **Board**: black cards, or white cards with dark letters. **Menus**: the right-click menu and Settings like your system, light or dark.
- The version and its release date are at the foot of Settings, with **Beer money**, for anyone who'd like to buy the developer a beer.

- Alarms have a **Reason**, for what each is for: when one rings, the end of its row flips to the reason, such as DENTIST, and stays there.
- Fixed: a ringing alarm's row no longer keeps flipping between NOW and ALARM. It flips to NOW (or its reason) once and stops; DELAYED while it snoozes.

- **Day and night**: an optional card at the end of each place's row, and after the main time, showing the sky there as a picture: morning (the sun rising over the horizon), day (the sun), evening (the sun setting into the sea) or night (the moon and stars). Morning is the hour around sunrise, from half an hour before to half an hour after, and evening the hour around sunset; they come from that place's own sunrise and sunset, worked out from where its time zone's city is, so midnight sun and polar night are right too. A place whose time is set by hand goes by its clock instead (sunrise at six, sunset at six). Turn it on in Settings, Display, **Day and night**: **Places** and **Main time**.

## 1.0.0 – 2026-10-28

Version 1: the clock, complete.

- **Alarms**, in Settings: as many as you like, each with a time, a label for the board, the days it repeats (or once), and whose time it follows: yours, or any place on the board. When one goes off, the board clatters, a row flips in under the time saying NOW and ALARM by turns, and an airport chime plays every few seconds, at its own volume, whether the clatter is on or not. Click the board to snooze for 9 minutes (the row says DELAYED and when it will ring again); right-click or press Esc to stop it. **Test** rings an alarm straight away. If the clock wasn't running or the computer was asleep when an alarm was due, it still rings if that was less than 10 minutes ago, and an alarm nobody answers stops by itself after 30 minutes.
- Settings opens next to the clock, to its right or, if there's no room, its left, instead of wherever the system puts new windows (on a widescreen, often the far side).
- The user guide covers alarms, has light pages as well as dark (it follows your system's setting, with a switch to flip it), and comes as two PDFs too, one dark and one light. Each release carries all three.

## 0.5.0 – 2026-10-28

- **No limit on places**, and an **Add** button: search for a time zone, pick it from the list (or take the best match) and press Add or Enter. The window never grows past the screen; the board shrinks to fit.
- **Set a place's time by hand**: choose "Set by hand" and enter the time it is there; the clock keeps that offset from UTC (without summer time changes).
- **Colours**: the main time, the date and each place can have their own colour, picked in Settings; Reset goes back to the board's yellow or white. Next to every colour button is a box showing the colour as `#RRGGBB`: type or paste a colour into it as `#RRGGBB`, `#RGB`, or `R, G, B`, which is what the picker's copy button gives.
- **Regions**: give places a region and the board groups them under a heading per region; places with no region go under OTHER unless they come first. Move regions up and down the board in Settings.
- A user guide: `Split-Flap Clock User Guide.html`, also attached to each release.
- Thanks to Oscar, who asked for all of the above and tested it.

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
