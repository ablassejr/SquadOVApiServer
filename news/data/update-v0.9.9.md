We're introducing user profiles in this patch which is super exciting!
You'll now have a URL that you can share with your friends (and the internet) where you can see your clips and VODs!
We're rolling this out slowly to iron out the bugs and to ensure everyone has a great experience with it.
If you're interested in getting your hands on it early, let us know in our Discord!

## Features
* User profiles (limited release) that you can make public or private or even only availabe to people who subscribe to you on Twitch.
* You can now share timestamped links for match VODs and clips.
* Reintroducing the ability to link Twitch accounts (all previously linked accounts were removed).

## Improvements
* Share links no longer redirect you to a super long URL.
* Update Hearthstone assets to the latest patch (21.2.0.91456).
* Update TFT assets to the latest patch (11.17).
 
## Bug Fixes
* (Hopefully) fix an issue where SquadOV would fail to record incomplete keystones due to SquadOV detecting that WoW changed processes.
* Fixed an issue where squadov_client_service.exe would constantly use 1%-2% CPU.
* Fixed an issue where sharing a friend's match would result in 0 videos being displayed (this fix requires you to recreate your share link).
* Fixed an issue where sharing a match would not share the other perspectives in that video (this fix requires you to recreate your share link).
* Fixed an issue where sharing a friend's match would not show the OEmbed metadata when posting in Discord, etc (this fix requires you to recreate your share link).

## Removed
* Removed the ability to disable syncing SquadOV's timestamp to an NTP server. We're doing this because it seems to be working fine for users and not causing any issues.