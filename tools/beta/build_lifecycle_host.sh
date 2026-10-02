#!/bin/sh
# Maintainer instrumentation only; never part of the customer's prerequisites.
set -eu
test "$#" -eq 3 || { echo 'usage: build_lifecycle_host.sh PINNED_SDK SDK_LIB_DIRECTORY OUTPUT_DIRECTORY' >&2; exit 2; }
lvb_sdk=$1
lvb_sdk_libs=$2
lvb_output=$3
lvb_sources=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
test "$(git -C "$lvb_sdk" rev-parse HEAD)" = 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
mkdir -p "$lvb_output"
g++ -std=c++20 -O2 -DNDEBUG -Wall -Wextra -Wpedantic -Werror \
    -I "$lvb_sdk" "$lvb_sources/tools/beta/lifecycle_host.cpp" \
    "$lvb_sdk/public.sdk/source/vst/hosting/module_linux.cpp" \
    -Wl,--start-group "$lvb_sdk_libs/libsdk_hosting.a" "$lvb_sdk_libs/libsdk.a" \
    "$lvb_sdk_libs/libsdk_common.a" "$lvb_sdk_libs/libbase.a" "$lvb_sdk_libs/libpluginterfaces.a" \
    -Wl,--end-group -pthread -ldl -o "$lvb_output/lifecycle-host"
g++ -std=c++20 -O2 -shared -fPIC -Wall -Wextra -Wpedantic -Werror \
    "$lvb_sources/native-vst3-proxy/tests/callback_audit.cpp" -ldl \
    -o "$lvb_output/libap3-callback-audit.so"
sha256sum "$lvb_output/lifecycle-host" "$lvb_output/libap3-callback-audit.so" \
    "$lvb_sources/tools/beta/lifecycle_host.cpp"
