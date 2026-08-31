#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=common.sh
source "$script_dir/common.sh"

usage() {
    printf 'usage: %s publish|inspect|remove [--bundle PATH] [--publication-root PATH] [--receipt PATH] [--test-mode --test-root PATH] [--simulate-after-stage] [--simulate-after-swap] [--simulate-receipt-failure] [--simulate-after-receipt-commit]\n' "${0##*/}" >&2
}

action="${1:-}"
[[ "$action" == publish || "$action" == inspect || "$action" == remove ]] || { usage; exit 2; }
shift

bundle=""
publication_root="$(hp0_default_publication_root)"
receipt=""
test_mode=false
test_root=""
simulate_after_stage=false
simulate_after_swap=false
simulate_receipt_failure=false
simulate_after_receipt_commit=false
while (($#)); do
    case "$1" in
        --bundle) bundle="${2:-}"; shift 2 ;;
        --publication-root) publication_root="${2:-}"; shift 2 ;;
        --receipt) receipt="${2:-}"; shift 2 ;;
        --test-mode) test_mode=true; shift ;;
        --test-root) test_root="${2:-}"; shift 2 ;;
        --simulate-after-stage) simulate_after_stage=true; shift ;;
        --simulate-after-swap) simulate_after_swap=true; shift ;;
        --simulate-receipt-failure) simulate_receipt_failure=true; shift ;;
        --simulate-after-receipt-commit) simulate_after_receipt_commit=true; shift ;;
        *) usage; exit 2 ;;
    esac
done

default_root="$(hp0_default_publication_root)"
if "$test_mode"; then
    [[ -n "$test_root" ]] || hp0_die "test publication requires an explicit --test-root"
    test_root="$(hp0_require_test_root "$test_root")"
    publication_root="$(hp0_require_test_descendant "$test_root" "$publication_root" directory-or-absent "test publication root")"
    if [[ "$action" == publish ]]; then
        [[ -n "$receipt" ]] || hp0_die "test publication requires an explicit alternate receipt path"
        receipt="$(hp0_require_test_descendant "$test_root" "$receipt" regular-file-or-absent "test receipt")"
    elif [[ -n "$receipt" ]]; then
        receipt="$(hp0_require_test_descendant "$test_root" "$receipt" regular-file-or-absent "test receipt")"
    fi
else
    [[ -z "$test_root" ]] || hp0_die "--test-root requires --test-mode"
    [[ "$publication_root" == "$default_root" ]] || hp0_die "non-default publication root requires --test-mode"
    [[ -n "$receipt" ]] || receipt="$(hp0_default_receipt_path)"
    [[ "$receipt" == "$(hp0_default_receipt_path)" ]] ||
        hp0_die "ordinary receipt path must equal the declared HP0 project receipt path"
fi
if "$simulate_after_stage" || "$simulate_after_swap" || "$simulate_receipt_failure" ||
   "$simulate_after_receipt_commit"; then
    "$test_mode" || hp0_die "simulated publication failures are available only in --test-mode"
fi

destination="$publication_root/$HP0_BUNDLE_NAME"

