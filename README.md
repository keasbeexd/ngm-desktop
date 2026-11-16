# NGM Desktop

The desktop app for [NGM Peon](https://ngmpeon.app) and the Nootropic Guild
Manager addon for WoW: Forever. It does the dashboard's Sync steps by
itself: when WoW saves NGM Addon's data (logout or `/reload`), it sends NGM Peon
what changed in game, and it keeps NGM Peon's news (events, Hall of Fame,
mains and roles) ready for the NGM Addon to load.

**Status:** working, being tested in one guild: it finds the game,
connects to NGM Peon, sends your changes when WoW saves, brings NGM Peon's
news back (with a notification for new events), and can keep NGM Addon up
to date. Installers and its own updates come next. Nothing to download yet.

## Building

Needs Rust (the version in `rust-toolchain.toml`) and Node.js, plus on Linux
the WebKitGTK 4.1, GTK 3, librsvg and an AppIndicator library.

```
npm install
npm run dev      # run it
npm run build    # installers for this system
```

`src-tauri/` is the app (Rust): `wow.rs` finds the game, `peon.rs` talks to
NGM Peon, `store.rs` keeps the settings and this computer's key, `saved.rs`
reads NGM Addon's update codes, `sync.rs` sends them, `news.rs` brings NGM Peon's
news back, `companion.rs` writes the app's small addon, `addon.rs` updates
NGM Addon. `src/` is its window (plain HTML, CSS and JavaScript).

## What it touches

- **Reads** one file: NGM Addon's saved data,
  `World of Warcraft/_forever_/WTF/Account/<account>/SavedVariables/NootropicGuildManager.lua`.
  It's read as data, never run.
- **Writes** one folder: its own data addon,
  `World of Warcraft/_forever_/Interface/AddOns/NootropicGuildManager_Peon/`.
  If you tick **Keep NGM Addon up to date**, it also replaces NGM Addon's own folder with
  new releases: only from GitHub's release page for NGM Addon, only after checking
  GitHub's SHA-256 for the file; with WoW open, a `/reload` loads it.
- **Talks** to `ngmpeon.app` over HTTPS, and to GitHub (api.github.com and
  its downloads) only to check for and fetch NGM Addon releases.
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
