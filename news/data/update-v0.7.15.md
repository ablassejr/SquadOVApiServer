A QOL update to hopefully smooth out our various features and make things run a little smoother for everybody.

## Features
* Support Astra and patch 2.05 for Valorant.
* OEmbed support is now (theoretically) working. This will probably take a few iterations to get right.
* Option to limit VOD upload speed for users with a limited upload speed (defaults to unlimited).

## Improvements
* Clipping: Clicking and dragging on the part of the bar between the start and stop handles now lets you move the clip segment around.
* Automatic video player pause when enabling the draw overlay.
* Display source of damage in WoW death recap.
* Made the VOD processing indicator a bit more obvious.
* Added WoW 9.0.5 data to our database.
* All our executables are now signed which means we're totally legit and can be trusted, yup.
* All future WoW matches will exclude Hunter feign deaths from being tracked as a death event.

## Bug Fixes
* Fix crash due to invalid settings file and regenerate the file if necessary.
* Fix WoW death recap events not going to the correct time in the VOD.
* Removed the ability to skip email verification by restarting SquadOV.
* Fixed an issue where submitting a bug could fail due to a large number of crash dumps.
* Handle the case where COMBATANT_INFOs would not print for keystones. This should fix cases where keystones aren't synced between squadmates.
* Fixed issue where not finding a spell in our database would cause all spells and auras to not show up.
* Fixed issue where SquadOV would crash when recording WoW in windowed mode when the GPU pipeline was not used.
* Fixed issue where SquadOV could fail to record Hearthstone matches.
* Death recaps now accurately only display damage/healing received by the player who died.