inspect_owned_bundle() {
    local owned_bundle="$1"
    hp0_require_bundle_shape "$owned_bundle"
    local owner="$owned_bundle/$HP0_OWNER_RELATIVE"
    local manifest="$owned_bundle/$HP0_MANIFEST_RELATIVE"
    [[ -f "$owner" && ! -L "$owner" ]] || hp0_die "publication lacks the HP0 ownership marker"
    [[ -f "$manifest" && ! -L "$manifest" ]] || hp0_die "publication lacks the HP0 source manifest"
    grep -Fxq 'schema=linux-vst-bridge-hp0-publication-owner/v1' "$owner" ||
        hp0_die "publication ownership marker schema is unknown"
    grep -Fxq "bundle=$HP0_BUNDLE_NAME" "$owner" || hp0_die "publication ownership marker names another bundle"
    grep -Fxq "processor_class_id=$HP0_PROCESSOR_CLASS_ID" "$owner" || hp0_die "publication processor ID is not owned by HP0"
    grep -Fxq "controller_class_id=$HP0_CONTROLLER_CLASS_ID" "$owner" || hp0_die "publication controller ID is not owned by HP0"

    if find "$owned_bundle" -mindepth 1 -maxdepth 8 ! -type d ! -type f -print -quit | grep -q .; then
        hp0_die "published bundle contains an unsupported file type"
    fi

    local verify_log
    verify_log="$(mktemp "${TMPDIR:-/tmp}/hp0-manifest-check.XXXXXX")"
    if ! (cd "$owned_bundle" && sha256sum -c "$HP0_MANIFEST_RELATIVE") >"$verify_log" 2>&1; then
        sed -n '1,40p' "$verify_log" >&2
        rm -f -- "$verify_log"
        hp0_die "published source manifest verification failed"
    fi
    rm -f -- "$verify_log"

    local retained_list actual_list
    retained_list="$(mktemp "${TMPDIR:-/tmp}/hp0-retained-list.XXXXXX")"
    actual_list="$(mktemp "${TMPDIR:-/tmp}/hp0-actual-list.XXXXXX")"
    awk '{sub(/^\*/, "", $2); print $2}' "$manifest" | LC_ALL=C sort >"$retained_list"
    (
        cd "$owned_bundle"
        find . -mindepth 1 -maxdepth 8 -type f \
            ! -path "./$HP0_OWNER_RELATIVE" ! -path "./$HP0_MANIFEST_RELATIVE" -print |
            LC_ALL=C sort
    ) >"$actual_list"
    if ! cmp -s "$retained_list" "$actual_list"; then
        rm -f -- "$retained_list" "$actual_list"
        hp0_die "published bundle file roster differs from its source manifest"
    fi
    rm -f -- "$retained_list" "$actual_list"

    printf 'publication_status=verified\n'
    printf 'bundle=%s\n' "$owned_bundle"
    printf 'module_sha256=%s\n' "$(sha256sum "$owned_bundle/$HP0_MODULE_RELATIVE" | awk '{print $1}')"
    printf 'source_manifest_sha256=%s\n' "$(sha256sum "$manifest" | awk '{print $1}')"
}

