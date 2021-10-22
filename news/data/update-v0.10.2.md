Sorry for the long delay between patches.
We're getting back in the swing of things though so expect to start seeing more frequence updates from us!

## Features
* Add ability to use Ready Player Me for profile pictures.
* Add filter for WoW release (retail, vanilla, TBC).
* Add additional filters for WoW (keystones, raid difficulty, POV spec, compositions, etc.)
* Add alert for when your local storage location is running low on allocated storage.
* Add ability to delete and download clips from the app.

## Removed
* Removed the ability to manually set the bitrate of videos to prevent user error.

## Improvements
* Re-enabled the DNS feature.
* Made HTTP requests more resilient to failures so people with bad internet connections will have more consistent video uploads.
* Slightly improved quality of CPU video encoding.
* By default use GDI to record games in windowed mode to prevent the yellow border - does not work for LoL, TFT, Aim Lab, and WoW (option in recording settings).
* Removed the performance/visualization tab and moved it into Aim Lab specific screens.
* Add visual indication of redirect to home page after joining squad in web browser from link.
* Enabled variable framerate by default for all users.

## Bug Fixes
* Added image assets for the Valorant map Fracture.
* Fix issue where local service error would not show up sometimes.
* Fix issue where profile pages could not be setup in the web browser.
* Fix issue where changing local recording folders could wipe videos if user manually moved files themselves.
* Fix issue where the setting to disable Valorant game mode recording would not actually work.
* Fix issue where WoW combat log time out was not being used.
* Fix issue where overlays would not start for WoW classic.