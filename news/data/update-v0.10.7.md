Another major patch - this time to finish up support for WoW classic (arenas)!
As an added benefit, this also enabled us to support RBGs for WoW retail as well.
Treat this support as being an early alpha and let us know if anything goes terribly wrong!

## Features
* Support for WoW instances: WoW classic arenas and WoW retail RBGs most notably.
* Ability to filter by WoW role in addition to specs.
* Added a prompt to ask you to tell your friends about SquadOV. Please.
* SquadOV now shows up after you exit out of a game with all your VODs. Check them out!

## Improvements
* On the per-game settings pages, reduces the distance between the option and the tooltip to improve clarity.
* After you create a clip, you're immediately redirected to the clip page.
* Clips can no longer be shared until they're fully processed to ensure that your Discord messages have that sweet preview attached. :)
* Added additional prompts in the web browser to tell you when certain features are only enabled in the desktop app.
* Two new WoW-specific settings: record full raids and minimum instance recording duration. Check the tooltips for more information.
* Better labeling of keys that we don't have a good name for instead of just ERROR.
* Removed some potentially offensive words from the list of words we use for generating fun URLs.

## Bug Fixes
* Hopefully fix an issue where SquadOV loads up with a white screen.
* Hopefully fix an issue where the VOD shows up with a media source error if you try to download it.
* Hopefully fix an issue where SquadOV starts to fail to detect processes if it runs for a long time (days+).
* Fixed Javascript error when SquadOV closes.
* Removed the duplicate "Use Combat Log Timeout" option on the WoW settings page.
* Fixed crash due to combat log timeout causing failed keystones to not record properly.
* Fixed one of the issues where upload could fail.
* Fixed issue where registering/logging in from a shared clip would show two navigation bars at the top.