require_owned_receipt() {
    local receipt_path="$1"
    local owned_bundle="$2"
    local expected_keys observed_keys transaction_id owner_transaction_id
    local source_module_sha published_module_sha source_manifest_sha published_manifest_sha tree_sha
    local current_tree_manifest

    [[ -f "$receipt_path" && ! -L "$receipt_path" ]] ||
        hp0_die "publication receipt is missing, non-regular, or a symlink"
    if ! awk 'NF == 0 || $0 !~ /^[A-Za-z0-9_]+=[^[:cntrl:]]*$/ { exit 1 }' "$receipt_path"; then
        hp0_die "publication receipt contains a malformed line"
    fi
    expected_keys=$'bundle\ncontroller_class_id\nprocessor_class_id\npublication_status\npublished_manifest_sha256\npublished_module_sha256\npublished_tree_sha256\nschema\nsource_manifest_sha256\nsource_module_sha256\ntransaction_id'
    observed_keys="$(cut -d= -f1 "$receipt_path" | LC_ALL=C sort)"
    [[ "$observed_keys" == "$expected_keys" ]] ||
        hp0_die "publication receipt key set differs from the accepted schema"

    [[ "$(hp0_read_kv_value "$receipt_path" schema)" == "$HP0_PUBLICATION_RECEIPT_SCHEMA" ]] ||
        hp0_die "publication receipt schema is unknown"
    [[ "$(hp0_read_kv_value "$receipt_path" bundle)" == "$HP0_BUNDLE_NAME" ]] ||
        hp0_die "publication receipt names another bundle"
    [[ "$(hp0_read_kv_value "$receipt_path" processor_class_id)" == "$HP0_PROCESSOR_CLASS_ID" ]] ||
        hp0_die "publication receipt processor ID is not owned by HP0"
    [[ "$(hp0_read_kv_value "$receipt_path" controller_class_id)" == "$HP0_CONTROLLER_CLASS_ID" ]] ||
        hp0_die "publication receipt controller ID is not owned by HP0"
    [[ "$(hp0_read_kv_value "$receipt_path" publication_status)" == verified_owned_copy ]] ||
        hp0_die "publication receipt does not record a verified owned copy"
    transaction_id="$(hp0_read_kv_value "$receipt_path" transaction_id)"
    [[ "$transaction_id" =~ ^\.hp0-stage\.[A-Za-z0-9]{6,}$ ]] ||
        hp0_die "publication receipt transaction identifier is malformed"

    inspect_owned_bundle "$owned_bundle" >/dev/null
    owner_transaction_id="$(hp0_read_kv_value "$owned_bundle/$HP0_OWNER_RELATIVE" transaction_id)"
    [[ "$owner_transaction_id" == "$transaction_id" ]] ||
        hp0_die "publication receipt transaction does not own the current bundle"

    source_module_sha="$(hp0_read_kv_value "$receipt_path" source_module_sha256)"
    published_module_sha="$(hp0_read_kv_value "$receipt_path" published_module_sha256)"
    source_manifest_sha="$(hp0_read_kv_value "$receipt_path" source_manifest_sha256)"
    published_manifest_sha="$(hp0_read_kv_value "$receipt_path" published_manifest_sha256)"
    tree_sha="$(hp0_read_kv_value "$receipt_path" published_tree_sha256)"
    [[ "$source_module_sha" =~ ^[0-9a-f]{64}$ && "$published_module_sha" =~ ^[0-9a-f]{64}$ &&
       "$source_manifest_sha" =~ ^[0-9a-f]{64}$ && "$published_manifest_sha" =~ ^[0-9a-f]{64}$ &&
       "$tree_sha" =~ ^[0-9a-f]{64}$ ]] || hp0_die "publication receipt contains a malformed SHA-256"
    [[ "$source_module_sha" == "$published_module_sha" ]] || hp0_die "publication receipt module hashes differ"
    [[ "$source_manifest_sha" == "$published_manifest_sha" ]] || hp0_die "publication receipt manifest hashes differ"
    [[ "$(sha256sum "$owned_bundle/$HP0_MODULE_RELATIVE" | awk '{print $1}')" == "$published_module_sha" ]] ||
        hp0_die "publication receipt module identity differs from the owned bundle"
    [[ "$(sha256sum "$owned_bundle/$HP0_MANIFEST_RELATIVE" | awk '{print $1}')" == "$published_manifest_sha" ]] ||
        hp0_die "publication receipt manifest identity differs from the owned bundle"
    current_tree_manifest="$(mktemp "${TMPDIR:-/tmp}/hp0-receipt-tree.XXXXXX")"
    hp0_tree_manifest "$owned_bundle" "$current_tree_manifest"
    if [[ "$(sha256sum "$current_tree_manifest" | awk '{print $1}')" != "$tree_sha" ]]; then
        rm -f -- "$current_tree_manifest"
        hp0_die "publication receipt complete-tree identity differs from the owned bundle"
    fi
    rm -f -- "$current_tree_manifest"

    printf 'receipt_status=verified_owned_receipt\n'
    printf 'receipt_schema=%s\n' "$HP0_PUBLICATION_RECEIPT_SCHEMA"
    printf 'receipt_sha256=%s\n' "$(sha256sum "$receipt_path" | awk '{print $1}')"
}

bundle_has_transaction_id() {
    local owned_bundle="$1"
    local transaction_id="$2"
    local owner="$owned_bundle/$HP0_OWNER_RELATIVE"
    [[ -d "$owned_bundle" && ! -L "$owned_bundle" && -f "$owner" && ! -L "$owner" ]] &&
        grep -Fxq "transaction_id=$transaction_id" "$owner"
}

receipt_has_transaction_id() {
    local receipt_path="$1"
    local transaction_id="$2"
    [[ -f "$receipt_path" && ! -L "$receipt_path" ]] &&
        grep -Fxq "transaction_id=$transaction_id" "$receipt_path"
}

hp0_assert_no_bitwig

