## Features
* Local recording management with ability to bulk upload and delete (Library > Local).
* Add ability for users to change their own username and/or email.

## Improvements
* We now package the VC++ 2019 redistributable with the installer and install it if necessary.
* We no longer require users to verify their email to use the app.
* The VOD picker and player now refresh every time you navigate to the page so that it'll always pick up the processed VOD when available.
* We now properly maintain your previous scroll position when navigating back/forward.

## Bug Fixes
* Fixed an issue where we would not display the full list of WoW matches available.
* Fixed an issue where share connections would not show for newer matches.
* Fixed an issue where Valorant event timings could be incorrect.
* Re-added background color to the Valorant game mode text on the match summary.
* Fixed an issue where SquadOV would not record from the correct default device if the device was changed after SquadOV was initialized.
* Fixed an issue where deleting a VOD would cause page state history to be wiped.
* Fixed an issue where Valorant account ownership would not refresh when each match starts.
* Fixed an issue where SquadOV's client service would crash if the GPU did not support certain D3D11 features.