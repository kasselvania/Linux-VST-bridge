# Current task: platform mechanisms, validated across every owned plug-in

## Goal

Any plug-in the musician installs goes through one door: install, scan,
authorize, load, open the editor, play with the editor touched, save and
recall, retire. It either enters the support matrix or produces a classified
failure that drives a general fix. No plug-in-named code, no campaigns.

## Established

Pure LoFi is clean for thirty minutes with Buffered 512, the product default.
The two symptom classes the operator still sees are general: "touching the
editor drops audio" (BEAM) and "the editor is a white screen" (Nibbi). The
bridge's own lane design is sound for editor input: controller updates run on
the UI-owner thread, never on the audio thread. What remains is CPU contention
from the editor, wineserver and graphics threads against a DAW with no
real-time policy, vendor-internal locks between editor and DSP, and an editor
presentation path the pinned runner cannot show.

## Platform work, in order

1. Host footprint policy (this change): every thread of the owned Windows
   family except the audio render thread, and every non-ordinary policy
   thread, is raised to nice 10 (LVB_HOST_NICE, 0 disables). The DAW's
   engine then wins contention against the editor by ten to one. Validate on
   BEAM, Pure LoFi and Serum with the editor touched during playback.
2. Delivery mode from measured cost: at prepare time, measure each plug-in's
   call cost per block size into profile data and choose Buffered unless the
   worst case is under a quarter of the period and the DAW's audio thread is
   real-time. Plain-language latency statement to the musician.
3. Graphics capability at intake (in progress): the assessment now decides a
   requirement by rule from what the editor loaded and this launch's probes,
   with a plain-language reason and next action. Still to do: apply the
   recommendation without an operator click, and run the assessment in every
   preparation. This is the white-screen class.
4. Intake as a manager operation: the staged check above as one button, with
   the first failing stage reported in plain words and mapped to a failure
   class. The agents have run it by hand for a week; the parts exist.

## Done

Every plug-in the operator owns has been through the intake once. Each is in
the matrix or has a classified failure with a general mechanism named.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
