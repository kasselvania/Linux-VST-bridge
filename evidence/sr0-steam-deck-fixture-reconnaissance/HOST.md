# SR0 host fixture

| Fact | Classification | Value |
|---|---|---|
| OS name | `observed` | SteamOS |
| OS release | `observed` | SteamOS |
| OS version | `observed` | 3.8.16 |
| OS build | `observed` | 20260716.1 |
| OS variant | `observed` | Steam Deck variant (literal VARIANT_ID withheld because it is identical to the hostname) |
| Kernel | `observed` | Linux 6.16.12-valve24.5-1-neptune-616-gb2f7cfe85e45 #1 SMP PREEMPT_DYNAMIC Wed, 15 Jul 2026 21:48:50 +0000 x86_64 GNU/Linux |
| Architecture | `observed` | x86_64 |
| Hardware model | `observed` | Galileo |
| CPU model | `observed` | AMD Custom APU 0932 |
| Logical cores | `observed` | 8 |
| Total memory (KiB) | `observed` | 15160360 |
| SteamOS read-only status | `observed` | enabled |
| Graphical session type | `observed` | wayland |
| Desktop | `observed` | KDE |
| Session census completeness | `observed` | loginctl session census completed within byte/time/row bounds |
| DISPLAY present in capture shell | `observed` | false |
| WAYLAND_DISPLAY present in capture shell | `observed` | false |
| Display socket values | `explicitly_out_of_scope` | presence only; socket values not retained |
| Filesystem summary completeness | `observed` | completed; rows=7 |

## Local block-device filesystem capacity

Only the requested columns are retained. External media labels beneath `/run/media` are replaced.

```text
Filesystem     Type         1B-blocks         Used        Avail Use% Mounted on
/dev/nvme0n1p4 btrfs       5368709120   3717763072    908554240  81% /
/dev/nvme0n1p6 ext4         241081344     43336704    180128768  20% /var
/dev/nvme0n1p2 vfat          33470464       743424     32727040   3% /efi
/dev/nvme0n1p1 vfat          66959360      1822720     65136640   3% /esp
/dev/nvme0n1p8 ext4     1007626104832 249767256064 757842071552  25% /home
/dev/mmcblk0p1 ext4     1006644932608 833671413760 172956741632  83% /run/media/<USER>/<EXTERNAL_MOUNT>
```
