#!/usr/bin/env bash
set -euo pipefail

export LC_ALL=C

readonly SR0_NAME="sr0-steam-deck-fixture-reconnaissance"
readonly MAX_RETAINED_FILE_BYTES=262144

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel)"
raw_dir="${1:-$repo_root/evidence/raw/$SR0_NAME}"
output_dir="${2:-$repo_root/evidence/$SR0_NAME}"
expected_raw_dir="$repo_root/evidence/raw/$SR0_NAME"
expected_output_dir="$repo_root/evidence/$SR0_NAME"

if [[ "$raw_dir" != "$expected_raw_dir" ]] || [[ "$output_dir" != "$expected_output_dir" ]]; then
  printf 'sanitize paths must be the exact SR0 raw and retained evidence directories\n' >&2
  exit 2
fi
if [[ ! -d "$raw_dir" ]]; then
  printf 'raw SR0 directory is absent\n' >&2
  exit 2
fi

actual_home="$HOME"
actual_user="$(id -un)"
if [[ -r /etc/hostname ]]; then
  actual_hostname="$(head -n 1 /etc/hostname)"
else
  actual_hostname=""
fi

cache_parent="${XDG_CACHE_HOME:-$HOME/.cache}/linux-vst-bridge"
mkdir -p -- "$cache_parent"
temp_dir="$(mktemp -d "$cache_parent/sr0-sanitize.XXXXXX")"

cleanup() {
  if [[ "$temp_dir" == "$cache_parent"/sr0-sanitize.* ]] && [[ -d "$temp_dir" ]]; then
    rm -rf -- "$temp_dir"
  fi
}
trap cleanup EXIT

sanitize_line() {
  local line="$1"

  line="${line//$actual_home/<HOME>}"
  if [[ -n "$actual_hostname" ]]; then
    line="${line//$actual_hostname/<HOST>}"
  fi
  if [[ -n "$actual_user" ]]; then
    local user_path_component="/$actual_user/"
    local redacted_path_component="/<USER>/"
    local user_assignment="=$actual_user"
    local redacted_assignment="=<USER>"
    line="${line//$user_path_component/$redacted_path_component}"
    line="${line//$actual_user@/<USER>@}"
    line="${line//$user_assignment/$redacted_assignment}"
    line="${line//$'\t'$actual_user$'\t'/$'\t<USER>\t'}"
  fi

  printf '%s\n' "$line" | sed -E \
    -e 's/[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/<EMAIL>/g' \
    -e 's/([[:xdigit:]]{2}:){5}[[:xdigit:]]{2}/<MAC>/g' \
    -e 's/([0-9]{1,3}\.){3}[0-9]{1,3}/<IPV4>/g' \
    -e 's/([[:xdigit:]]{1,4}:){3,7}[[:xdigit:]]{0,4}/<IPV6>/g' \
    -e 's/(^|[^[:alnum:]])::1([^[:alnum:]]|$)/\1<IPV6>\2/g' \
    -e 's/[[:xdigit:]]{8}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{4}-[[:xdigit:]]{12}/<UUID>/g' \
    -e 's#(/run/media/<USER>/)[^[:space:]]+#\1<EXTERNAL_MOUNT>#g' \
    -e 's/<<EMAIL>>/<EMAIL>/g'
}

sanitize_file() {
  local source="$1"
  local destination="$2"
  local line

  : > "$destination"
  while IFS= read -r line || [[ -n "$line" ]]; do
    sanitize_line "$line" >> "$destination"
  done < "$source"
}

raw_files=(
  facts.tsv
  toolchain.tsv
  compatibility-tools.tsv
  runners.tsv
  prefixes.tsv
  artifacts.tsv
  processes.tsv
  filesystems.txt
  bitwig-flatpak-info.txt
  bitwig-flatpak-permissions.txt
  bitwig-flatpak-metadata.txt
  bitwig-flatpak-relevant-metadata.txt
  bitwig-flatpak-overrides.txt
  freedesktop-runtimes.tsv
  yabridgectl-help.txt
  yabridgectl-list.txt
  collection-status.tsv
)

for raw_name in "${raw_files[@]}"; do
  if [[ ! -f "$raw_dir/$raw_name" ]]; then
    printf 'required raw normalized file missing: %s\n' "$raw_name" >&2
    exit 2
  fi
  sanitize_file "$raw_dir/$raw_name" "$temp_dir/$raw_name"
done

facts_file="$temp_dir/facts.tsv"
toolchain_file="$temp_dir/toolchain.tsv"
compatibility_tools_file="$temp_dir/compatibility-tools.tsv"
runners_file="$temp_dir/runners.tsv"
prefixes_file="$temp_dir/prefixes.tsv"
artifacts_file="$temp_dir/artifacts.tsv"
processes_file="$temp_dir/processes.tsv"
collection_status_file="$temp_dir/collection-status.tsv"

fact_state() {
  local section="$1"
  local key="$2"
  awk -F '\t' -v expected_section="$section" -v expected_key="$key" \
    '$1 == expected_section && $2 == expected_key {print $3; exit}' "$facts_file"
}

fact_value() {
  local section="$1"
  local key="$2"
  awk -F '\t' -v expected_section="$section" -v expected_key="$key" \
    '$1 == expected_section && $2 == expected_key {print $4; exit}' "$facts_file"
}

md_escape() {
  local value="${1:-}"
  value="${value//\\/\\\\}"
  value="${value//|/\\|}"
  value="${value//\`/\\\`}"
  if [[ -z "$value" ]]; then
    printf '—'
  else
    printf '%s' "$value"
  fi
}

json_quote() {
  local value="${1:-}"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\b'/\\b}"
  value="${value//$'\f'/\\f}"
  value="${value//$'\n'/\\n}"
  value="${value//$'\r'/\\r}"
  value="${value//$'\t'/\\t}"
  printf '"%s"' "$value"
}