case "$action" in
    inspect)
        [[ -d "$destination" ]] || hp0_die "owned publication is missing: $destination"
        inspect_owned_bundle "$destination"
        if ! "$test_mode"; then
            hp0_require_no_symlink_ancestors "$receipt" regular-file-or-absent "HP0 publication receipt"
            require_owned_receipt "$receipt" "$destination"
        elif [[ -n "$receipt" ]]; then
            require_owned_receipt "$receipt" "$destination"
        fi
        ;;
    remove)
        inspect_owned_bundle "$destination" >/dev/null
        real_home="$(hp0_real_home)"
        cache_root="${XDG_CACHE_HOME:-$real_home/.cache}/linux-vst-bridge/hp0-removed"
        mkdir -p "$cache_root"
        removal_destination="$cache_root/LabHostProbe.vst3.$(date -u +%Y%m%dT%H%M%SZ).$$"
        [[ ! -e "$removal_destination" ]] || hp0_die "safe removal destination already exists"
        mv -T "$destination" "$removal_destination"
        printf 'publication_status=removed_to_recoverable_cache\n'
        printf 'removed_bundle=%s\n' "$removal_destination"
        ;;
    publish)
        [[ -n "$bundle" ]] || hp0_die "publish requires --bundle"
        [[ "$bundle" == /* ]] || hp0_die "source bundle path must be absolute"
        hp0_require_bundle_shape "$bundle"

        if [[ -e "$destination" ]]; then
            [[ -d "$destination" && ! -L "$destination" ]] || hp0_die "refusing unknown publication destination"
            if ! (inspect_owned_bundle "$destination" >/dev/null 2>&1); then
                hp0_die "refusing unknown publication destination"
            fi
        fi

        if "$test_mode"; then
            hp0_prepare_test_file_parent "$test_root" "$receipt" "test receipt"
        else
            hp0_prepare_normal_receipt_path "$receipt"
        fi
        case "$receipt" in
            "$destination"|"$destination"/*) hp0_die "receipt must remain outside the published bundle" ;;
        esac

        if [[ -e "$receipt" ]]; then
            [[ -e "$destination" ]] || hp0_die "refusing unknown receipt destination"
            if ! (require_owned_receipt "$receipt" "$destination" >/dev/null 2>&1); then
                hp0_die "refusing unknown receipt destination"
            fi
        fi

        if "$test_mode"; then
            hp0_prepare_test_directory "$test_root" "$publication_root" "test publication root"
        else
            mkdir -p "$publication_root"
        fi
        stage_parent="$(mktemp -d "$publication_root/.hp0-stage.XXXXXX")"
        stage="$stage_parent/$HP0_BUNDLE_NAME"
        transaction_id="${stage_parent##*/}"
        previous_publication=false
        previous_moved=false
        replacement_installed=false
        transaction_committed=false
        backup=""
        receipt_stage=""
        receipt_backup=""
        receipt_replaced=false
        prior_receipt_present=false
        prior_receipt_sha256=""
        prior_complete_manifest="$stage_parent/prior-complete.sha256"
        replacement_complete_manifest="$stage_parent/replacement-complete.sha256"
        source_manifest="$stage_parent/source.sha256"
        destination_manifest="$stage_parent/destination.sha256"
        expected_receipt="$stage_parent/expected-publication.receipt"
        final_manifest="$stage_parent/final-complete.sha256"

        cleanup_publish() {
            local original_status="$1"
            local rollback_failed=0
            local rollback_manifest="${stage_parent:-}/rollback-complete.sha256"
            trap - EXIT HUP INT TERM
            set +e

            if ! "$transaction_committed"; then
                if "$replacement_installed" && [[ -e "$destination" ]]; then
                    if bundle_has_transaction_id "$destination" "$transaction_id"; then
                        rm -rf -- "$destination" || rollback_failed=1
                    else
                        printf 'HP0_ROLLBACK_ERROR: refusing to remove a destination not owned by this transaction\n' >&2
                        rollback_failed=1
                    fi
                fi

                if "$previous_moved"; then
                    if [[ -e "$destination" ]]; then
                        printf 'HP0_ROLLBACK_ERROR: destination remained occupied before prior-publication restore\n' >&2
                        rollback_failed=1
                    elif [[ -d "$backup" ]] && mv -T "$backup" "$destination"; then
                        backup=""
                        if (inspect_owned_bundle "$destination" >/dev/null &&
                            hp0_tree_manifest "$destination" "$rollback_manifest") &&
                           cmp -s "$prior_complete_manifest" "$rollback_manifest"; then
                            :
                        else
                            printf 'HP0_ROLLBACK_ERROR: restored publication differs from the complete pre-transaction roster or hashes\n' >&2
                            rollback_failed=1
                        fi
                    else
                        printf 'HP0_ROLLBACK_ERROR: exact prior publication could not be restored\n' >&2
                        rollback_failed=1
                    fi
                elif "$previous_publication"; then
                    if (inspect_owned_bundle "$destination" >/dev/null &&
                        hp0_tree_manifest "$destination" "$rollback_manifest") &&
                       cmp -s "$prior_complete_manifest" "$rollback_manifest"; then
                        :
                    else
                        printf 'HP0_ROLLBACK_ERROR: unswapped prior publication changed during the failed transaction\n' >&2
                        rollback_failed=1
                    fi
                elif [[ -e "$destination" ]]; then
                    printf 'HP0_ROLLBACK_ERROR: failed first publication did not restore absence\n' >&2
                    rollback_failed=1
                fi

                if "$receipt_replaced"; then
                    if [[ -e "$receipt" ]]; then
                        if receipt_has_transaction_id "$receipt" "$transaction_id"; then
                            rm -f -- "$receipt" || rollback_failed=1
                        else
                            printf 'HP0_ROLLBACK_ERROR: refusing to remove a receipt not owned by this transaction\n' >&2
                            rollback_failed=1
                        fi
                    fi
                    if "$prior_receipt_present"; then
                        if [[ ! -e "$receipt" && -f "$receipt_backup" ]] && mv -T "$receipt_backup" "$receipt"; then
                            receipt_backup=""
                            if [[ "$(sha256sum "$receipt" | awk '{print $1}')" != "$prior_receipt_sha256" ]]; then
                                printf 'HP0_ROLLBACK_ERROR: prior receipt bytes were not restored\n' >&2
                                rollback_failed=1
                            fi
                        else
                            printf 'HP0_ROLLBACK_ERROR: prior receipt could not be restored\n' >&2
                            rollback_failed=1
                        fi
                    elif [[ -e "$receipt" ]]; then
                        printf 'HP0_ROLLBACK_ERROR: failed first receipt commit did not restore absence\n' >&2
                        rollback_failed=1
                    fi
                elif [[ -n "$receipt_backup" && -e "$receipt_backup" ]]; then
                    if "$prior_receipt_present" && [[ -f "$receipt" ]] &&
                       [[ "$(sha256sum "$receipt" | awk '{print $1}')" == "$prior_receipt_sha256" ]]; then
                        rm -f -- "$receipt_backup" || rollback_failed=1
                        receipt_backup=""
                    else
                        printf 'HP0_ROLLBACK_ERROR: prior receipt changed before atomic replacement\n' >&2
                        rollback_failed=1
                    fi
                fi
            fi

            [[ -z "${receipt_stage:-}" || ! -e "$receipt_stage" ]] || rm -f -- "$receipt_stage"
            if ((rollback_failed == 0)) || "$transaction_committed"; then
                [[ -z "${stage_parent:-}" || ! -d "$stage_parent" ]] || rm -rf -- "$stage_parent"
            fi
            if "$transaction_committed"; then
                [[ -z "${backup:-}" || ! -e "$backup" ]] || rm -rf -- "$backup"
                [[ -z "${receipt_backup:-}" || ! -e "$receipt_backup" ]] || rm -f -- "$receipt_backup"
            fi
            if ((rollback_failed != 0)); then
                printf 'HP0_ROLLBACK_ERROR: publication rollback did not complete cleanly\n' >&2
            fi
            exit "$original_status"
        }
        trap 'cleanup_publish "$?"' EXIT
        trap 'exit 129' HUP
        trap 'exit 130' INT
        trap 'exit 143' TERM

        if [[ -e "$destination" ]]; then
            previous_publication=true
            hp0_tree_manifest "$destination" "$prior_complete_manifest"
        fi

        mkdir "$stage"
        cp -a --no-preserve=ownership "$bundle/." "$stage/"
        mkdir -p "$stage/Contents/Resources"
        hp0_source_tree_manifest "$bundle" "$stage/$HP0_MANIFEST_RELATIVE"
        cat >"$stage/$HP0_OWNER_RELATIVE" <<EOF
schema=linux-vst-bridge-hp0-publication-owner/v1
bundle=$HP0_BUNDLE_NAME
processor_class_id=$HP0_PROCESSOR_CLASS_ID
controller_class_id=$HP0_CONTROLLER_CLASS_ID
transaction_id=$transaction_id
EOF
        inspect_owned_bundle "$stage" >/dev/null
        hp0_tree_manifest "$stage" "$replacement_complete_manifest"

        if "$simulate_after_stage"; then
            hp0_die "simulated interruption after verified staging"
        fi

        if "$previous_publication"; then
            backup="$publication_root/.LabHostProbe.vst3.previous.$transaction_id"
            [[ ! -e "$backup" ]] || hp0_die "publication backup path already exists"
            mv -T "$destination" "$backup"
            previous_moved=true
        fi
        mv -T "$stage" "$destination"
        stage=""
        replacement_installed=true

        if "$simulate_after_swap"; then
            hp0_die "simulated failure after destination swap"
        fi

        inspect_owned_bundle "$destination" >/dev/null
        bundle_has_transaction_id "$destination" "$transaction_id" || hp0_die "published replacement lost its transaction identity"
        hp0_tree_manifest "$destination" "$destination_manifest"
        cmp -s "$replacement_complete_manifest" "$destination_manifest" ||
            hp0_die "published replacement differs from the complete staged roster or hashes"
        hp0_source_tree_manifest "$bundle" "$source_manifest"
        published_manifest="$destination/$HP0_MANIFEST_RELATIVE"
        cmp -s "$source_manifest" "$published_manifest" || hp0_die "source-to-publication manifest mismatch"

        cat >"$expected_receipt" <<EOF
schema=$HP0_PUBLICATION_RECEIPT_SCHEMA
transaction_id=$transaction_id
bundle=$HP0_BUNDLE_NAME
source_module_sha256=$(sha256sum "$bundle/$HP0_MODULE_RELATIVE" | awk '{print $1}')
published_module_sha256=$(sha256sum "$destination/$HP0_MODULE_RELATIVE" | awk '{print $1}')
source_manifest_sha256=$(sha256sum "$source_manifest" | awk '{print $1}')
published_manifest_sha256=$(sha256sum "$published_manifest" | awk '{print $1}')
published_tree_sha256=$(sha256sum "$destination_manifest" | awk '{print $1}')
processor_class_id=$HP0_PROCESSOR_CLASS_ID
controller_class_id=$HP0_CONTROLLER_CLASS_ID
publication_status=verified_owned_copy
EOF
        require_owned_receipt "$expected_receipt" "$destination" >/dev/null

        if "$simulate_receipt_failure"; then
            hp0_die "simulated receipt staging failure"
        fi

        receipt_parent="$(dirname "$receipt")"
        receipt_stage="$(mktemp "$receipt_parent/.hp0-receipt-stage.XXXXXX")"
        chmod 0600 "$receipt_stage"
        cp "$expected_receipt" "$receipt_stage"
        cmp -s "$expected_receipt" "$receipt_stage" || hp0_die "staged receipt content validation failed"

        if [[ -e "$receipt" ]]; then
            prior_receipt_present=true
            prior_receipt_sha256="$(sha256sum "$receipt" | awk '{print $1}')"
            receipt_backup="$receipt_parent/.hp0-receipt-previous.$transaction_id"
            [[ ! -e "$receipt_backup" ]] || hp0_die "receipt backup path already exists"
            cp -p -- "$receipt" "$receipt_backup"
        fi
        mv -T "$receipt_stage" "$receipt"
        receipt_stage=""
        receipt_replaced=true
        require_owned_receipt "$receipt" "$destination" >/dev/null
        receipt_has_transaction_id "$receipt" "$transaction_id" || hp0_die "committed receipt lost its transaction identity"
        cmp -s "$expected_receipt" "$receipt" || hp0_die "atomic receipt readback differs from staged content"

        if "$simulate_after_receipt_commit"; then
            hp0_die "simulated failure after atomic receipt commit"
        fi

        final_inspection="$(inspect_owned_bundle "$destination")"
        bundle_has_transaction_id "$destination" "$transaction_id" || hp0_die "final publication readback lost transaction identity"
        hp0_tree_manifest "$destination" "$final_manifest"
        cmp -s "$replacement_complete_manifest" "$final_manifest" ||
            hp0_die "final publication readback differs from the verified staged copy"
        cmp -s "$source_manifest" "$destination/$HP0_MANIFEST_RELATIVE" ||
            hp0_die "final source-to-publication readback failed"
        cmp -s "$expected_receipt" "$receipt" || hp0_die "final receipt readback failed"

        transaction_committed=true
        if [[ -n "$backup" ]]; then
            rm -rf -- "$backup"
            backup=""
        fi
        if [[ -n "$receipt_backup" ]]; then
            rm -f -- "$receipt_backup"
            receipt_backup=""
        fi
        rm -rf -- "$stage_parent"
        stage_parent=""
        trap - EXIT HUP INT TERM

        printf '%s\n' "$final_inspection"
        printf 'transaction_status=committed\n'
        printf 'receipt_schema=%s\n' "$HP0_PUBLICATION_RECEIPT_SCHEMA"
        printf 'receipt=%s\n' "$receipt"
        ;;
esac

hp0_assert_no_bitwig
