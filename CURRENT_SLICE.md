# AP12 accepted — installed Arturia workflow

[PR #79](https://github.com/kasselvania/Linux-VST-bridge/pull/79) at reviewed implementation `66430ac40d5a398ea2a0943b330b1056dc7ecb03` is accepted as a bounded engineering preview by [technical review 5149097123](https://github.com/kasselvania/Linux-VST-bridge/pull/79#pullrequestreview-5149097123). This status reconciliation retires the old AP12 implementation and 'do not merge yet' instructions. It changes no production code, tests, binaries or historical evidence.

The installed Pure LoFi → Efx FRAGMENTS workflow now includes normal Bitwig browser selection, independent processing instances and real editors, recorded/replayed automation, edited-state saving, actual Deck reboot/automatic startup/recall, and removing FRAGMENTS while the same LoFi instance continues producing audio. See [AP12 results](docs/AP12.md), [musical/reboot evidence](evidence/AP12/musical-pair-reboot-removal.json), and [causal repair](evidence/AP12/delivery-cause-and-repair.json).

The reproduced LoFi editor-removal crash is addressed by the explicit process-scoped Windows-accessibility override. This is a compatibility workaround, not a Wine UI Automation implementation fix. It disables that accessibility interface for selected Windows hosts, not musical VST automation. Retain the working registrations, independent cleanup, state/refusal semantics and minimal fault visibility.

## Limits remain

The setting remains 512 added bridge frames per device. At the recorded 48 kHz setup, 512+48 and 512+192 produce 1,264 frames / 26.333 ms of reported chain latency, including 1,024 bridge frames / 21.333 ms. This is not physical loopback. Short gaps remain; the old untraced timeout is not retrospectively explained. Original-installer, operator-supplied module and authorization claims remain separate. The acceptance does not certify universal reliability or arbitrary Arturia installations.

[#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) retains delivery consistency and lower-latency work; [#74](https://github.com/kasselvania/Linux-VST-bridge/issues/74) retains prompt known-endpoint-loss handling; [#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) retains editor expansion/redraw issues. Serum qualification is #77 and the historical native engine-close issue is #73.

## Next work

No new implementation is authorized by this closure card alone. Prepare AP13 from integrated main for **consistent delivery and a validated lower-delay option on the installed musical chain**. Its fresh branch/work order must identify the selected behavior and tests. Do not repeat AP12 installation, replace vendor files, migrate runtimes speculatively or reopen every historical failure.

Follow [AGENTS.md](AGENTS.md). Preserve the operator's open project and live sessions; GitHub integration does not authorize taking over the desktop or deleting local worktrees. Leave the installed environment, publications and automatic service available. Ordinary future repairs belong to their selected task, not a new receipt or campaign system.
