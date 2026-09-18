#!/usr/bin/env python3
"""Bounded Unix-socket service for the ShieldXL 128x64 SSD1322 OLED."""

from __future__ import annotations

import argparse
import glob
import json
import os
import re
import signal
import socket
import sys
import time
from pathlib import Path


WIDTH = 128
HEIGHT = 64
MAX_REQUEST = 4096
MAX_HZ = 20

# Five columns encoded as seven low-to-high row bits. The status surface needs
# uppercase ASCII, digits, and a small punctuation set; unsupported glyphs are '?'.
FONT = {
    " ": (0, 0, 0, 0, 0), "!": (0, 0, 95, 0, 0), "%": (35, 19, 8, 100, 98),
    "-": (8, 8, 8, 8, 8), ".": (0, 96, 96, 0, 0), "/": (32, 16, 8, 4, 2),
    ":": (0, 54, 54, 0, 0), "?": (2, 1, 81, 9, 6), "_": (64, 64, 64, 64, 64),
    "0": (62, 81, 73, 69, 62), "1": (0, 66, 127, 64, 0), "2": (66, 97, 81, 73, 70),
    "3": (33, 65, 69, 75, 49), "4": (24, 20, 18, 127, 16), "5": (39, 69, 69, 69, 57),
    "6": (60, 74, 73, 73, 48), "7": (1, 113, 9, 5, 3), "8": (54, 73, 73, 73, 54),
    "9": (6, 73, 73, 41, 30), "A": (126, 17, 17, 17, 126), "B": (127, 73, 73, 73, 54),
    "C": (62, 65, 65, 65, 34), "D": (127, 65, 65, 34, 28), "E": (127, 73, 73, 73, 65),
    "F": (127, 9, 9, 9, 1), "G": (62, 65, 73, 73, 122), "H": (127, 8, 8, 8, 127),
    "I": (0, 65, 127, 65, 0), "J": (32, 64, 65, 63, 1), "K": (127, 8, 20, 34, 65),
    "L": (127, 64, 64, 64, 64), "M": (127, 2, 12, 2, 127), "N": (127, 4, 8, 16, 127),
    "O": (62, 65, 65, 65, 62), "P": (127, 9, 9, 9, 6), "Q": (62, 65, 81, 33, 94),
    "R": (127, 9, 25, 41, 70), "S": (70, 73, 73, 73, 49), "T": (1, 1, 127, 1, 1),
    "U": (63, 64, 64, 64, 63), "V": (31, 32, 64, 32, 31), "W": (63, 64, 56, 64, 63),
    "X": (99, 20, 8, 20, 99), "Y": (3, 4, 120, 4, 3), "Z": (97, 81, 73, 69, 67),
}


class Frame:
    def __init__(self) -> None:
        self.pixels = bytearray(WIDTH * HEIGHT)

    def clear(self, gray: int = 0) -> None:
        if not 0 <= gray <= 15:
            raise ValueError("gray must be in 0..15")
        self.pixels[:] = bytes((gray,)) * len(self.pixels)

    def set_pixel(self, x: int, y: int, gray: int) -> None:
        if 0 <= x < WIDTH and 0 <= y < HEIGHT:
            self.pixels[y * WIDTH + x] = gray & 0x0F

    def text(self, x: int, y: int, value: str, gray: int = 15) -> None:
        if len(value) > 64:
            raise ValueError("text is bounded to 64 characters")
        for character in value.upper():
            glyph = FONT.get(character, FONT["?"])
            for column, bits in enumerate(glyph):
                for row in range(7):
                    if bits & (1 << row):
                        self.set_pixel(x + column, y + row, gray)
            x += 6

    def wire_bytes(self) -> bytes:
        # The retained driver treats the panel as 128 logical columns and duplicates
        # each four-bit logical pixel into the two nibbles of one SSD1322 data byte.
        return bytes((pixel << 4) | pixel for pixel in self.pixels)


