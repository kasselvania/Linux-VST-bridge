#!/usr/bin/env python3
"""Build independent Windows fixtures after a candidate package is frozen.

Only a disposable copy of our first-party fixture source is changed. The
existing MSVC/SDK build and state/DSP contract tests remain the build path.
No generated identity or module is added to the frozen package or its kit.
"""
import argparse
import hashlib
import io
import json
import os
import pathlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import uuid

ROOT = pathlib.Path(__file__).resolve().parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def replace_once(source, anchor, replacement):
    if source.count(anchor) != 1:
        raise ValueError('Exact companion fixture source seam changed')
    return source.replace(anchor, replacement)


def require_companion(source, generation):
    if not re.fullmatch('[0-9A-F]{8}', generation):
        raise ValueError('Exact first-party generation required')
    source = '#define NOMINMAX\n#define WIN32_LEAN_AND_MEAN\n' + source
    loader = '''#include <cstring>
#include <windows.h>
#include <string>
// The separate installer owns this dependency; no fallback or loader search.
struct FixtureCompanion {
    using Gain = double (__cdecl *)(double, double);
    using Generation = unsigned int (__cdecl *)();
    HMODULE module = nullptr;
    Gain gain = nullptr;
    FixtureCompanion() = default;
    FixtureCompanion(const FixtureCompanion&) = delete;
    FixtureCompanion& operator=(const FixtureCompanion&) = delete;
    ~FixtureCompanion() {
        gain = nullptr;
        if (module) FreeLibrary(module);
    }
    bool open() {
        wchar_t common[32768]{};
        const DWORD size = GetEnvironmentVariableW(L"CommonProgramFiles", common, 32768);
        if (!size || size >= 32768 || size < 3 || common[1] != L':' || common[2] != L'\\\\') return false;
        const std::wstring path = std::wstring(common) + L"\\\\LVBUnfamiliar\\\\GENERATION\\\\companion.dll";
        module = LoadLibraryExW(path.c_str(), nullptr,
            LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
        if (!module) return false;
        const auto version = reinterpret_cast<Generation>(GetProcAddress(module, "lvb_companion_generation"));
        gain = reinterpret_cast<Gain>(GetProcAddress(module, "lvb_companion_gain"));
        if (!version || !gain || version() != 0xGENERATIONu) { gain = nullptr; return false; }
        return true;
    }
};'''.replace('GENERATION', generation)
    source = replace_once(source, '#include <cstring>', loader)
    source = replace_once(source, 'class ReferenceProcessor final : public AudioEffect {\n    Settings settings;',
                          'class ReferenceProcessor final : public AudioEffect {\n    FixtureCompanion companion;\n    Settings settings;')
    anchor = '''    ReferenceProcessor() { setControllerClass(controllerID); }
    static FUnknown* create(void*) { return static_cast<IAudioProcessor*>(new ReferenceProcessor); }
    tresult PLUGIN_API initialize(FUnknown* host) override {
        auto result = AudioEffect::initialize(host);
        if (result != kResultOk) return result;'''
    source = replace_once(source, anchor, anchor + '\n        if (!companion.open()) return kResultFalse;')
    anchor = '    tresult PLUGIN_API process(ProcessData& data) override {\n'
    source = replace_once(source, anchor, anchor + '        if (!companion.gain) return kResultFalse;\n')
    anchor = '''                float value = float(settings.gain * (evolved ? .75 + settings.trim : 1.)
                    * (LVB_BETA_INSTRUMENT ? input / 16. : input * (.5 + settings.colour)));'''
    return replace_once(source, anchor, '''                float value = float(companion.gain(settings.gain * (evolved ? .75 + settings.trim : 1.),
                    (LVB_BETA_INSTRUMENT ? input / 16. : input * (.5 + settings.colour))));''')


def companion_contract(source):
    anchor = 'int main() {\n    HostApplication host;'
    return replace_once(source, anchor, anchor + '''
    if (std::getenv("LVB_BETA_EXPECT_COMPANION_REFUSAL")) {
        ReferenceProcessor unavailable;
        need(unavailable.initialize(&host) != kResultOk, "missing/wrong companion refuses initialization");
        std::cout << "LVB_COMPANION_CONTRACT_V1 unusable_dependency=refused\\n";
        return 0;
    }''')


