We're introducing match filters in this patch!
Every game that we support has a filters available for use under game logs.
Check it out and let us know what other filters you'd like to see!

## Features
* Match Filters on game log page.
  * Aim Lab Filters
    * Task
    * Has VOD
  * Hearthstone Filters
    * Has VOD
  * CS:GO Filters
    * Mode
    * Map
    * Has VOD
    * Has Demo
  * League of Legends Filters
    * Map
    * Mode
    * Has VOD
  * Teamfight Tactics Filters
    * Has VOD
  * Valorant Filters
    * Map
    * Mode
    * Is Ranked
    * Has VOD
  * World of Warcraft Filters
    * Encounter (Shadowlands raids only)
    * Instance (Shadowlands only)
    * Has VOD

## Improvements
* Improved client performance when uploading a VOD.
* VOD clipping now has millisecond resolution.
* Going back/forward in history now preserves state (i.e. going 'back' from a match brings you back to the game logs page in the exact state you left it).
* Updated client-side VALORANT support to patch 2.09 to support the Replication game mode.
* Updated server-side Hearthstone support to support the latest patch (Quillboars).
* Locally recorded matches/VODs now show up in the recent VODs list, favorites, and watchlist.
* Added some audio de-sync compensation.

## Bug Fixes
* Fixed an issue with client-side WoW combat log parsing which caused certain boss encounters would not be recorded (namely one with commas).
* Fixed an issue (potentially) that caused Hearthstone games to stop recording properly after a certain amount of time.
* Fixed an issue where the 'VOD Processing...' indicator would show up for locally recorded VODs in the web client.
* Alleviated an issue where a temporary inability to connect to SquadOV's servers when checking for Riot account ownership would cause games to not record.
* Fixed an issue where going to the pause menu in CS:GO would be interpreted as the match ending.
* Fixed an issue where certain CS:GO demos would fail to parse immediately after the match.
* Fixed an issue where favorited/watchlisted matches and clips of other users would not show up.
* Fixed an issue where the 'Users' filter for clips would not work properly.
* Fixed an issue where the 'Load More' button for recent VODs/clips would not respect certain filters.