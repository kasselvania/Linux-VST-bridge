# SR0 Windows-audio compatibility state

This is an immediate-entry inventory. No runner contents, registries, complete prefixes, or process command lines were retained. Prefix sizes are intentionally unknown because recursive `du` was not allowed.

## Completeness summary

| Fact | Classification | Value |
|---|---|---|
| Runner inventory completeness | `observed` | all non-alias declared runner roots completed within bounds |
| Prefix roster completeness | `observed` | all non-alias declared prefix roots completed within bounds; retained_prefixes=34 |
## Runner directories

| Declared root | Immediate directory | Classification | Modified/detail |
|---|---|---|---|
| `<HOME>/.steam/root/compatibilitytools.d` | `not_applicable` | `unknown` | known_symlink_alias_not_followed; canonical Steam root inspected separately |
| `<HOME>/.local/share/Steam/compatibilitytools.d` | `not_applicable` | `not_found_in_bounded_locations` | root_present_no_immediate_directories |
| `<HOME>/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d` | `not_applicable` | `not_found_in_bounded_locations` | root_absent |
| `<HOME>/.steam/root/steamapps/common` | `not_applicable` | `unknown` | known_symlink_alias_not_followed; canonical Steam root inspected separately |
| `<HOME>/.local/share/Steam/steamapps/common` | `Proton 11.0` | `observed` | 2026-08-21 17:52:37.680266210 -0700 |
| `<HOME>/.local/share/Steam/steamapps/common` | `SteamLinuxRuntime` | `observed` | 2026-08-17 18:37:31.693611657 -0700 |
| `<HOME>/.local/share/Steam/steamapps/common` | `SteamLinuxRuntime_4` | `observed` | 2026-08-18 19:16:19.811368451 -0700 |
| `<HOME>/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common` | `not_applicable` | `not_found_in_bounded_locations` | root_absent |
| `<HOME>/.local/share/umu` | `steamrt3` | `observed` | 2026-04-04 00:10:51.589084001 -0700 |

### Runner-root collection completion

| Role | Root | Classification | Status | Detail |
|---|---|---|---|---|
| `all_immediate_directories` | `<HOME>/.steam/root/compatibilitytools.d` | `observed` | `alias_not_traversed` | canonical_equivalent_declared_separately |
| `all_immediate_directories` | `<HOME>/.local/share/Steam/compatibilitytools.d` | `observed` | `completed` | rows=0 |
| `all_immediate_directories` | `<HOME>/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d` | `observed` | `completed` | root_absent |
| `steam_common` | `<HOME>/.steam/root/steamapps/common` | `observed` | `alias_not_traversed` | canonical_equivalent_declared_separately |
| `steam_common` | `<HOME>/.local/share/Steam/steamapps/common` | `observed` | `completed` | rows=3 |
| `steam_common` | `<HOME>/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common` | `observed` | `completed` | root_absent |
| `all_immediate_directories` | `<HOME>/.local/share/umu` | `observed` | `completed` | rows=1 |

## Known prefix roots and immediate environments