def unfamiliar_installer(source, generation, version):
    values = dict(FAMILY='unfamiliar', TITLE=f'LVB Unfamiliar {generation} Plug-ins {version}',
        DISPLAY_NAME=f'LVB Unfamiliar {generation} Plug-ins', REGISTRATION=f'LVBUnfamiliar{generation}',
        INSTRUMENT_NAME=f'LVBUnfamiliar{generation}Instrument.vst3',
        EFFECT_NAME=f'LVBUnfamiliar{generation}Effect.vst3', MARKER='LVB_UNFAMILIAR_INSTALLER_V1')
    for name, value in values.items():
        anchors = re.findall(r'^const ' + name + r': &str = .*;$', source, re.MULTILINE)
        if len(anchors) != 1:
            raise ValueError('Exact unfamiliar installer source seam changed')
        source = replace_once(source, anchors[0], f'const {name}: &str = "{value}";')
    source = replace_once(source, '("DisplayVersion", "1.0.0")', f'("DisplayVersion", "{version}")')
    # The unfamiliar installer must preserve both pre-existing fixture families.
    anchor = '''    let other_names = if cfg!(beta_completion) { ["LVBReferenceInstrument.vst3", "LVBReferenceEffect.vst3"] }
        else { ["LVBCompletionInstrument.vst3", "LVBCompletionEffect.vst3"] };'''
    return replace_once(source, anchor, '''    let other_names = ["LVBReferenceInstrument.vst3", "LVBReferenceEffect.vst3",
        "LVBCompletionInstrument.vst3", "LVBCompletionEffect.vst3"];''')


