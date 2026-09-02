# WF0 Windows build and private artifact custody

Workflow
`.github/workflows/wf0-windows-msvc-build.yml` (Git blob `95757dc1746bb2df59418b0ed06f5f40544dd3a3`) ran as
numeric run `33601279364`, attempt
`1`, for the exact source commit.

Two distinct empty Release roots produced a `byte_identical` comparison
across `48` complete transfer paths. The static MSVC
runtime was selected; dependency network was closed for configure/build; no
runtime DLL was copied. The canonical artifact manifest is
`217d38dddb5e8ae4ee6b60245cc03b3174710cf2692e2e6e1a4c4ea1c04b7684`; the payload archive SHA-256 is
`7baa825ed9e393896da001d02d9cb71f046cfa7a30c3ff6c0344191c1263f31a`.

The exact private Actions artifact is numeric ID `9835459546`, name
`wf0-windows-build-8b76ab886fd75079c72e3f820781beb5d1b36ae9-run-33601279364-attempt-1`, upload-action bare digest
`7ccb8f53aee02748d98abaa641a53e479ebd6f1d7b8bf381c1c2f11644d44d53`, and REST digest
`sha256:7ccb8f53aee02748d98abaa641a53e479ebd6f1d7b8bf381c1c2f11644d44d53`. The Mac downloaded only its exact REST ID
route. The raw wrapper SHA-256 `7ccb8f53aee02748d98abaa641a53e479ebd6f1d7b8bf381c1c2f11644d44d53` equaled the
upload-action digest and the hexadecimal portion of the REST digest. The
upload result's human-facing URL and REST API URL were retained as distinct
typed values and joined by repository, run, source, name, ID, and normalized
digest bytes.

The scanner (`36643c2447811b52e1ad1455eb47e0ced9b5f5bf03849f9625d80979d7c5798f`) and AGain module
(`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`) are PE32+ AMD64. Every scanner, adapter, fault module,
and AGain import/export receipt closed against the declared Windows system-DLL
roster. Produced PE files were not executed on Windows.
