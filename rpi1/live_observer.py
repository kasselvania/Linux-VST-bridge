#!/usr/bin/env python3
"""Read-only RPI1 live view on the Mac. Never owns or controls the audio session."""

import argparse
from collections import deque
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import threading
import time


FIELD = re.compile(r"([a-z_]+)=([^ ]+)")
RECORDS = {
    "RPI1_READY", "RPI1_AUDIO", "RPI1_CALLBACK_LIVE", "RPI1_CALLBACK",
    "RPI1_EDITOR", "RPI1_MASTER_READBACK", "RPI1_STATE_RESTORED",
    "RPI1_TRANSPORT", "RPI1_RETIREMENT", "RPI1_CLEAN_SHUTDOWN",
    "RPI1_AUDIO_UNAVAILABLE",
}
EDITOR_LIFECYCLE = {
    0: "absent", 1: "opening", 2: "opened", 3: "awaiting focus",
    4: "focused", 5: "focus refused", 6: "closing",
    7: "closed by vendor", 8: "closed by host", 9: "open refused",
    10: "editor failed",
}


def integer(value):
    try:
        return int(value)
    except (TypeError, ValueError):
        return None


def number(value):
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


class Model:
    def __init__(self):
        self.lock = threading.Lock()
        self.started = time.time()
        self.health = None
        self.audio = None
        self.callback = None
        self.editor = None
        self.host = {"state": "waiting for RPI1_READY"}
        self.screen = {"tunnel": "unknown", "x11": "unmeasured"}
        self.visual_report = None
        self.events = deque(maxlen=24)
        self.last = {}
        self.errors = {}
        self.children = []
        self.stop = threading.Event()

    def _event(self, lane, message):
        self.events.appendleft({"at": round(time.time(), 3), "lane": lane, "message": message})

    def ingest_health(self, line):
        if not line.startswith("RPI1_HEALTH "):
            return
        try:
            raw = json.loads(line[len("RPI1_HEALTH "):])
            flags = int(raw["throttled"].split("=0x", 1)[1], 16)
        except (ValueError, KeyError, TypeError, json.JSONDecodeError):
            self.error("health", "invalid health record")
            return
        voltage = re.search(r"([0-9.]+)V$", raw.get("voltage") or "")
        cohorts = raw.get("cohorts") or {}
        windows = [v for k, v in cohorts.items() if re.fullmatch(r"lvb-rpi1-[0-9a-f]{32}\.service", k)]
        outer = cohorts.get("lvb-rpi1-audio-gate.service", {})
        health = {
            "temperature_c": number(raw.get("temperature_millidegrees")),
            "voltage_v": number(voltage.group(1)) if voltage else None,
            "throttled_hex": f"0x{flags:x}",
            "undervoltage_now": bool(flags & 0x1),
            "frequency_cap_now": bool(flags & 0x2),
            "throttled_now": bool(flags & 0x4),
            "soft_temperature_limit_now": bool(flags & 0x8),
            "undervoltage_seen": bool(flags & 0x10000),
            "thermal_limit_seen": bool(flags & 0x80000),
            "windows_cohort_processes": sum(integer(v.get("processes")) or 0 for v in windows),
            "windows_cohort_memory_bytes": sum(integer(v.get("memory.current")) or 0 for v in windows),
            "outer_processes": integer(outer.get("processes")),
            "mem_available_kb": integer((raw.get("mem_available") or "").split(" ", 1)[0]),
            "pi_monotonic_seconds": number(raw.get("monotonic_seconds")),
        }
        if health["temperature_c"] is not None:
            health["temperature_c"] /= 1000
        with self.lock:
            old = self.health
            self.health = health
            self.last["health"] = time.time()
            if health["undervoltage_now"] and not (old or {}).get("undervoltage_now"):
                self._event("Pi power", "active undervoltage flag")
            if health["soft_temperature_limit_now"] and not (old or {}).get("soft_temperature_limit_now"):
                self._event("Pi thermal", "active soft temperature limit")
            if health["throttled_now"] and not (old or {}).get("throttled_now"):
                self._event("Pi thermal", "active CPU throttling")

    def ingest_journal(self, line):
        kind = line.split(" ", 1)[0]
        if kind not in RECORDS:
            return
        fields = dict(FIELD.findall(line))
        now = time.time()
        with self.lock:
            self.last["journal"] = now
            if kind == "RPI1_READY":
                self.host = {"state": "ready", "jack_frames": integer(fields.get("jack_frames")),
                             "bridge_frames": integer(fields.get("bridge_frames"))}
                self.audio = None
                self.callback = None
                self.editor = None
                for lane in ("audio", "callback", "editor"):
                    self.last.pop(lane, None)
                self._event("Pigments host", "ready")
            elif kind == "RPI1_AUDIO":
                old = self.audio or {}
                self.audio = {k: integer(fields.get(k)) for k in (
                    "midi_accepted", "output_nonzero_l", "output_nonzero_r", "output_nonfinite",
                    "xruns", "process_failures", "bridge_processed", "fault")}
                for k in ("output_peak_l", "output_peak_r"):
                    self.audio[k] = number(fields.get(k))
                self.last["audio"] = now
                if self.audio["fault"] and self.audio["fault"] != old.get("fault"):
                    self._event("Transport", f"terminal fault {self.audio['fault']}")
                if self.audio["process_failures"] and self.audio["process_failures"] != old.get("process_failures"):
                    self._event("Audio", "processing failures increased")
            elif kind in ("RPI1_CALLBACK_LIVE", "RPI1_CALLBACK"):
                old = self.callback or {}
                self.callback = {k: integer(fields.get(k)) for k in (
                    "callbacks", "failures", "deadline_misses", "callback_ns_max",
                    "missing_frames", "gaps", "delivered_frames", "paused_frames")}
                self.callback["source"] = "live" if kind.endswith("LIVE") else "shutdown"
                self.last["callback"] = now
                if old and (self.callback["gaps"] or 0) > (old.get("gaps") or 0):
                    added = (self.callback["missing_frames"] or 0) - (old.get("missing_frames") or 0)
                    self._event("Transport", f"gap increased; {added} newly missing frames")
                elif not old and kind == "RPI1_CALLBACK" and (self.callback["gaps"] or 0) > 1:
                    self._event("Transport", "shutdown revealed multiple gaps; timing was not live-observed")
            elif kind == "RPI1_EDITOR":
                code = integer(fields.get("lifecycle"))
                self.editor = {"lifecycle": code, "meaning": EDITOR_LIFECYCLE.get(code, "unknown"),
                               "result": integer(fields.get("result"))}
                self.last["editor"] = now
                self._event("Editor", f"{self.editor['meaning']}; result {self.editor['result']}")
            elif kind == "RPI1_CLEAN_SHUTDOWN":
                self.host["state"] = "clean shutdown"
                self._event("Pigments host", "clean shutdown")
            elif kind == "RPI1_AUDIO_UNAVAILABLE":
                self._event("Audio", "status unavailable; see private native journal")
            elif kind == "RPI1_TRANSPORT":
                self.host["final_fault"] = integer(fields.get("fault"))
            elif kind == "RPI1_RETIREMENT":
                self.host["retirement_milestones"] = integer(fields.get("milestones"))
            elif kind == "RPI1_MASTER_READBACK":
                self.host["master_readback"] = number(fields.get("normalized"))
            elif kind == "RPI1_STATE_RESTORED":
                self._event("Pigments state", "state restored")

    def screen_probe(self, tunnel, x11, editor_window, x11_ms, ssh_ms=None):
        with self.lock:
            self.screen = {"tunnel": tunnel, "x11": x11,
                           "pigments_window": editor_window, "x11_round_trip_ms": x11_ms,
                           "ssh_round_trip_ms": ssh_ms}
            self.last["screen"] = time.time()

    def observation(self, value):
        if value not in ("responsive", "delayed", "frozen", "not visible"):
            return False
        with self.lock:
            self.visual_report = {"at": time.time(), "value": value}
            self._event("RPI0 viewer", f"operator reported {value}")
        return True

    def error(self, source, detail):
        with self.lock:
            self.errors[source] = detail
            self._event("Observer", f"{source}: {detail}")

    def snapshot(self):
        with self.lock:
            now = time.time()
            return {"at": now, "health": self.health, "audio": self.audio,
                    "callback": self.callback, "editor": self.editor, "host": dict(self.host),
                    "screen": dict(self.screen), "visual_report": self.visual_report,
                    "age_seconds": {k: round(now - v, 1) for k, v in self.last.items()},
                    "events": list(self.events), "errors": dict(self.errors)}