json_fact() {
  local section="$1"
  local key="$2"
  local state
  local value
  state="$(fact_state "$section" "$key")"
  value="$(fact_value "$section" "$key")"
  [[ -n "$state" ]] || state=unknown
  printf '{"classification":%s,"value":' "$(json_quote "$state")"
  if [[ -n "$value" ]]; then
    json_quote "$value"
  else
    printf 'null'
  fi
  printf '}'
}

markdown_fact_row() {
  local label="$1"
  local section="$2"
  local key="$3"
  printf '| %s | `%s` | %s |\n' \
    "$label" \
    "$(fact_state "$section" "$key")" \
    "$(md_escape "$(fact_value "$section" "$key")")"
}

emit_tool_markdown_rows() {
  local source="$1"
  local name
  local classification
  local version
  while IFS=$'\t' read -r name classification version; do
    [[ -n "$name" ]] || continue
    printf '| `%s` | `%s` | %s |\n' \
      "$(md_escape "$name")" \
      "$(md_escape "$classification")" \
      "$(md_escape "$version")"
  done < "$source"
}

emit_tool_json_array() {
  local source="$1"
  local name
  local classification
  local version
  local first=true
  printf '['
  while IFS=$'\t' read -r name classification version; do
    [[ -n "$name" ]] || continue
    if [[ "$first" == true ]]; then
      first=false
    else
      printf ','
    fi
    printf '{"name":%s,"classification":%s,"version":' \
      "$(json_quote "$name")" \
      "$(json_quote "$classification")"
    if [[ -n "$version" ]]; then
      json_quote "$version"
    else
      printf 'null'
    fi
    printf '}'
  done < "$source"
  printf ']'
}

emit_runner_json_array() {
  local root
  local name
  local classification
  local modified
  local first=true
  printf '['
  while IFS=$'\t' read -r root name classification modified; do
    [[ -n "$root" ]] || continue
    if [[ "$first" == true ]]; then first=false; else printf ','; fi
    printf '{"root":%s,"name":' "$(json_quote "$root")"
    if [[ -n "$name" ]]; then json_quote "$name"; else printf 'null'; fi
    printf ',"classification":%s,"modified_or_detail":' "$(json_quote "$classification")"
    if [[ -n "$modified" ]]; then json_quote "$modified"; else printf 'null'; fi
    printf '}'
  done < "$runners_file"
  printf ']'
}

emit_prefix_json_array() {
  local root
  local path
  local classification
  local modified
  local size
  local first=true
  printf '['
  while IFS=$'\t' read -r root path classification modified size; do
    [[ -n "$root" ]] || continue
    if [[ "$first" == true ]]; then first=false; else printf ','; fi
    printf '{"root":%s,"path":' "$(json_quote "$root")"
    if [[ -n "$path" ]]; then json_quote "$path"; else printf 'null'; fi
    printf ',"classification":%s,"modified_or_detail":' "$(json_quote "$classification")"
    if [[ -n "$modified" ]]; then json_quote "$modified"; else printf 'null'; fi
    printf ',"approximate_size":%s}' "$(json_quote "$size")"
  done < "$prefixes_file"
  printf ']'
}

emit_process_json_array() {
  local name
  local classification
  local count
  local first=true
  printf '['
  while IFS=$'\t' read -r name classification count; do
    [[ -n "$name" ]] || continue
    if [[ "$first" == true ]]; then first=false; else printf ','; fi
    printf '{"name":%s,"classification":%s,"matching_process_count":%s}' \
      "$(json_quote "$name")" \
      "$(json_quote "$classification")" \
      "$(json_quote "$count")"
  done < "$processes_file"
  printf ']'
}

emit_runtime_json_array() {
  local application architecture branch version installation origin
  local first=true
  printf '['
  while IFS=$'\t' read -r application architecture branch version installation origin; do
    [[ -n "$application" ]] || continue
    if [[ "$first" == true ]]; then first=false; else printf ','; fi
    printf '{"application":%s,"architecture":%s,"branch":%s,"version":%s,"installation":%s,"origin":%s}' \
      "$(json_quote "$application")" \
      "$(json_quote "$architecture")" \
      "$(json_quote "$branch")" \
      "$(json_quote "$version")" \
      "$(json_quote "$installation")" \
      "$(json_quote "$origin")"
  done < "$temp_dir/freedesktop-runtimes.tsv"
  printf ']'
}

emit_collection_status_json_array() {
  local expected_scope="$1"
  local scope role root classification status detail
  local first=true
  printf '['
  while IFS=$'\t' read -r scope role root classification status detail; do
    [[ "$scope" == "$expected_scope" ]] || continue
    if [[ "$first" == true ]]; then first=false; else printf ','; fi
    printf '{"scope":%s,"role":%s,"root":%s,"classification":%s,"status":%s,"detail":%s}' \
      "$(json_quote "$scope")" \
      "$(json_quote "$role")" \
      "$(json_quote "$root")" \
      "$(json_quote "$classification")" \
      "$(json_quote "$status")" \
      "$(json_quote "$detail")"
  done < "$collection_status_file"
  printf ']'
}

emit_collection_status_markdown_rows() {
  local expected_scope="$1"
  local scope role root classification status detail
  while IFS=$'\t' read -r scope role root classification status detail; do
    [[ "$scope" == "$expected_scope" ]] || continue
    printf '| `%s` | `%s` | `%s` | `%s` | %s |\n' \
      "$(md_escape "$role")" \
      "$(md_escape "$root")" \
      "$(md_escape "$classification")" \
      "$(md_escape "$status")" \
      "$(md_escape "$detail")"
  done < "$collection_status_file"
}

