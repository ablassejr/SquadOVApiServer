The big ticket feature in this patch is push-to-talk.
Now you can set your push-to-talk key in SquadOV to be the same as in Discord and SquadOV will only record you when you're also talking in Discord (or whatever).
Treat this as an initial release, you may experience some bugs (please report them!).
Behind the scenes, this patch is also prepping us for our release for League of Legends and Teamfight Tactics, stay tuned!

## Features
* Push-to-Talk.

## Improvements
* We transitioned to a new system of building and updating SquadOV. This should hopefully fix the infinite-update problem some people were having.
* Added a link to your favorite matches/VODs under the "Library" dropdown.
* Display the reason you favorited a match next to the favorite star.
* Upload/Download progress no longer requires you to stay on the match page for its entire duration.

## Bug Fixes
* Local folder button for downloaded VODs should work again.
* Local folder cleanup now also runs after you record every VOD to ensure that SquadOV doesn't go over your set limit.
* Make fullscreen recording capable of selecting the proper GPU for the desired monitor (primarily for laptop users).