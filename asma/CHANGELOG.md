# Ark Server Manager: Ascended Changelog

[0.3.43] - Fix #70 - Fix ArkAPI url
* Fix erroneous ArkAPI url

[0.3.42] - Fix #68 - ArkAPI update, more config updates
* Add Scorched Earth map
* Add `exclusivejoin` option
* Add `bDisablePhotoMode` option
* Add `PhotoModeRangeLimit` option
* Remove `AllowFlyerSpeedLeveling` option
* Remove `insecure` option
* Remove `UseVivox` option
* Update ArkAPI (ServerAPI) url

[0.3.41] - Add cluster and other options
* `ClusterDirOverride` added
* `clusterid` added
* `NoTransferFromFiltering` added
* `TributeCharacterExpirationSeconds` added
* `TributeDinoExpirationSeconds` added
* `TributeItemExpirationSeconds` added
* Thanks @Lacoi!

[0.3.40] - Add various new config options
* `DisabledInonetRangeScaling` added
* `passivemods` added
* `DisableCustomCosmetics` added
* `AllowCaveBuildingPvP` added
* `CosmeticWhitelistOverride` added
* `MaxTributeDinos` added
* `bUseDinoLevelUpAnimations` added
* `HarvestResourceItemAmountClassMultipliers` added
* Thanks @Lacoi!

[0.3.39] - Compilation instructions for Linux
* Add instructions to HOW_TO_BUILD.md for compiling on Linux (native and cross-compilation)

[0.3.38] - Fix #62 - Empty vectors of non-strings produce warnings on load
* These will no longer produce warnings.  Easiest to produce this when importing complex vector
  values from INI files because structured import is broken.  That is a separate issue.

[0.3.37] - Fix issue with not allowing spaces in strings
* I broke this with in 0.3.34.  Oops.

[0.3.36] - Fix #47 - Race condition on server start not detecting server
* There was an issue with sometimes not detecting a server you just started.  This should be fixed now.

[0.3.35] - Fix issue with vectors now showing editor controls
* An issue with string values (affecting `mods`) was causing vector editor controls to not be visible.
* There is a larger layout issue here but this is a simple temporary fix

[0.3.34] - Fix #56 - Change `mods` to String type to allow `-dev`
* This allows mod developers to load mods in development
* Also we now trim whitespace from the start and end of string values for settings.
* Fix some clippy warnings

[0.3.33] - Taming limits settings, misc UI issues
* Search bar no longer scrolls away
* Fix issue with very long setting names causing buttons to disappear
* `DestroyTamesOverTheSoftTameLimit` added
* `MaxTamedDinos_SoftTameLimit` added
* `MaxTamedDinos_SoftTameLimit_CountdownForDeletionDuration` added
* Thanks @Lacoi!

[0.3.32] - Cryopod settings added
* `DisableCryopodEnemyCheck`
* `AllowCryoFridgeOnSaddle`
* `DisableCryopodFridgeRequirement`
* Thanks @Lacoi!

[0.3.31] - Fix Multihome specification
* The `-MULTIHOME` command-line option now specifies the IP address to bind to, rather than the incorrect GUS setting.
* Thanks @Lacoi and @!Perlop

[0.3.30] - Allow setting arbitrary command-line and map url parameters with `additionalOptions`
* The `additionalOptions` parameter is a special parameter which allows setting switch options with start with `-`
  and map url options which start with `?` in case ASMA hasn't caught up to WCs latest changes, or if you need access to
  mod-specific or unusual command-line overrides

[0.3.29] - Allow removing ServerAPI
* There is now a button to remove the ServerAPI once it is installed

[0.3.28] - Move ServerAPI install button
* Most people don't need to install it, but it's location was making it look like it was a normal
  part of setting up the server

[0.3.27] - Metadata updated
* `ActiveMods` removed. Use `mods` instead
* `SupplyCrateLootQualityMultiplier` and `FishingLootQualityMultiplier` added

