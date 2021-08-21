Small update this time but we have a couple of changes that should make new and old users happy!
In other news, we've performed the vast majority of the WoW data migration so you should access to your old WoW VODs again (just without the data).
Thanks for your patience!

## Features
* Added the ability to disable recording for specific game modes in Valorant and World of Warcraft.

## Improvements
* We now have the fancy code signing certificate so Microsoft Smartscreen shouldn't warn users trying to install SquadOV anymore.
* Added Sanctum of Domination (raid and boss fights) to WoW match filters.
* Now, when you select a WoW raid for filtering, the UI will only show the relevant boss fights for that raid for filtering.
* Switched to using Cloudflare/Google DNS servers by default (can be disabled in settings) for users with worse internet connections.
* Fixed an issue where our shared match URLs could have profanity in them and switched them to using cute (hopefully) animal names instead.
* We've setup a CDN in front of our VODs now (except for the ones you've shared publically, WIP). Users with worse internet connections should hopefully see less VOD buffering for processed videos.

## Bug Fixes
* Fixed an issue where sometimes League of Legends/TFT matches would not record properly.
* Fixed an issue where you could not share your friends' matches.