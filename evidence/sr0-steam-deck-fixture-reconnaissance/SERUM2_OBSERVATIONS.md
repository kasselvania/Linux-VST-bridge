# SR0 Serum 2 observations

Every non-finding is limited to the declared bounded locations. No candidate was copied, opened, or executed.

| Question | Classification | Result |
|---|---|---|
| Any Serum/Xfer-named artifact | `observed` | 5 retained candidate path(s) matched Serum/Xfer; all contributing roots completed within declared bounds |
| Lawful installer candidate | `not_found_in_bounded_locations` | no installer candidate in declared fixture-input or Downloads search |
| Installed/module candidate | `observed` | 5 retained module candidate(s); search_complete=true |
| Other content/directory candidate | `not_found_in_bounded_locations` | no other Serum/Xfer named candidate in declared roots |
| Artifact-search completeness | `observed` | all contributing roots completed within declared bounds |
| Artifact-retention completeness | `observed` | 5/200 candidate slots used; no retention truncation |
| Exact license channel | `operator_input_required` | Xfer direct/owned, Splice paid-off lifetime/Xfer, active Splice Rent-to-Own, or another exact channel |
| Credentials | `explicitly_out_of_scope` | vendor credentials are never collected |
| Installer launch | `explicitly_out_of_scope` | belongs to a separately authorized slice |
| Authorization | `explicitly_out_of_scope` | belongs to a separately authorized slice |

## Bounded locations

| Location class | Classification | Bound |
|---|---|---|
| Private fixture input | `observed` | <HOME>/.local/share/linux-vst-bridge-fixtures/serum2 depth=3 |
| Downloads name search | `observed` | <HOME>/Downloads depth=2 names=Serum\|Xfer |
| Linux plug-in roots | `observed` | <HOME>/.vst3;<HOME>/.vst;<HOME>/.clap depth=5 names=Serum\|Xfer |
| Discovered-prefix policy | `observed` | declared roots; immediate prefixes; known Windows VST directories only |

## Exact candidates

| Kind | Source role | Sanitized path | Object type | File type | Size bytes | SHA-256 |
|---|---|---|---|---|---|---|
| `module_candidate` | `linux_plugin_location` | `<HOME>/.vst3/yabridge/Serum2.vst3` | `directory_not_recursively_hashed` | directory | `4096` | `not_applicable_directory` |
| `module_candidate` | `linux_plugin_location` | `<HOME>/.vst3/yabridge/Serum2.vst3/Contents/x86_64-linux/Serum2.so` | `regular_file` | ELF 64-bit LSB shared object, x86-64, version 1 (SYSV), dynamically linked, BuildID[sha1]=b7dcbfcb67aab2d7ed3c31d29a01c66139ba9892, stripped | `88888` | `317d70f95a3c7559ff3d43b014c3e7b792fd03362d5e999d550a5566cf25b184` |
| `module_candidate` | `linux_plugin_location` | `<HOME>/.vst3/yabridge/Serum2.vst3/Contents/x86_64-win/Serum2.vst3` | `symlink_not_followed` | symlink metadata only | `unknown` | `not_applicable_symlink` |
| `module_candidate` | `windows_plugin_location` | `<HOME>/.wine/drive_c/Program Files/Common Files/VST3/Serum2.vst3` | `directory_not_recursively_hashed` | directory | `4096` | `not_applicable_directory` |
| `module_candidate` | `windows_plugin_location` | `<HOME>/.wine/drive_c/Program Files/Common Files/VST3/Serum2.vst3/Contents/x86_64-win/Serum2.vst3` | `regular_file` | PE32+ executable for MS Windows 6.00 (DLL), x86-64, 7 sections | `18062336` | `838bc7ab42d5d039156768680ffc3e0175d6e6bed9f99802989a01d695e13175` |

Modification timestamps are retained in `fixture.json`; hashes apply only to regular files. Directories and symlinks are not recursively hashed.

## Per-root artifact-search completion

| Role | Root | Classification | Status | Detail |
|---|---|---|---|---|
| `installer_location` | `<HOME>/.local/share/linux-vst-bridge-fixtures/serum2` | `observed` | `completed` | root_absent |
| `installer_location` | `<HOME>/Downloads` | `observed` | `completed` | rows=0 |
| `linux_plugin_location` | `<HOME>/.vst3` | `observed` | `completed` | rows=3 |
| `linux_plugin_location` | `<HOME>/.vst` | `observed` | `completed` | root_absent |
| `linux_plugin_location` | `<HOME>/.clap` | `observed` | `completed` | rows=0 |
| `windows_plugin_location` | `<HOME>/.wine/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | rows=2 |
| `windows_plugin_location` | `<HOME>/.wine/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.wine/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.wine/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.wine/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1059990/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1059990/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1059990/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1059990/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1059990/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1086940/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1091500/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1147860/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1147860/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1147860/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1147860/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1147860/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1158470/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1158470/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1158470/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1158470/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1158470/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1167630/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1167630/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1167630/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1167630/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1167630/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1173820/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1173820/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1173820/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1173820/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1173820/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1196590/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1196590/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1196590/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1196590/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1196590/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1252330/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1252330/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1252330/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1252330/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1252330/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1289810/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1289810/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1289810/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1289810/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1289810/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1325200/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1325200/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1325200/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1325200/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1325200/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1420170/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1420170/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1420170/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1420170/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1420170/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1493710/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1493710/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1493710/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1493710/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1493710/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1592280/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1592280/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1592280/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1592280/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1592280/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1668520/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1668520/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1668520/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1668520/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1668520/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1861290/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1861290/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1861290/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1861290/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1861290/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1971650/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1971650/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1971650/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1971650/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/1971650/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2074630/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2074630/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2074630/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2074630/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2074630/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2379780/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2379780/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2379780/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2379780/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2379780/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2679460/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2679460/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2679460/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2679460/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2679460/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2701660/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2701660/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2701660/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2701660/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2701660/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2805730/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2805730/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2805730/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2805730/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/2805730/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3088832635/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3088832635/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3088832635/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3088832635/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3088832635/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3154818954/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3154818954/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3154818954/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3154818954/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3154818954/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3658110/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3658110/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3658110/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3658110/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/3658110/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4081520/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4081520/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4081520/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4081520/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4081520/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418370/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418370/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418370/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418370/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418370/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418530/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418530/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418530/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418530/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/418530/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4234670/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4234670/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4234670/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4234670/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4234670/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4279256024/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4279256024/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4279256024/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4279256024/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4279256024/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4628710/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4628710/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4628710/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4628710/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/4628710/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/502500/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/502500/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/502500/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/502500/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/502500/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/601360/pfx/drive_c/Program Files/Common Files/VST3` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/601360/pfx/drive_c/Program Files/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/601360/pfx/drive_c/Program Files/Steinberg/VstPlugins` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/601360/pfx/drive_c/Program Files/Common Files/CLAP` | `observed` | `completed` | root_absent |
| `windows_plugin_location` | `<HOME>/.local/share/Steam/steamapps/compatdata/601360/pfx/drive_c/Program Files (x86)/Common Files/VST3` | `observed` | `completed` | root_absent |