[0.3.26] - Favorite settings enabled
* The checkbox on the right side of a setting lets you mark it as a favorite, so it will always
  show up at the top of the list

[0.3.25] - Add various IP options
* `MULTIHOME` for allowing multi-homed servers
* `ip` and `ServerIP` which are supported on Nitrado.  Undocumented.

[0.3.24] - Svartalfheim map option
* Temporarily adding this to the main map list until we get full mod-setting support

[0.3.23] - Additional metadata updates
* Remove duplicated "OverrideNamedEngramEntries"
* Fix wrong quantiity type for
  * ConfigAddNPCSpawnEntriesContainer
  * ConfigSubtractNPCSpawnEntriesContainer
  * ConfigOverrideNPCSpawnEntriesContainer
  * DinoSpawnWeightMultipliers
  * DinoClassDamageMultipliers
  * TamedDinoClassDamageMultipliers
  * OverrideEngramEntries
  * OverrideNamedEngramEntries
  * EngramEntryAutoUnlocks
  * ConfigOverrideItemCraftingCosts
  * ConfigOverrideItemMaxQuantity
  * ConfigOverrideSupplyCrateItems
* Fix wrong value type in "ConfigOverrideItemCraftingCosts"
* Thanks @Lacoi

[0.3.22] - Fix metadata for ConfigOverrideItemMaxQuantity
* There was a bug preventing more and one entry from being created

[0.3.21] - Add oldconsole and serverkey setting
* `-oldconsole` uses the old style console window rather than the new graphical one
* `-serverkey` is used by ServerAPI to rename the map for plugins

[0.3.20] - Fixed crash when reading profiles when they are not in their own directory
* If profiles were stored in a directory with other files, we were failing to read
  them and then crashing out.  This makes the loader much more tolerant.

[0.3.19] - Fixed or added metadata
* The following settings have been fixed or updated
  * ConfigAddNPCSpawnEntriesContainer
  * ConfigSubtractNPCSpawnEntriesContainer
  * ConfigOverrideNPCSpawnEntriesContainer
  * DinoSpawnWeightMultipliers
  * NPCReplacements
  * DinoClassDamageMultipliers
  * TamedDinoClassDamageMultipliers
  * OverrideEngramEntries
  * OverrideNamedEngramEntries
  * EngramEntryAutoUnlocks
  * ConfigOverrideItemCraftingCosts
  * ConfigOverrideItemMaxQuantity
  * ConfigOverrideSupplyCrateItems
  * ItemStatClamps
  * PlayerBaseStatMultipliers
  * PerLevelStatsMultiplier_DinoTamed
  * PerLevelStatsMultiplier_DinoWild
  * PerLevelStatsMultiplier_Player
* Thanks @Lacoi

[0.3.18] - Fix #50 - Vectors have various serialization modes
* A new field is added to metadata: `vector_serialization`.  It has the following values:
  * `CommaSeparated` - The values should all be on one line with commas between them.  
                       This is the default if not specified.
  * `Indexed` - The entry appears multiple times with an index after it, such as:
                `Entry[0]=value`
                `Entry[1]=value`
  * `Repeated` - The entry appears multiple times with no index, such as:
                 `Entry=value`
                 `Entry=value`

[0.3.17] - Dark mode is moderately more readable now

[0.3.16] - Fixed incorrect formatting for structured data
* Structures and vectors of structures now no longer get quoted or escaped.

[0.3.15] - Fixed or added metadata
* The following settings have been fixed or updated:
  * ActiveMods
  * AdminListURL
  * AllowCaveBuildingPvE
  * ClampItemSpoilingTimes
  * ClampItemStats
  * ClampResourceHarvestDamage
  * CustomDynamicConfigUrl
  * PreventUploadDinos
  * PreventUploadItems
  * PreventUploadSurvivors
  * CraftXPMultiplier
  * GlobalCorpseDecompositionTimeMultiplier
  * LayEggIntervalMultiplier
  * MatingIntervalMultiplier
  * MatingSpeedMultiplier
  * PoopIntervalMultiplier
  * ResourceNoReplenishRadiusPlayers
  * ResourceNoReplenishRadiusStructures
  * SpecialXPMultiplier
  * bAllowUnclaimDinos
  * bAllowCustomRecipes
  * bDisableFriendlyFire
  * bPvEDisableFriendlyFire