emit_artifact_json_array() {
  local role path classification kind object_type file_type size modified digest
  local first=true
  printf '['
  while IFS=$'\t' read -r role path classification kind object_type file_type size modified digest; do
    [[ -n "$path" ]] || continue
    if [[ "$first" == true ]]; then first=false; else printf ','; fi
    printf '{"role":%s,"path":%s,"classification":%s,"candidate_kind":%s,"object_type":%s,"file_type":%s,"size_bytes":%s,"modified":%s,"sha256":%s}' \
      "$(json_quote "$role")" \
      "$(json_quote "$path")" \
      "$(json_quote "$classification")" \
      "$(json_quote "$kind")" \
      "$(json_quote "$object_type")" \
      "$(json_quote "$file_type")" \
      "$(json_quote "$size")" \
      "$(json_quote "$modified")" \
      "$(json_quote "$digest")"
  done < "$artifacts_file"
  printf ']'
}

emit_artifact_markdown_rows() {
  local role path classification kind object_type file_type size modified digest
  while IFS=$'\t' read -r role path classification kind object_type file_type size modified digest; do
    [[ -n "$path" ]] || continue
    printf '| `%s` | `%s` | `%s` | `%s` | %s | `%s` | `%s` |\n' \
      "$(md_escape "$kind")" \
      "$(md_escape "$role")" \
      "$(md_escape "$path")" \
      "$(md_escape "$object_type")" \
      "$(md_escape "$file_type")" \
      "$(md_escape "$size")" \
      "$(md_escape "$digest")"
  done < "$artifacts_file"
}

primary_claim="$(fact_value claim primary)"
claim_ceiling="$(fact_value claim ceiling)"

{
  printf '# SR0 basis\n\n'
  printf '## Primary claim\n\n%s\n\n' "$primary_claim"
  printf '| Fact | Classification | Value |\n|---|---|---|\n'
  markdown_fact_row 'Repository path' basis repository_path
  markdown_fact_row 'Expected basis commit' basis expected_basis_commit
  markdown_fact_row 'Expected basis tree' basis expected_basis_tree
  markdown_fact_row 'Captured commit' basis commit
  markdown_fact_row 'Captured tree' basis tree
  markdown_fact_row 'Branch' basis branch
  markdown_fact_row 'Capture-tool revision' basis capture_tool_revision
  markdown_fact_row 'Capture timestamp' basis capture_timestamp
  markdown_fact_row 'Git version' basis git_version
  printf '\n## Claim ceiling\n\n`explicitly_out_of_scope`: %s\n' "$claim_ceiling"
} > "$temp_dir/BASIS.md"

{
  printf '# SR0 host fixture\n\n'
  printf '| Fact | Classification | Value |\n|---|---|---|\n'
  markdown_fact_row 'OS name' host os_name
  markdown_fact_row 'OS release' host os_pretty_name
  markdown_fact_row 'OS version' host os_version_id
  markdown_fact_row 'OS build' host os_build_id
  markdown_fact_row 'OS variant' host os_variant_id
  markdown_fact_row 'Kernel' host kernel
  markdown_fact_row 'Architecture' host architecture
  markdown_fact_row 'Hardware model' host hardware_model
  markdown_fact_row 'CPU model' host cpu_model
  markdown_fact_row 'Logical cores' host logical_core_count
  markdown_fact_row 'Total memory (KiB)' host total_memory_kib
  markdown_fact_row 'SteamOS read-only status' host steamos_readonly_status
  markdown_fact_row 'Graphical session type' session type
  markdown_fact_row 'Desktop' session desktop
  markdown_fact_row 'Session census completeness' session discovery_completeness
  markdown_fact_row 'DISPLAY present in capture shell' session display_present
  markdown_fact_row 'WAYLAND_DISPLAY present in capture shell' session wayland_display_present
  markdown_fact_row 'Display socket values' session display_socket_names
  markdown_fact_row 'Filesystem summary completeness' host filesystem_summary_completeness
  printf '\n## Local block-device filesystem capacity\n\nOnly the requested columns are retained. External media labels beneath `/run/media` are replaced.\n\n```text\n'
  if [[ -s "$temp_dir/filesystems.txt" ]]; then
    sed -n '1,64p' "$temp_dir/filesystems.txt"
  else
    printf '%s: no retained local block-device rows returned\n' \
      "$(fact_state host filesystem_summary_completeness)"
  fi
  printf '```\n'
} > "$temp_dir/HOST.md"

