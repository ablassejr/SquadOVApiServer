We introduced a major performance improvement for SquadOV in this patch and is enabled by default for all users.
If you find this to in fact hurt your performance, please do let us know and you can revert back to the old pipeline in the settings menu (turn off hardware acceleration).
But if all goes well, all of you should now be getting snappy 60fps videos now!

# Features
* Hardware acceleration for capturing video.

# Bug Fixes
* Fixed an issue with clipping WoW VODs.
* Audio fallback to the default device if the user's selected device could not be opened. If that fails too, we'll just remove the audio stream instead of crashing.
* Fixed issue with video player where the buttons would take focus away from the keyboard shortcuts.
* Fixed an issue where users would not be prompted to restart for SquadOV updates.