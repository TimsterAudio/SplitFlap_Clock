# Split-Flap Clock graphics test

A small test bench, kept apart from Split-Flap Clock itself, for one
problem seen on an older Windows laptop: with the title bar hidden, a black
strip where the title bar had been, and the right-click menu picking the
item below the one under the pointer.

It draws a pink frame round the whole window and a green ring where it
thinks the mouse pointer is, with rows and a right-click menu to point at,
and it can draw four ways: OpenGL (as the clock does), DirectX 12, Vulkan,
and OpenGL by another route. An engine that won't start on a computer comes
back in OpenGL with a note saying why. Its report, to copy or save beside
the program, says what the window and the graphics say about themselves.

## Trying it

1. Run `splitflap-graphics-test.exe`. Windows may say it protected your PC:
   **More info**, then **Run anyway**.
2. Untick **Title bar**. Is the pink frame still right round the edge, with
   no black strip outside it? Does the green ring sit on the tip of the
   pointer? Point at the rows, and right-click one for its menu: is the one
   that lights up the one under the pointer?
3. Try **Nudge the size**, as the clock now does, and look again.
4. Use the buttons to start it again with each engine, with and without the
   title bar, and look again each time.
5. **Save the report** after each try (it's added to
   `graphics-test-report.txt` beside the program) and send that file back,
   with a word on which ways looked right.

## Building

    cargo build --release

The `graphics-test` workflow builds the Windows exe on every push to this
branch; it's under Artifacts on the run's page.
