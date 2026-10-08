# Hold resume position while unknown-size pages load

Status: ready-for-agent
Type: AFK
Blocked by: 01

## Parent

`.scratch/reading-progress/spec.md`

## What to build

A page whose size probe failed uses a 2/3 placeholder ratio until its image loads. Pages above the resume target stay unloaded, so the target position can shift after restore. Keep the saved page at the viewport top: re-apply the anchor when pages near the target finish loading. Stop at the first wheel, key or touch input.

## Acceptance criteria

- [ ] A comic with several unknown-size pages above the saved page resumes on the saved page, and it stays there after nearby images load.
- [ ] After the reader scrolls, key presses or touches, the position is never moved by the correction.
- [ ] The correction ends when another comic opens.