{
  printf '# SR0 Bitwig Flatpak\n\n'
  printf 'Bitwig was not launched. Values are read from installed Flatpak metadata only.\n\n'
  printf '| Fact | Classification | Value |\n|---|---|---|\n'
  markdown_fact_row 'Installed' bitwig_flatpak installed
  markdown_fact_row 'Version' bitwig_flatpak version
  markdown_fact_row 'Branch' bitwig_flatpak branch
  markdown_fact_row 'Origin' bitwig_flatpak origin
  markdown_fact_row 'Architecture' bitwig_flatpak architecture
  markdown_fact_row 'Runtime' bitwig_flatpak runtime
  markdown_fact_row 'Installation scope' bitwig_flatpak installation_scope
  markdown_fact_row 'Plug-in sandbox mode' bitwig_flatpak plugin_sandbox_mode
  markdown_fact_row 'Declared VST path' bitwig_flatpak vst_path
  markdown_fact_row 'Declared VST3 path' bitwig_flatpak vst3_path
  markdown_fact_row 'Declared CLAP path' bitwig_flatpak clap_path
  markdown_fact_row 'Effective VST path' bitwig_flatpak effective_vst_path
  markdown_fact_row 'Effective VST3 path' bitwig_flatpak effective_vst3_path
  markdown_fact_row 'Effective CLAP path' bitwig_flatpak effective_clap_path
  markdown_fact_row 'Graphics sockets' bitwig_flatpak graphics_socket_posture
  markdown_fact_row 'Installed Linux Audio extensions' bitwig_flatpak linux_audio_extensions_installed
  markdown_fact_row 'Runtime/extension census completeness' bitwig_flatpak runtime_census_completeness
  markdown_fact_row 'Override inspection completeness' bitwig_flatpak override_inspection_completeness
  printf '\n## Effective installed permissions\n\nFrom `flatpak info --show-permissions`:\n\n```ini\n'
  if [[ -s "$temp_dir/bitwig-flatpak-permissions.txt" ]]; then
    sed -n '1,256p' "$temp_dir/bitwig-flatpak-permissions.txt"
  else
    printf 'not available\n'
  fi
  printf '```\n\n## Package metadata declarations\n\n```ini\n'
  if [[ -s "$temp_dir/bitwig-flatpak-relevant-metadata.txt" ]]; then
    sed -n '1,128p' "$temp_dir/bitwig-flatpak-relevant-metadata.txt"
  else
    printf 'not available\n'
  fi
  printf '```\n\n## Explicit Flatpak overrides\n\nRead-only user and system override inspection:\n\n```ini\n'
  if [[ -s "$temp_dir/bitwig-flatpak-overrides.txt" ]]; then
    sed -n '1,256p' "$temp_dir/bitwig-flatpak-overrides.txt"
  else
    printf 'not available\n'
  fi
  printf '```\n\n## Matching Freedesktop runtime and Linux Audio extensions\n\n```text\n'
  if [[ -s "$temp_dir/freedesktop-runtimes.tsv" ]]; then
    sed -n '1,128p' "$temp_dir/freedesktop-runtimes.tsv"
  else
    printf '%s: %s\n' \
      "$(fact_state bitwig_flatpak linux_audio_extensions_installed)" \
      "$(fact_value bitwig_flatpak linux_audio_extensions_installed)"
  fi
  printf '```\n\n## Flatpak collection completion\n\n| Role | Application | Classification | Status | Detail |\n|---|---|---|---|---|\n'
  emit_collection_status_markdown_rows flatpak
} > "$temp_dir/BITWIG_FLATPAK.md"

{
  printf '# SR0 toolchain and runtime inventory\n\n'
  printf '## Development toolchain\n\n| Tool | Classification | Version/result |\n|---|---|---|\n'
  emit_tool_markdown_rows "$toolchain_file"
  printf '\n## Audio and compatibility commands\n\n| Tool | Classification | Version/result |\n|---|---|---|\n'
  emit_tool_markdown_rows "$compatibility_tools_file"
  printf '\n## Audio/session facts\n\n| Fact | Classification | Value |\n|---|---|---|\n'
  markdown_fact_row 'PipeWire session available' audio pipewire_session_available
  markdown_fact_row 'PipeWire Audio/Sink count' audio class_count_Audio_Sink
  markdown_fact_row 'PipeWire Audio/Source count' audio class_count_Audio_Source
  markdown_fact_row 'PipeWire Audio/Device count' audio class_count_Audio_Device
  markdown_fact_row 'PipeWire output-stream count' audio class_count_Stream_Output_Audio
  markdown_fact_row 'PipeWire input-stream count' audio class_count_Stream_Input_Audio
  markdown_fact_row 'PulseAudio compatibility available' audio pulseaudio_compatibility_available
  markdown_fact_row 'ALSA available' audio alsa_available
  markdown_fact_row 'ALSA playback-card count' audio alsa_playback_card_count
  markdown_fact_row 'Default route class' audio default_route_class
  markdown_fact_row 'Default route API' audio default_route_api
  markdown_fact_row 'Default route device name' audio default_route_device_name
} > "$temp_dir/TOOLCHAIN_AND_RUNTIME.md"

{
  printf '# SR0 Windows-audio compatibility state\n\n'
  printf 'This is an immediate-entry inventory. No runner contents, registries, complete prefixes, or process command lines were retained. Prefix sizes are intentionally unknown because recursive `du` was not allowed.\n\n'
  printf '## Completeness summary\n\n| Fact | Classification | Value |\n|---|---|---|\n'
  markdown_fact_row 'Runner inventory completeness' compatibility_runtime_inventory runner_inventory_completeness
  markdown_fact_row 'Prefix roster completeness' compatibility_runtime_inventory prefix_roster_completeness
  printf '## Runner directories\n\n| Declared root | Immediate directory | Classification | Modified/detail |\n|---|---|---|---|\n'
  while IFS=$'\t' read -r root name classification modified; do
    [[ -n "$root" ]] || continue
    printf '| `%s` | `%s` | `%s` | %s |\n' \
      "$(md_escape "$root")" "$(md_escape "$name")" \
      "$(md_escape "$classification")" "$(md_escape "$modified")"
  done < "$runners_file"
  printf '\n### Runner-root collection completion\n\n| Role | Root | Classification | Status | Detail |\n|---|---|---|---|---|\n'
  emit_collection_status_markdown_rows runner_inventory
  printf '\n## Known prefix roots and immediate environments\n\n| Declared root | Prefix | Classification | Modified/detail | Approximate size |\n|---|---|---|---|---|\n'
  while IFS=$'\t' read -r root path classification modified size; do
    [[ -n "$root" ]] || continue
    printf '| `%s` | `%s` | `%s` | %s | `%s` |\n' \
      "$(md_escape "$root")" "$(md_escape "$path")" \
      "$(md_escape "$classification")" "$(md_escape "$modified")" \
      "$(md_escape "$size")"
  done < "$prefixes_file"
  printf '\n### Prefix-root collection completion\n\n| Role | Root | Classification | Status | Detail |\n|---|---|---|---|---|\n'
  emit_collection_status_markdown_rows prefix_inventory
  printf '\n## Current relevant process state\n\nCounts come only from exact executable-name queries.\n\n| Group | Classification | Matching process count |\n|---|---|---|\n'
  while IFS=$'\t' read -r name classification count; do
    [[ -n "$name" ]] || continue
    printf '| `%s` | `%s` | `%s` |\n' \
      "$(md_escape "$name")" "$(md_escape "$classification")" "$(md_escape "$count")"
  done < "$processes_file"
  printf '\n## yabridgectl read-only inspection\n\n'
  printf '| Fact | Classification | Value |\n|---|---|---|\n'
  markdown_fact_row 'Invoked operation after help inspection' compatibility_runtime_inventory yabridgectl_read_only_operation
  printf '\n```text\n'
  if [[ -s "$temp_dir/yabridgectl-list.txt" ]]; then
    sed -n '1,128p' "$temp_dir/yabridgectl-list.txt"
  else
    printf 'no read-only list output retained\n'
  fi
  printf '```\n\n### Configured-path validation completion\n\n| Role | Root | Classification | Status | Detail |\n|---|---|---|---|---|\n'
  if grep -F -q $'yabridge_config\t' "$collection_status_file"; then
    emit_collection_status_markdown_rows yabridge_config
  else
    printf '| `not_applicable` | `not_applicable` | `not_installed` | `not_run` | yabridgectl unavailable |\n'
  fi
} > "$temp_dir/WINDOWS_AUDIO_STATE.md"