PAGE = """<!doctype html><html><head><meta charset="utf-8"><title>RPI1 live view</title>
<style>body{font:16px system-ui;background:#10151d;color:#ecf3fb;margin:24px;max-width:1200px}
h1{font-size:25px} .grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(340px,1fr));gap:12px}
section{background:#1b2633;border:1px solid #35485b;border-radius:10px;padding:15px;min-height:120px}
h2{font-size:17px;margin:0 0 10px} .line{margin:5px 0} .bad{color:#ff8f8f}.warn{color:#ffd17b}
.good{color:#8de6bd}.muted{color:#a6b4c5}button{padding:9px;margin:3px;border-radius:7px}
pre{white-space:pre-wrap;font:13px ui-monospace,monospace}</style></head><body>
<h1>RPI1 live view <small id="fresh" class="muted"></small></h1>
<p class="muted">Read-only Pi and Pigments measurements. The buttons record only what you see in the existing RPI0 window.</p>
<div class="grid">
<section><h2>Pi power and heat</h2><div id="pi"></div></section>
<section><h2>MIDI and audio</h2><div id="audio"></div></section>
<section><h2>Bridge continuity</h2><div id="transport"></div></section>
<section><h2>Pigments process and editor</h2><div id="host"></div></section>
<section><h2>Local display versus screen sharing</h2><div id="screen"></div>
<div id="buttons"></div></section>
<section><h2>First changes</h2><div id="events"></div></section></div>
<script>
const el=id=>document.getElementById(id), fmt=(v,d=2)=>v==null?'unavailable':typeof v==='number'?v.toFixed(d):v;
const line=(label,value,cl='')=>`<div class="line ${cl}">${label}: <b>${value}</b></div>`;
for(const name of ['responsive','delayed','frozen','not visible']){
 let b=document.createElement('button');b.textContent=name;b.onclick=()=>fetch('/observation',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({value:name})});el('buttons').append(b);
}
async function update(){try{let r=await fetch('/snapshot'),s=await r.json(),h=s.health||{},a=s.audio||{},c=s.callback||{},e=s.editor||{},x=s.screen||{},age=s.age_seconds||{};
 el('fresh').textContent=`Pi sample ${age.health??'—'}s old · audio ${age.audio??'—'}s old`;
 el('pi').innerHTML=line('Temperature',fmt(h.temperature_c,1)+' °C',h.temperature_c>=80?'bad':'')+line('Pi input',fmt(h.voltage_v,3)+' V')+line('Firmware flags',h.throttled_hex||'unavailable',(h.undervoltage_now||h.throttled_now||h.soft_temperature_limit_now)?'bad':'')+line('Active undervoltage',h.undervoltage_now?'YES':'no',h.undervoltage_now?'bad':'')+line('Active thermal limit',h.soft_temperature_limit_now?'YES':'no',h.soft_temperature_limit_now?'bad':'')+line('Translated processes',h.windows_cohort_processes??'unavailable');
 el('audio').innerHTML=line('MIDI accepted',a.midi_accepted??'unavailable')+line('Nonzero stereo samples',(a.output_nonzero_l??'—')+' / '+(a.output_nonzero_r??'—'))+line('Peak L/R',fmt(a.output_peak_l,3)+' / '+fmt(a.output_peak_r,3))+line('XRUNs',a.xruns??'unavailable',a.xruns?'bad':'')+line('Process failures',a.process_failures??'unavailable',a.process_failures?'bad':'');
 el('transport').innerHTML=line('Bridge blocks processed',a.bridge_processed??'unavailable')+line('Missing frames',c.missing_frames??'unavailable')+line('Gap count',c.gaps??'unavailable',c.gaps>1?'bad':'')+line('Callback deadline misses',c.deadline_misses??'unavailable',c.deadline_misses?'bad':'')+line('Terminal fault',a.fault??s.host.final_fault??'unavailable',a.fault?'bad':'')+line('Counter source',c.source||'needs status-enabled binary');
 el('host').innerHTML=line('Process',s.host.state||'unknown')+line('Editor lifecycle',e.meaning||'unobserved')+line('Editor result',e.result??'unavailable')+line('Master readback',fmt(s.host.master_readback,3))+line('Retirement milestones',s.host.retirement_milestones??'unavailable');
 el('screen').innerHTML=line('Existing RPI0 TCP connection',x.tunnel||'unavailable')+line('Pi X11 query',x.x11||'unavailable')+line('Pigments window on Pi',x.pigments_window==null?'unavailable':x.pigments_window?'present':'absent')+line('Pi X11 response',fmt(x.x11_round_trip_ms,0)+' ms')+line('SSH plus X11 response',fmt(x.ssh_round_trip_ms,0)+' ms')+line('What you saw',s.visual_report?.value||'not reported');
 el('events').innerHTML=s.events.slice(0,9).map(v=>line(new Date(v.at*1000).toLocaleTimeString()+' '+v.lane,v.message)).join('');
 if(age.health>3||s.host.state==='ready'&&age.audio>5)el('fresh').className='bad';else el('fresh').className='muted';
}catch(err){el('fresh').textContent='Observer disconnected';el('fresh').className='bad'}}
update();setInterval(update,1000);
</script></body></html>"""


