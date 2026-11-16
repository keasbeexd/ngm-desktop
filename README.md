# NGM Desktop

The desktop app for [NGM Peon](https://ngmpeon.app) and the Nootropic Guild
Manager addon for WoW: Forever. It does the dashboard's Sync steps by
itself: when WoW saves NGM's data (logout or `/reload`), it sends NGM Peon
what changed in game, and it keeps NGM Peon's news (events, Hall of Fame,
mains and roles) ready for the addon to load.

**Status:** early skeleton (finds the game, connects to NGM Peon, lives in
the tray). Syncing comes next. Nothing to download yet.

## Building

Needs Rust (the version in `rust-toolchain.toml`) and Node.js, plus on Linux
the WebKitGTK 4.1, GTK 3, librsvg and an AppIndicator library.

```
npm install
npm run dev      # run it
npm run build    # installers for this system
```

`src-tauri/` is the app (Rust): `wow.rs` finds the game, `peon.rs` talks to
NGM Peon, `store.rs` keeps the settings and this computer's key. `src/` is
its window (plain HTML, CSS and JavaScript).

## What it touches

- **Reads** one file: NGM's saved data,
  `World of Warcraft/_forever_/WTF/Account/<account>/SavedVariables/NootropicGuildManager.lua`.
  It's read as data, never run.
- **Writes** one folder: its own data addon,
  `World of Warcraft/_forever_/Interface/AddOns/NootropicGuildManager_Peon/`.
- **Talks** to `ngmpeon.app` only, over HTTPS.
- Never reads game memory, never sends input to the game, never touches
  other addons or WoW's settings.

The code is public so you can check all of that yourself, and releases are
built from it in GitHub Actions with build attestations.

## License

[PolyForm Shield 1.0.0](LICENSE): free to use, read, change and share, but
not to build a competing product.

The fonts in `src/fonts` (Cinzel and Rubik) are under the SIL Open Font
License (their own license files are next to them).

Not affiliated with Blizzard Entertainment or Discord.
