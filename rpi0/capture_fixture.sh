#!/bin/sh
set -eu
read_file() { if [ -r "$1" ]; then tr '\000' ' ' < "$1"; else printf 'unavailable'; fi; }
run_optional() { if command -v "$1" >/dev/null 2>&1; then "$@" 2>&1; else printf 'unavailable\n'; fi; }
printf 'schema=linux-vst-bridge-rpi0-fixture/v1\n'
printf 'board='; read_file /proc/device-tree/model; printf '\n'
printf 'revision='; if [ -r /proc/cpuinfo ]; then awk -F: '/^Revision/{gsub(/^[ \t]+/,"",$2);print $2}' /proc/cpuinfo; else printf 'unavailable\n'; fi
printf 'cpu='; if [ -r /proc/cpuinfo ]; then awk -F: '/^(model name|Model)/{gsub(/^[ \t]+/,"",$2);print $2;exit}' /proc/cpuinfo; else printf 'unavailable\n'; fi
printf 'isa='; if [ -r /proc/cpuinfo ]; then awk -F: '/^Features/{gsub(/^[ \t]+/,"",$2);print $2;exit}' /proc/cpuinfo; else printf 'unavailable\n'; fi
printf 'ram='; if [ -r /proc/meminfo ]; then awk '/^MemTotal/{print $2" "$3}' /proc/meminfo; else printf 'unavailable\n'; fi
printf 'os='; read_file /etc/os-release; printf '\n'
printf 'kernel='; uname -a
printf 'page_size='; getconf PAGESIZE
printf 'firmware='; run_optional vcgencmd version
printf 'root_storage='; run_optional findmnt -no SOURCE,FSTYPE,OPTIONS /
printf 'jack='; run_optional jackd --version
printf 'pipewire='; run_optional pipewire --version
printf 'display_server=%s\n' "${XDG_SESSION_TYPE:-unavailable}"
printf 'desktop=%s\n' "${XDG_CURRENT_DESKTOP:-unavailable}"
printf 'cooling='; for item in /sys/class/thermal/cooling_device*/type; do [ -e "$item" ] && { read_file "$item"; printf ','; }; done; printf '\n'
printf 'temperature='; run_optional vcgencmd measure_temp
printf 'throttled='; run_optional vcgencmd get_throttled
printf 'governor='; read_file /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor; printf '\n'
printf 'clocks_khz='; for item in /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq; do [ -e "$item" ] && { read_file "$item"; printf ','; }; done; printf '\n'
