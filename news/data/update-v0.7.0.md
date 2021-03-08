A version bump to indicate that older versions are no longer compatible with the backend server (mainly for World of Warcraft).
But on the backend, we've seen a 90% reducion in the storage space required to store all of your data for World of Warcraft which means we'll be able to focus on making a better app for a little while longer before worrying about money. :)

## Features
* Refresh button to refresh recent games.

## Improvements
* We've made the WoW armory link more reliable by detecting the region more accurately using data about connected realms.
* Reduced some overhead in the HW acceleration pipeline, you may see some performance gains (or not).

## Bug Fixes
* Fixed an issue with using the AMD H264 encoder when using the HW accelerated pipeline. We've thus re-enabled the HW acceleration option for AMD users. If you are still experiencing issues please report a bug and let us know which AMD card you have!
* Fixed erroneous clipping on AMD GPUs which would result in either a crash or bizarre looking videos.