#!/usr/bin/env bash
set -euo pipefail

export LC_ALL=C

readonly SR0_NAME="sr0-steam-deck-fixture-reconnaissance"
readonly APP_ID="com.bitwig.BitwigStudio"
readonly EXPECTED_BASIS_COMMIT="7319ba8dea7cf2e81a2e4d32c39907bdc6279cf4"
readonly EXPECTED_BASIS_TREE="b3dc324444f769e7d145bb85f5d56a7d587ea6f2"
readonly MAX_SCALAR_BYTES=2048
readonly MAX_DIRECTORY_ROWS=128
readonly MAX_ARTIFACTS=200
readonly MAX_PATH_BYTES=4096
readonly DIRECTORY_COMMAND_SECONDS=20

mode="capture"
if [[ "${1:-}" == "--self-test" ]]; then
  mode="self_test"
  shift
fi
if [[ "$#" -ne 0 ]]; then
  printf 'usage: %s [--self-test]\n' "${0##*/}" >&2
  exit 2
fi

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repo_root="$(git -C "$script_dir" rev-parse --show-toplevel)"

if [[ "$script_dir" != "$repo_root/tools/sr0-fixture-capture" ]]; then
  printf 'capture tool must run from tools/sr0-fixture-capture beneath its repository\n' >&2
  exit 2
fi

if [[ "$(id -u)" -eq 0 ]]; then
  printf 'SR0 capture refuses to run as root\n' >&2
  exit 2
fi

timeout_command="$(type -P timeout || true)"
if [[ -z "$timeout_command" ]]; then
  printf 'trusted timeout command is required; refusing unbounded collection\n' >&2
  exit 2
fi
timeout_command="$(realpath -e -- "$timeout_command" 2>/dev/null || true)"
if [[ -z "$timeout_command" ]] \
  || [[ "$(stat -c '%u' -- "$timeout_command" 2>/dev/null || printf 'unknown')" != "0" ]] \
  || [[ -n "$(find -P "$timeout_command" -maxdepth 0 -perm /022 -print -quit 2>/dev/null)" ]]; then
  printf 'timeout command is not an absolute root-owned non-writable executable; refusing collection\n' >&2
  exit 2
fi
timeout_version="$("$timeout_command" --version 2>/dev/null | head -n 1 || true)"
if [[ "$timeout_version" != timeout\ \(GNU\ coreutils\)* ]]; then
  printf 'GNU coreutils timeout with --signal/--kill-after support is required\n' >&2
  exit 2
fi
readonly timeout_command

if ! command -v realpath >/dev/null 2>&1; then
  printf 'realpath is required for search-root containment validation\n' >&2
  exit 2
fi

self_test_dir=""
if [[ "$mode" == "self_test" ]]; then
  cache_parent="${XDG_CACHE_HOME:-$HOME/.cache}/linux-vst-bridge"
  mkdir -p -- "$cache_parent"
  self_test_dir="$(mktemp -d "$cache_parent/sr0-capture-self-test.XXXXXX")"
  raw_dir="$self_test_dir/raw"
  output_dir="$self_test_dir/output"
else
  raw_dir="$repo_root/evidence/raw/$SR0_NAME"
  output_dir="$repo_root/evidence/$SR0_NAME"
fi

mkdir -p -- "$raw_dir" "$output_dir"

if [[ "$mode" == "capture" ]]; then
  if ! git -C "$repo_root" check-ignore -q -- "evidence/raw/$SR0_NAME/probe"; then
    printf 'raw evidence path is not ignored; refusing capture\n' >&2
    exit 2
  fi

  if git -C "$repo_root" ls-files -- "evidence/raw/$SR0_NAME" | grep -q .; then
    printf 'raw evidence is tracked; refusing capture\n' >&2
    exit 2
  fi
fi

# Reruns replace only fixed raw files under the exact ignored SR0 directory.
find -P "$raw_dir" -mindepth 1 -maxdepth 1 -type f -delete

cleanup_raw_files() {
  if [[ "$mode" == "self_test" ]] \
    && [[ -n "$self_test_dir" ]] \
    && [[ "$self_test_dir" == "${XDG_CACHE_HOME:-$HOME/.cache}/linux-vst-bridge"/sr0-capture-self-test.* ]] \
    && [[ -d "$self_test_dir" ]]; then
    rm -rf -- "$self_test_dir"
  elif [[ "$raw_dir" == "$repo_root/evidence/raw/$SR0_NAME" ]] && [[ -d "$raw_dir" ]]; then
    find -P "$raw_dir" -mindepth 1 -maxdepth 1 -type f -delete
  fi
}
trap cleanup_raw_files EXIT

facts_file="$raw_dir/facts.tsv"
toolchain_file="$raw_dir/toolchain.tsv"
compatibility_tools_file="$raw_dir/compatibility-tools.tsv"
runners_file="$raw_dir/runners.tsv"
prefixes_file="$raw_dir/prefixes.tsv"
artifacts_file="$raw_dir/artifacts.tsv"
processes_file="$raw_dir/processes.tsv"
filesystems_file="$raw_dir/filesystems.txt"
flatpak_info_file="$raw_dir/bitwig-flatpak-info.txt"
flatpak_permissions_file="$raw_dir/bitwig-flatpak-permissions.txt"
flatpak_metadata_file="$raw_dir/bitwig-flatpak-metadata.txt"
flatpak_relevant_metadata_file="$raw_dir/bitwig-flatpak-relevant-metadata.txt"
flatpak_overrides_file="$raw_dir/bitwig-flatpak-overrides.txt"
flatpak_runtimes_file="$raw_dir/freedesktop-runtimes.tsv"
yabridgectl_help_file="$raw_dir/yabridgectl-help.txt"
yabridgectl_list_file="$raw_dir/yabridgectl-list.txt"
collection_status_file="$raw_dir/collection-status.tsv"

for raw_file in \
  "$facts_file" \
  "$toolchain_file" \
  "$compatibility_tools_file" \
  "$runners_file" \
  "$prefixes_file" \
  "$artifacts_file" \
  "$processes_file" \
  "$filesystems_file" \
  "$flatpak_info_file" \
  "$flatpak_permissions_file" \
  "$flatpak_metadata_file" \
  "$flatpak_relevant_metadata_file" \
  "$flatpak_overrides_file" \
  "$flatpak_runtimes_file" \
  "$yabridgectl_help_file" \
  "$yabridgectl_list_file" \
  "$collection_status_file"; do
  : > "$raw_file"
done

normalize_scalar() {
  local value="${1:-}"
  value="${value//$'\t'/ }"
  value="${value//$'\r'/ }"
  value="${value//$'\n'/ }"
  printf '%.*s' "$MAX_SCALAR_BYTES" "$value"
}

record_fact() {
  local section="$1"
  local key="$2"
  local classification="$3"
  local value="${4:-}"
  printf '%s\t%s\t%s\t%s\n' \
    "$section" \
    "$key" \
    "$classification" \
    "$(normalize_scalar "$value")" >> "$facts_file"
}

BOUNDED_STATUS="not_run"
BOUNDED_COMMAND_EXIT="unknown"
BOUNDED_FILTER_EXIT="unknown"
BOUNDED_OUTPUT_BYTES=0
BOUNDED_ROWS_SEEN=0
BOUNDED_ROWS_RETAINED=0

bounded_text_capture() {
  local destination="$1"
  local byte_limit="$2"
  local seconds="$3"
  shift 3
  local temporary
  local command_exit
  local filter_exit
  local output_bytes
  local -a pipeline_status

  temporary="$(mktemp "$raw_dir/.bounded-text.XXXXXX")"
  set +e
  set +o pipefail
  "$timeout_command" --signal=TERM --kill-after=2 "$seconds" "$@" 2>&1 \
    | head -c "$((byte_limit + 1))" > "$temporary"
  pipeline_status=("${PIPESTATUS[@]}")
  set -o pipefail
  set -e

  command_exit="${pipeline_status[0]}"
  filter_exit="${pipeline_status[1]}"
  output_bytes="$(stat -c '%s' -- "$temporary")"
  BOUNDED_COMMAND_EXIT="$command_exit"
  BOUNDED_FILTER_EXIT="$filter_exit"
  BOUNDED_OUTPUT_BYTES="$output_bytes"
  BOUNDED_ROWS_SEEN=0
  BOUNDED_ROWS_RETAINED=0

  if [[ "$command_exit" -eq 124 || "$command_exit" -eq 137 ]]; then
    BOUNDED_STATUS="timed_out"
  elif [[ "$output_bytes" -gt "$byte_limit" ]]; then
    BOUNDED_STATUS="output_truncated"
  elif [[ "$command_exit" -ne 0 || "$filter_exit" -ne 0 ]]; then
    BOUNDED_STATUS="command_failed"
  else
    BOUNDED_STATUS="completed"
  fi

  if [[ "$output_bytes" -gt "$byte_limit" ]]; then
    head -c "$byte_limit" "$temporary" > "$destination"
  else
    mv -f -- "$temporary" "$destination"
    temporary=""
  fi
  if [[ -n "$temporary" ]]; then
    rm -f -- "$temporary"
  fi
}

bounded_nul_capture() {
  local destination="$1"
  local seconds="$2"
  local row_limit="$3"
  shift 3
  local byte_limit=$(((row_limit + 1) * (MAX_PATH_BYTES + 1)))
  local temporary
  local error_file
  local command_exit
  local row_filter_exit
  local byte_filter_exit
  local output_bytes
  local rows_seen
  local retained=0
  local path
  local -a pipeline_status

  temporary="$(mktemp "$raw_dir/.bounded-nul.XXXXXX")"
  error_file="$(mktemp "$raw_dir/.bounded-nul-error.XXXXXX")"
  set +e
  set +o pipefail
  "$timeout_command" --signal=TERM --kill-after=2 "$seconds" "$@" 2> "$error_file" \
    | head -z -n "$((row_limit + 1))" \
    | head -c "$((byte_limit + 1))" > "$temporary"
  pipeline_status=("${PIPESTATUS[@]}")
  set -o pipefail
  set -e

  command_exit="${pipeline_status[0]}"
  row_filter_exit="${pipeline_status[1]}"
  byte_filter_exit="${pipeline_status[2]}"
  output_bytes="$(stat -c '%s' -- "$temporary")"
  rows_seen="$(tr -cd '\000' < "$temporary" | wc -c | tr -d ' ')"

  BOUNDED_COMMAND_EXIT="$command_exit"
  BOUNDED_FILTER_EXIT="$row_filter_exit,$byte_filter_exit"
  BOUNDED_OUTPUT_BYTES="$output_bytes"
  BOUNDED_ROWS_SEEN="$rows_seen"

  if [[ "$command_exit" -eq 124 || "$command_exit" -eq 137 ]]; then
    BOUNDED_STATUS="timed_out"
  elif [[ "$output_bytes" -gt "$byte_limit" ]]; then
    BOUNDED_STATUS="output_truncated"
  elif [[ "$rows_seen" -gt "$row_limit" ]]; then
    BOUNDED_STATUS="row_truncated"
  elif [[ "$command_exit" -ne 0 || "$row_filter_exit" -ne 0 || "$byte_filter_exit" -ne 0 ]]; then
    BOUNDED_STATUS="command_failed"
  else
    BOUNDED_STATUS="completed"
  fi

  : > "$destination"
  while [[ "$retained" -lt "$row_limit" ]] && IFS= read -r -d '' path; do
    printf '%s\0' "$path" >> "$destination"
    retained=$((retained + 1))
  done < "$temporary"
  if [[ "$retained" -gt 0 ]]; then
    sort -z -o "$destination" "$destination"
  fi
  BOUNDED_ROWS_RETAINED="$retained"
  rm -f -- "$temporary" "$error_file"
}

