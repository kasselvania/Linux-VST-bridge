#!/usr/bin/env python3
"""Observe the exact Pigments X11 window in the existing Pi display session.

No new screen-sharing server or window is created. Discovery is a bounded exact
title match; the RPI1_EDITOR target_x11 journal record must be compared with
the observed window ID before treating this as the vendor editor timeline.
"""
import argparse
import ctypes as c
import json
import re
import subprocess
import time


TITLE = re.compile(r'^\s*(0x[0-9a-f]+) "Pigments":', re.MULTILINE)
STRUCTURE_NOTIFY = 1 << 17
FOCUS_CHANGE = 1 << 21
MAP_NOTIFY, UNMAP_NOTIFY, DESTROY_NOTIFY = 19, 18, 17
FOCUS_IN, FOCUS_OUT = 9, 10
DAMAGE_REPORT_NONEMPTY = 2


class XAnyEvent(c.Structure):
    _fields_ = [("type", c.c_int), ("serial", c.c_ulong),
                ("send_event", c.c_int), ("display", c.c_void_p),
                ("window", c.c_ulong)]


class XRectangle(c.Structure):
    _fields_ = [("x", c.c_short), ("y", c.c_short),
                ("width", c.c_ushort), ("height", c.c_ushort)]


class DamageNotify(c.Structure):
    _fields_ = XAnyEvent._fields_ + [
        ("damage", c.c_ulong), ("timestamp", c.c_ulong),
        ("area", XRectangle), ("geometry", XRectangle)]


def exact_window(tree):
    matches = {int(value, 16) for value in TITLE.findall(tree)}
    if len(matches) > 1:
        raise ValueError("multiple exact Pigments window titles")
    return next(iter(matches), None)


def discover():
    result = subprocess.run(["xwininfo", "-root", "-tree"], capture_output=True,
                            text=True, timeout=2, check=True)
    return exact_window(result.stdout)


def window_info(window):
    result = subprocess.run(["xwininfo", "-id", f"0x{window:x}"],
                            capture_output=True, text=True, timeout=2)
    if result.returncode != 0:
        return None, None
    mapped = (True if "Map State: IsViewable" in result.stdout else
              False if "Map State: IsUnMapped" in result.stdout or
              "Map State: IsUnviewable" in result.stdout else None)
    size = []
    for label in ("Width", "Height"):
        match = re.search(r"^\s*" + label + r":\s*(\d+)\s*$", result.stdout, re.MULTILINE)
        size.append(int(match.group(1)) if match else None)
    return mapped, tuple(size) if all(value is not None for value in size) else None


def emit(name, window=0, **fields):
    print("RPI1_XDAMAGE " + json.dumps({
        "monotonic_ns": time.monotonic_ns(), "event": name,
        "window_id": f"0x{window:x}", **fields,
    }, separators=(",", ":")), flush=True)


def libraries():
    x = c.CDLL("libX11.so.6")
    d = c.CDLL("libXdamage.so.1")
    x.XOpenDisplay.argtypes = [c.c_char_p]
    x.XOpenDisplay.restype = c.c_void_p
    x.XCloseDisplay.argtypes = [c.c_void_p]
    x.XSelectInput.argtypes = [c.c_void_p, c.c_ulong, c.c_long]
    x.XSelectInput.restype = c.c_int
    x.XPending.argtypes = [c.c_void_p]
    x.XPending.restype = c.c_int
    x.XFlush.argtypes = [c.c_void_p]
    x.XNextEvent.argtypes = [c.c_void_p, c.c_void_p]
    d.XDamageQueryExtension.argtypes = [c.c_void_p, c.POINTER(c.c_int), c.POINTER(c.c_int)]
    d.XDamageQueryExtension.restype = c.c_int
    d.XDamageCreate.argtypes = [c.c_void_p, c.c_ulong, c.c_int]
    d.XDamageCreate.restype = c.c_ulong
    d.XDamageSubtract.argtypes = [c.c_void_p, c.c_ulong, c.c_ulong, c.c_ulong]
    d.XDamageDestroy.argtypes = [c.c_void_p, c.c_ulong]
    return x, d


