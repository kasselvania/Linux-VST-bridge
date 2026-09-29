#!/usr/bin/env python3
"""Send one bounded JSON operation to the ShieldXL OLED service."""

from __future__ import annotations

import argparse
import json
import socket
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", type=Path, default=Path("/run/shieldxl0/oled.sock"))
    parser.add_argument("request", help='JSON object, for example {"op":"clear"}')
    args = parser.parse_args()
    request = json.loads(args.request)
    if not isinstance(request, dict):
        parser.error("request must be a JSON object")
    payload = json.dumps(request, separators=(",", ":")).encode("utf-8")
    if len(payload) > 4096:
        parser.error("request exceeds 4096 bytes")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(2.0)
        client.connect(str(args.socket))
        client.sendall(payload)
        client.shutdown(socket.SHUT_WR)
        response = client.recv(4096)
    print(response.decode("utf-8").strip())
    return 0 if json.loads(response)["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