record_collection_status() {
  local scope="$1"
  local role="$2"
  local root="$3"
  local classification="$4"
  local status="$5"
  local detail="$6"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$(normalize_scalar "$scope")" \
    "$(normalize_scalar "$role")" \
    "$(normalize_scalar "$root")" \
    "$(normalize_scalar "$classification")" \
    "$(normalize_scalar "$status")" \
    "$(normalize_scalar "$detail")" >> "$collection_status_file"
}

bounded_status_detail() {
  printf 'status=%s;command_exit=%s;filter_exit=%s;bytes=%s;rows_seen=%s;rows_retained=%s' \
    "$BOUNDED_STATUS" "$BOUNDED_COMMAND_EXIT" "$BOUNDED_FILTER_EXIT" \
    "$BOUNDED_OUTPUT_BYTES" "$BOUNDED_ROWS_SEEN" "$BOUNDED_ROWS_RETAINED"
}

CAPTURE_VALUE=""
CAPTURE_STATUS="not_run"
CAPTURE_EXIT="unknown"

capture_scalar_command() {
  local byte_limit="$1"
  local seconds="$2"
  shift 2
  local temporary

  temporary="$(mktemp "$raw_dir/.scalar.XXXXXX")"
  bounded_text_capture "$temporary" "$byte_limit" "$seconds" "$@"
  CAPTURE_STATUS="$BOUNDED_STATUS"
  CAPTURE_EXIT="$BOUNDED_COMMAND_EXIT"
  CAPTURE_VALUE="$(first_nonempty_line < "$temporary")"
  rm -f -- "$temporary"
}

first_nonempty_line() {
  awk 'NF { print; exit }' | cut -c "1-$MAX_SCALAR_BYTES"
}

capture_version() {
  local destination="$1"
  local label="$2"
  local executable="$3"
  shift 3
  if ! command -v "$executable" >/dev/null 2>&1; then
    printf '%s\tnot_installed\t\n' "$label" >> "$destination"
    return
  fi

  capture_scalar_command 4096 10 "$executable" "$@"
  if [[ "$CAPTURE_STATUS" == "completed" ]] && [[ -n "$CAPTURE_VALUE" ]]; then
    printf '%s\tobserved\t%s\n' "$label" "$(normalize_scalar "$CAPTURE_VALUE")" >> "$destination"
  else
    printf '%s\tunknown\tversion command %s (exit %s)\n' \
      "$label" "$CAPTURE_STATUS" "$CAPTURE_EXIT" >> "$destination"
  fi
}

capture_multiline_version() {
  local destination="$1"
  local label="$2"
  local executable="$3"
  shift 3
  local version
  local temporary

  if ! command -v "$executable" >/dev/null 2>&1; then
    printf '%s\tnot_installed\t\n' "$label" >> "$destination"
    return
  fi

  temporary="$(mktemp "$raw_dir/.version.XXXXXX")"
  bounded_text_capture "$temporary" 4096 10 "$executable" "$@"
  version="$(awk 'NF { if (count > 0) printf "; "; printf "%s", $0; count++; if (count == 3) exit }' "$temporary")"
  if [[ "$BOUNDED_STATUS" == "completed" ]] && [[ -n "$version" ]]; then
    printf '%s\tobserved\t%s\n' "$label" "$(normalize_scalar "$version")" >> "$destination"
  else
    printf '%s\tunknown\tversion command %s (exit %s)\n' \
      "$label" "$BOUNDED_STATUS" "$BOUNDED_COMMAND_EXIT" >> "$destination"
  fi
  rm -f -- "$temporary"
}

os_release_value() {
  local key="$1"
  local value
  value="$(sed -n "s/^${key}=//p" /etc/os-release 2>/dev/null | head -n 1)"
  value="${value#\"}"
  value="${value%\"}"
  normalize_scalar "$value"
}

flatpak_info_value() {
  local label="$1"
  sed -n "s/^[[:space:]]*${label}:[[:space:]]*//p" "$flatpak_info_file" | head -n 1
}

stat_mtime() {
  capture_scalar_command 128 5 stat -c '%y' -- "$1"
  if [[ "$CAPTURE_STATUS" == "completed" ]] && [[ -n "$CAPTURE_VALUE" ]]; then
    printf '%s' "$CAPTURE_VALUE"
  else
    printf 'unknown_%s' "$CAPTURE_STATUS"
  fi
}

capture_process_group() {
  local label="$1"
  shift
  local executable_name
  local matches=0
  local count
  local temporary
  local line_count
  local non_pid_line_count
  local unqueryable_name_count=0
  local incomplete=false

  if ! command -v pgrep >/dev/null 2>&1; then
    printf '%s\tunknown\tpgrep not installed\n' "$label" >> "$processes_file"
    return
  fi

  for executable_name in "$@"; do
    if [[ "${#executable_name}" -gt 15 ]]; then
      unqueryable_name_count=$((unqueryable_name_count + 1))
      incomplete=true
      continue
    fi
    temporary="$(mktemp "$raw_dir/.processes.XXXXXX")"
    bounded_text_capture "$temporary" 4096 5 pgrep -x "$executable_name"
    line_count="$(awk '/^[0-9]+$/ {count++} END {print count + 0}' "$temporary")"
    non_pid_line_count="$(awk 'NF && $0 !~ /^[0-9]+$/ {count++} END {print count + 0}' "$temporary")"
    if [[ "$non_pid_line_count" -gt 0 ]]; then
      incomplete=true
    fi
    if [[ "$BOUNDED_STATUS" == "completed" ]]; then
      :
    elif [[ "$BOUNDED_STATUS" == "command_failed" ]] \
      && [[ "$BOUNDED_COMMAND_EXIT" -eq 1 ]] \
      && [[ "$line_count" -eq 0 ]]; then
      : # pgrep exit 1 is the documented zero-match result.
    else
      incomplete=true
    fi
    if [[ "$line_count" -gt 64 ]]; then
      incomplete=true
      line_count=64
    fi
    count="$line_count"
    matches=$((matches + count))
    rm -f -- "$temporary"
  done
  if [[ "$incomplete" == true ]]; then
    printf '%s\tunknown\tsearch_incomplete; queryable exact-name matches=%s; aliases over Linux comm limit=%s\n' \
      "$label" "$matches" "$unqueryable_name_count" >> "$processes_file"
  else
    printf '%s\tobserved\t%s\n' "$label" "$matches" >> "$processes_file"
  fi
}

ROOT_VALIDATION_STATUS="not_run"
ROOT_VALIDATION_DETAIL=""
validation_home="$HOME"

validate_home_root() {
  local candidate="$1"
  local declared_home="${2:-$validation_home}"
  local canonical_home
  local canonical_candidate
  local relative
  local current
  local component
  local -a components=()

  ROOT_VALIDATION_STATUS="rejected"
  ROOT_VALIDATION_DETAIL="validation_failed"
  canonical_home="$(realpath -e -- "$declared_home" 2>/dev/null || true)"
  if [[ -z "$canonical_home" ]] || [[ ! -d "$canonical_home" ]]; then
    ROOT_VALIDATION_DETAIL="declared_home_canonicalization_failed"
    return
  fi
  if [[ "$candidate" != "$declared_home" ]] && [[ "$candidate" != "$declared_home/"* ]]; then
    ROOT_VALIDATION_DETAIL="lexical_escape_from_declared_home"
    return
  fi

  relative="${candidate#"$declared_home"}"
  relative="${relative#/}"
  current="$declared_home"
  if [[ -n "$relative" ]]; then
    IFS='/' read -r -a components <<< "$relative"
  fi
  for component in "${components[@]}"; do
    if [[ -z "$component" || "$component" == "." || "$component" == ".." ]]; then
      ROOT_VALIDATION_DETAIL="noncanonical_path_component"
      return
    fi
    current="$current/$component"
    if [[ -L "$current" ]]; then
      ROOT_VALIDATION_DETAIL="symlinked_path_component"
      return
    fi
    if [[ ! -e "$current" ]]; then
      ROOT_VALIDATION_STATUS="valid_absent"
      ROOT_VALIDATION_DETAIL="path_absent"
      return
    fi
    if [[ -d "$current" ]] && [[ ! -x "$current" ]]; then
      ROOT_VALIDATION_DETAIL="ancestor_not_searchable"
      return
    fi
  done

  canonical_candidate="$(realpath -e -- "$candidate" 2>/dev/null || true)"
  if [[ -z "$canonical_candidate" ]]; then
    ROOT_VALIDATION_DETAIL="candidate_canonicalization_failed"
    return
  fi
  if [[ "$canonical_candidate" != "$canonical_home" ]] \
    && [[ "$canonical_candidate" != "$canonical_home/"* ]]; then
    ROOT_VALIDATION_DETAIL="canonical_escape_from_declared_home"
    return
  fi
  ROOT_VALIDATION_STATUS="valid_existing"
  ROOT_VALIDATION_DETAIL="canonical_home_containment_proved"
}

