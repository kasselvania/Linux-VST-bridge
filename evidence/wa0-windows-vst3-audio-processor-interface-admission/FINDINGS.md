# WA0 findings and claim ceiling

WA0 establishes only
that the exact initialized AGain processor component exposes the mandatory
`Steinberg::Vst::IAudioProcessor` interface through the exact query, and that
one acquired reference can be retired to the component-owner baseline before
the accepted WC0 lifecycle and inherited module shutdown continue.

No `IAudioProcessor` method was called. WA0 did not create a controller; query
or connect `IConnectionPoint`; access buses, parameters, state, processing,
audio, events, timing, automation, presets, or an editor; create IPC or a native
proxy; run Bitwig or Serum; authorize, package, sign, or select a runner; test
another plug-in or DAW; or establish general VST3 or Linux compatibility.