| Declared root | Prefix | Classification | Modified/detail | Approximate size |
|---|---|---|---|---|
| `<HOME>/.wine` | `<HOME>/.wine` | `observed` | 2025-04-29 23:08:21.439973157 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/wineprefixes` | `not_applicable` | `not_found_in_bounded_locations` | root_absent | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/bottles/bottles` | `not_applicable` | `not_found_in_bounded_locations` | root_absent | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.steam/root/steamapps/compatdata` | `not_applicable` | `unknown` | known_symlink_alias_not_followed; canonical Steam root inspected separately | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1059990/pfx` | `observed` | 2024-11-26 07:27:06.493524664 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1086940/pfx` | `observed` | 2025-05-08 12:59:10.596142385 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1091500/pfx` | `observed` | 2025-12-07 21:16:55.270898149 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1147860/pfx` | `observed` | 2024-12-10 11:27:58.402892795 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1158470/pfx` | `observed` | 2026-06-19 22:25:36.516674235 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1167630/pfx` | `observed` | 2026-08-31 10:17:25.090642679 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1173820/pfx` | `observed` | 2025-12-25 13:08:09.742947196 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1196590/pfx` | `observed` | 2026-06-11 19:17:57.544222354 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1252330/pfx` | `observed` | 2026-08-17 18:40:05.106705700 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1289810/pfx` | `observed` | 2025-12-25 13:10:00.870552808 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1325200/pfx` | `observed` | 2025-01-30 16:43:11.569146540 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1420170/pfx` | `observed` | 2026-08-24 11:34:23.277307986 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1493710/pfx` | `observed` | 2026-08-24 11:33:26.492603758 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1592280/pfx` | `observed` | 2026-08-17 18:41:39.945842326 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1668520/pfx` | `observed` | 2026-08-28 21:33:10.003493023 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1861290/pfx` | `observed` | 2024-11-26 07:27:34.640903064 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/1971650/pfx` | `observed` | 2024-11-26 07:27:35.280919122 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/2074630/pfx` | `observed` | 2026-04-03 23:50:23.918630090 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/2379780/pfx` | `observed` | 2025-05-08 12:59:19.596342886 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/2679460/pfx` | `observed` | 2025-07-11 20:36:29.346888971 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/2701660/pfx` | `observed` | 2026-01-08 13:05:42.000706504 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/2805730/pfx` | `observed` | 2026-08-24 11:33:45.069957134 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/3088832635/pfx` | `observed` | 2026-06-20 00:05:56.713441027 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/3154818954/pfx` | `observed` | 2026-06-21 19:18:32.352376569 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/3658110/pfx` | `observed` | 2026-08-24 11:34:00.373816341 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/4081520/pfx` | `observed` | 2025-12-15 17:02:05.219233296 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/418370/pfx` | `observed` | 2026-07-03 08:29:57.957753857 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/418530/pfx` | `observed` | 2024-12-13 23:05:21.274847613 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/4234670/pfx` | `observed` | 2026-01-08 20:55:09.100217367 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/4279256024/pfx` | `observed` | 2026-06-20 00:12:50.005545387 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/4628710/pfx` | `observed` | 2026-08-24 11:32:53.449182157 -0700 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/502500/pfx` | `observed` | 2025-12-18 15:48:46.890366828 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.local/share/Steam/steamapps/compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata/601360/pfx` | `observed` | 2025-12-07 20:49:08.481360720 -0800 | `unknown_not_collected_to_avoid_recursive_scan` |
| `<HOME>/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/compatdata` | `not_applicable` | `not_found_in_bounded_locations` | root_absent | `unknown_not_collected_to_avoid_recursive_scan` |

### Prefix-root collection completion

| Role | Root | Classification | Status | Detail |
|---|---|---|---|---|
| `direct_prefix` | `<HOME>/.wine` | `observed` | `completed` | prefix_present |
| `direct_children` | `<HOME>/.local/share/wineprefixes` | `observed` | `completed` | root_absent |
| `direct_children` | `<HOME>/.local/share/bottles/bottles` | `observed` | `completed` | root_absent |
| `steam_compatdata` | `<HOME>/.steam/root/steamapps/compatdata` | `observed` | `alias_not_traversed` | canonical_equivalent_declared_separately |
| `steam_compatdata` | `<HOME>/.local/share/Steam/steamapps/compatdata` | `observed` | `completed` | rows=33 |
| `steam_compatdata` | `<HOME>/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/compatdata` | `observed` | `completed` | root_absent |

## Current relevant process state

Counts come only from exact executable-name queries.

| Group | Classification | Matching process count |
|---|---|---|
| `Bitwig` | `observed` | `0` |
| `Wine` | `unknown` | `search_incomplete; queryable exact-name matches=0; aliases over Linux comm limit=1` |
| `wineserver` | `observed` | `0` |
| `Proton_UMU` | `observed` | `0` |
| `yabridge_hosts` | `unknown` | `search_incomplete; queryable exact-name matches=0; aliases over Linux comm limit=2` |
| `Steam` | `observed` | `10` |

## yabridgectl read-only inspection

| Fact | Classification | Value |
|---|---|---|
| Invoked operation after help inspection | `not_installed` | — |

```text
no read-only list output retained
```

### Configured-path validation completion

| Role | Root | Classification | Status | Detail |
|---|---|---|---|---|
| `not_applicable` | `not_applicable` | `not_installed` | `not_run` | yabridgectl unavailable |