class Ssd1322:
    def __init__(self, spi_path: Path, rotate: int) -> None:
        import gpiod
        import spidev

        resolved = Path(os.path.realpath(spi_path))
        match = re.fullmatch(r"spidev(\d+)\.(\d+)", resolved.name)
        if not match:
            raise RuntimeError(f"unexpected SPI device path: {resolved}")

        chip_path = self._find_gpiochip(gpiod)
        settings = gpiod.LineSettings(
            direction=gpiod.line.Direction.OUTPUT,
            output_value=gpiod.line.Value.INACTIVE,
        )
        self.lines = gpiod.request_lines(
            chip_path,
            consumer="shieldxl0-oled",
            config={5: settings, 6: settings},
        )
        self.gpiod = gpiod
        self.spi = spidev.SpiDev()
        self.spi.open(int(match.group(1)), int(match.group(2)))
        self.spi.max_speed_hz = 16_000_000
        self.spi.mode = 0
        self.rotate = rotate
        self._reset()
        self._initialize()

    @staticmethod
    def _find_gpiochip(gpiod_module) -> str:
        admitted = []
        for candidate in sorted(glob.glob("/dev/gpiochip*")):
            chip = gpiod_module.Chip(candidate)
            try:
                label = chip.get_info().label
            finally:
                chip.close()
            if label in ("pinctrl-bcm2711", "pinctrl-rp1"):
                admitted.append(candidate)
        if len(admitted) != 1:
            raise RuntimeError(f"expected one admitted Pi GPIO chip, observed {admitted}")
        return admitted[0]

    def _line(self, offset: int, high: bool) -> None:
        value = self.gpiod.line.Value.ACTIVE if high else self.gpiod.line.Value.INACTIVE
        self.lines.set_value(offset, value)

    def _reset(self) -> None:
        self._line(6, True)
        time.sleep(0.001)
        self._line(6, False)
        time.sleep(0.010)
        self._line(6, True)
        time.sleep(0.010)

    def command(self, *values: int) -> None:
        self._line(5, False)
        self.spi.xfer2(list(values))

    def data(self, payload: bytes) -> None:
        self._line(5, True)
        for offset in range(0, len(payload), 4096):
            self.spi.xfer2(list(payload[offset : offset + 4096]))

    def _initialize(self) -> None:
        self.command(0xFD, 0x12)
        self.command(0xAE)
        self.command(0xB9)
        self.command(0xB3, 0x91)
        self.command(0xCA, 0x3F)
        self.command(0xA2, 0x00)
        self.command(0xA1, 0x00)
        self.command(0xA0, 0x04 if self.rotate == 180 else 0x16, 0x11)
        self.command(0xAB, 0x01)
        self.command(0xB4, 0xA0, 0xFD)
        self.command(0xC1, 0x7F)
        self.command(0xC7, 0x0F)
        self.command(0xB1, 0xF2)
        self.command(0xBB, 0x1F)
        self.command(0xBE, 0x04)
        self.command(0xA6)
        self.command(0xAF)

    def display(self, frame: Frame) -> None:
        self.command(0x15, 28, 91)
        self.command(0x75, 0, 63)
        self.command(0x5C)
        self.data(frame.wire_bytes())

    def close(self) -> None:
        try:
            frame = Frame()
            self.display(frame)
            self.command(0xAE)
        finally:
            self.spi.close()
            self.lines.release()


def render_request(frame: Frame, request: dict) -> None:
    operation = request.get("op")
    if operation == "clear":
        frame.clear()
    elif operation == "fill":
        frame.clear(int(request.get("gray", 15)))
    elif operation == "text":
        frame.clear()
        frame.text(int(request.get("x", 0)), int(request.get("y", 0)), str(request["text"]))
    elif operation == "status":
        frame.clear()
        preset = str(request.get("preset", "NO PRESET"))[:20]
        macros = request.get("macros", [])
        if not isinstance(macros, list) or len(macros) > 4:
            raise ValueError("macros must be a list of at most four values")
        frame.text(0, 0, preset)
        frame.text(0, 10, "M " + " ".join(str(value)[:4] for value in macros))
        frame.text(0, 20, f"CPU {request.get('cpu_percent', '?')}% GAP {request.get('missing_frames', '?')}")
        frame.text(0, 30, f"TEMP {request.get('temperature_c', '?')}C THR {request.get('throttled', '?')}")
    else:
        raise ValueError("unsupported operation")


def serve(display: Ssd1322, socket_path: Path) -> None:
    frame = Frame()
    frame.text(0, 0, "SHIELDXL0 READY")
    display.display(frame)
    socket_path.parent.mkdir(mode=0o750, parents=True, exist_ok=True)
    if socket_path.exists():
        socket_path.unlink()
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(str(socket_path))
    os.chmod(socket_path, 0o660)
    server.listen(4)
    server.settimeout(0.5)
    stopping = False
    last_update = 0.0

    def stop(_signum, _frame):
        nonlocal stopping
        stopping = True

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    try:
        while not stopping:
            try:
                connection, _ = server.accept()
            except TimeoutError:
                continue
            with connection:
                connection.settimeout(1.0)
                payload = connection.recv(MAX_REQUEST + 1)
                try:
                    if len(payload) > MAX_REQUEST:
                        raise ValueError("request exceeds 4096 bytes")
                    request = json.loads(payload.decode("utf-8"))
                    if not isinstance(request, dict):
                        raise ValueError("request must be an object")
                    render_request(frame, request)
                    delay = (1.0 / MAX_HZ) - (time.monotonic() - last_update)
                    if delay > 0:
                        time.sleep(delay)
                    display.display(frame)
                    last_update = time.monotonic()
                    reply = {"ok": True, "width": WIDTH, "height": HEIGHT}
                except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
                    reply = {"ok": False, "error": str(error)}
                connection.sendall((json.dumps(reply, sort_keys=True) + "\n").encode("utf-8"))
    finally:
        server.close()
        if socket_path.exists():
            socket_path.unlink()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--socket", type=Path, default=Path("/run/shieldxl0/oled.sock"))
    parser.add_argument("--spi", type=Path, default=Path("/dev/shieldxl-oled-spi"))
    parser.add_argument("--rotate", type=int, choices=(0, 180), default=180)
    args = parser.parse_args()
    if not args.spi.exists():
        print(f"missing stable OLED SPI alias: {args.spi}", file=sys.stderr)
        return 1
    try:
        display = Ssd1322(args.spi, args.rotate)
        serve(display, args.socket)
    except Exception as error:
        print(f"OLED service failed: {error}", file=sys.stderr)
        return 1
    finally:
        if "display" in locals():
            display.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
