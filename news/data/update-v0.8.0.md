In this patch we're finally introducing CS:GO support, so if you still think that CS:GO is the superior game, this patch is for you!
We've also implemented a lot of other stuff so read on!

## Features
* CS:GO support. We support casual and competitive modes for now. For matchmaking competitive games, we support automatic upload and parsing of demos.
* Add setting to delay the end of the VOD by a number of seconds.
* Ability to change your password via the Security tab in settings.
* Ability to enable/disable 2FA via the Security tab in settings.

## Improvements
* Updated our OEmbed implementation to support Embedly (for previews on Reddit), hoping to get that integrated soon!
* We now sync timestamps to multiple NTP servers for better timestamps to handle cases where users did not have synced clocks.
* Increased the maximum clip length to 3 minutes.
* Slightly improved clip video quality.
* Removed the PTT setting from the quick recording settings popout.
* Added the match date-time to VALORANT match summaries.
* Added team scores to the round timeline in VALORANT.

## Bug Fixes
* Fix various minor typos.
* Removed the "VOD processing..." progress circle when no VOD exists.
* Fixed issues where users would get into an infinite trying to connect to server loop or a white screen on startup.
* Fixed an issue where clip view counts would not increase when accessed via the shared URL.
* (Hopefullly) fixed an issue where our audio recording library would detect the incorrect default device.
* Fixed an issue where shared clips/match previews would break after a few hours.
* Fixed the recent match VOD listing for users who aren't in a squad.
* Fixed the user filter for the clip library.
* Fixed an issue where duplicate LoL/TFT matches would show up in the match history.
* Fixed an internal issue where we would not be tracking users with special symbols in their email properly.
* (Hopefully) fixed an issue where our authentication server would crash every once in awhile.
* (Hopefully) fixed an issue where VODs would no longer process (generally around 7am ET) until the API server is restarted.

## Minor Updates since Last Post (v0.7.23 - v0.7.29)
* Enabled LoL/TFT for all players and added the games to the setup wizard.
* Fixed an issue where playing VALORANT games would crash SquadOV.
* Fixed an issue where VODs and clips would not play on Mac and iOS devices.
* Reduced the amount of CPU used by the PTT functionality.
* Added support for mouse buttons for PTT.
* Fixed a potential issue with our OEmbed implementation.
* Fixed an issue with setting the local recording path to a path that contains non-ASCII characters.
* Support for new VALORANT match (Breeze).