The latest patch of the v0.10.X line with some features to help keep squad owners sane, especially those with large squads.
Other than that, just a bunch of fixes here and there.
Let us know what you think in our Discord!

## Features
* New native audio recording on Windows that should be more reliable! This is by default disabled as we iron out issues with it as they crop up.
* If native audio recording is enabled, we now have the ability to selectively record audio from processes. Only available on Windows 10 21H1 (and later) and Windows 11.
* LoL match events can now be filtered by participant.
* Ability to link your Discord account to SquadOV (we aren't doing anything with this yet but soon^TM).
* Ability to tag VODs and clips and search for them using those tags - note that tags are shared across everyone who has acess to that VOD/clip.
* You can now popout the VOD into a new window so you can have one monitor watching the VOD and another monitor looking at the data/events.
* Added functionality to disable certain WoW instances from being recorded.
* We now have the ability/option to record your mouse cursor in fullscreen games (will not work if you play in windowed mode currently).
* Squads now have the ability to more selectively disable matches/clips for certain WoW releases (retail, vanilla, TBC) from being shared with them.
* WoW instance filters so you can filter out all the RBGs your friends are playing if you would like.
* WoW scenarios are now recorded properly so go finish up your Legion Mage Tower with SquadOV recordings (fingers crossed)!
* We will now track Boss HPs for WoW encounters going forward, and the remaining HP% will be displayed for wipes.
* Valorant match filters on the Recent VODs page. We also added the ability to filter by POV, winning, rank, key events (2K, 3K, 4K, 5K+), and team comps.
* There's now a new "Summary" tab for WoW matches that includes total damage dealt, total heals, and total damage received for all players.
* We now have a manual stop recording button - users beware.
* For future WoW encounters, we now will group identical pulls (encounter, difficulty, players) together so you can more easily navigate between them (pull 1, 2, 3, etc.).

## Improvements
* For WoW, the spell analysis graph will now shift its range to make sure the current time is always in focus.
* The squad side panel on the Recent VODs page is now collapsible.
* Clicking on a user's name on the Recent VODs page in the squad member listing will go to their profile page.
* Reduced the possibility of scroll bars in WoW match summaries by only showing the character icon of the player's POV.
* All users who sign up for SquadOV using your squad invite link will also be counted towards your referral count (we're keeping track so we can reward you soon^TM).
* WoW instances will now also display the instance name along with the instance type.
* Disabling recording of games completely will require an explicit acknowledgement, and we will always remind you of the fact that you are no longer recording certain games.
* You can now search for squads and squad members on the Game Log page dropdown.
* You now have the ability to temporarily hide the post-game popup from the post-game report page. An option to fully disable it is in the settings menu (but only if you promise to never forget to use the app on a daily basis).
* The MFA code input is now the standard input rather than the janky text box.
* If you and your squadmates all played in the same game, that is now a single entry on the Recent VODs page with a dropdown in the bottom right to select a different POV.
* We now do a quick speedcheck the first time you run SquadOV, if your internet speed sucks we will disable automatic upload for you (and let you know what we did and why).
* You can now go to a player's profile directly from the match page (big button) and clip page (clicking on their name).

## Bug Fixes
* Fixed "Recommended" squads typo on the Recent VODs page.
* Fixed display of long squad names for recommended squads.
* Fixed an issue where events for certain characters/minions would sometimes not respect filters.
* Fixed typo on the overlay page ("make take").
* Fixed an issue where squad member pagination on the Recent VODs page would not work reliably.
* Fixed an issue where navigating away from clips using the forward/back buttons and then going back to the clip will result in "No VOD Available."
* A "Connecting to server..." dialog will now actually popup properly when we're having trouble connecting to SquadOV's servers on initial startup.
* Fixed a potential issue where after a long period of time, the link to watch VODs expires and certain actions(like clipping) will no longer work.
* Fixed an issue where we would fail to detect CS:GO installations if it was installed in a Steam library location that isn't where Steam was originally installed.
* Fixed a potential issue where recording Hearthstone could crash if exiting out of Hearthstone too early while things are still processing.
* Fixed a potential issue where we would take a long time to process existing WoW combat logs and sometimes even skip key events in a combat log.
* Support the latest Aim Lab update with a new EXE name.
* Fixed an issue where typing in timestamps for clipping without an explicit milliseconds component would fail.