collect_runner_root() {
  local root="$1"
  local selection="$2"
  local entry
  local row_count=0
  local temporary
  local empty_detail="root_present_no_immediate_directories"
  local traversal_status
  local traversal_detail
  local -a find_arguments

  if [[ "$root" == "$HOME/.steam/root/"* ]] && [[ -L "$HOME/.steam/root" ]]; then
    printf '%s\tnot_applicable\tunknown\tknown_symlink_alias_not_followed; canonical Steam root inspected separately\n' \
      "$(normalize_scalar "$root")" >> "$runners_file"
    record_collection_status runner_inventory "$selection" "$root" observed alias_not_traversed \
      canonical_equivalent_declared_separately
    return
  fi
  validate_home_root "$root"
  if [[ "$ROOT_VALIDATION_STATUS" == "valid_absent" ]]; then
    printf '%s\tnot_applicable\tnot_found_in_bounded_locations\troot_absent\n' \
      "$(normalize_scalar "$root")" >> "$runners_file"
    record_collection_status runner_inventory "$selection" "$root" observed completed root_absent
    return
  fi
  if [[ "$ROOT_VALIDATION_STATUS" != "valid_existing" ]]; then
    printf '%s\tnot_applicable\tunknown\troot_rejected_%s\n' \
      "$(normalize_scalar "$root")" "$ROOT_VALIDATION_DETAIL" >> "$runners_file"
    record_collection_status runner_inventory "$selection" "$root" unknown root_rejected "$ROOT_VALIDATION_DETAIL"
    return
  fi
  if [[ ! -d "$root" ]]; then
    printf '%s\tnot_applicable\tunknown\troot_not_directory\n' \
      "$(normalize_scalar "$root")" >> "$runners_file"
    record_collection_status runner_inventory "$selection" "$root" unknown root_rejected root_not_directory
    return
  fi

  find_arguments=(find -P "$root" -xdev -mindepth 1 -maxdepth 1 -type d)
  if [[ "$selection" == "steam_common" ]]; then
    find_arguments+=( '(' -iname 'Proton*' -o -iname 'GE-Proton*' -o -iname 'SteamLinuxRuntime*' ')' )
    empty_detail="root_present_no_matching_runner_directories"
  fi
  find_arguments+=(-print0)
  temporary="$(mktemp "$raw_dir/.runner-entries.XXXXXX")"
  bounded_nul_capture "$temporary" "$DIRECTORY_COMMAND_SECONDS" "$MAX_DIRECTORY_ROWS" "${find_arguments[@]}"
  traversal_status="$BOUNDED_STATUS"
  traversal_detail="$(bounded_status_detail)"

  while IFS= read -r -d '' entry; do
    printf '%s\t%s\tobserved\t%s\n' \
      "$(normalize_scalar "$root")" \
      "$(normalize_scalar "${entry##*/}")" \
      "$(stat_mtime "$entry")" >> "$runners_file"
    row_count=$((row_count + 1))
  done < "$temporary"

  if [[ "$traversal_status" == "completed" ]]; then
    record_collection_status runner_inventory "$selection" "$root" observed completed "rows=$row_count"
    if [[ "$row_count" -eq 0 ]]; then
      printf '%s\tnot_applicable\tnot_found_in_bounded_locations\t%s\n' \
        "$(normalize_scalar "$root")" "$empty_detail" >> "$runners_file"
    fi
  else
      printf '%s\tnot_applicable\tunknown\tinventory_%s;%s\n' \
      "$(normalize_scalar "$root")" "$traversal_status" "$traversal_detail" >> "$runners_file"
    record_collection_status runner_inventory "$selection" "$root" unknown "$traversal_status" "$traversal_detail"
  fi
  rm -f -- "$temporary"
}

record_runner_root() {
  collect_runner_root "$1" all_immediate_directories
}

record_steam_common_runners() {
  collect_runner_root "$1" steam_common
}

declare -a discovered_prefixes=()
prefix_roster_complete=true
prefix_global_cap_reported=false

register_prefix() {
  local source_root="$1"
  local prefix="$2"
  local classification="$3"
  local detail="$4"

  if [[ "$classification" == "observed" ]]; then
    if [[ "${#discovered_prefixes[@]}" -ge "$MAX_DIRECTORY_ROWS" ]]; then
      prefix_roster_complete=false
      if [[ "$prefix_global_cap_reported" == false ]]; then
        printf '%s\tnot_applicable\tunknown\tglobal_directory_row_cap_truncated\tunknown_not_collected_to_avoid_recursive_scan\n' \
          "$(normalize_scalar "$source_root")" >> "$prefixes_file"
        record_collection_status prefix_inventory all_declared_roots "$source_root" unknown row_truncated \
          "global_retained_limit=$MAX_DIRECTORY_ROWS"
        prefix_global_cap_reported=true
      fi
      return
    fi
    printf '%s\t%s\t%s\t%s\tunknown_not_collected_to_avoid_recursive_scan\n' \
      "$(normalize_scalar "$source_root")" \
      "$(normalize_scalar "$prefix")" \
      "$classification" \
      "$(stat_mtime "$prefix")" >> "$prefixes_file"
    discovered_prefixes+=("$prefix")
  else
    local retained_prefix="${prefix:-not_applicable}"
    printf '%s\t%s\t%s\t%s\tunknown_not_collected_to_avoid_recursive_scan\n' \
      "$(normalize_scalar "$source_root")" \
      "$(normalize_scalar "$retained_prefix")" \
      "$classification" \
      "$(normalize_scalar "$detail")" >> "$prefixes_file"
  fi
}

capture_direct_prefix() {
  local prefix="$1"
  validate_home_root "$prefix"
  if [[ "$ROOT_VALIDATION_STATUS" == "valid_existing" ]] && [[ -d "$prefix" ]]; then
    register_prefix "$prefix" "$prefix" observed direct_prefix
    record_collection_status prefix_inventory direct_prefix "$prefix" observed completed prefix_present
  elif [[ "$ROOT_VALIDATION_STATUS" == "valid_absent" ]]; then
    register_prefix "$prefix" "" not_found_in_bounded_locations root_absent
    record_collection_status prefix_inventory direct_prefix "$prefix" observed completed root_absent
  else
    prefix_roster_complete=false
    register_prefix "$prefix" "" unknown "root_rejected_$ROOT_VALIDATION_DETAIL"
    record_collection_status prefix_inventory direct_prefix "$prefix" unknown root_rejected "$ROOT_VALIDATION_DETAIL"
  fi
}

capture_child_prefixes() {
  local root="$1"
  local mode="$2"
  local entry
  local prefix
  local row_count=0
  local temporary
  local traversal_status
  local traversal_detail

  if [[ "$root" == "$HOME/.steam/root/"* ]] && [[ -L "$HOME/.steam/root" ]]; then
    register_prefix "$root" "" unknown \
      "known_symlink_alias_not_followed; canonical Steam root inspected separately"
    record_collection_status prefix_inventory "$mode" "$root" observed alias_not_traversed \
      canonical_equivalent_declared_separately
    return
  fi
  validate_home_root "$root"
  if [[ "$ROOT_VALIDATION_STATUS" == "valid_absent" ]]; then
    register_prefix "$root" "" not_found_in_bounded_locations root_absent
    record_collection_status prefix_inventory "$mode" "$root" observed completed root_absent
    return
  fi
  if [[ "$ROOT_VALIDATION_STATUS" != "valid_existing" ]]; then
    prefix_roster_complete=false
    register_prefix "$root" "" unknown "root_rejected_$ROOT_VALIDATION_DETAIL"
    record_collection_status prefix_inventory "$mode" "$root" unknown root_rejected "$ROOT_VALIDATION_DETAIL"
    return
  fi
  if [[ ! -d "$root" ]]; then
    prefix_roster_complete=false
    register_prefix "$root" "" unknown root_not_directory
    record_collection_status prefix_inventory "$mode" "$root" unknown root_rejected root_not_directory
    return
  fi

  temporary="$(mktemp "$raw_dir/.prefix-entries.XXXXXX")"
  bounded_nul_capture "$temporary" "$DIRECTORY_COMMAND_SECONDS" "$MAX_DIRECTORY_ROWS" \
    find -P "$root" -xdev -mindepth 1 -maxdepth 1 -type d -print0
  traversal_status="$BOUNDED_STATUS"
  traversal_detail="$(bounded_status_detail)"
  while IFS= read -r -d '' entry; do
    if [[ "$mode" == "steam_compatdata" ]]; then
      prefix="$entry/pfx"
    else
      prefix="$entry"
    fi
    validate_home_root "$prefix"
    if [[ "$ROOT_VALIDATION_STATUS" == "valid_existing" ]] && [[ -d "$prefix" ]]; then
      register_prefix "$root" "$prefix" observed immediate_directory
    elif [[ "$ROOT_VALIDATION_STATUS" == "valid_absent" ]]; then
      register_prefix "$root" "$prefix" not_found_in_bounded_locations expected_prefix_directory_absent
    else
      prefix_roster_complete=false
      register_prefix "$root" "$prefix" unknown "prefix_rejected_$ROOT_VALIDATION_DETAIL"
    fi
    row_count=$((row_count + 1))
  done < "$temporary"

  if [[ "$traversal_status" == "completed" ]]; then
    record_collection_status prefix_inventory "$mode" "$root" observed completed "rows=$row_count"
    if [[ "$row_count" -eq 0 ]]; then
      register_prefix "$root" "" not_found_in_bounded_locations root_present_no_immediate_directories
    fi
  else
    prefix_roster_complete=false
    register_prefix "$root" "" unknown "inventory_$traversal_status;$traversal_detail"
    record_collection_status prefix_inventory "$mode" "$root" unknown "$traversal_status" "$traversal_detail"
  fi
  rm -f -- "$temporary"
}

declare -a seen_artifact_paths=()
artifact_count=0
installer_candidate_count=0
module_candidate_count=0
content_candidate_count=0
unretained_candidate_count=0
artifact_retention_truncated=false
installer_search_complete=true
module_search_complete=true
content_search_complete=true

mark_artifact_role_incomplete() {
  local role="$1"
  case "$role" in
    installer_location)
      installer_search_complete=false
      content_search_complete=false
      ;;
    linux_plugin_location|windows_plugin_location|yabridge_read_only_configured_location)
      module_search_complete=false
      content_search_complete=false
      ;;
    all)
      installer_search_complete=false
      module_search_complete=false
      content_search_complete=false
      ;;
  esac
}

reset_artifact_state() {
  seen_artifact_paths=()
  artifact_count=0
  installer_candidate_count=0
  module_candidate_count=0
  content_candidate_count=0
  unretained_candidate_count=0
  artifact_retention_truncated=false
  installer_search_complete=true
  module_search_complete=true
  content_search_complete=true
  : > "$artifacts_file"
}

