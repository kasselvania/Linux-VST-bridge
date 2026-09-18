# NAD2 — Exact NTKDaemon stop-response characterization

Active basis:

- merged source: `710b0f3be642cca342f2a5915d37158246023aed`
- reviewed/merged tree: `77efd70fac7985a051343407d4ec13ca183422a8`
- retained production evidence: `b175fdf5317a6afb512b17dc02cd40fdd907a201`
- installed immutable generation: `67e3004b087fe1dbb9c1bbd2c0b667a3eab1d1ac8cf14f7931f5ac37bb38880f`
- installed software record: `bd831fb3659ea27bc52d07bd63327206e46efb87e76af29c604c6ee281a96553`

NAD1 established the exact Native Access renderer policy, exact NTKDaemon payload
and registration, fresh process/listener readiness, one-stop ownership, truthful
retirement reporting, bounded fallback cleanup, and one rendered signed-in Native
Access session. Preserve all NAD1 and NAUI2 results.

The latest one-shot real operation `f4a2b703d8ea518d14c893c480fd93ed`
established readiness and both owned listeners, then submitted one SCM stop request.
The API returned without a control error, but SCM remained `RUNNING`, the exact
process did not exit, and both listeners remained throughout the 12,045 ms bounded
observation. Exit-receipt fallback was correctly inapplicable. Forced cleanup
recovered the bridge and keepers, and no successful preparation receipt was created.

Selected next slice: **NAD2 — exact NTKDaemon stop-response characterization**.

Authoritative slice document: `docs/NAD2.md`.

NAD2 adds bounded diagnostic characterization only. It must not introduce another
stop request, a longer production wait, a replacement shutdown strategy, direct
daemon termination as success, or a Native Access session.

Development review uses an **external VST cut-review tool** after the draft PR exists.
No Jev client, semantic annex, evidence packet, candidate ledger, or review receipt
belongs in this repository. Do not add or restore `tools/jev-preflight`, a
`docs/review/*jev*` contract, or Jev-specific repository process machinery.

During implementation and PR qualification:

- do not install a candidate;
- do not start or stop the real NTKDaemon;
- do not launch Native Access, a DAW, a plug-in, an updater, or a product installer;
- do not replay the daemon installer;
- do not increase the 12-second production observation bound;
- do not turn forced cleanup or bridge recovery into successful retirement;
- do not rewrite prior evidence.

Return one draft PR, uninstalled and unmerged, for independent tech-lead review.
Reliable real-daemon shutdown remains unqualified.