{
  printf '# SR0 Serum 2 observations\n\n'
  printf 'Every non-finding is limited to the declared bounded locations. No candidate was copied, opened, or executed.\n\n'
  printf '| Question | Classification | Result |\n|---|---|---|\n'
  markdown_fact_row 'Any Serum/Xfer-named artifact' serum2 overall_artifact_result
  markdown_fact_row 'Lawful installer candidate' serum2 installer_result
  markdown_fact_row 'Installed/module candidate' serum2 module_result
  markdown_fact_row 'Other content/directory candidate' serum2 content_result
  markdown_fact_row 'Artifact-search completeness' existing_plugin_state artifact_search_completeness
  markdown_fact_row 'Artifact-retention completeness' existing_plugin_state artifact_retention
  markdown_fact_row 'Exact license channel' serum2 license_channel
  markdown_fact_row 'Credentials' serum2 credentials
  markdown_fact_row 'Installer launch' serum2 installer_launch
  markdown_fact_row 'Authorization' serum2 authorization
  printf '\n## Bounded locations\n\n| Location class | Classification | Bound |\n|---|---|---|\n'
  markdown_fact_row 'Private fixture input' existing_plugin_state bounded_installer_root
  markdown_fact_row 'Downloads name search' existing_plugin_state bounded_download_root
  markdown_fact_row 'Linux plug-in roots' existing_plugin_state bounded_linux_plugin_roots
  markdown_fact_row 'Discovered-prefix policy' existing_plugin_state bounded_prefix_policy
  printf '\n## Exact candidates\n\n'
  if [[ -s "$artifacts_file" ]]; then
    printf '| Kind | Source role | Sanitized path | Object type | File type | Size bytes | SHA-256 |\n|---|---|---|---|---|---|---|\n'
    emit_artifact_markdown_rows
  else
    printf '`%s`: %s\n' \
      "$(fact_state serum2 overall_artifact_result)" \
      "$(fact_value serum2 overall_artifact_result)"
  fi
  printf '\nModification timestamps are retained in `fixture.json`; hashes apply only to regular files. Directories and symlinks are not recursively hashed.\n\n'
  printf '## Per-root artifact-search completion\n\n| Role | Root | Classification | Status | Detail |\n|---|---|---|---|---|\n'
  emit_collection_status_markdown_rows artifact_search
} > "$temp_dir/SERUM2_OBSERVATIONS.md"

{
  printf '# SR0 findings\n\n'
  printf '| Technical-lead question | Classification | Answer |\n|---|---|---|\n'
  markdown_fact_row 'Exact host hardware model' host hardware_model
  markdown_fact_row 'Exact host OS release' host os_pretty_name
  markdown_fact_row 'Exact graphical session type' session type
  markdown_fact_row 'Exact Bitwig Flatpak installed state' bitwig_flatpak installed
  markdown_fact_row 'Exact Bitwig version' bitwig_flatpak version
  markdown_fact_row 'Exact Bitwig runtime' bitwig_flatpak runtime
  markdown_fact_row 'Installed Linux Audio extension state' bitwig_flatpak linux_audio_extensions_installed
  markdown_fact_row 'Flatpak runtime census completeness' bitwig_flatpak runtime_census_completeness
  markdown_fact_row 'Effective Bitwig VST3 path' bitwig_flatpak effective_vst3_path
  markdown_fact_row 'Effective Bitwig CLAP path' bitwig_flatpak effective_clap_path
  markdown_fact_row 'Serum 2 installer in bounded locations' serum2 installer_result
  markdown_fact_row 'Serum 2 module in bounded locations' serum2 module_result
  markdown_fact_row 'Other Serum/Xfer content in bounded locations' serum2 content_result
  markdown_fact_row 'Artifact-search completeness' existing_plugin_state artifact_search_completeness
  markdown_fact_row 'Prefix-roster completeness' compatibility_runtime_inventory prefix_roster_completeness
  markdown_fact_row 'License channel' serum2 license_channel
  markdown_fact_row 'Next GUI step' claim gui_next_step
  printf '\n## Observed\n\n'
  printf 'The retained packet records the host, session discovery result, local capacity, installed Flatpak metadata, audio command/session posture, tool versions, declared runner roots, immediate prefix inventory, bounded Serum/Xfer matches, and current exact-name process counts. Every completeness-sensitive census also retains its completion or incomplete state.\n\n'
  printf 'The host graphical session is `%s`/`%s`; Bitwig declares effective sockets `%s`. Bitwig launch and actual XWayland behavior remain unexercised.\n\n' \
    "$(fact_value session type)" "$(fact_value session desktop)" \
    "$(fact_value bitwig_flatpak graphics_socket_posture)"
  printf 'Package metadata declares VST3 and CLAP extension paths, while the current read-only user override inspection reports effective VST3 and CLAP values as `%s` and `%s`. No override was changed.\n\n' \
    "$(fact_value bitwig_flatpak effective_vst3_path)" \
    "$(fact_value bitwig_flatpak effective_clap_path)"
  if grep -F -q '<HOME>/.vst3/yabridge/Serum2.vst3' "$artifacts_file" \
    && grep -F -q '<HOME>/.wine/drive_c/Program Files/Common Files/VST3/Serum2.vst3' "$artifacts_file"; then
    printf 'A native yabridge-style Serum2 proxy bundle and corresponding Windows Serum2 module are observed by metadata/hash. The `yabridgectl` and `yabridge` commands are not installed in the SSH capture PATH; the existing artifacts are not an operability claim.\n\n'
  fi
  printf '## Unknown or operator-controlled\n\n'
  printf -- '- The exact lawful Serum 2 license channel remains `operator_input_required`.\n'
  printf -- '- Any installation outside the declared bounded paths remains `unknown`.\n'
  printf -- '- Credential entry and graphical installer/authorization behavior require an operator-controlled GUI in a later authorized slice.\n'
  printf -- '- Prefix approximate sizes remain `unknown` because recursive traversal was deliberately not performed.\n\n'
  printf '## Explicitly out of scope\n\n%s\n' "$claim_ceiling"
} > "$temp_dir/FINDINGS.md"

