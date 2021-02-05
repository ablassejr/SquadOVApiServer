If you've been using our alpha World of Warcraft integration you may have noticed that the VOD generally does not start exactly when your Arena/Keystone/Encounter started.
This is due to the fact the World of Warcraft "buffers" the combat log so it only writes to disk every 200 lines or so.
In a busy fight, this could mean that we are able to detect the start of the match within a second; however, in some cases this can take up anywhere in the range of 30 seconds to minutes.
This patch addresses that by introducing what we call "DVR recording" for World of Warcraft where we always store the most recent 3 minutes of footage on disk in 30 second increments.
This way, even if the combat log is buffered by some amount, we'll be able to recall the exact time when the match starts so that your final VOD displays the entire match.
There are some trade-offs to this, namely that full World of Warcraft VODs won't be played properly until they're processed by the server which may take some time; however, we think that it's worth it to ultimately give you more accurate VODs.
Enjoy!

## Improvements
* DVR recording when playing World of Warcraft so that we can always capture the start of Arena/Keystones/Encounters precisely instead of waiting for the buffered combat log to write to disk.

## What's Coming
* Match sharing. Now you can share your best matches with friends using a link via Discord, Reddit, Facebook, email, whatever!
* VOD clipping and sharing. Same as above but with 30 second clips!
* Multiple bug fixes. We're aware of some issues where VODs fail to record properly and how the live user status doesn't update until you restart the client. We're working on it!