# Camera dead-zone witness

Open kiln.toml and Play. The invisible hero starts at (160,120), then moves to
(180,120). A [64,32] world-unit dead zone keeps the camera at (0,0). The stationary
red marker turns green only after three ticks, preventing an initial-frame false
positive in pixel checks. Its 64×64 green screen bounds are [128,88,192,152].

Change 180.0 to 220.0 in scripts/main.ks and Restart: the target leaves the box,
so the camera moves only to x=28. Green screen bounds become [100,88,164,152].
Centered follow without the dead zone would move to x=20 or x=60 respectively.
The native/browser replay observers accept KILN_PIXEL_EXPECT_GREEN_BOUNDS to verify
positions, not just color counts. No art files or engine edits are needed.

Requires matching v7 runtime tools. See docs/CAMERAS_2D.md from the repository root.
