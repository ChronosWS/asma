# Ark Server Manager: Ascended Changelog

[0.2.3] - Fix #30 - Self-update for ASMA
* ASMA will now check for updates every several minutes, with option to check manually

[0.2.2] - Fix #12 - Support for complex values
* Struct-type values are now supported for editing, such as `ConfigAddNPCSpawnEntriesContainer`, However
  we do not yet have metadata for these configuration items yet and their use in .INI files has not been testes.
* Vectors of Structs are not yet supported

[0.2.1] - Fix #11 - Support for vectors
* Vector-type values, such as `mods` now have a vector editor.  The `mods` config value
  is now also converted to a vector type.

[0.2.0] - Fix #10 - Support enumerated values
* This change lays the foundation for complex-valued types like enumerations,
  vectors and structured data such as for engram and spawn definitions.  This may
  be unstable and *could* cause breaks loading servers.
* BACKUP YOUR DATA AND asma.exe in case you need to roll back

[0.1.22] - Add AlwaysTickDedicatedSkeletalMeshes and CustomNotificationURL
* Thanks @Lacoi for the PR

[0.1.21] - Fix #34 - Set state to Stopped if we can't find the server

[0.1.20] - Fix bug with external INI management

[0.1.19] - Fix #32 - Support Crossplay settings
* Thanks @Lacoi for the PR

[0.1.18] - Fix #20 - Import, Forget and Obliterate servers
* Existing unmanaged servers can now be imported into ASMA
* Managed servers can now be forgotten or obliterated

[0.1.17] - Fix #13 - New servers now use named directories rather than ids

[0.1.16] - Settings for external server management
* Allow other programs to manage INI files
* Allow other programs to handle RCON

[0.1.15] - Support clean stops via RCON
* The 'Stop' button will now save and shutdown cleanly, if you have RCON enabled

[0.1.14] - Fix #22 - Support short descriptions
* On server settings we now display the first part of the description
* Also made clear what the location and type are in server settings

[0.1.13] - RCON Support, players count
* We now automatically establish an RCON connection if configured, and grab the player list
* Currently we only display the player count

[0.1.12] - Add GBUsageToForceRestart option 
* This option will auto-restart server on high memory usage

[0.1.11] - Fix Port being the wrong kind of setting, Duration for MOTD
* Thanks @Lacoi for the PR

[0.1.10] - Fix #30 (Escaping) and #28 (Removing settings)
* Strings which need escaping are now escaped in the INI files
* When an override is removed from Settings, it will also be removed from the INI next time the server runs
* Remove old "Map" and "Port" values from profile

[0.1.9] - Add direct link to forum for ASA Patch Notes

[0.1.8] - Add bAllowSpeedLeveling and bAllowFlyerSpeedLeveling

[0.1.7] - Fix Issue #27 (again) - Saving server settings crashes

[0.1.6] - Fix Issue #27 - Saving server settings crashes

[0.1.5] - Add bAllowUnlimitedRespecs and bAllowCustomRecipes

[0.1.4] - Get Server Version, RCON lib

[0.1.3] - Logging, Server Kill state fix
* [Issue 21](https://github.com/ChronosWS/asma/issues/21) We now log to `asma.log` next to the `asma.exe`
* Server state after kill should now end at Stopped, rather than Stopping.

[0.1.2] - Fix Issue #8 - Shared process enumeration
* [Issue 8](https://github.com/ChronosWS/asma/issues/8) Process enumeration now shared among all running servers, improving performance

[0.1.1] - Many changes
* Built-in default config metadata
* Fix command-line options handling to remove extraneous `?`
* Show descriptions for server settings
* Misc other fixes and updates
* Version updates will now march forward as expected

[0.1.0] - SteamCMD download/update implemented