* Thanks @Lacoi!

[0.3.14] - Support full INI import
* ASMA now will optionally import INI settings when importing an existing server
* Also fixed a metadata issue with RaidDinoCharacterFoodDrainMultiplier

[0.3.13] - Another fix for issue where SteamCMD.exe path has spaces in it

[0.3.12] - Fix issue where SteamCMD.exe path has spaces in it

[0.3.11] - Fix issue with ServerAPI servers taking over the console, not detecting properly
* Servers run with the ServerAPI want their own console so now they get one.  Also we won't kill them
  when ASMA exits

[0.3.10] - Fix issue with servers not starting
* Reversed logic was causing servers not using the ServerAPI to use the wrong startup executeable, and vice versa
* There is still an issue with servers using the ServerAPI taking over the console and doing strange things.

[0.3.9] - Basic support for Server API
* We now support downloading and updating the [ServerAPI](https://gameservershub.com/forums/resources/ark-survival-ascended-serverapi-crossplay-supported.683/) which allows adding custom plugins to your server. No additional special support for this feature yet.

[0.3.8] - Auto check for mod updates
* We will now automatically check for mod updates and deprecated mods

[0.3.7] - Auto fixup config values with incorrect metadata
* When possible, if a server's config values have metadata which differs from the 
  built-in metadata, we will attempt to convert it into the correct format

[0.3.6] - Truncate display of long values in search list

[0.3.5] - Misc UI updates
* Moved ASA patch notes button to servers area
* Made server name more prominent in the server card, and the ID less prominent
* New buttons to quickly go to the logs and inis directories for servers
* Easier to read app icon for the taskbar/window

[0.3.4] - Fix issue with spaces in path name for server installs
* Installs to servers with spaces in their names should now work.

[0.3.3] - Notify of new Server updates available
* The server card will now indicate whether there is a new server build available and
  what the time of the last server build was.

[0.3.2] - Fix bug in ASMA updater
* Incorrect file reference broke downloads

[0.3.1] - Metadata for bDisableStructurePlacementCollision and UseDynamicConfig
* bDisableStructurePlacementCollision is added
* UseDynamicConfig is fixed

[0.3.0] - Version bump to overcome version checking issue in earlier builds

[0.2.11] - Fix bug in version checking

[0.2.10] - Fix bug in updater trying to copy to itself

[0.2.9] - Compatibility Versions and improved installer
* There is now a version for Win 2016 that doesn't use the newer console APIs.  This means
  you won't get to see incremental installation progress, but ASMA will otherwise work as
  normal.
* Server 2016 versions are at [latest-dev.win2016.zip](https://arkservermanager.s3.us-west-2.amazonaws.com/asma/release/latest-dev.win2016.zip)
* Other versions remain at [latest-dev.zip](https://arkservermanager.s3.us-west-2.amazonaws.com/asma/release/latest-devzip)
* If update fails, you will now get a dialog box suggesting how to fix it.

[0.2.8] - Improved installation experience
* Can now cancel creation of a new server
* Can't leave the server create without setting installation directory
* Accurate progress bar during install

[0.2.7] - Fix metadata for CustomNotificationUrl

[0.2.6] - Add Logo and icon, change default window size
* Thanks @SteveLastics for the art

[0.2.5] - UI cleanup and INI support for structures
* Cleaned up the UI a little for the editor
* Made structs serialize the INI files correctly

[0.2.4] - Support nested complex structures
* The configuration system now supports arbitrarily nested vectors and structures
  for complex value editing

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