{
  printf '# SR0 operator handoff\n\n'
  printf 'All safe noninteractive observations authorized by SR0 were attempted. Any command, root, or aggregate that did not complete within every declared bound is classified `unknown`/`search_incomplete`, never as absence. Before any later installer or authorization slice, the operator must privately provide or decide only the following:\n\n'
  printf '1. Classify the lawful Serum 2 license channel as exactly one of: Xfer direct/owned; Splice paid-off lifetime/Xfer; active Splice Rent-to-Own; or another exact channel. Do not put account identifiers or proof-of-purchase data in Git.\n'
  printf '2. Make the lawful Windows installer available locally. The recommended private, untracked input root is `<HOME>/.local/share/linux-vst-bridge-fixtures/serum2/`. Do not copy the installer into this repository.\n'
  printf '3. State whether Serum 2 is already installed in an uninspected location. SR0 searched only the locations declared in `SERUM2_OBSERVATIONS.md`.\n'
  printf '4. Select the graphical remote-control method for the later GUI session. SR0 does not select, install, or configure one.\n'
  printf '5. Enter any vendor credentials directly into the vendor-owned graphical flow. Never send account names, emails, serials, order numbers, license files, tokens, or credentials to the evidence agent or repository.\n'
  printf '6. Select a separate next slice to own installer launch and, separately where appropriate, vendor authorization observation. SR0 does not authorize either action.\n\n'
  printf 'Current bounded installer result: `%s` — %s\n' \
    "$(fact_state serum2 installer_result)" "$(fact_value serum2 installer_result)"
  printf 'Current bounded module result: `%s` — %s\n' \
    "$(fact_state serum2 module_result)" "$(fact_value serum2 module_result)"
} > "$temp_dir/OPERATOR_HANDOFF.md"

{
  printf '# SR0 sanitization\n\n'
  printf '## Collection minimization\n\n'
  printf -- '- Raw normalized data exists only transiently under the ignored `evidence/raw/sr0-steam-deck-fixture-reconnaissance/` path and is removed when capture exits.\n'
  printf -- '- No environment dump, registry dump, prefix copy, settings dump, full process list, process command line, display socket name, device serial, browser state, or proprietary file content is retained.\n'
  printf -- '- File candidates are represented only by sanitized path, type, size, modification time, and SHA-256 for regular files.\n'
  printf -- '- Directory traversal is bounded by declared roots, depth, filesystem, row count, time, and bytes. Every traversal retains completion, timeout, failure, or truncation status.\n'
  printf -- '- Search roots are canonicalized beneath the canonical home and rejected if any existing component is a symlink; rejected roots are not traversed.\n\n'
  printf '## Replacements\n\n'
  printf -- '- Real home path -> `<HOME>`\n'
  printf -- '- Username -> `<USER>`\n'
  printf -- '- Hostname -> `<HOST>`\n'
  printf -- '- Email, IPv4, IPv6, MAC, UUID-like, and external-media label patterns -> typed placeholders\n\n'
  printf 'The word “Deck” remains only where it denotes the Steam Deck fixture or an exact project/branch identifier; username-bearing path and account contexts are replaced.\n\n'
  printf '## Claim discipline\n\nA missing Serum/Xfer match is retained as `not_found_in_bounded_locations` only when every contributing root and command completed within time, byte, row, recursion, filesystem, symlink, and candidate bounds. Any incomplete contributor propagates to `unknown`/`search_incomplete`; it is never generalized to absence. Legitimate prose such as “license channel” remains because it contains no license material.\n\n'
  printf '## Raw/staging posture\n\nThe capture checks that the raw path is ignored and contains no tracked file before and after packet generation. `hashes.sha256` covers every retained packet file other than itself.\n'
} > "$temp_dir/SANITIZATION.md"