def observe(display_name, duration):
    x, d = libraries()
    display = x.XOpenDisplay(display_name.encode())
    if not display:
        raise RuntimeError("existing X11 display unavailable")
    event_base, error_base = c.c_int(), c.c_int()
    if not d.XDamageQueryExtension(display, c.byref(event_base), c.byref(error_base)):
        x.XCloseDisplay(display)
        raise RuntimeError("X Damage unavailable")
    emit("observer_ready", event_base=event_base.value)
    window = damage = 0
    mapped = None
    first_damage = None
    last_damage = None
    damage_count = area_total = 0
    window_start = time.monotonic_ns()
    last_size = None
    stable_count = 0
    stable_reported = False
    end = time.monotonic() + duration
    last_probe = last_report = 0.
    try:
        while time.monotonic() < end:
            now = time.monotonic()
            if not window and now - last_probe >= 0.25:
                last_probe = now
                candidate = discover()
                if candidate:
                    window = candidate
                    x.XSelectInput(display, window, STRUCTURE_NOTIFY | FOCUS_CHANGE)
                    damage = d.XDamageCreate(display, window, DAMAGE_REPORT_NONEMPTY)
                    if not damage:
                        raise RuntimeError("XDamageCreate failed")
                    window_start = time.monotonic_ns()
                    first_damage = last_damage = None
                    mapped, last_size = window_info(window)
                    stable_count = 1 if last_size else 0
                    stable_reported = False
                    emit("window_discovered", window,
                         mapped=mapped, size=last_size,
                         map_time_exact=False, discovery_poll_ms=250)
            while x.XPending(display):
                buffer = c.create_string_buffer(192)
                x.XNextEvent(display, buffer)
                any_event = c.cast(buffer, c.POINTER(XAnyEvent)).contents
                if not window or any_event.window != window:
                    continue
                if any_event.type == event_base.value:
                    damage_event = c.cast(buffer, c.POINTER(DamageNotify)).contents
                    area = int(damage_event.area.width) * int(damage_event.area.height)
                    is_first = first_damage is None
                    first_damage = first_damage or time.monotonic_ns()
                    last_damage = time.monotonic_ns()
                    damage_count += 1
                    area_total += area
                    if is_first:
                        emit("first_observed_damage", window,
                             since_discovery_ns=last_damage - window_start,
                             first_ever_unproven=True)
                    d.XDamageSubtract(display, damage, 0, 0)
                    x.XFlush(display)
                elif any_event.type in (MAP_NOTIFY, UNMAP_NOTIFY, DESTROY_NOTIFY,
                                        FOCUS_IN, FOCUS_OUT):
                    name = {MAP_NOTIFY:"mapped", UNMAP_NOTIFY:"unmapped",
                            DESTROY_NOTIFY:"destroyed", FOCUS_IN:"focus_in",
                            FOCUS_OUT:"focus_out"}[any_event.type]
                    emit(name, window)
                    if any_event.type == MAP_NOTIFY:
                        mapped = True
                    elif any_event.type == UNMAP_NOTIFY:
                        mapped = False
                    elif any_event.type == DESTROY_NOTIFY:
                        d.XDamageDestroy(display, damage)
                        window = damage = 0
                        mapped = False
            if now - last_report >= 1:
                last_report = now
                if window:
                    mapped_now, size = window_info(window)
                    if mapped_now is not None:
                        mapped = mapped_now
                    stable_count = (stable_count + 1 if size == last_size else 1) if size else 0
                    last_size = size
                    if stable_count >= 3 and not stable_reported:
                        emit("stable_geometry", window, size=size)
                        stable_reported = True
                    emit("damage_interval", window, mapped=mapped,
                         size=size, events=damage_count, bounding_area_sum=area_total,
                         no_damage_ns=(time.monotonic_ns() - last_damage) if last_damage else None)
                    damage_count = area_total = 0
            time.sleep(0.02)
    finally:
        if damage:
            d.XDamageDestroy(display, damage)
        x.XCloseDisplay(display)
        emit("observer_retired")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--display", default=":1")
    parser.add_argument("--duration", type=int, default=900)
    arguments = parser.parse_args()
    if not 1 <= arguments.duration <= 3600:
        parser.error("duration must be 1..3600 seconds")
    observe(arguments.display, arguments.duration)
