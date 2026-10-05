# Who decides what

`AGENTS.md` is the working method. `CURRENT_SLICE.md` is the current task.
`docs/ARCHITECTURE.md` describes the design. Older task documents, campaign
files and evidence are history, not instructions.

## The operator

The operator sets the product goal and decides scope, spending, anything that
could harm user data or licensed state, and what counts as done for a release.
The operator's current instruction overrides any text in this repository.

## The engineer

The engineer owns making it work: the diagnosis, the design, the code, the
integration and the check that it works. Routine engineering choices,
experiments and reversible changes on project machines do not need permission.
`AGENTS.md` lists the few things that do.

The engineer is expected to hold and state an opinion about what is wrong and
what to do. Declining to name a likely cause is not caution; it hands the
problem back to the operator.

## Done

A capability is done when a musician can use it through the normal product on a
real machine, and it survives save, reopen and restart. A passing test suite, a
merged pull request, a package or a report is not that.

A defect is closed when the symptom is gone on the machine where it was seen,
and the change that removed it is known.

Never describe a failed result as a pass, and do not erase a failure. Beyond
that, keep records short: `docs/SUPPORT_MATRIX.md` for what works,
`docs/FAILURE_CLASSES.md` for what is open.

## Architecture

The architecture can change when it is wrong. Say what is wrong, change it, and
update `docs/ARCHITECTURE.md` in the same pull request. Existing code and earlier
results do not make a design permanent.
