"""Loopback browser gateway for one temporary sharing session."""
import argparse
from pathlib import Path
import signal
import socket
import struct
import threading

from websockify.websocketproxy import LibProxyServer, ProxyRequestHandler

work = None
authorities = set()
origins = set()
active = set()
active_lock = threading.Lock()
modifier_releases = b"".join(struct.pack(">BBHI", 4, 0, 0, key) for key in
    (0xffe1, 0xffe2, 0xffe3, 0xffe4, 0xffe9, 0xffea, 0xffeb, 0xffec))


class ForwardingTarget:
    """Serialize the final releases after all in-flight browser writes."""
    def __init__(self, target):
        self.target = target
        self.target.settimeout(1)
        self.lock = threading.Lock()
        self.stopping = False

    def __getattr__(self, name):
        return getattr(self.target, name)

    def send(self, data):
        with self.lock:
            if self.stopping:
                raise OSError("Gateway is stopping")
            return self.target.send(data)

    def finish(self):
        with self.lock:
            if self.stopping:
                return
            self.stopping = True
            try:
                self.target.settimeout(1)
                self.target.sendall(modifier_releases)
            except OSError:
                pass
            try:
                self.target.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


class LocalHandler(ProxyRequestHandler):
    def permitted(self):
        return (self.headers.get("Host") in authorities
                and (self.headers.get("Origin") is None or self.headers.get("Origin") in origins))

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Frame-Options", "DENY")
        self.send_header("Content-Security-Policy", "frame-ancestors 'none'")
        super().end_headers()

    def do_GET(self):
        if not self.permitted():
            self.send_error(403)
            return
        if self.path == "/credentials.json":
            raw = (work / "credentials.json").read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(raw)))
            self.end_headers()
            self.wfile.write(raw)
            return
        super().do_GET()

    def do_HEAD(self):
        if not self.permitted():
            self.send_error(403)
            return
        super().do_HEAD()

    def do_proxy(self, target):
        guarded = ForwardingTarget(target)
        with active_lock:
            active.add(guarded)
        try:
            return super().do_proxy(guarded)
        finally:
            guarded.finish()
            with active_lock:
                active.discard(guarded)

    def validate_connection(self):
        if not self.permitted():
            raise self.CClose(1008, "Local origin required")
        return super().validate_connection()


def stopped(_number, _frame):
    # SIGTERM otherwise exits before daemon connection threads run their finally.
    with active_lock:
        targets = list(active)
    for target in targets:
        target.finish()
    raise SystemExit(0)


def main():
    global work, authorities, origins
    parser = argparse.ArgumentParser()
    parser.add_argument("work", type=Path)
    parser.add_argument("--port", type=int, default=18080)
    parser.add_argument("--vnc-port", type=int, default=15900)
    options = parser.parse_args()
    work = options.work.resolve()
    authorities = {f"127.0.0.1:{options.port}", f"localhost:{options.port}"}
    origins = {"http://" + host for host in authorities}

    signal.signal(signal.SIGTERM, stopped)
    signal.signal(signal.SIGINT, stopped)
    LibProxyServer.allow_reuse_address = True
    LibProxyServer.daemon_threads = True
    server = LibProxyServer(RequestHandlerClass=LocalHandler, listen_host="127.0.0.1",
        listen_port=options.port, target_host="127.0.0.1", target_port=options.vnc_port,
        web=str(work / "noVNC-1.6.0"), heartbeat=10)
    server.file_only = True
    server.unix_listen = None
    server.unix_listen_mode = None
    server.ssl_target = False
    try:
        server.serve_forever()
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
