# Current slice — user control over programs

Goal: choose and remember a portable Windows program in an existing native
managed setup, then explicitly use changed bytes without installer registration
or maintainer permission. Keep the setup's runtime, prefix and licensed state.

Baseline: delivered Test69 and retained SteamOS/CachyOS installations remain.
CachyOS is the assigned installed-validation target; its saved project/history
and six Available classes must survive. Shared hardware has one owner at a time.

First source increment is reviewed, green and merged into main. Implemented:
1. Setup offers Choose program. A user selection records exact executable bytes
   and its existing working folder independently of installer observations.
2. Open uses the current setup's supervised runtime and the existing Windows
   suspended-launch adapter. Validate the actual child image before running it;
   keep the original executable path and neighboring resources in place.
3. Show and normal Close target the exact owned program. Force Stop remains an
   explicit recovery control. Replacing a path keeps the old running image's
   Focus/Close ownership; it never labels a replacement as the old image.
4. Program updated offers Use this version bound to the displayed new digest.
   Refuse another replacement before adoption. Keep the old exact record and
   operations readable; remember the successor after a manager restart.
5. This slice launches without extra arguments. Wine settings, local runtime
   import and the Windows DAW workspace remain later direction work.

Installed Test70 opens chosen build A and reads its sibling resource. Its active
readback still rejects application schema 3, leaving Show/Close controls stale.
The repair extends the existing exact live binding to this user-selected origin;
its regression checks current controls with the bridge service suspended.

Corrected-build installed acceptance is still due:
choose source-owned build A with a sibling resource, open/show/close it, replace
only its executable with distinguishable B, adopt B, restart the manager and
open/close B. Confirm retained plug-ins play and reopen the saved DAW state,
with projects, content, activation and previous application history preserved.

Land this one reviewed, green increment, then incorporate current main before
continuing. Source checks do not substitute for the installed musician flow.
