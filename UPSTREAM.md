# Herkunft

- TurtleClient launcher: https://github.com/TurtleClient/TurtleClient-launcher
  branch `tauri/v2`, revision `8c5a81ba75bdaf2852178e23d47a62f064a0c934`.
- Original launcher: LiquidLauncher, © CCBlueX, GPL-3.0-or-later.
- Kept: Svelte 4 / Tauri 2 architecture, Turtle branding, icons, Inter fonts and background.
- Inter fonts: The Inter Project Authors, SIL Open Font License 1.1 (see `Inter-OFL.txt`).
- Replaced: Java/Fabric launch pipeline, LiquidBounce APIs and update feed with the bundled native
  TurtleClient and Horizon command backend. The old launcher is not overwritten.
- Horizon: https://github.com/horizon-rust/horizon, MIT; pinned client revision:
  https://github.com/TurtleClient/TurtleClient-Mod/blob/codex/rust-26.3/turtleclient-base.json.
- The redistributable launcher/preparer support binaries in this local package come from the installed
  official Horizon Windows nightly `nightly-2026-11-05-38609b7`. The payload manifest records SHA-256 hashes.
- Ocean-Rest: TurtleClient/Ocean-Rest, revision and localhost patch recorded in the parent project.

All used launcher source files are included in this directory. Binary payloads and generated
Minecraft resources are excluded from source control. Native TurtleClient changes are in the
client source tree at https://github.com/TurtleClient/TurtleClient-Mod/tree/codex/rust-26.3;
reconstruction of the private Ocean-Rest checkout uses that repository's `patches/` directory.
