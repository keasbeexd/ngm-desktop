# NGM Desktop

The desktop app for [NGM Peon](https://ngmpeon.app) and the Nootropic Guild
Manager addon for WoW: Forever. It does the dashboard's Sync steps by
itself: when WoW saves NGM's data (logout or `/reload`), it sends NGM Peon
what changed in game, and it keeps NGM Peon's news (events, Hall of Fame,
mains and roles) ready for the addon to load.

**Status:** being designed. Nothing to download yet.

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

Not affiliated with Blizzard Entertainment or Discord.
