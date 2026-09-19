"""Audit the built K8I1 Windows artifacts on the pinned AP8 worker."""
from __future__ import annotations

import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import generate_forwarders


def output(*args: str) -> str:
    return subprocess.check_output(args, text=True, errors="strict", timeout=30)


def machine(path: pathlib.Path) -> int:
    data = path.read_bytes()
    if data[:2] != b"MZ" or len(data) < 64:
        raise AssertionError("not_pe")
    offset = int.from_bytes(data[60:64], "little")
    if data[offset:offset + 4] != b"PE\0\0":
        raise AssertionError("not_pe")
    return int.from_bytes(data[offset + 4:offset + 6], "little")


def main() -> None:
    if len(sys.argv) != 4:
        raise SystemExit("usage: windows_test.py SHIM REGISTRY SHIM_FIXTURE")
    shim, registry, fixture = map(lambda value: pathlib.Path(value).resolve(), sys.argv[1:])
    assert machine(shim) == 0x14C
    assert machine(registry) == 0x8664
    assert machine(fixture) == 0x14C
    rows = generate_forwarders.parse(HERE / "pinned-msi-exports.txt")
    exports = output("dumpbin", "/nologo", "/exports", str(shim))
    observed = {}
    for line in exports.splitlines():
        match = re.match(r"^\s*([0-9]+)\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+([A-Za-z_][A-Za-z0-9_]*)(?:\s+=\s+([^\s]+))?\s*$", line)
        if match:
            observed[int(match[1])] = (match[2], match[3])
    assert len(observed) == 296, ("export_count", len(observed))
    for ordinal, name in rows:
        assert ordinal in observed and observed[ordinal][0] == name, (ordinal, name, observed.get(ordinal))
        forwarded = observed[ordinal][1]
        if name in generate_forwarders.INTERCEPTED:
            assert forwarded is None, ("intercept_forwarded", name, forwarded)
        else:
            assert forwarded == "msi_lvb_real." + name, ("forwarder", name, forwarded)
    imports = output("dumpbin", "/nologo", "/imports", str(shim))
    libraries = {match.group(1).upper() for match in re.finditer(r"(?m)^\s*([A-Za-z0-9_.-]+\.dll)\s*$", imports)}
    assert libraries == {"KERNEL32.DLL"}, ("unexpected_imports", libraries)
    source = (HERE / "msi_shim.c").read_text(encoding="utf-8")
    for forbidden in ("CreateProcess", "ShellExecute", "WinExec", "WinHttp", "InternetOpen", "URLDownloadToFile"):
        assert forbidden not in source, forbidden
    subprocess.run([str(registry), "--self-test"], check=True, timeout=10)
    with tempfile.TemporaryDirectory(prefix="k8i1-shim-") as raw:
        root=pathlib.Path(raw).resolve();setup=root/"Kontakt 8 Setup PC.exe";package=root/"Kontakt 8 Setup PC.msi"
        shutil.copy2(shim,root/"msi.dll");shutil.copy2(fixture,setup)
        with setup.open("r+b") as handle:handle.truncate(1_188_804_208)
        with package.open("wb") as handle:handle.truncate(4_222_976)
        operation="a"*32;nonce="b"*64;plan="c"*64;receipt="d"*64
        request=root/"request.private";result=root/"result.private"
        config="\n".join(["K8I1_CONFIG_V1",operation,nonce,"e"*64,"1188804208","f"*64,
                           "4222976",str(request),str(result),plan,"10000",""])
        (root/"k8i1.private").write_bytes(config.encode("utf-16le"))
        def response(state):
            result.write_bytes("\n".join(["K8I1_RESULT_V1",operation,nonce,"1",state,plan,receipt,""]).encode("utf-16le"))
        response("verified")
        completed=subprocess.run([str(setup),"W",str(package)],cwd=root,timeout=20)
        assert completed.returncode==0,completed.returncode
        lines=request.read_bytes().decode("utf-16le").splitlines()
        assert lines[:5]==["K8I1_REQUEST_V1",operation,nonce,"1","W"]
        request.unlink();result.unlink();response("failed")
        refused=subprocess.run([str(setup),"A",str(package)],cwd=root,timeout=20)
        assert refused.returncode==1603,refused.returncode
        assert request.read_bytes().decode("utf-16le").splitlines()[4]=="A"
    print("K8I1_WINDOWS_FIXTURE_V1 exports=296 forwarded=294 intercepted=2 success=1 forced_failure=1603 registry=closed")


if __name__ == "__main__":
    main()
