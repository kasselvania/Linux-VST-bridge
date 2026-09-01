# HP1 operator attestation

- Classification: `operator_observed_supporting`.
- Exact one-use nonce receipt SHA-256: `cc7fabbb70a9c680eb34941fdeefc1cf46eafd2a1b08e8683505e325632d9ad1`.
- Operator transport SHA-256: `9e453363284e49227446909e5ac097654a816929d4b8ddbe07df6a119bb487b4`.
- Normalized semantic confirmation SHA-256: `1135fce6a25025ae5f770711468a461f7a145bd6af6b25a5a9f1b2e0b4f044cb`.
- Confirmation transport: `markdown_underscore_escaped`.
- Normal desktop launch attested: `true`.
- Session 1 bounded steps attested: `true`.
- Session 2 bounded steps attested: `true`.
- Gain in Bitwig generic parameter view: `observed`.
- Bypass as a separately exposed generic parameter: `not_observed`.
- Binding between the VST3 `kIsBypass` parameter and Bitwig's host device bypass control: `unknown`.

Only uniform Markdown underscore escaping is normalized; mixed or other escapes are refused. The parameter observations are supporting evidence only. HP1 did not exercise parameter changes or DSP, so host bypass binding remains unknown. The nonce itself is raw session material and is not committed.
