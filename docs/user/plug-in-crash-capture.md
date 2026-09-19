# Capture a plug-in failure

Steam Deck + Bitwig | Linux Audio Compatibility Manager | 19 September 2026

Crash capture is **manual and one-shot**. Arm it before launching a fresh instance of the plug-in. It cannot recover detailed diagnostics from an earlier crash or attach to an already running instance.

## 1. Prepare one small test

Save your work. Use a separate test project or a copy of the failing project. Keep the settings and plug-in version that trigger the problem. For the simplest test, use one instance of the target plug-in.

Before arming, remove the target device from the test project. If reproducing a project-load crash, close that project first. Close other projects containing the same plug-in so they cannot consume the next-launch capture.

**Give the plug-in a signal when needed.** For an instrument, create a MIDI clip with notes and play it. For an effect, play an audio clip or an upstream instrument through it. An audio recording track is only needed if you also want to record the sound; crash capture itself does not require one.

## 2. Arm capture in the manager

Open **Linux Audio Compatibility Manager** from Applications. Find the target plug-in, expand **Details and management**, then click **Arm crash capture for next launch** once.

Expand **System status and diagnostics**. Wait for **Crash capture: armed for next admitted launch**. Use **Refresh** if needed. If the button is disabled or the request is refused, read the reason before proceeding; see page 3.

## 3. Launch the instance you want to capture

In Bitwig, add the target device again, or reopen the test project for a project-load failure. Launch only the intended test instance first. Capture follows the next matching processing instance, not just an editor window being reopened.

Refresh the manager. For an instance that stays running, expect **Crash capture: retaining an active instance**. A crash during launch may already have moved to **Recent incidents**. If the status stays armed, the intended instance has not claimed capture yet.

## 4. Reproduce the actual problem

Perform the known failing actions once, in order. Note the time, preset or control involved, and what stopped: the editor, plug-in audio, or all of Bitwig. Capture does not make a plug-in crash on command.

Use the normal action that caused the problem. Force-killing a process tests a different failure and may provide no Windows exception stack. Keep this focused: active diagnostic retention expires after two hours.

<!-- pagebreak -->

# Finish and save the report

Keep the first useful result before trying again.

## 5. End the attempt and let the report finish

**If it crashes:** note the visible message and last action. Let the bridge finish cleaning up, then return to the manager and click **Refresh**. Avoid immediately reloading the plug-in before checking the incident.

**If it does not crash:** remove the test device or close the test project normally. If the instance remains active, quit Bitwig normally. Closing only the plug-in editor does not necessarily end its audio process. Then refresh the manager.

Expand **Recent incidents** and open the newest entry for this attempt. A report becomes available after the owned instance and its audio transport have finished cleanup. Look for **Outcome**, **Cleanup**, and **Transport retirement**.

If there is no completed report, or cleanup is unconfirmed, retain the incident ID and visible message. A hang can leave capture unfinished. Do not count that as a successful cleanup or a clean run; send the observation for investigation before another attempt.

## 6. Export the sanitized report

Inside the incident, click **Export sanitized report**. If the button is absent, the shareable report is not available yet. The export is a local JSON file; it is not uploaded automatically.

Open the **Dolphin** file manager. Click its location bar, enter this folder, and press Enter:

```text
/home/deck/.local/share/linux-vst-bridge/managed/exports
```

Find **incident-<incident ID>.json**, matching the ID in Recent incidents. Attach that file and your reproduction notes when asking for help. The manager's **System status and diagnostics > Last operation receipt** also contains the export path after the export completes.

Use the exported file for sharing. It selects diagnostic facts and omits private paths, raw traces, plug-in state and account material. Keep the internal reports and managed environment private.

## 7. Leave capture off when finished

Click **Disarm crash capture** near the bottom of the manager, below **Recent incidents**, to cancel any unused arm or active retention. An armed capture also shows this button inside **System status and diagnostics**.

Refresh and check **Crash capture: off**. Disarming does not kill the plug-in. A process launched with diagnostics keeps those launch settings until it exits normally. Finish this test instance before judging normal performance.

**Read the result carefully.** A report is evidence to inspect, not a verdict that the fault is fixed. An exception count alone is not proof of a crash; some exceptions are handled normally. A vanished editor, audio failure, hang and full Bitwig exit are different observations.

<!-- pagebreak -->

# Check a fix and report what happened

Use the same failing steps so the comparison means something.

## Validate an attempted fix

1. Keep the original exported incident and reproduction notes.
2. Record the new plug-in or bridge version being tested. Keep the same test project, signal, preset, sample rate and buffer settings unless one of those is the intended change.
3. Arm a **new** capture after the change, launch a fresh instance, and repeat the same trigger. An old arm can become invalid when software or publication changes.
4. Finish and export the new attempt using page 2. Record the number of attempts and the actual result: for example, **"The previous trigger did not crash in 2 attempts; audio continued and cleanup was confirmed."**

If project recall is part of the reported problem, also save, close and reopen the test project, then check its sound and state. A successful scan or an open editor does not establish that result.

## If capture is unavailable

**Disabled arm button:** capture currently requires a verified, normally published profile. Experimental or newly installed products can be unavailable. **An exact ordinary publication is required** means a feature limitation, not a missed step. Keep your notes and request capture support for that plug-in. Do not switch working profiles just to enable the button.

**Status still armed:** end the intended instance normally, then create a fresh one. Check that you chose the correct plug-in, including instrument versus FX variants.

**Off with no report:** refresh Recent incidents and note any refusal or stale/expired status. Re-arm after an update if necessary. Missing diagnostics do not prove that no crash occurred.

This workflow captures a supervised plug-in processing instance. It does not automatically cover vendor installers, licensing applications, scanner crashes or every failure inside Bitwig itself.

## Notes to send with the exported JSON

- **Plug-in, Bitwig and bridge versions; date/time; incident ID:**
- **Project, preset, signal source, sample rate and buffer settings:**
- **Exact steps:** 1. ... 2. ... 3. ...
- **Expected result / actual result:**
- **What stopped, what still worked, and any error message:**
- **Cleanup result and exported filename:**
- **Reproduced in ___ of ___ attempts:**

Send the exported JSON and these notes. Keep paid content, license files and account details private.
