If you haven't seen already, we recently received funding from ERA NYC (woohoo!). [Check it out!](https://www.eranyc.com/2021/06/28/nycs-era-announces-participants-summer-2021-program-companies-receive-100000-investments/).
This program started recently so we've been super busy hence why there was no patch last week.
In exchange, you get this patch which contains a lot of things some of you have been asking for: overlays for hiding chat and more audio recording options.
The overlay feature is EXPERIMENTAL - if you run into performance issues while using it please let us know in Discord or via a bug report.
We also had to enable anonymous analytics to the app so start figuring out what you all are using the app for.
This will be used to drive future feature development and to prove to investors that we know what we're talking about. :)
If you do not wish your usage to be included in this data, you can turn this off in the general settings menu.
Finally, just a heads up that we plan to have extended maintenance on July 12th - the servers may be down for an extended period of time during working hours Eastern Time (GMT-4), we apologize in advance for the inconvenience!

## Features
* Enabled anonymous analytics (Google Analytics, Mixpanel). We ask that you keep these on so we can better learn how you're using the app (and justify our choices to investors). If you really don't want them though, you can disable this in the general settings.
* Add ability to create an overlay that shows up in the games of choice. We envision this to primarily be used for hiding chat.
* Add ability to record from multiple output/input devices.
* Ability to force an audio device to be recorded as if it were a mono device.
* Add browser-like navigation keyboard shortcuts: Alt+Arrow Keys to go back/forward, Ctrl+R to refresh.
* Add ability to disable recordings for the games we support.
* Ability to link your Twitch account (used for nothing yet).

## Improvements
* Syncing Riot accounts now happens instantly.
* Share URLs now look more friendly with known English words by default.
* Additional text to new users that state that we do not record VODs from the past.
* WoW arena matches are labeled as being 'War Games' if we detect the '5v5' game mode.
* Add support for Valorant 3.0 (hello KAY/O!).

## Bug Fixes
* Fixed an issue users would not be able to access their League of Legends games.
* Fixed an issue where changing the local storage path to another disk/partition would not work.
* Fixed an issue where the recording would go black if you are recording in windowed mode and you resize the window.
* Fixed an issue where keyboard shortcuts would not work if the video player does not have focus.
* Fixed an issue where the draw overlay for videos would not resize properly when you enter/exit full-screen mode.
* Fixed an issue where SquadOV would try to record background processes named the same as the actual game (looking at you Wow.exe).
* We increased the number of places where we check for account ownership for Valorant to prevent certain cases where restarting Valorant would cause us to not detect your account ownership and thus not record the game.
* Fixed an issue where SquadOV would not properly reconnect after losing internet access.
* Fixed (hopefully) an issue where SquadOV would start up with a white screen and get stuck.
* Fixed an issue where you could not clip past the original end of the VOD when you have a VOD end delay set.