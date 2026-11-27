# Changelog

Every release is on the [releases page](https://github.com/keasbeexd/ngm-desktop/releases), for Windows, macOS and Linux. The app updates itself (Keep NGM Desktop up to date), so there's nothing to download again.


## Unreleased

- **No "new event" notice for events made in game.** With NGM Addon 3.1, an event created in game came back from NGM Peon as a "New raid" notice asking you to /reload, though you'd just made it. Events made in game aren't announced now: whoever made it knows, and the guild sees it in game already.
- **Security updates** for two libraries the app uses (`cookie` and `url`).

## 0.2.7 · 25 November 2026

- **Resets and holidays.** NGM Addon 3.1 sends the game's resets and holidays with its update, and NGM Desktop now says when NGM Peon took them: "resets and holidays updated".
- **Send now says when it's already sent.** WoW saves the NGM Addon's update when you log out or /reload, and NGM Desktop sends it right then, so pressing Send now afterwards found nothing left and said "Nothing new to send", as if nothing had reached NGM Peon. It now says "Already sent: nothing new since the last send." The tray's Send now says the same.

## 0.2.6 · 23 November 2026

- **Send now in the tray menu.** Right-click NGM Desktop's tray icon and pick Send now: it sends what changed in game and brings NGM Peon's news, then a notification says how it went, without opening the window.
- **Events no longer go missing (Linux).** Reading the NGM Addon's saved data counted as the game changing it, so NGM Desktop sent again every few seconds, and NGM Peon soon refused its news checks as too many. New events then never reached the game. It now reacts only when the game writes the file.
- **It can't get stuck.** If the system's secure store is slow or locked, NGM Desktop waits 10 seconds at most instead of stopping until it restarts, and Send now always checks NGM Peon's news.
- **When the news was last checked.** Under Syncing, the window shows the last check and how many upcoming events it brought. If a check fails, it says why.

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
