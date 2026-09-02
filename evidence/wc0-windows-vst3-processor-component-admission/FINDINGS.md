# WC0 findings and claim ceiling

WC0 establishes the
narrow processor-component admission claim on the exact accepted fixture. The
existing supervised Windows VST3 path created the exact AGain processor directly
as one `IComponent`, verified its declared controller class ID, initialized it
with the minimal repository-owned `IHostApplication`, terminated and released
it correctly, returned host ownership to zero, proved object quiescence, and
then completed the already-proven factory/module shutdown without residue.

WC0 does not create a controller; connect processor and controller; query
`IConnectionPoint` or `IAudioProcessor`; enumerate or activate buses; access
parameters or state; configure processing; call `setActive`, `setProcessing`, or
`process`; handle audio/events; open an editor; create IPC or a native proxy;
run Bitwig or Serum; authorize; package; select a runner; or establish general
VST3, plug-in, Linux, or product compatibility.
