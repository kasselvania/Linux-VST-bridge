#!/usr/bin/env python3
"""Show the active audible MIDI fixture and message count on the existing OLED."""
import json
import re
import socket
import subprocess
import time


def show(text):
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(2)
        client.connect('/run/shieldxl0/oled.sock')
        client.sendall(json.dumps({'op': 'text', 'text': text}).encode())
        client.shutdown(socket.SHUT_WR)
        if not json.loads(client.recv(4096))['ok']:
            raise RuntimeError('OLED rejected status')


def read(command):
    return subprocess.run(command, text=True, capture_output=True, timeout=3).stdout.strip()


unit = 'shieldxl-midi-tone.service'
invocation = read(['systemctl', '--user', 'show', unit, '-p', 'InvocationID', '--value'])
if not re.fullmatch('[a-f0-9]{32}', invocation):
    raise SystemExit('No exact MIDI tone session to display')
for _ in range(1830):
    active = read(['systemctl', '--user', 'is-active', unit]) == 'active'
    uart = read(['systemctl', 'is-active', 'shieldxl-uart-midi@shieldxl.service']) == 'active'
    current = read(['systemctl', '--user', 'show', unit, '-p', 'InvocationID', '--value'])
    if not active or current != invocation or not uart:
        show('MIDI TEST OFF' if not active else 'UART INPUT OFF')
        break
    latest = read(['journalctl', '--user', '_SYSTEMD_INVOCATION_ID=' + invocation,
                   '-n', '1', '-o', 'cat', '--no-pager'])
    count = re.search(r'MIDI_TONE_STATUS messages=(\d+)', latest)
    show('MIDI ON RX:' + (count[1] if count else '0'))
    time.sleep(1)
else:
    show('MIDI TEST OFF')