record_artifact() {
  local role="$1"
  local path="$2"
  local kind="content_candidate"
  local object_type="unknown"
  local file_description=""
  local size=""
  local modified=""
  local digest=""
  local lower_path="${path,,}"
  local existing_path

  for existing_path in "${seen_artifact_paths[@]}"; do
    if [[ "$existing_path" == "$path" ]]; then
      return
    fi
  done

  if [[ "$artifact_count" -ge "$MAX_ARTIFACTS" ]]; then
    artifact_retention_truncated=true
    unretained_candidate_count=$((unretained_candidate_count + 1))
    mark_artifact_role_incomplete "$role"
    return
  fi
  seen_artifact_paths+=("$path")
  artifact_count=$((artifact_count + 1))

  if [[ "$role" == "installer_location" && "$lower_path" =~ \.(exe|msi|zip)$ ]]; then
    kind="installer_candidate"
    installer_candidate_count=$((installer_candidate_count + 1))
  elif [[ "$role" != "installer_location" && "$lower_path" =~ \.(vst3|dll|clap|so)$ ]]; then
    kind="module_candidate"
    module_candidate_count=$((module_candidate_count + 1))
  else
    content_candidate_count=$((content_candidate_count + 1))
  fi

  if [[ -L "$path" ]]; then
    object_type="symlink_not_followed"
    file_description="symlink metadata only"
    size="unknown"
    modified="$(stat_mtime "$path")"
    digest="not_applicable_symlink"
  elif [[ -f "$path" ]]; then
    object_type="regular_file"
    capture_scalar_command 512 10 file -b -- "$path"
    if [[ "$CAPTURE_STATUS" == "completed" ]]; then
      file_description="$CAPTURE_VALUE"
    else
      file_description="unknown_$CAPTURE_STATUS"
    fi
    capture_scalar_command 64 5 stat -c '%s' -- "$path"
    if [[ "$CAPTURE_STATUS" == "completed" ]]; then size="$CAPTURE_VALUE"; else size="unknown_$CAPTURE_STATUS"; fi
    modified="$(stat_mtime "$path")"
    capture_scalar_command 256 120 sha256sum -- "$path"
    if [[ "$CAPTURE_STATUS" == "completed" ]]; then
      digest="${CAPTURE_VALUE%% *}"
    else
      digest="unknown_hash_$CAPTURE_STATUS"
    fi
    if [[ ! "$digest" =~ ^[0-9a-f]{64}$ ]]; then
      digest="unknown_hash_failed_or_timed_out"
    fi
  elif [[ -d "$path" ]]; then
    object_type="directory_not_recursively_hashed"
    file_description="directory"
    capture_scalar_command 64 5 stat -c '%s' -- "$path"
    if [[ "$CAPTURE_STATUS" == "completed" ]]; then size="$CAPTURE_VALUE"; else size="unknown_$CAPTURE_STATUS"; fi
    modified="$(stat_mtime "$path")"
    digest="not_applicable_directory"
  else
    object_type="other"
    capture_scalar_command 512 10 file -b -- "$path"
    if [[ "$CAPTURE_STATUS" == "completed" ]]; then file_description="$CAPTURE_VALUE"; else file_description="unknown_$CAPTURE_STATUS"; fi
    capture_scalar_command 64 5 stat -c '%s' -- "$path"
    if [[ "$CAPTURE_STATUS" == "completed" ]]; then size="$CAPTURE_VALUE"; else size="unknown_$CAPTURE_STATUS"; fi
    modified="$(stat_mtime "$path")"
    digest="not_applicable_non_regular_file"
  fi

  printf '%s\t%s\tobserved\t%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$(normalize_scalar "$role")" \
    "$(normalize_scalar "$path")" \
    "$kind" \
    "$object_type" \
    "$(normalize_scalar "$file_description")" \
    "$(normalize_scalar "$size")" \
    "$(normalize_scalar "$modified")" \
    "$(normalize_scalar "$digest")" >> "$artifacts_file"
}

search_artifact_root() {
  local role="$1"
  local root="$2"
  local depth="$3"
  local path
  local temporary
  local search_status
  local search_detail
  local rows_retained

  validate_home_root "$root"
  if [[ "$ROOT_VALIDATION_STATUS" == "valid_absent" ]]; then
    record_collection_status artifact_search "$role" "$root" observed completed root_absent
    return
  fi
  if [[ "$ROOT_VALIDATION_STATUS" != "valid_existing" ]]; then
    mark_artifact_role_incomplete "$role"
    record_collection_status artifact_search "$role" "$root" unknown root_rejected "$ROOT_VALIDATION_DETAIL"
    return
  fi
  if [[ ! -d "$root" ]]; then
    mark_artifact_role_incomplete "$role"
    record_collection_status artifact_search "$role" "$root" unknown root_rejected root_not_directory
    return
  fi

  temporary="$(mktemp "$raw_dir/.artifact-paths.XXXXXX")"
  bounded_nul_capture "$temporary" "$DIRECTORY_COMMAND_SECONDS" "$MAX_ARTIFACTS" \
    find -P "$root" -xdev -mindepth 1 -maxdepth "$depth" \
      '(' -iname '*serum*' -o -iname '*xfer*' ')' -print0
  search_status="$BOUNDED_STATUS"
  search_detail="$(bounded_status_detail)"
  rows_retained="$BOUNDED_ROWS_RETAINED"
  while IFS= read -r -d '' path; do
    record_artifact "$role" "$path"
  done < "$temporary"

  if [[ "$search_status" == "completed" ]]; then
    record_collection_status artifact_search "$role" "$root" observed completed "rows=$rows_retained"
  else
    mark_artifact_role_incomplete "$role"
    record_collection_status artifact_search "$role" "$root" unknown "$search_status" "$search_detail"
  fi
  rm -f -- "$temporary"
}

record_serum_results() {
  local overall_complete=true
  local completeness_detail="all contributing roots completed within declared bounds"
  if [[ "$installer_search_complete" != true ]] \
    || [[ "$module_search_complete" != true ]] \
    || [[ "$content_search_complete" != true ]]; then
    overall_complete=false
    completeness_detail="search_incomplete; one or more contributing roots or candidate-retention operations did not complete within declared bounds"
  fi

  if [[ "$artifact_count" -gt 0 ]]; then
    record_fact serum2 overall_artifact_result observed \
      "$artifact_count retained candidate path(s) matched Serum/Xfer; $completeness_detail"
  elif [[ "$overall_complete" == true ]]; then
    record_fact serum2 overall_artifact_result not_found_in_bounded_locations "no Serum/Xfer name matches"
  else
    record_fact serum2 overall_artifact_result unknown "$completeness_detail"
  fi

  if [[ "$installer_candidate_count" -gt 0 ]]; then
    record_fact serum2 installer_result observed \
      "$installer_candidate_count retained installer candidate(s); search_complete=$installer_search_complete"
  elif [[ "$installer_search_complete" == true ]]; then
    record_fact serum2 installer_result not_found_in_bounded_locations \
      "no installer candidate in declared fixture-input or Downloads search"
  else
    record_fact serum2 installer_result unknown \
      "search_incomplete; installer absence cannot be established"
  fi

  if [[ "$module_candidate_count" -gt 0 ]]; then
    record_fact serum2 module_result observed \
      "$module_candidate_count retained module candidate(s); search_complete=$module_search_complete"
  elif [[ "$module_search_complete" == true ]]; then
    record_fact serum2 module_result not_found_in_bounded_locations \
      "no module candidate in declared Linux or discovered-prefix plug-in roots"
  else
    record_fact serum2 module_result unknown \
      "search_incomplete; module absence cannot be established"
  fi

  if [[ "$content_candidate_count" -gt 0 ]]; then
    record_fact serum2 content_result observed \
      "$content_candidate_count retained content/directory candidate(s); search_complete=$content_search_complete"
  elif [[ "$content_search_complete" == true ]]; then
    record_fact serum2 content_result not_found_in_bounded_locations \
      "no other Serum/Xfer named candidate in declared roots"
  else
    record_fact serum2 content_result unknown \
      "search_incomplete; other content absence cannot be established"
  fi

  if [[ "$overall_complete" == true ]]; then
    record_fact existing_plugin_state artifact_search_completeness observed "$completeness_detail"
  else
    record_fact existing_plugin_state artifact_search_completeness unknown "$completeness_detail"
  fi
  if [[ "$artifact_retention_truncated" == true ]]; then
    record_fact existing_plugin_state artifact_retention unknown \
      "global candidate cap reached; $unretained_candidate_count additional candidate(s) were not retained"
  else
    record_fact existing_plugin_state artifact_retention observed \
      "$artifact_count/$MAX_ARTIFACTS candidate slots used; no retention truncation"
  fi
}

FLATPAK_FILTER_STATUS="not_run"
FLATPAK_FILTER_MATCHES=0

filter_matching_flatpak_runtimes() {
  local source="$1"
  local expected_branch="$2"
  local destination="$3"
  local row_limit="${4:-$MAX_DIRECTORY_ROWS}"
  local application architecture branch version installation origin remainder
  local matching_rows=0

  : > "$destination"
  while IFS=$'\t' read -r application architecture branch version installation origin remainder; do
    [[ -n "$application" ]] || continue
    if { [[ "$application" == "org.freedesktop.Platform" ]] \
      || [[ "$application" == org.freedesktop.LinuxAudio* ]]; } \
      && [[ "$branch" == "$expected_branch" ]]; then
      matching_rows=$((matching_rows + 1))
      if [[ "$matching_rows" -le "$row_limit" ]]; then
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
          "$application" "$architecture" "$branch" "$version" "$installation" "$origin" \
          >> "$destination"
      fi
    fi
  done < "$source"

  FLATPAK_FILTER_MATCHES="$matching_rows"
  if [[ "$matching_rows" -gt "$row_limit" ]]; then
    FLATPAK_FILTER_STATUS="row_truncated"
  else
    FLATPAK_FILTER_STATUS="completed"
  fi
}

self_test_report() {
  local name="$1"
  local passed="$2"
  if [[ "$passed" == true ]]; then
    printf 'SELF_TEST %s: PASS\n' "$name"
  else
    printf 'SELF_TEST %s: FAIL\n' "$name"
    self_test_failures=$((self_test_failures + 1))
  fi
}

