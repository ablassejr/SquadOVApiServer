Primarily just bug fixes and the like.
We also switched our encoding to target a certain quality instead of a bitrate; this way we can ensure that lower resolution videos actually end up being smaller and thus faster to upload as well.
However, we also opened up the option to manually specify the bitrate of the recorded video (capped at 6000Kbps for now).
Enjoy!

## Features
* Ability to manually set the target bitrate of the recorded video (up to 6000Kbps).

## Improvements
* Going to an event using the event timeline will now go to a time that's ~3 seconds before the event to ensure that you can watch the event fully.
* Added tooltips for all the tools in the VOD picker (under the VOD).

## Bug Fixes
* We were seeing some issues with variable framerate recording with regards to video/audio sync so we're preemptively turning it off by default.
* Fixed an issue with local recording processing.
* Fixed issue with being unable to switch off a POV that has been downloaded (or just originally recorded) on the local computer.
* Fixed issue where Valorant custom games with spectators would fail to record.