{
  printf '{\n'
  printf '  "schema": "linux-vst-bridge-sr0-fixture/v1",\n'
  printf '  "basis": {\n'
  printf '    "repository_path": %s,\n' "$(json_fact basis repository_path)"
  printf '    "expected_commit": %s,\n' "$(json_fact basis expected_basis_commit)"
  printf '    "expected_tree": %s,\n' "$(json_fact basis expected_basis_tree)"
  printf '    "captured_commit": %s,\n' "$(json_fact basis commit)"
  printf '    "captured_tree": %s,\n' "$(json_fact basis tree)"
  printf '    "branch": %s,\n' "$(json_fact basis branch)"
  printf '    "capture_tool_revision": %s,\n' "$(json_fact basis capture_tool_revision)"
  printf '    "capture_timestamp": %s,\n' "$(json_fact basis capture_timestamp)"
  printf '    "git_version": %s\n' "$(json_fact basis git_version)"
  printf '  },\n'
  printf '  "host": {\n'
  printf '    "os_name": %s,\n' "$(json_fact host os_name)"
  printf '    "os_pretty_name": %s,\n' "$(json_fact host os_pretty_name)"
  printf '    "os_version_id": %s,\n' "$(json_fact host os_version_id)"
  printf '    "os_build_id": %s,\n' "$(json_fact host os_build_id)"
  printf '    "os_variant_id": %s,\n' "$(json_fact host os_variant_id)"
  printf '    "kernel": %s,\n' "$(json_fact host kernel)"
  printf '    "architecture": %s,\n' "$(json_fact host architecture)"
  printf '    "hardware_model": %s,\n' "$(json_fact host hardware_model)"
  printf '    "cpu_model": %s,\n' "$(json_fact host cpu_model)"
  printf '    "logical_core_count": %s,\n' "$(json_fact host logical_core_count)"
  printf '    "total_memory_kib": %s,\n' "$(json_fact host total_memory_kib)"
  printf '    "steamos_readonly_status": %s,\n' "$(json_fact host steamos_readonly_status)"
  printf '    "filesystem_summary_completeness": %s\n' "$(json_fact host filesystem_summary_completeness)"
  printf '  },\n'
  printf '  "session": {\n'
  printf '    "type": %s,\n' "$(json_fact session type)"
  printf '    "desktop": %s,\n' "$(json_fact session desktop)"
  printf '    "discovery_completeness": %s,\n' "$(json_fact session discovery_completeness)"
  printf '    "display_present": %s,\n' "$(json_fact session display_present)"
  printf '    "wayland_display_present": %s,\n' "$(json_fact session wayland_display_present)"
  printf '    "display_socket_names": %s\n' "$(json_fact session display_socket_names)"
  printf '  },\n'
  printf '  "bitwig_flatpak": {\n'
  printf '    "installed": %s,\n' "$(json_fact bitwig_flatpak installed)"
  printf '    "version": %s,\n' "$(json_fact bitwig_flatpak version)"
  printf '    "branch": %s,\n' "$(json_fact bitwig_flatpak branch)"
  printf '    "origin": %s,\n' "$(json_fact bitwig_flatpak origin)"
  printf '    "architecture": %s,\n' "$(json_fact bitwig_flatpak architecture)"
  printf '    "runtime": %s,\n' "$(json_fact bitwig_flatpak runtime)"
  printf '    "installation_scope": %s,\n' "$(json_fact bitwig_flatpak installation_scope)"
  printf '    "plugin_sandbox_mode": %s,\n' "$(json_fact bitwig_flatpak plugin_sandbox_mode)"
  printf '    "vst_path": %s,\n' "$(json_fact bitwig_flatpak vst_path)"
  printf '    "vst3_path": %s,\n' "$(json_fact bitwig_flatpak vst3_path)"
  printf '    "clap_path": %s,\n' "$(json_fact bitwig_flatpak clap_path)"
  printf '    "effective_vst_path": %s,\n' "$(json_fact bitwig_flatpak effective_vst_path)"
  printf '    "effective_vst3_path": %s,\n' "$(json_fact bitwig_flatpak effective_vst3_path)"
  printf '    "effective_clap_path": %s,\n' "$(json_fact bitwig_flatpak effective_clap_path)"
  printf '    "graphics_socket_posture": %s,\n' "$(json_fact bitwig_flatpak graphics_socket_posture)"
  printf '    "linux_audio_extensions_installed": %s,\n' "$(json_fact bitwig_flatpak linux_audio_extensions_installed)"
  printf '    "runtime_census_completeness": %s,\n' "$(json_fact bitwig_flatpak runtime_census_completeness)"
  printf '    "override_inspection_completeness": %s,\n' "$(json_fact bitwig_flatpak override_inspection_completeness)"
  printf '    "collection_status": %s\n' "$(emit_collection_status_json_array flatpak)"
  printf '  },\n'
  printf '  "audio": {\n'
  printf '    "pipewire_session_available": %s,\n' "$(json_fact audio pipewire_session_available)"
  printf '    "pipewire_audio_sink_count": %s,\n' "$(json_fact audio class_count_Audio_Sink)"
  printf '    "pipewire_audio_source_count": %s,\n' "$(json_fact audio class_count_Audio_Source)"
  printf '    "pipewire_audio_device_count": %s,\n' "$(json_fact audio class_count_Audio_Device)"
  printf '    "pipewire_output_stream_count": %s,\n' "$(json_fact audio class_count_Stream_Output_Audio)"
  printf '    "pipewire_input_stream_count": %s,\n' "$(json_fact audio class_count_Stream_Input_Audio)"
  printf '    "pulseaudio_compatibility_available": %s,\n' "$(json_fact audio pulseaudio_compatibility_available)"
  printf '    "alsa_available": %s,\n' "$(json_fact audio alsa_available)"
  printf '    "alsa_playback_card_count": %s,\n' "$(json_fact audio alsa_playback_card_count)"
  printf '    "default_route_class": %s,\n' "$(json_fact audio default_route_class)"
  printf '    "default_route_api": %s,\n' "$(json_fact audio default_route_api)"
  printf '    "default_route_device_name": %s\n' "$(json_fact audio default_route_device_name)"
  printf '  },\n'
  printf '  "toolchain": {"tools": %s},\n' "$(emit_tool_json_array "$toolchain_file")"
  printf '  "compatibility_runtime_inventory": {\n'
  printf '    "tools": %s,\n' "$(emit_tool_json_array "$compatibility_tools_file")"
  printf '    "freedesktop_runtimes": %s,\n' "$(emit_runtime_json_array)"
  printf '    "runner_inventory_completeness": %s,\n' "$(json_fact compatibility_runtime_inventory runner_inventory_completeness)"
  printf '    "prefix_roster_completeness": %s,\n' "$(json_fact compatibility_runtime_inventory prefix_roster_completeness)"
  printf '    "runner_directories": %s,\n' "$(emit_runner_json_array)"
  printf '    "runner_collection_status": %s,\n' "$(emit_collection_status_json_array runner_inventory)"
  printf '    "wine_environments": %s,\n' "$(emit_prefix_json_array)"
  printf '    "prefix_collection_status": %s,\n' "$(emit_collection_status_json_array prefix_inventory)"
  printf '    "current_processes": %s,\n' "$(emit_process_json_array)"
  printf '    "yabridgectl_read_only_operation": %s,\n' "$(json_fact compatibility_runtime_inventory yabridgectl_read_only_operation)"
  printf '    "yabridge_config_collection_status": %s\n' "$(emit_collection_status_json_array yabridge_config)"
  printf '  },\n'
  printf '  "existing_plugin_state": {\n'
  printf '    "bounded_installer_root": %s,\n' "$(json_fact existing_plugin_state bounded_installer_root)"
  printf '    "bounded_download_root": %s,\n' "$(json_fact existing_plugin_state bounded_download_root)"
  printf '    "bounded_linux_plugin_roots": %s,\n' "$(json_fact existing_plugin_state bounded_linux_plugin_roots)"
  printf '    "bounded_prefix_policy": %s,\n' "$(json_fact existing_plugin_state bounded_prefix_policy)"
  printf '    "artifact_search_completeness": %s,\n' "$(json_fact existing_plugin_state artifact_search_completeness)"
  printf '    "artifact_retention": %s,\n' "$(json_fact existing_plugin_state artifact_retention)"
  printf '    "artifact_search_status": %s,\n' "$(emit_collection_status_json_array artifact_search)"
  printf '    "artifacts": %s\n' "$(emit_artifact_json_array)"
  printf '  },\n'
  printf '  "serum2": {\n'
  printf '    "overall_artifact_result": %s,\n' "$(json_fact serum2 overall_artifact_result)"
  printf '    "installer_result": %s,\n' "$(json_fact serum2 installer_result)"
  printf '    "module_result": %s,\n' "$(json_fact serum2 module_result)"
  printf '    "content_result": %s,\n' "$(json_fact serum2 content_result)"
  printf '    "license_channel": %s,\n' "$(json_fact serum2 license_channel)"
  printf '    "credentials": %s,\n' "$(json_fact serum2 credentials)"
  printf '    "installer_launch": %s,\n' "$(json_fact serum2 installer_launch)"
  printf '    "authorization": %s\n' "$(json_fact serum2 authorization)"
  printf '  },\n'
  printf '  "sanitization": {\n'
  printf '    "home_path": {"classification":"observed","value":"<HOME>"},\n'
  printf '    "username": {"classification":"observed","value":"<USER>"},\n'
  printf '    "hostname": {"classification":"observed","value":"<HOST>"},\n'
  printf '    "raw_evidence_committed": {"classification":"observed","value":"false"}\n'
  printf '  },\n'
  printf '  "claim": {\n'
  printf '    "primary": %s,\n' "$(json_fact claim primary)"
  printf '    "ceiling": %s,\n' "$(json_fact claim ceiling)"
  printf '    "gui_next_step": %s\n' "$(json_fact claim gui_next_step)"
  printf '  }\n'
  printf '}\n'
} > "$temp_dir/fixture.json"