def ssh_command(args, remote):
    return ["ssh", "-S", args.control, "-o", "BatchMode=yes", "-o", "ConnectTimeout=3",
            args.host, remote]


def stream(model, name, command, consume):
    try:
        process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                   text=True, bufsize=1)
        with model.lock:
            model.children.append(process)
        for line in process.stdout:
            consume(line.rstrip("\n"))
        status = process.wait()
        if not model.stop.is_set():
            model.error(name, f"stream ended with status {status}")
    except OSError as exc:
        model.error(name, f"could not start: {exc.strerror}")


def probe(model, args):
    script = ("import json,subprocess,time;"
              "start=time.monotonic();"
              "p=subprocess.run(['xwininfo','-root','-tree'],capture_output=True,text=True,timeout=1);"
              "print(json.dumps({'ok':p.returncode==0 and 'Root window id:' in p.stdout,"
              "'pigments':'\"Pigments\"' in p.stdout,"
              "'x11_ms':round((time.monotonic()-start)*1000)}))")
    remote = "DISPLAY=:1 XAUTHORITY=$HOME/.Xauthority /usr/bin/python3 -c " + shlex.quote(script)
    while not model.stop.is_set():
        try:
            result = subprocess.run(["lsof", "-nP", "-a", "-p", str(args.vnc_pid), "-iTCP"],
                                    capture_output=True, text=True, timeout=2)
            tunnel = "connected" if "127.0.0.1:5901 (ESTABLISHED)" in result.stdout else "disconnected"
            began = time.monotonic()
            display = subprocess.run(ssh_command(args, remote), capture_output=True, text=True, timeout=4)
            elapsed = round((time.monotonic() - began) * 1000)
            result = json.loads(display.stdout) if display.returncode == 0 else {}
            good = result.get("ok") is True
            model.screen_probe(tunnel, "responding" if good else "unavailable",
                               result.get("pigments") if good else None,
                               result.get("x11_ms") if good else None, elapsed)
        except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError):
            model.screen_probe("unavailable", "unavailable", None, None)
        model.stop.wait(2)