run_self_tests() {
  local test_home="$self_test_dir/home"
  local test_output
  local test_root
  local later_root
  local early_root
  local flatpak_input
  local flatpak_output
  local outside_root
  local index
  local classification
  local elapsed
  local started
  local passed
  local prefix_test_root
  local original_validation_home="$validation_home"
  self_test_failures=0

  mkdir -p -- "$test_home"
  validation_home="$test_home"

  # 1. A stalled producer must be terminated and make absence unknown.
  reset_artifact_state
  : > "$facts_file"
  test_output="$raw_dir/self-test-timeout.nul"
  started=$SECONDS
  bounded_nul_capture "$test_output" 1 4 bash -c 'sleep 5; printf "SerumTooLate\\0"'
  elapsed=$((SECONDS - started))
  if [[ "$BOUNDED_STATUS" != "completed" ]]; then
    mark_artifact_role_incomplete installer_location
  fi
  record_serum_results
  classification="$(awk -F '\t' '$1 == "serum2" && $2 == "installer_result" {print $3; exit}' "$facts_file")"
  passed=false
  if [[ "$BOUNDED_STATUS" == "timed_out" ]] && [[ "$elapsed" -le 4 ]] && [[ "$classification" == "unknown" ]]; then
    passed=true
  fi
  self_test_report timeout "$passed"

  # 2. Exhausting retained candidate slots must not skip or falsify a later role.
  reset_artifact_state
  : > "$facts_file"
  : > "$collection_status_file"
  early_root="$test_home/artifact-cap-early"
  later_root="$test_home/artifact-cap-later"
  mkdir -p -- "$early_root" "$later_root"
  for ((index = 1; index <= MAX_ARTIFACTS; index++)); do
    mkdir -p -- "$early_root/SerumNoise$index"
  done
  : > "$later_root/Serum2Installer.exe"
  search_artifact_root installer_location "$early_root" 1
  search_artifact_root installer_location "$later_root" 1
  record_serum_results
  classification="$(awk -F '\t' '$1 == "serum2" && $2 == "installer_result" {print $3; exit}' "$facts_file")"
  passed=false
  if [[ "$artifact_retention_truncated" == true ]] \
    && [[ "$classification" == "unknown" ]] \
    && awk -F '\t' -v root="$later_root" '$1 == "artifact_search" && $3 == root && $5 == "completed" {found=1} END {exit !found}' "$collection_status_file"; then
    passed=true
  fi
  self_test_report global_artifact_cap "$passed"

  # 3. MAX_DIRECTORY_ROWS + 1 prefixes must expose truncation downstream.
  reset_artifact_state
  discovered_prefixes=()
  prefix_roster_complete=true
  prefix_global_cap_reported=false
  : > "$facts_file"
  : > "$prefixes_file"
  : > "$collection_status_file"
  prefix_test_root="$test_home/prefix-row-cap"
  mkdir -p -- "$prefix_test_root"
  for ((index = 1; index <= MAX_DIRECTORY_ROWS + 1; index++)); do
    mkdir -p -- "$prefix_test_root/prefix$index"
  done
  capture_child_prefixes "$prefix_test_root" direct_children
  if [[ "$prefix_roster_complete" != true ]]; then
    mark_artifact_role_incomplete windows_plugin_location
  fi
  record_serum_results
  classification="$(awk -F '\t' '$1 == "serum2" && $2 == "module_result" {print $3; exit}' "$facts_file")"
  passed=false
  if [[ "$prefix_roster_complete" == false ]] \
    && [[ "$classification" == "unknown" ]] \
    && grep -F -q $'\trow_truncated\t' "$collection_status_file"; then
    passed=true
  fi
  self_test_report directory_row_cap "$passed"

  # 4. The root named serum2 is not itself an artifact.
  reset_artifact_state
  : > "$facts_file"
  : > "$collection_status_file"
  test_root="$test_home/root-self-match/serum2"
  mkdir -p -- "$test_root"
  search_artifact_root installer_location "$test_root" 3
  record_serum_results
  passed=false
  if [[ "$artifact_count" -eq 0 ]] \
    && [[ "$(awk -F '\t' '$1 == "serum2" && $2 == "installer_result" {print $3; exit}' "$facts_file")" == "not_found_in_bounded_locations" ]]; then
    passed=true
  fi
  self_test_report search_root_self_match "$passed"

  # 5. A 24.08 Linux Audio extension does not match a 25.08 runtime.
  flatpak_input="$raw_dir/self-test-flatpak-input.tsv"
  flatpak_output="$raw_dir/self-test-flatpak-output.tsv"
  printf 'org.freedesktop.Platform\tx86_64\t25.08\tplatform-version\tsystem\ttest-origin\norg.freedesktop.LinuxAudio.Plugins\tx86_64\t24.08\textension-version\tsystem\ttest-origin\n' \
    > "$flatpak_input"
  filter_matching_flatpak_runtimes "$flatpak_input" 25.08 "$flatpak_output"
  passed=false
  if [[ "$FLATPAK_FILTER_STATUS" == "completed" ]] \
    && [[ "$(awk -F '\t' '$1 ~ /^org\.freedesktop\.LinuxAudio/ {count++} END {print count + 0}' "$flatpak_output")" -eq 0 ]] \
    && [[ "$(awk -F '\t' '$1 == "org.freedesktop.Platform" {count++} END {print count + 0}' "$flatpak_output")" -eq 1 ]]; then
    passed=true
  fi
  self_test_report flatpak_runtime_branch "$passed"

  # 6. A symlinked ancestor beneath home is rejected without traversal.
  reset_artifact_state
  : > "$facts_file"
  : > "$collection_status_file"
  outside_root="$self_test_dir/outside-root"
  mkdir -p -- "$outside_root/vendor" "$test_home/symlink-case"
  : > "$outside_root/vendor/SerumMustNotBeSearched.vst3"
  ln -s -- "$outside_root" "$test_home/symlink-case/plugins-link"
  search_artifact_root yabridge_read_only_configured_location \
    "$test_home/symlink-case/plugins-link/vendor" 3
  record_serum_results
  passed=false
  if [[ "$artifact_count" -eq 0 ]] \
    && [[ "$module_search_complete" == false ]] \
    && grep -F -q $'\troot_rejected\tsymlinked_path_component' "$collection_status_file"; then
    passed=true
  fi
  self_test_report symlinked_ancestor "$passed"

  # 7. A normal empty bounded root remains a lawful bounded non-finding.
  reset_artifact_state
  : > "$facts_file"
  : > "$collection_status_file"
  test_root="$test_home/normal-completion"
  mkdir -p -- "$test_root"
  search_artifact_root installer_location "$test_root" 2
  record_serum_results
  passed=false
  if [[ "$(awk -F '\t' '$1 == "serum2" && $2 == "installer_result" {print $3; exit}' "$facts_file")" == "not_found_in_bounded_locations" ]] \
    && awk -F '\t' -v root="$test_root" '$1 == "artifact_search" && $3 == root && $5 == "completed" {found=1} END {exit !found}' "$collection_status_file"; then
    passed=true
  fi
  self_test_report normal_completion "$passed"

  validation_home="$original_validation_home"
  if [[ "$self_test_failures" -ne 0 ]]; then
    printf 'SELF_TEST_SUMMARY: FAIL (%s/7 failed)\n' "$self_test_failures" >&2
    return 1
  fi
  printf 'SELF_TEST_SUMMARY: PASS (7/7)\n'
}

if [[ "$mode" == "self_test" ]]; then
  run_self_tests
  exit 0
fi

# A. Repository basis
record_fact basis repository_path observed "$repo_root"
record_fact basis expected_basis_commit observed "$EXPECTED_BASIS_COMMIT"
record_fact basis expected_basis_tree observed "$EXPECTED_BASIS_TREE"
record_fact basis commit observed "$(git -C "$repo_root" rev-parse HEAD)"
record_fact basis tree observed "$(git -C "$repo_root" rev-parse 'HEAD^{tree}')"
record_fact basis branch observed "$(git -C "$repo_root" symbolic-ref -q --short HEAD || printf 'DETACHED')"
record_fact basis capture_timestamp observed "$(date --iso-8601=seconds)"
record_fact basis git_version observed "$(git --version)"
tool_revision="$({ sha256sum \
  "$script_dir/capture.sh" \
  "$script_dir/sanitize.sh" \
  "$script_dir/README.md"; } | sha256sum | awk '{print $1}')"
record_fact basis capture_tool_revision observed "sha256:$tool_revision"

# B. Host and session
record_fact host os_name observed "$(os_release_value NAME)"
record_fact host os_pretty_name observed "$(os_release_value PRETTY_NAME)"
record_fact host os_version_id observed "$(os_release_value VERSION_ID)"
record_fact host os_build_id observed "$(os_release_value BUILD_ID)"
os_variant_id="$(os_release_value VARIANT_ID)"
host_identifier="$(head -n 1 /etc/hostname 2>/dev/null || true)"
if [[ -n "$host_identifier" ]] && [[ "$os_variant_id" == "$host_identifier" ]]; then
  record_fact host os_variant_id observed "Steam Deck variant (literal VARIANT_ID withheld because it is identical to the hostname)"
else
  record_fact host os_variant_id observed "$os_variant_id"
fi
record_fact host kernel observed "$(uname -srvmo)"
record_fact host architecture observed "$(uname -m)"
if [[ -r /sys/devices/virtual/dmi/id/product_name ]]; then
  record_fact host hardware_model observed "$(head -n 1 /sys/devices/virtual/dmi/id/product_name)"
else
  record_fact host hardware_model unknown "DMI product name unavailable"
fi
cpu_model="$(awk -F ': ' '/^model name[[:space:]]*:/{print $2; exit}' /proc/cpuinfo 2>/dev/null || true)"
if [[ -n "$cpu_model" ]]; then
  record_fact host cpu_model observed "$cpu_model"
else
  record_fact host cpu_model unknown ""
fi
logical_cores="$(getconf _NPROCESSORS_ONLN 2>/dev/null || true)"
if [[ -n "$logical_cores" ]]; then
  record_fact host logical_core_count observed "$logical_cores"
else
  record_fact host logical_core_count unknown ""
fi
memory_kib="$(awk '/^MemTotal:/{print $2; exit}' /proc/meminfo 2>/dev/null || true)"
if [[ -n "$memory_kib" ]]; then
  record_fact host total_memory_kib observed "$memory_kib"
else
  record_fact host total_memory_kib unknown ""
fi

if command -v steamos-readonly >/dev/null 2>&1; then
  capture_scalar_command 512 10 steamos-readonly status
  if [[ "$CAPTURE_STATUS" == "completed" ]] && [[ -n "$CAPTURE_VALUE" ]]; then
    record_fact host steamos_readonly_status observed "$CAPTURE_VALUE"
  else
    record_fact host steamos_readonly_status unknown "status command $CAPTURE_STATUS"
  fi
else
  record_fact host steamos_readonly_status not_installed "status command unavailable"
fi

session_type="${XDG_SESSION_TYPE:-}"
session_desktop="${XDG_CURRENT_DESKTOP:-}"
session_discovery_complete=true
if command -v loginctl >/dev/null 2>&1; then
  session_list_file="$(mktemp "$raw_dir/.sessions.XXXXXX")"
  session_ids_file="$(mktemp "$raw_dir/.session-ids.XXXXXX")"
  bounded_text_capture "$session_list_file" 4096 8 loginctl list-sessions --no-legend --no-pager
  session_list_status="$BOUNDED_STATUS"
  session_list_detail="$(bounded_status_detail)"
  awk -v expected_uid="$(id -u)" '$2 == expected_uid {print $1}' "$session_list_file" > "$session_ids_file"
  session_id_count="$(awk 'NF {count++} END {print count + 0}' "$session_ids_file")"
  if [[ "$session_list_status" != "completed" ]] || [[ "$session_id_count" -gt 8 ]]; then
    session_discovery_complete=false
  fi
  while IFS= read -r session_id; do
    [[ -n "$session_id" ]] || continue
    session_details_file="$(mktemp "$raw_dir/.session-details.XXXXXX")"
    bounded_text_capture "$session_details_file" 4096 8 loginctl show-session "$session_id" \
      -p Type -p Class -p State -p Remote -p Service -p Desktop --no-pager
    if [[ "$BOUNDED_STATUS" != "completed" ]]; then
      session_discovery_complete=false
      rm -f -- "$session_details_file"
      continue
    fi
    candidate_type="$(sed -n 's/^Type=//p' "$session_details_file" | head -n 1)"
    if [[ "$candidate_type" == "wayland" || "$candidate_type" == "x11" ]]; then
      session_type="$candidate_type"
      candidate_desktop="$(sed -n 's/^Desktop=//p' "$session_details_file" | head -n 1)"
      if [[ -n "$candidate_desktop" ]]; then
        session_desktop="$candidate_desktop"
      fi
      rm -f -- "$session_details_file"
      break
    fi
    rm -f -- "$session_details_file"
  done < <(sed -n '1,8p' "$session_ids_file")
  rm -f -- "$session_list_file" "$session_ids_file"
fi
if [[ -n "$session_type" ]]; then
  record_fact session type observed "$session_type"
else
  record_fact session type unknown "not visible to SSH capture and no graphical loginctl session found"
fi
if [[ -n "$session_desktop" ]]; then
  record_fact session desktop observed "$session_desktop"
else
  record_fact session desktop unknown ""
fi
if [[ "$session_discovery_complete" == true ]]; then
  record_fact session discovery_completeness observed "loginctl session census completed within byte/time/row bounds"
