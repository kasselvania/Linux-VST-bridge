# Linux Plugin Bridge private beta — start here

Use the installer for your system: **SteamOS** or **CachyOS**, on an x86-64
computer. On Steam Deck, switch to **Desktop Mode** first. These are private test
builds; the installer's “Internal test build” label is expected.

Bring your own native Linux DAW and lawful Windows plug-in installer or vendor
application. This beta accepts unfamiliar plug-ins: try the ones you use and
report what happens. The current managed connection is for **64-bit Windows
VST3** plug-ins. Choose the VST3 option in the vendor installer when offered.
An **Available** result means the plug-in has been connected; test its sound,
editor and saved-project recall in your DAW.

1. Download the matching `.install` file to your Linux computer. In the file
   manager, right-click it, open **Properties → Permissions**, and enable
   **Is executable** (or **Allow executing file as program**). Double-click it
   and choose **Execute** if asked.
2. Click **Install application files**, then **Open Setup**. Follow **Set up
   application** and **Start bridge service** when offered. Later, reopen
   **Linux Audio Compatibility Manager** from Applications; use **Linux VST
   Bridge Setup and Updates** to update or restore the application.
3. In the manager, open **Setup** and **Choose Windows installer**. Select your
   vendor's Windows `.exe` or `.msi`, then choose your Linux DAW: **Bitwig**,
   **REAPER**, or **Another Linux VST3 DAW**. Click **Install and connect to your
   DAW**, then **Start installation**. Keep an internet connection available:
   the manager downloads and prepares its own compatibility runtime when needed.
   First setup may take several minutes. Wine, Proton, Steam, an SDK and a
   compiler do not need to be installed beforehand.
4. Finish the vendor's installer, sign-in and authorization in its own window.
   For a vendor application, choose **Add plug-ins through [application name]**
   in Setup to install more products in that same managed installation. Close
   the vendor application normally when finished; the manager waits for its
   helpers, finds the installed plug-ins and connects them automatically.
5. When a plug-in shows **Available**, open your Linux DAW and add it to a track.
   Rescan VST3 plug-ins if needed. Test playback and the editor, then save a test
   project, quit your DAW and reopen the project to check that its sound and
   settings return. Try a separate test project before using it for a session.

If a step needs attention, read its message and use the continuation offered
there. Use **Refresh** or **Check again** to reload the current status. Send the
maintainer your system and DAW versions, plug-in name/version, the step that
failed, its message and what you expected. A screenshot helps. In **Diagnostics**
you can choose **Create sanitized support export**; review the local export
before sharing it privately. Nothing uploads automatically. Optional experience
reports are under the plug-in's **Help and testing** controls.

Vendor accounts, licenses and content remain yours. Download vendor software
from the vendor and authorize it normally; the bridge installer includes none
of those items. Do not send passwords, serial numbers or license files with a
problem report. The application includes its third-party notices.
