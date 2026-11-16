# Security policy

## Reporting a vulnerability

Please report privately, not in a public issue or pull request:

- Use GitHub's **Report a vulnerability** button on the Security tab of
  this repository (private vulnerability reporting):
  https://github.com/keasbeexd/ngm-desktop/security/advisories/new

Include what you found, how to reproduce it, and which version you tested. A
proof of concept helps; please test only on your own computer and accounts.

You can expect an acknowledgement within a week. Fixes for confirmed issues
are released as soon as they are ready, with an advisory that credits you
unless you ask not to be named.

## What counts

NGM Desktop runs on players' computers, holds a key for NGM Peon, reads the
NGM Addon's saved data, writes into the game folder and updates itself and
(when asked) the NGM Addon. Examples of what we want to hear about:

- running anything from the files it reads (NGM Addon's saved data, NGM
  Peon's or GitHub's answers), or anything reaching the window as more than
  plain text;
- writing anywhere but its own small addon folder, its settings, its login
  and launcher entries and (when ticked) the NGM Addon's own folder; or a
  download unpacking outside those;
- installing an update of itself that isn't signed with its update key, or
  an NGM Addon release that doesn't match GitHub's checksum;
- the device key leaving the system's secure store other than to
  ngmpeon.app over HTTPS, or the app talking to anything but ngmpeon.app and
  GitHub;
- the window opening any address other than its fixed links.

Problems in NGM Peon itself (ngmpeon.app): please report on the NGM Addon's
repository, https://github.com/keasbeexd/nootropic-guild-manager/security/advisories/new

## Supported versions

Only the newest release gets security fixes; NGM Desktop updates itself to
it unless you untick that.