else
  record_fact session discovery_completeness unknown \
    "session census search_incomplete; list status=${session_list_status:-not_run}; ${session_list_detail:-no_detail}"
fi
if [[ -n "${DISPLAY:-}" ]]; then
  record_fact session display_present observed true
else
  record_fact session display_present observed false
fi
if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
  record_fact session wayland_display_present observed true
else
  record_fact session wayland_display_present observed false
fi
record_fact session display_socket_names explicitly_out_of_scope "presence only; socket values not retained"

df_raw_file="$(mktemp "$raw_dir/.df.XXXXXX")"
df_filtered_file="$(mktemp "$raw_dir/.df-filtered.XXXXXX")"
bounded_text_capture "$df_raw_file" 32768 10 df -B1 --output=source,fstype,size,used,avail,pcent,target
df_status="$BOUNDED_STATUS"
df_detail="$(bounded_status_detail)"
awk 'NR == 1 || $1 ~ /^\/dev\//' "$df_raw_file" > "$df_filtered_file"
df_row_count="$(awk 'NF {count++} END {print count + 0}' "$df_filtered_file")"
sed -n '1,64p' "$df_filtered_file" > "$filesystems_file"
if [[ "$df_status" == "completed" ]] && [[ "$df_row_count" -le 64 ]]; then
  record_fact host filesystem_summary_completeness observed "completed; rows=$df_row_count"
else
  record_fact host filesystem_summary_completeness unknown \
    "search_incomplete; status=$df_status; rows_seen=$df_row_count; $df_detail"
fi
rm -f -- "$df_raw_file" "$df_filtered_file"

# C. Bitwig Flatpak
if ! command -v flatpak >/dev/null 2>&1; then
  record_fact bitwig_flatpak installed not_installed false
  for key in version branch origin architecture runtime installation_scope vst_path vst3_path clap_path \
    effective_vst_path effective_vst3_path effective_clap_path graphics_socket_posture \
    linux_audio_extensions_installed; do
    record_fact bitwig_flatpak "$key" unknown "flatpak command not installed"
  done
  record_fact bitwig_flatpak plugin_sandbox_mode unknown "flatpak command not installed"
  record_fact bitwig_flatpak runtime_census_completeness unknown "flatpak command not installed"
  record_fact bitwig_flatpak override_inspection_completeness unknown "flatpak command not installed"
