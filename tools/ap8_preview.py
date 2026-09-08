#!/usr/bin/env python3
"""Single prepared commercial instrument, with the existing session owner.

Keeps this task's vendor environment across instance lifetimes. Never reads or
changes the original Wine installation. No scanner, installer or activation API.
"""
import argparse,fcntl,json,os,pathlib,secrets,signal,socket,subprocess,threading,time,types
import ap8_fixture as fixture
owner=fixture.owner

def launch_environment(environment, desktop, *, trace_delivery=False, disable_windows_accessibility=False):
    values = {**owner.runtime.controlled_environment(environment), **desktop}
    if trace_delivery:
        values['LVB_AP10_TRACE'] = '1'
    if disable_windows_accessibility:
        # Explicit per-process compatibility setting; never change the runner,
        # registry or other environments. This is Windows UIA, not DAW automation.
        existing = values.get('WINEDLLOVERRIDES', '')
        values['WINEDLLOVERRIDES'] = (existing + ';' if existing else '') + 'uiautomationcore='
    return values

def main():
    p=argparse.ArgumentParser();p.add_argument('--environment',type=pathlib.Path,required=True);p.add_argument('--class-id',default='');p.add_argument('--performance',choices=['reference','serum']);p.add_argument('--trace-delivery',action='store_true');p.add_argument('--disable-windows-accessibility',action='store_true',help='Disable Windows UI Automation in this owned process; screen-reader support unavailable');a=p.parse_args()
    if a.performance!='reference' and len(a.class_id)!=32:raise ValueError('class ID length')
    cid=bytes.fromhex(a.class_id)
    os.chdir(owner.ROOT);os.umask(0o077)
    root=a.environment;marker=json.loads((root/'.wf0-owner.json').read_bytes());env=fixture.Environment(marker['run_id'],root,marker)
    fixture.verify_environment(env,runner_identity_sha256=fixture.verify_runtime()['launch_critical_manifest_sha256'])
    # One active endpoint in this prepared persistent prefix. Other vendor and
    # AGain owners are independent; no mutable factory/instance global is shared.
    lock=open(root/'ap8-owner.lock','a+b');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
    desktop={}
    for line in subprocess.check_output(['systemctl','--user','show-environment'],text=True).splitlines():
        k,_,v=line.partition('=')
        if k in ('DISPLAY','XAUTHORITY','WAYLAND_DISPLAY'):desktop[k]=v
    if not desktop.get('DISPLAY'):raise RuntimeError('existing desktop display absent')
    profile=types.SimpleNamespace(MODE=('ap9-'+('commercial' if a.performance=='serum' else 'reference')) if a.performance else 'ap8-commercial-preview',verify_runtime=fixture.verify_runtime,
        verify_environment=fixture.verify_environment,command_vector=fixture.command_vector,StreamState=fixture.ReferenceStreamState if a.performance=='reference' else fixture.StreamState,
        controlled_environment=lambda e:launch_environment(e,desktop,trace_delivery=a.trace_delivery,disable_windows_accessibility=a.disable_windows_accessibility))
    address_root=pathlib.Path.home()/(('AP9-Performance/'+a.performance) if a.performance else 'AP8-Commercial-Test/preview');owner.private_directory(address_root)
    output=address_root/'results';owner.private_directory(output)
    profile.native_report_directory=output
    stopping=threading.Event()
    signal.signal(signal.SIGTERM,lambda *_:stopping.set());signal.signal(signal.SIGINT,lambda *_:stopping.set())
    def create():
        fixture.verify_environment(env,runner_identity_sha256=marker['runner_identity_sha256'])
        if any(env.session.iterdir()):raise RuntimeError('previous commercial session disposition unresolved')
        return env
    def retire(e):
        # Called by serve_connected only after positive process containment.
        retired=output/('session-'+secrets.token_hex(16));e.session.rename(retired);e.session.mkdir(mode=0o700)
    def run(peer):
        attempt=output/('instance-'+secrets.token_hex(16));owner.private_directory(attempt)
        owner.serve_connected(peer,create,retire,attempt,stopping.is_set,supervision=profile,
            component_case='exact-again' if a.performance=='reference' else 'class:'+a.class_id,
            greeting_data=(b'AP9\n' if a.performance else b'AP8\n')+(b'' if a.performance=='reference' else cid+bytes.fromhex(marker['module_sha256'])))
    address=address_root/'owner.sock'
    with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as listener:
        listener.bind(str(address));inode=address.stat().st_ino;listener.listen(4);listener.setblocking(False)
        sessions=owner.Sessions(run);sessions.capacity=1
        try:
            print('AP8 prepared commercial preview ready; Windows accessibility '+('disabled' if a.disable_windows_accessibility else 'runner default'),flush=True)
            while not stopping.is_set():
                sessions.reap()
                try:peer,_=listener.accept()
                except BlockingIOError:time.sleep(.05);continue
                sessions.admit(peer)
        finally:
            stopping.set();sessions.join()
            if address.is_socket() and address.stat().st_ino==inode:address.unlink()
if __name__=='__main__':main()