def build_companion(output, generation):
    directory = output / 'companion'
    directory.mkdir()
    library = directory / 'companion.dll'
    environment = dict(os.environ, LVB_BETA_GENERATION=generation)
    subprocess.run(['rustc', str(ROOT / 'tools/beta/companion.rs'), '--edition=2024',
        '--crate-type', 'cdylib', '-D', 'warnings', '-C', 'opt-level=2',
        '-C', 'target-feature=+crt-static', '-o', str(library)],
        env=environment, check=True, timeout=120)
    environment['LVB_BETA_COMPANION'] = str(library.resolve())
    installers = []
    for name, configuration in [('LVB_Unfamiliar_Companion.exe', None),
        ('LVB_Unfamiliar_Companion_Hold.exe', 'beta_hold'),
        ('LVB_Unfamiliar_Companion_Partial.exe', 'beta_partial_hold')]:
        path = directory / name
        command = ['rustc', str(ROOT / 'tools/beta/companion_installer.rs'),
            '--edition=2024', '-D', 'warnings', '-C', 'opt-level=2',
            '-C', 'target-feature=+crt-static', '--check-cfg', 'cfg(beta_hold)',
            '--check-cfg', 'cfg(beta_partial_hold)', '-o', str(path)]
        if configuration:
            command += ['--cfg', configuration]
        subprocess.run(command, env=environment, check=True, timeout=120)
        subprocess.run([str(path), '--self-test'], check=True, timeout=30)
        installers.append(dict(file='companion/' + name, sha256=sha(path),
                               configuration=configuration or 'ordinary'))
    return dict(file='companion/companion.dll', sha256=sha(library),
                relative_install_path=f'LVBUnfamiliar/{generation}/companion.dll',
                source_sha256=sha(ROOT / 'tools/beta/companion.rs'),
                installer_source_sha256=sha(ROOT / 'tools/beta/companion_installer.rs'),
                installers=installers)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sdk', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    parser.add_argument('--evolve-generation', help='Existing first-party identity; build schema/parameter migration and refusal successors')
    parser.add_argument('--with-companion', action='store_true',
                        help='Require a separate same-environment DSP dependency installer')
    parser.add_argument('--frozen-package-sha256', required=True)
    parser.add_argument('--frozen-engine-sha256', required=True)
    args = parser.parse_args()
    if os.name != 'nt':
        raise ValueError('Native Windows MSVC fixture build required')
    for value in (args.frozen_package_sha256, args.frozen_engine_sha256):
        if not re.fullmatch('[0-9a-f]{64}', value):
            raise ValueError('Exact frozen artifact digest required')
    if args.output.exists():
        raise ValueError('Retain earlier fixture artifacts; choose a new output')
    args.output.mkdir(parents=True)
    sdk = args.sdk.resolve(strict=True)
    git = lambda *values: subprocess.check_output(['git', '-C', str(ROOT), *values]).strip()
    if git('status', '--porcelain'):
        raise ValueError('Commit the fixture generator before running it')
    generation = args.evolve_generation or uuid.uuid4().hex[:8].upper()
    if not re.fullmatch('[0-9A-F]{8}',generation):
        raise ValueError('Exact first-party generation required')
    if generation == '45544131':
        raise ValueError('Generated identity collides with the original fixture')
    original = (ROOT / 'windows-fixtures/beta/stateful.cpp').read_text()
    if original.count('0x45544131') != 2 or original.count('"1.0.0"') != 2:
        raise ValueError('Reference fixture identity/version seam changed')
    manifest = dict(schema=1, classification='first_party_test_instrumentation',
        generator_source_head=git('rev-parse', 'HEAD').decode(),
        generator_source_sha256=sha(pathlib.Path(__file__)),
        reference_source_sha256=sha(ROOT / 'windows-fixtures/beta/stateful.cpp'),
        frozen_package_sha256=args.frozen_package_sha256,
        frozen_engine_sha256=args.frozen_engine_sha256,
        generation=generation, evolving_contract=bool(args.evolve_generation), revisions=[])
    if args.with_companion:
        manifest['companion'] = build_companion(args.output, generation)
    with tempfile.TemporaryDirectory(prefix='lvb-unfamiliar-') as tmp:
        work = pathlib.Path(tmp)
        source, build = work / 'source', work / 'build'
        source.mkdir()
        source_archive = subprocess.check_output(['git', '-C', str(ROOT), 'archive', '--format=tar', 'HEAD'])
        with tarfile.open(fileobj=io.BytesIO(source_archive)) as archive:
            archive.extractall(source, filter='data')
        if args.with_companion:
            contract = source / 'windows-fixtures/beta/contract_tests.cpp'
            contract.write_text(companion_contract(contract.read_text()), newline='\n')
        subprocess.run(['cmake', '-S', str(source), '-B', str(build),
            '-G', 'Visual Studio 17 2022', '-A', 'x64', '-T', 'v143',
            '-DCMAKE_SYSTEM_VERSION=10.0.19041.0', '-DWF0_BUILD_ONLY=ON',
            '-DWF0_VST3_SDK_ROOT:PATH=' + str(sdk)], check=True, timeout=120)
        if args.with_companion:
            wrong_companion = work / 'wrong-companion.dll'
            subprocess.run(['rustc', str(ROOT / 'tools/beta/companion.rs'), '--edition=2024',
                '--crate-type', 'cdylib', '-D', 'warnings', '-C', 'opt-level=2',
                '-C', 'target-feature=+crt-static', '-o', str(wrong_companion)],
                env=dict(os.environ, LVB_BETA_GENERATION=f'{int(generation, 16) ^ 1:08X}'),
                check=True, timeout=120)
        for revision in ((3,4,5) if args.evolve_generation else (1,2)):
            version = f'1.0.{revision}'
            generated = original.replace('0x45544131', '0x' + generation)
            if args.evolve_generation:
                generated = f'#define LVB_BETA_STATE_VERSION 2\n#define LVB_BETA_RESTORE_POLICY {revision-3}\n' + generated
            generated = generated.replace('"1.0.0"', '"' + version + '"')
            generated = generated.replace('LVB Reference ', 'LVB Unfamiliar ')
            if args.with_companion:
                generated = require_companion(generated, generation)
            generated_path = source / 'windows-fixtures/beta/stateful.cpp'
            generated_path.write_text(generated, newline='\n')
            targets = [f'lvb-reference-{role}{suffix}'
                       for role in ('instrument', 'effect') for suffix in ('', '-tests')]
            subprocess.run(['cmake', '--build', str(build), '--config', 'Release',
                '--target', *targets, '--parallel', '2'], check=True, timeout=600)
            binaries = build / 'wf0/bin/Release'
            destination = args.output / version
            destination.mkdir()
            row = dict(version=version, state_schema=2 if args.evolve_generation else 1,
                       parameter_ids=[0,1,17] if args.evolve_generation else [0,1],
                       restore_policy=(revision-3) if args.evolve_generation else 0,
                       generated_source_sha256=sha(generated_path),
                       modules=[], contract_tests=[])
            for role in ('instrument', 'effect'):
                test_command = [str(binaries / f'lvb-reference-{role}-tests.exe')]
                if args.with_companion:
                    # Each rejected load is a new process. Test-only environment
                    # variables never alter an installed product environment.
                    common = work / ('common-' + role)
                    dependency = common / 'LVBUnfamiliar' / generation / 'companion.dll'
                    dependency.parent.mkdir(parents=True, exist_ok=True)
                    # Windows normalizes os.environ keys to uppercase; replace
                    # the inherited key instead of adding a case alias.
                    test_environment = dict(os.environ, COMMONPROGRAMFILES=str(common),
                                            LVB_BETA_EXPECT_COMPANION_REFUSAL='1')
                    refusal_tests = []
                    for case in ('missing', 'wrong-exports', 'wrong-generation'):
                        if case == 'wrong-exports':
                            shutil.copyfile(binaries / f'lvb-reference-{role}.vst3', dependency)
                        elif case == 'wrong-generation':
                            shutil.copyfile(wrong_companion, dependency)
                        result = subprocess.check_output(test_command, env=test_environment, timeout=30)
                        if result.decode().strip() != 'LVB_COMPANION_CONTRACT_V1 unusable_dependency=refused':
                            raise ValueError('Actual dependency refusal was not observed')
                        refusal_tests.append(dict(case=case, result=result.decode().strip()))
                    shutil.copyfile(args.output / manifest['companion']['file'], dependency)
                    test_environment.pop('LVB_BETA_EXPECT_COMPANION_REFUSAL')
                    test = subprocess.check_output(test_command, env=test_environment, timeout=30)
                    row['contract_tests'].append(dict(role=role, result=test.decode().strip(),
                        dependency_refusals=refusal_tests, installed_dependency='required_and_used'))
                    shutil.rmtree(common)
                else:
                    test = subprocess.check_output(test_command, timeout=30)
                    row['contract_tests'].append(dict(role=role, result=test.decode().strip()))
                name = f'lvb-reference-{role}.vst3'
                shutil.copyfile(binaries / name, destination / name)
                role_id = '494E5354' if role == 'instrument' else '45464658'
                row['modules'].append(dict(role=role, file=version + '/' + name,
                    sha256=sha(destination / name),
                    processor_class='4C564242' + generation + role_id + '00000001',
                    controller_class='4C564242' + generation + role_id + '00000002'))
            env = dict(os.environ,
                LVB_BETA_INSTRUMENT=str((destination / 'lvb-reference-instrument.vst3').resolve()),
                LVB_BETA_EFFECT=str((destination / 'lvb-reference-effect.vst3').resolve()))
            installer = destination / ('LVB_Unfamiliar_' + version.replace('.', '_') + '.exe')
            installer_source = source / 'tools/beta/unfamiliar_installer.rs'
            installer_text = (ROOT / 'tools/beta/reference_installer.rs').read_text()
            if args.with_companion:
                installer_text = unfamiliar_installer(installer_text, generation, version)
            installer_source.write_text(installer_text, newline='\n')
            subprocess.run(['rustc', str(installer_source),
                '--edition=2024', '-D', 'warnings', '-C', 'opt-level=2',
                '-C', 'target-feature=+crt-static', '--check-cfg', 'cfg(beta_hold)',
                '--check-cfg', 'cfg(beta_partial_hold)', '--check-cfg', 'cfg(beta_completion)', '-o', str(installer)],
                env=env, check=True, timeout=120)
            subprocess.run([str(installer), '--self-test'], check=True, timeout=30)
            row['installer'] = dict(file=version + '/' + installer.name, sha256=sha(installer),
                generated_source_sha256=sha(installer_source),
                installed_modules=[f'LVBUnfamiliar{generation}{role.title()}.vst3'
                                   if args.with_companion else f'LVBReference{role.title()}.vst3'
                                   for role in ('instrument', 'effect')])
            manifest['revisions'].append(row)
    (args.output / 'UNFAMILIAR_FIXTURES.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(json.dumps(manifest, sort_keys=True))


if __name__ == '__main__':
    main()