else
  flatpak_apps_file="$(mktemp "$raw_dir/.flatpak-apps.XXXXXX")"
  bounded_text_capture "$flatpak_apps_file" 65536 20 flatpak list --app --columns=application
  flatpak_apps_status="$BOUNDED_STATUS"
  flatpak_apps_detail="$(bounded_status_detail)"
  flatpak_app_rows="$(awk 'NF {count++} END {print count + 0}' "$flatpak_apps_file")"

  if [[ "$flatpak_apps_status" != "completed" ]] || [[ "$flatpak_app_rows" -gt "$MAX_DIRECTORY_ROWS" ]]; then
    record_fact bitwig_flatpak installed unknown \
      "Flatpak application census search_incomplete; status=$flatpak_apps_status; rows=$flatpak_app_rows"
    for key in version branch origin architecture runtime installation_scope vst_path vst3_path clap_path \
      effective_vst_path effective_vst3_path effective_clap_path graphics_socket_posture \
      linux_audio_extensions_installed; do
      record_fact bitwig_flatpak "$key" unknown "Flatpak application census incomplete"
    done
    record_fact bitwig_flatpak plugin_sandbox_mode unknown "Flatpak application census incomplete"
    record_fact bitwig_flatpak runtime_census_completeness unknown "$flatpak_apps_detail"
    record_fact bitwig_flatpak override_inspection_completeness unknown "Flatpak application census incomplete"
    record_collection_status flatpak application_census "$APP_ID" unknown "$flatpak_apps_status" "$flatpak_apps_detail"
  elif ! grep -F -x -q "$APP_ID" "$flatpak_apps_file"; then
    record_fact bitwig_flatpak installed not_installed false
    for key in version branch origin architecture runtime installation_scope vst_path vst3_path clap_path \
      effective_vst_path effective_vst3_path effective_clap_path graphics_socket_posture \
      linux_audio_extensions_installed; do
      record_fact bitwig_flatpak "$key" unknown "application not installed"
    done
    record_fact bitwig_flatpak plugin_sandbox_mode unknown "application not installed"
    record_fact bitwig_flatpak runtime_census_completeness observed "application census completed; app absent"
    record_fact bitwig_flatpak override_inspection_completeness unknown "application not installed"
    record_collection_status flatpak application_census "$APP_ID" observed completed application_absent
  else
    record_fact bitwig_flatpak installed observed true
    record_collection_status flatpak application_census "$APP_ID" observed completed application_present

    bounded_text_capture "$flatpak_info_file" 32768 20 flatpak info "$APP_ID"
    flatpak_info_status="$BOUNDED_STATUS"
    flatpak_info_detail="$(bounded_status_detail)"
    record_collection_status flatpak app_info "$APP_ID" \
      "$([[ "$flatpak_info_status" == completed ]] && printf observed || printf unknown)" \
      "$flatpak_info_status" "$flatpak_info_detail"

    bounded_text_capture "$flatpak_permissions_file" 32768 20 flatpak info --show-permissions "$APP_ID"
    flatpak_permissions_status="$BOUNDED_STATUS"
    flatpak_permissions_detail="$(bounded_status_detail)"
    flatpak_permissions_rows="$(awk 'END {print NR + 0}' "$flatpak_permissions_file")"
    if [[ "$flatpak_permissions_status" == completed ]] && [[ "$flatpak_permissions_rows" -gt 256 ]]; then
      flatpak_permissions_status="row_truncated"
      flatpak_permissions_detail="retained_row_limit=256; rows_seen=$flatpak_permissions_rows"
    fi
    record_collection_status flatpak permissions "$APP_ID" \
      "$([[ "$flatpak_permissions_status" == completed ]] && printf observed || printf unknown)" \
      "$flatpak_permissions_status" "$flatpak_permissions_detail"

    bounded_text_capture "$flatpak_metadata_file" 65536 20 flatpak info --show-metadata "$APP_ID"
    flatpak_metadata_status="$BOUNDED_STATUS"
    flatpak_metadata_detail="$(bounded_status_detail)"
    record_collection_status flatpak metadata "$APP_ID" \
      "$([[ "$flatpak_metadata_status" == completed ]] && printf observed || printf unknown)" \
      "$flatpak_metadata_status" "$flatpak_metadata_detail"

    flatpak_user_override_file="$(mktemp "$raw_dir/.flatpak-user-override.XXXXXX")"
    flatpak_system_override_file="$(mktemp "$raw_dir/.flatpak-system-override.XXXXXX")"
    bounded_text_capture "$flatpak_user_override_file" 16384 15 flatpak override --user --show "$APP_ID"
    flatpak_user_override_status="$BOUNDED_STATUS"
    flatpak_user_override_detail="$(bounded_status_detail)"
    flatpak_user_override_rows="$(awk 'END {print NR + 0}' "$flatpak_user_override_file")"
    if [[ "$flatpak_user_override_status" == completed ]] && [[ "$flatpak_user_override_rows" -gt 256 ]]; then
      flatpak_user_override_status="row_truncated"
      flatpak_user_override_detail="retained_row_limit=256; rows_seen=$flatpak_user_override_rows"
    fi
    bounded_text_capture "$flatpak_system_override_file" 16384 15 flatpak override --system --show "$APP_ID"
    flatpak_system_override_status="$BOUNDED_STATUS"
    flatpak_system_override_detail="$(bounded_status_detail)"
    flatpak_system_override_rows="$(awk 'END {print NR + 0}' "$flatpak_system_override_file")"
    if [[ "$flatpak_system_override_status" == completed ]] && [[ "$flatpak_system_override_rows" -gt 256 ]]; then
      flatpak_system_override_status="row_truncated"
      flatpak_system_override_detail="retained_row_limit=256; rows_seen=$flatpak_system_override_rows"
    fi
    {
      printf '[user overrides]\n'
      sed -n '1,256p' "$flatpak_user_override_file"
      printf '\n[system overrides]\n'
      sed -n '1,256p' "$flatpak_system_override_file"
    } > "$flatpak_overrides_file"
    if [[ "$flatpak_user_override_status" == completed ]] \
      && [[ "$flatpak_system_override_status" == completed ]]; then
      record_fact bitwig_flatpak override_inspection_completeness observed \
        "user and system override commands completed"
    else
      record_fact bitwig_flatpak override_inspection_completeness unknown \
        "search_incomplete; user=$flatpak_user_override_status; system=$flatpak_system_override_status"
    fi
    record_collection_status flatpak user_overrides "$APP_ID" \
      "$([[ "$flatpak_user_override_status" == completed ]] && printf observed || printf unknown)" \
      "$flatpak_user_override_status" "$flatpak_user_override_detail"
    record_collection_status flatpak system_overrides "$APP_ID" \
      "$([[ "$flatpak_system_override_status" == completed ]] && printf observed || printf unknown)" \
      "$flatpak_system_override_status" "$flatpak_system_override_detail"
    rm -f -- "$flatpak_user_override_file" "$flatpak_system_override_file"

    if [[ "$flatpak_info_status" == "completed" ]]; then
      record_fact bitwig_flatpak version observed "$(flatpak_info_value Version)"
      record_fact bitwig_flatpak branch observed "$(flatpak_info_value Branch)"
      record_fact bitwig_flatpak origin observed "$(flatpak_info_value Origin)"
      record_fact bitwig_flatpak architecture observed "$(flatpak_info_value Arch)"
      bitwig_runtime="$(flatpak_info_value Runtime)"
      record_fact bitwig_flatpak runtime observed "$bitwig_runtime"
      record_fact bitwig_flatpak installation_scope observed "$(flatpak_info_value Installation)"
    else
      bitwig_runtime=""
      for key in version branch origin architecture runtime installation_scope; do
        record_fact bitwig_flatpak "$key" unknown "flatpak info $flatpak_info_status"
      done
    fi
    record_fact bitwig_flatpak plugin_sandbox_mode unknown \
      "not available from Flatpak metadata; Bitwig settings not inspected"

    metadata_relevant_all="$(mktemp "$raw_dir/.flatpak-relevant-all.XXXXXX")"
    awk '
      /^\[/ {
        in_relevant = ($0 == "[Context]" || $0 == "[Environment]" ||
                       $0 == "[Session Bus Policy]" || $0 == "[System Bus Policy]" ||
                       $0 ~ /^\[Extension org\.freedesktop\.LinuxAudio/)
        if (in_relevant) print
        next
      }
      in_relevant { print }
    ' "$flatpak_metadata_file" > "$metadata_relevant_all"
    metadata_relevant_rows="$(awk 'END {print NR + 0}' "$metadata_relevant_all")"
    sed -n '1,128p' "$metadata_relevant_all" > "$flatpak_relevant_metadata_file"
    if [[ "$flatpak_metadata_status" != completed ]] || [[ "$metadata_relevant_rows" -gt 128 ]]; then
      record_collection_status flatpak metadata_excerpt "$APP_ID" unknown output_truncated \
        "source_status=$flatpak_metadata_status; relevant_rows=$metadata_relevant_rows; retained_limit=128"
    else
      record_collection_status flatpak metadata_excerpt "$APP_ID" observed completed \
        "relevant_rows=$metadata_relevant_rows"
    fi
    rm -f -- "$metadata_relevant_all"

    for path_key in VST_PATH VST3_PATH CLAP_PATH; do
      path_value="$(sed -n "s/^${path_key}=//p" "$flatpak_metadata_file" | head -n 1)"
      if [[ "$flatpak_metadata_status" == "completed" ]] && [[ -n "$path_value" ]]; then
        record_fact bitwig_flatpak "${path_key,,}" observed "$path_value"
      elif [[ "$flatpak_metadata_status" == "completed" ]]; then
        record_fact bitwig_flatpak "${path_key,,}" unknown "not visible in installed metadata"
      else
        record_fact bitwig_flatpak "${path_key,,}" unknown "metadata $flatpak_metadata_status"
      fi
    done

    for path_key in VST_PATH VST3_PATH CLAP_PATH; do
      effective_line="$(grep -m 1 -E "^${path_key}=" "$flatpak_permissions_file" || true)"
      effective_value="${effective_line#*=}"
      if [[ "$flatpak_permissions_status" == "completed" ]] && [[ -n "$effective_line" ]]; then
        [[ -n "$effective_value" ]] || effective_value="empty"
        record_fact bitwig_flatpak "effective_${path_key,,}" observed "$effective_value"
      elif [[ "$flatpak_permissions_status" == "completed" ]]; then
        record_fact bitwig_flatpak "effective_${path_key,,}" unknown "not visible in effective permissions"
      else
        record_fact bitwig_flatpak "effective_${path_key,,}" unknown "permissions $flatpak_permissions_status"
      fi
    done

    socket_line="$(grep -m 1 '^sockets=' "$flatpak_permissions_file" || true)"
    if [[ "$flatpak_permissions_status" == "completed" ]] && [[ -n "$socket_line" ]]; then
      record_fact bitwig_flatpak graphics_socket_posture observed "${socket_line#*=}"
    elif [[ "$flatpak_permissions_status" == "completed" ]]; then
      record_fact bitwig_flatpak graphics_socket_posture unknown "not visible in effective permissions"
    else
      record_fact bitwig_flatpak graphics_socket_posture unknown "permissions $flatpak_permissions_status"
    fi

    if [[ -n "$bitwig_runtime" ]]; then
      runtime_branch="${bitwig_runtime##*/}"
      flatpak_runtime_all_file="$(mktemp "$raw_dir/.flatpak-runtimes-all.XXXXXX")"
      bounded_text_capture "$flatpak_runtime_all_file" 65536 30 flatpak list --runtime \
        --columns=application,arch,branch,version,installation,origin
      flatpak_runtime_status="$BOUNDED_STATUS"
      flatpak_runtime_detail="$(bounded_status_detail)"
      filter_matching_flatpak_runtimes "$flatpak_runtime_all_file" "$runtime_branch" "$flatpak_runtimes_file"
      flatpak_filter_status="$FLATPAK_FILTER_STATUS"
      linux_audio_extension_count="$(awk -F '\t' '$1 ~ /^org\.freedesktop\.LinuxAudio/ {count++} END {print count + 0}' "$flatpak_runtimes_file")"
      if [[ "$flatpak_runtime_status" == completed ]] && [[ "$flatpak_filter_status" == completed ]]; then
        record_fact bitwig_flatpak runtime_census_completeness observed \
          "completed for exact branch $runtime_branch"
        if [[ "$linux_audio_extension_count" -eq 0 ]]; then
          record_fact bitwig_flatpak linux_audio_extensions_installed not_found_in_bounded_locations \
            "no installed org.freedesktop.LinuxAudio runtime/extension on exact branch $runtime_branch"
        else
          record_fact bitwig_flatpak linux_audio_extensions_installed observed \
            "$linux_audio_extension_count matching installed extension(s) on exact branch $runtime_branch"
        fi
      else
        record_fact bitwig_flatpak runtime_census_completeness unknown \
          "search_incomplete; command=$flatpak_runtime_status; filter=$flatpak_filter_status"
        if [[ "$linux_audio_extension_count" -gt 0 ]]; then
          record_fact bitwig_flatpak linux_audio_extensions_installed observed \
            "$linux_audio_extension_count retained exact-branch extension(s); census search_incomplete"
        else
          record_fact bitwig_flatpak linux_audio_extensions_installed unknown \
            "runtime census search_incomplete; matching extension absence cannot be established"
        fi
      fi
      record_collection_status flatpak runtime_census "$APP_ID" \
        "$([[ "$flatpak_runtime_status" == completed && "$flatpak_filter_status" == completed ]] && printf observed || printf unknown)" \
        "$([[ "$flatpak_runtime_status" != completed ]] && printf '%s' "$flatpak_runtime_status" || printf '%s' "$flatpak_filter_status")" \
        "$flatpak_runtime_detail; expected_branch=$runtime_branch; matching_rows=$FLATPAK_FILTER_MATCHES"
      rm -f -- "$flatpak_runtime_all_file"
    else
      record_fact bitwig_flatpak runtime_census_completeness unknown "Bitwig runtime unavailable"
      record_fact bitwig_flatpak linux_audio_extensions_installed unknown "Bitwig runtime branch unavailable"
      record_collection_status flatpak runtime_census "$APP_ID" unknown not_run runtime_branch_unavailable
    fi
  fi
  rm -f -- "$flatpak_apps_file"
fi

# D. Audio and session environment
capture_multiline_version "$compatibility_tools_file" pipewire pipewire --version
capture_multiline_version "$compatibility_tools_file" wireplumber wireplumber --version
capture_version "$compatibility_tools_file" pactl pactl --version
capture_version "$compatibility_tools_file" aplay aplay --version
if command -v wpctl >/dev/null 2>&1; then
  printf 'wpctl\tobserved\tinstalled; this build exposes no read-only version option\n' >> "$compatibility_tools_file"
else
  printf 'wpctl\tnot_installed\t\n' >> "$compatibility_tools_file"
fi
capture_multiline_version "$compatibility_tools_file" pw-cli pw-cli --version

if command -v pw-cli >/dev/null 2>&1; then
  pipewire_info_file="$(mktemp "$raw_dir/.pipewire-info.XXXXXX")"
  bounded_text_capture "$pipewire_info_file" 4096 8 pw-cli info 0
  pipewire_info_status="$BOUNDED_STATUS"
  rm -f -- "$pipewire_info_file"
  if [[ "$pipewire_info_status" == "completed" ]]; then
    record_fact audio pipewire_session_available observed true

    pipewire_nodes_file="$(mktemp "$raw_dir/.pipewire-nodes.XXXXXX")"
    bounded_text_capture "$pipewire_nodes_file" 65536 10 pw-cli ls Node
    pipewire_nodes_status="$BOUNDED_STATUS"
    for audio_class in Audio/Sink Audio/Source Stream/Output/Audio Stream/Input/Audio; do
      if [[ "$pipewire_nodes_status" == "completed" ]]; then
        class_count="$(grep -F -c "media.class = \"$audio_class\"" "$pipewire_nodes_file" || true)"
        record_fact audio "class_count_${audio_class//\//_}" observed "$class_count"
      else
        record_fact audio "class_count_${audio_class//\//_}" unknown \
          "PipeWire node census $pipewire_nodes_status"
      fi
    done
    rm -f -- "$pipewire_nodes_file"

    pipewire_devices_file="$(mktemp "$raw_dir/.pipewire-devices.XXXXXX")"
    bounded_text_capture "$pipewire_devices_file" 65536 10 pw-cli ls Device
    pipewire_devices_status="$BOUNDED_STATUS"
    if [[ "$pipewire_devices_status" == "completed" ]]; then
      device_count="$(grep -c 'type PipeWire:Interface:Device' "$pipewire_devices_file" || true)"
      record_fact audio class_count_Audio_Device observed "$device_count"
    else
      record_fact audio class_count_Audio_Device unknown "PipeWire device census $pipewire_devices_status"
    fi
    rm -f -- "$pipewire_devices_file"
  else
    record_fact audio pipewire_session_available unknown "pw-cli info $pipewire_info_status"
    for audio_class in Audio/Sink Audio/Source Audio/Device Stream/Output/Audio Stream/Input/Audio; do
      record_fact audio "class_count_${audio_class//\//_}" unknown "PipeWire session census incomplete"
    done
  fi
else
  record_fact audio pipewire_session_available unknown false
  for audio_class in Audio/Sink Audio/Source Audio/Device Stream/Output/Audio Stream/Input/Audio; do
    record_fact audio "class_count_${audio_class//\//_}" unknown ""
  done
fi

if command -v pactl >/dev/null 2>&1; then
  pactl_info_file="$(mktemp "$raw_dir/.pactl-info.XXXXXX")"
  bounded_text_capture "$pactl_info_file" 8192 8 pactl info
  pactl_info_status="$BOUNDED_STATUS"
  rm -f -- "$pactl_info_file"
  if [[ "$pactl_info_status" == "completed" ]]; then
    record_fact audio pulseaudio_compatibility_available observed true
  else
    record_fact audio pulseaudio_compatibility_available unknown "pactl info $pactl_info_status"
  fi
else
  record_fact audio pulseaudio_compatibility_available not_installed false
fi

if command -v aplay >/dev/null 2>&1; then
  record_fact audio alsa_available observed true
  aplay_list_file="$(mktemp "$raw_dir/.aplay-list.XXXXXX")"
  bounded_text_capture "$aplay_list_file" 32768 10 aplay -l
  aplay_list_status="$BOUNDED_STATUS"
  if [[ "$aplay_list_status" == "completed" ]]; then
    alsa_card_count="$(awk '/^card [0-9]+:/{count++} END{print count + 0}' "$aplay_list_file")"
    record_fact audio alsa_playback_card_count observed "$alsa_card_count"
  else
    record_fact audio alsa_playback_card_count unknown "ALSA card census $aplay_list_status"
  fi
  rm -f -- "$aplay_list_file"
else
  record_fact audio alsa_available not_installed false
  record_fact audio alsa_playback_card_count unknown ""
fi

if command -v wpctl >/dev/null 2>&1; then
  default_route_file="$(mktemp "$raw_dir/.default-route.XXXXXX")"
  bounded_text_capture "$default_route_file" 16384 8 wpctl inspect @DEFAULT_AUDIO_SINK@
  default_route_status="$BOUNDED_STATUS"
  default_route_class="$(sed -n 's/^[[:space:]]*media.class = "\([^"]*\)"/\1/p' "$default_route_file" | head -n 1)"
  default_route_api="$(sed -n 's/^[[:space:]]*device.api = "\([^"]*\)"/\1/p' "$default_route_file" | head -n 1)"
  if [[ "$default_route_status" != "completed" ]]; then
    record_fact audio default_route_class unknown "default-route inspection $default_route_status"
    record_fact audio default_route_api unknown "default-route inspection $default_route_status"
  elif [[ -n "$default_route_class" || -n "$default_route_api" ]]; then
    if [[ -n "$default_route_class" ]]; then
      record_fact audio default_route_class observed "$default_route_class"
    else
      record_fact audio default_route_class unknown "not exposed without retaining device names"
    fi
    if [[ -n "$default_route_api" ]]; then
      record_fact audio default_route_api observed "$default_route_api"
    else
      record_fact audio default_route_api unknown "not exposed without retaining device names"
    fi
  else
    record_fact audio default_route_class unknown "not exposed without retaining device names"
    record_fact audio default_route_api unknown "not exposed without retaining device names"
  fi
  rm -f -- "$default_route_file"
else
  record_fact audio default_route_class not_installed "wpctl unavailable"
  record_fact audio default_route_api not_installed "wpctl unavailable"
fi
record_fact audio default_route_device_name explicitly_out_of_scope "private device names are not retained"

# E. Development toolchain inventory
capture_version "$toolchain_file" bash bash --version
capture_version "$toolchain_file" git git --version
capture_version "$toolchain_file" python3 python3 --version
capture_version "$toolchain_file" codex codex --version
capture_version "$toolchain_file" rustc rustc -V
capture_version "$toolchain_file" cargo cargo -V
capture_version "$toolchain_file" cmake cmake --version
capture_version "$toolchain_file" ninja ninja --version
capture_version "$toolchain_file" gcc gcc --version
capture_version "$toolchain_file" g++ g++ --version
capture_version "$toolchain_file" clang clang --version
capture_version "$toolchain_file" clang++ clang++ --version
capture_version "$toolchain_file" pkg-config pkg-config --version
capture_version "$toolchain_file" flatpak flatpak --version

# F. Windows compatibility inventory
capture_version "$compatibility_tools_file" wine wine --version
capture_version "$compatibility_tools_file" wineserver wineserver --version
capture_version "$compatibility_tools_file" winetricks winetricks --version
capture_version "$compatibility_tools_file" umu-run umu-run --version
capture_version "$compatibility_tools_file" protontricks protontricks --version
capture_version "$compatibility_tools_file" yabridgectl yabridgectl --version
capture_version "$compatibility_tools_file" yabridge yabridge --version

runner_roots=(
  "$HOME/.steam/root/compatibilitytools.d"
  "$HOME/.local/share/Steam/compatibilitytools.d"
  "$HOME/.var/app/com.valvesoftware.Steam/data/Steam/compatibilitytools.d"
)
for runner_root in "${runner_roots[@]}"; do
  record_runner_root "$runner_root"
done
record_steam_common_runners "$HOME/.steam/root/steamapps/common"
record_steam_common_runners "$HOME/.local/share/Steam/steamapps/common"
record_steam_common_runners "$HOME/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/common"
record_runner_root "$HOME/.local/share/umu"

capture_direct_prefix "$HOME/.wine"
capture_child_prefixes "$HOME/.local/share/wineprefixes" direct_children
capture_child_prefixes "$HOME/.local/share/bottles/bottles" direct_children
capture_child_prefixes "$HOME/.steam/root/steamapps/compatdata" steam_compatdata
capture_child_prefixes "$HOME/.local/share/Steam/steamapps/compatdata" steam_compatdata
capture_child_prefixes "$HOME/.var/app/com.valvesoftware.Steam/data/Steam/steamapps/compatdata" steam_compatdata

if awk -F '\t' '$1 == "runner_inventory" && $4 == "unknown" {found=1} END {exit !found}' "$collection_status_file"; then
  record_fact compatibility_runtime_inventory runner_inventory_completeness unknown \
    "one or more declared runner roots were rejected or search_incomplete"
else
  record_fact compatibility_runtime_inventory runner_inventory_completeness observed \
    "all non-alias declared runner roots completed within bounds"
fi
if [[ "$prefix_roster_complete" == true ]]; then
  record_fact compatibility_runtime_inventory prefix_roster_completeness observed \
    "all non-alias declared prefix roots completed within bounds; retained_prefixes=${#discovered_prefixes[@]}"
else
  record_fact compatibility_runtime_inventory prefix_roster_completeness unknown \
    "prefix roster search_incomplete; downstream discovered-prefix artifact coverage is incomplete"
  mark_artifact_role_incomplete windows_plugin_location
fi

declare -a yabridge_validated_paths=()
if command -v yabridgectl >/dev/null 2>&1; then
  bounded_text_capture "$yabridgectl_help_file" 32768 10 yabridgectl --help
  yabridgectl_help_status="$BOUNDED_STATUS"
  if [[ "$yabridgectl_help_status" == "completed" ]] \
    && grep -Eq '(^|[[:space:]])list([[:space:]]|$)' "$yabridgectl_help_file"; then
    bounded_text_capture "$yabridgectl_list_file" 32768 15 yabridgectl list
    yabridgectl_list_status="$BOUNDED_STATUS"
    if [[ "$yabridgectl_list_status" == "completed" ]]; then
      record_fact compatibility_runtime_inventory yabridgectl_read_only_operation observed list
    else
      record_fact compatibility_runtime_inventory yabridgectl_read_only_operation unknown \
        "list operation $yabridgectl_list_status; configured-path search_incomplete"
      mark_artifact_role_incomplete yabridge_read_only_configured_location
    fi
    configured_path_rows=0
    while IFS= read -r configured_path; do
      configured_path="${configured_path#"${configured_path%%[![:space:]]*}"}"
      configured_path="${configured_path%"${configured_path##*[![:space:]]}"}"
      [[ -n "$configured_path" ]] || continue
      configured_path_rows=$((configured_path_rows + 1))
      if [[ "$configured_path_rows" -gt 20 ]]; then
        mark_artifact_role_incomplete yabridge_read_only_configured_location
        continue
      fi
      configured_path="${configured_path/#\~/$HOME}"
      validate_home_root "$configured_path"
      if [[ "$ROOT_VALIDATION_STATUS" == "valid_existing" ]] && [[ -d "$configured_path" ]]; then
        yabridge_validated_paths+=("$configured_path")
        record_collection_status yabridge_config configured_path "$configured_path" observed completed \
          canonical_home_containment_proved
      elif [[ "$ROOT_VALIDATION_STATUS" == "valid_absent" ]]; then
        record_collection_status yabridge_config configured_path "$configured_path" observed completed path_absent
      else
        mark_artifact_role_incomplete yabridge_read_only_configured_location
        record_collection_status yabridge_config configured_path '<rejected-configured-path>' unknown root_rejected \
          "$ROOT_VALIDATION_DETAIL"
      fi
    done < "$yabridgectl_list_file"
    if [[ "$configured_path_rows" -gt 20 ]]; then
      record_collection_status yabridge_config configured_path '<additional-configured-paths>' unknown row_truncated \
        "retained_path_limit=20; rows_seen=$configured_path_rows"
    fi
  else
    record_fact compatibility_runtime_inventory yabridgectl_read_only_operation unknown \
      "help $yabridgectl_help_status or did not demonstrate a list command; no list operation invoked"
    mark_artifact_role_incomplete yabridge_read_only_configured_location
    record_collection_status yabridge_config help '<command>' unknown "$yabridgectl_help_status" \
      "list operation not demonstrably available"
  fi
else
  record_fact compatibility_runtime_inventory yabridgectl_read_only_operation not_installed ""
fi

# G. Existing relevant plug-in and Serum/Xfer state
record_fact existing_plugin_state bounded_installer_root observed "$HOME/.local/share/linux-vst-bridge-fixtures/serum2 depth=3"
record_fact existing_plugin_state bounded_download_root observed "$HOME/Downloads depth=2 names=Serum|Xfer"
record_fact existing_plugin_state bounded_linux_plugin_roots observed "$HOME/.vst3;$HOME/.vst;$HOME/.clap depth=5 names=Serum|Xfer"
record_fact existing_plugin_state bounded_prefix_policy observed "declared roots; immediate prefixes; known Windows VST directories only"

search_artifact_root installer_location "$HOME/.local/share/linux-vst-bridge-fixtures/serum2" 3
search_artifact_root installer_location "$HOME/Downloads" 2
search_artifact_root linux_plugin_location "$HOME/.vst3" 5
search_artifact_root linux_plugin_location "$HOME/.vst" 5
search_artifact_root linux_plugin_location "$HOME/.clap" 5

for prefix in "${discovered_prefixes[@]}"; do
  windows_plugin_roots=(
    "$prefix/drive_c/Program Files/Common Files/VST3"
    "$prefix/drive_c/Program Files/VstPlugins"
    "$prefix/drive_c/Program Files/Steinberg/VstPlugins"
    "$prefix/drive_c/Program Files/Common Files/CLAP"
    "$prefix/drive_c/Program Files (x86)/Common Files/VST3"
  )
  for windows_plugin_root in "${windows_plugin_roots[@]}"; do
    search_artifact_root windows_plugin_location "$windows_plugin_root" 5
  done
done

for yabridge_path in "${yabridge_validated_paths[@]}"; do
  search_artifact_root yabridge_read_only_configured_location "$yabridge_path" 5
done

record_serum_results
record_fact serum2 license_channel operator_input_required "Xfer direct/owned, Splice paid-off lifetime/Xfer, active Splice Rent-to-Own, or another exact channel"
record_fact serum2 credentials explicitly_out_of_scope "vendor credentials are never collected"
record_fact serum2 installer_launch explicitly_out_of_scope "belongs to a separately authorized slice"
record_fact serum2 authorization explicitly_out_of_scope "belongs to a separately authorized slice"

# H. Current relevant process state, using exact executable-name queries only.
capture_process_group Bitwig BitwigStudio bitwig-studio
capture_process_group Wine wine wine64 wine-preloader wine64-preloader
capture_process_group wineserver wineserver wineserver64
capture_process_group Proton_UMU proton proton-wrapper umu umu-run pressure-vessel pv-bwrap
capture_process_group yabridge_hosts yabridge-host yabridge-host.exe yabridge-host-32.exe
capture_process_group Steam steam steamwebhelper

record_fact claim primary observed "Establish a reproducible, sanitized, exact baseline of the current Steam Deck + Bitwig Flatpak fixture and its existing Windows-audio compatibility state, including the observed presence or absence of Serum 2 artifacts, without mutating commercial software, compatibility environments, DAW configuration, or SteamOS."
record_fact claim ceiling explicitly_out_of_scope "No claim that Serum 2 installs, authorizes, scans, opens, processes audio, appears in Bitwig, or interoperates with Proton or a bridge."
record_fact claim gui_next_step gui_session_required "Any later installer or authorization work requires an operator-controlled graphical session."

"$script_dir/sanitize.sh" "$raw_dir" "$output_dir"

if git -C "$repo_root" ls-files -- "evidence/raw/$SR0_NAME" | grep -q .; then
  printf 'raw evidence became tracked; refusing successful completion\n' >&2
  exit 2
fi

printf 'SR0 sanitized evidence written to %s\n' "$output_dir"
