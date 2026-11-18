# Changelog

Every release is on the [releases page](https://github.com/keasbeexd/ngm-desktop/releases), for Windows, macOS and Linux. The app updates itself (Keep NGM Desktop up to date), so there's nothing to download again.


## 0.2.5 · 18 November 2026

- **The same links as ngmpeon.app** at the bottom of the window: What's new, Privacy, Terms, Support on Ko-fi, GitHub Sponsors, and who made it, next to the dashboard and the bug and feature forms.
- Wording: it says "the NGM Addon" everywhere, including the note on its small addon in WoW's AddOns list.

## 0.2.4 · 16 November 2026

- **Installer text names the NGM Addon.** The installer's descriptions say NGM Addon, like the rest of the app.

## 0.2.3 · 16 November 2026

- **Signed for macOS.** The Mac app is signed and notarized by Apple, so macOS opens it without the "unidentified developer" warning.

## 0.2.2 · 16 November 2026

- **Linux: closing the terminal no longer blanks the window.** Started from a terminal, the AppImage now carries on by itself, so closing the terminal leaves NGM Desktop running.

## 0.2.1 · 16 November 2026

The first release: NGM Peon and the NGM Addon, kept in sync without copy and paste.

- **Finds WoW: Forever by itself** at first start, and shows which folder it's using (Choose folder to pick another).
- **Connects with a one-time code** from NGM Peon's dashboard. The computer's key is kept in the system's own keyring, and the dashboard can remove it at any time.
- **Sends what changed in game.** Whenever WoW saves (logout or `/reload`), NGM Desktop sends the NGM Addon's update to NGM Peon: the same as `/ngm discord`, without the copy and paste.
- **Brings NGM Peon's news to the game.** Events, mains and roles, Discord names and the Hall of Fame are checked every 10 minutes while WoW runs and load at your next login or `/reload`. A desktop notification says when there's a new raid, dungeon or event. **Reload and Sync** in the NGM Addon does it now.
- **Updates, with a checkbox each:** Keep NGM Addon up to date (off unless you turn it on) and Keep NGM Desktop up to date (signed updates only). Check for updates shows an Update button for whichever is behind.
- **Start with my computer** (on by default): NGM Desktop starts in the tray when you log in.
- Links to ngmpeon.app, the dashboard, and the bug and feature forms.
