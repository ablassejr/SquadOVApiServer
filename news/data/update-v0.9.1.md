Sorry it's been so long since our last patch.
This patch resolves some long-standing issues coming from v0.8.10 and has some quality improvements in some other areas as well.
Check it out and let us know if you run into any problems!

## Features
* Add ability to create invite links that you can share with your friends to join squads!
* Noise filter and speech noise suppression in the recording settings for all inputs marked as 'voice.'
* We're rolling out email campaigns in this patch as well - this is mainly for new users and inactive users. Expect to see more emails from us welcoming you to the app and telling you how to set it up.

## Improvements
* We now track referral codes better through the registration/login process.
* Squad member lists are now sorted alphabetically.
* Dashboard re-organization: the news column is now gone and is now in a button in the navigation menu. The profile dropdown menu is now a smaller profile icon button. We also removed the "Recent Playtime" section. 
* The Setup Wizard is now more prevalent in the registration process and is completing it is the only way a user can download the app.
* Users can now fully remove output/input devices (instead of having to set the default device to have a volume of 0).
* Users now have the ability to delete overlay layers.
* We now are able to better handle pre-existing combat logs so we don't have to read the entire old combat log upon startup.
* On the WoW match page, clicking on a player icon in the match summary header goes to the player tab.
* There is now a 'Select All' button on the local storage management page.
* Made the overlay preview more reliable.
* For OAuth workflows, we popup a dialog with a manual URL to copy just in case users don't see the browser popup.
* We now timeout the WoW combat log after 30 seconds of no activity by default (changeable in the settings and can be dsiabled).
* We expanded the dictionary of words for the share URL and thus removed the hash at the end as well.
  
## Other Changes
* We removed the option to limit your upload bandwidth. If you found that you needed this option, please disable automatic upload instead.

## Bug Fixes
* The forgot password screen will now work even if you're already logged into the web client.
* Finally fixed issue where video players in the background would respond to keyboard shortcuts.
* Fixed issue where a massively unsynced clock would cause us to not detect LoL/TFT logs.
* Fixed the drawn blur when it's drawn to the left/top.
* Fixed (hopefully) an issue where we'd try to record devices with reported 8 channels by falling back to 2 channels.
* Fixed issues where the overlay settings would cause a local service error.
* Fixed a potential issue where the clip could fail due to invalid start/stop times.
* Fixed an issue where users with PTT enabled would have the microphone enabled by default until they toggled the PTT button.
* Fixed a potential crash due in local storage migration if the user decided to delete folders manually.
* Fixed an issue where we would crash in Valorant recording in certain cases where Valorant would tell us the match started twice.
* Fixed the squad filter for recent VODs.
* Fixed an issue where linking your Valorant account after launching Valorant would cause games to not record unless you restart Valorant/SquadOV.
* Fixed an issue where users with spaces in the Riot account name would cause us to not detect their account as being linked.
* Fixed an issue where the share match button would be missing for some users in League of Legends games.
* Fixed issue where the WoW armory redirect would point to the incorrect URL.
* Fixed issue where the Twitch OAuth redirect URL would be incorrect.
* Fixed issue where VOD processing would stop being performed if we lose connection to our message servers.
* Fixed an issue where VOD processing would fail if our servers lose connection to AWS S3 when uploading.
* Fixed an issue where VOD processing would fail if we generated an invalid thumbnail.
* Fixed issue where it's possible for users to have a bad session heartbeat and be forced to logout.