def serve(model, port):
    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path == "/":
                body, mime = PAGE.encode(), "text/html; charset=utf-8"
            elif self.path == "/snapshot":
                body, mime = json.dumps(model.snapshot()).encode(), "application/json"
            else:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Type", mime)
            self.send_header("Cache-Control", "no-store")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_POST(self):
            if self.path != "/observation" or integer(self.headers.get("Content-Length")) not in range(1, 128):
                self.send_error(400)
                return
            try:
                value = json.loads(self.rfile.read(int(self.headers["Content-Length"]))).get("value")
            except (ValueError, AttributeError):
                self.send_error(400)
                return
            if not model.observation(value):
                self.send_error(400)
                return
            self.send_response(204)
            self.end_headers()

        def log_message(self, *_):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    print(f"RPI1 observer: http://127.0.0.1:{server.server_port}/", flush=True)
    try:
        server.serve_forever()
    finally:
        server.server_close()


def record(model, fd):
    with os.fdopen(fd, "w") as output:
        while not model.stop.is_set():
            output.write(json.dumps(model.snapshot(), separators=(",", ":")) + "\n")
            output.flush()
            os.fsync(output.fileno())
            model.stop.wait(1)
        output.write(json.dumps(model.snapshot(), separators=(",", ":")) + "\n")
        output.flush()
        os.fsync(output.fileno())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--host", required=True, help="Pi SSH target")
    parser.add_argument("--control", required=True, help="existing SSH control socket")
    parser.add_argument("--health-script", required=True, help="source-owned health.py on the Pi")
    parser.add_argument("--vnc-pid", type=int, required=True, help="existing Mac RPI0 Screen Sharing PID")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--record", required=True, help="new private Mac JSONL receipt path")
    args = parser.parse_args()
    if args.host.startswith("-") or not re.fullmatch(r"[A-Za-z0-9_.@:-]+", args.host):
        parser.error("invalid SSH target")
    if not args.health_script.startswith("/") or args.vnc_pid <= 0:
        parser.error("absolute health script and positive viewer PID required")
    if not Path(args.record).is_absolute():
        parser.error("absolute record path required")
    receipt_fd = os.open(args.record, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    model = Model()
    health = shlex.join(["/usr/bin/python3", args.health_script])
    journal = "journalctl --user -u lvb-rpi1-audio-gate.service -f -n 0 -o cat --no-pager"
    workers = [
        threading.Thread(target=stream, args=(model, "health", ssh_command(args, health), model.ingest_health), daemon=True),
        threading.Thread(target=stream, args=(model, "journal", ssh_command(args, journal), model.ingest_journal), daemon=True),
        threading.Thread(target=probe, args=(model, args), daemon=True),
        threading.Thread(target=record, args=(model, receipt_fd), daemon=True),
    ]
    for worker in workers:
        worker.start()
    try:
        serve(model, args.port)
    except KeyboardInterrupt:
        pass
    finally:
        model.stop.set()
        with model.lock:
            children = list(model.children)
        for process in children:
            if process.poll() is None:
                process.terminate()
        for process in children:
            try:
                process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=3)
        for worker in workers:
            worker.join(timeout=3)


if __name__ == "__main__":
    main()