retained_files=(
  BASIS.md
  fixture.json
  HOST.md
  BITWIG_FLATPAK.md
  TOOLCHAIN_AND_RUNTIME.md
  WINDOWS_AUDIO_STATE.md
  SERUM2_OBSERVATIONS.md
  FINDINGS.md
  OPERATOR_HANDOFF.md
  SANITIZATION.md
)

for retained_name in "${retained_files[@]}"; do
  retained_path="$temp_dir/$retained_name"
  if [[ ! -f "$retained_path" ]]; then
    printf 'retained packet file was not generated: %s\n' "$retained_name" >&2
    exit 2
  fi
  retained_size="$(stat -c '%s' "$retained_path")"
  if [[ "$retained_size" -gt "$MAX_RETAINED_FILE_BYTES" ]]; then
    printf 'retained packet file exceeds size bound: %s (%s bytes)\n' "$retained_name" "$retained_size" >&2
    exit 2
  fi
  if command -v iconv >/dev/null 2>&1; then
    iconv -f UTF-8 -t UTF-8 "$retained_path" >/dev/null
  fi
done

if command -v python3 >/dev/null 2>&1; then
  python3 -m json.tool "$temp_dir/fixture.json" >/dev/null
fi

if grep -R -F -n -- "$actual_home" "${retained_files[@]/#/$temp_dir/}" >/dev/null 2>&1; then
  printf 'real home path remains in retained packet\n' >&2
  exit 2
fi
if [[ -n "$actual_hostname" ]] \
  && grep -R -F -n -- "$actual_hostname" "${retained_files[@]/#/$temp_dir/}" >/dev/null 2>&1; then
  printf 'real hostname remains in retained packet\n' >&2
  exit 2
fi
if [[ -n "$actual_user" ]] \
  && grep -R -E -n -- "(^|[^[:alnum:]_-])${actual_user}([^[:alnum:]_-]|$)" \
    "${retained_files[@]/#/$temp_dir/}" >/dev/null 2>&1; then
  printf 'real username remains in retained packet\n' >&2
  exit 2
fi

(
  cd "$temp_dir"
  sha256sum "${retained_files[@]}" > hashes.sha256
)

mkdir -p -- "$output_dir"
for retained_name in "${retained_files[@]}" hashes.sha256; do
  mv -f -- "$temp_dir/$retained_name" "$output_dir/$retained_name"
done
