#!/usr/bin/env python3
"""Merge only Pi-monotonic RPI1 evidence; derive bounded phase summaries.

The Mac observer's wall clock is intentionally excluded. A missing record is
reported as missing evidence, never filled by inference from another domain.
"""
import argparse
from collections import defaultdict, deque
import json
import os
from pathlib import Path


AP10_STAGES = ("queued", "started", "prepared", "sent", "replied",
               "validated", "published")


def records(path):
    with Path(path).open() as stream:
        for line in stream:
            text = line.strip()
            source = None
            for prefix in ("RPI1_HEALTH ", "RPI1_HEALTH_EVENT ", "RPI1_THREAD ",
                           "RPI1_XDAMAGE "):
                if text.startswith(prefix):
                    text = text[len(prefix):]
                    source = prefix.strip().lower()
                    break
            try:
                value = json.loads(text)
            except json.JSONDecodeError:
                continue
            if not isinstance(value, dict):
                continue
            if source:
                value.setdefault("event", source)
            if value.get("event") == "ap10_linux_request":
                times = value.get("monotonic_ns", [])
                for stage, stamp in zip(AP10_STAGES, times):
                    if isinstance(stamp, int) and stamp > 0:
                        yield {"event":"ap10_" + stage, "monotonic_ns":stamp,
                               "domain":"native_bridge_worker",
                               "bridge_position":value.get("position"),
                               "sequence":value.get("sequence")}
                continue
            if value.get("event") == "ap11_gap_clock":
                value["monotonic_ns"] = value.get("gap_monotonic_ns")
            if "monotonic_ns" not in value and "monotonic_seconds" in value:
                value["monotonic_ns"] = round(value["monotonic_seconds"] * 1e9)
            stamp = value.get("monotonic_ns")
            if isinstance(stamp, int) and stamp > 0:
                yield value


def percentile(values, fraction):
    if not values:
        return None
    values = sorted(values)
    return values[min(len(values)-1, max(0, int(len(values)*fraction + 0.999999)-1))]


def distribution(values):
    return {"count":len(values), "p50_ns":percentile(values, .50),
            "p95_ns":percentile(values, .95), "p99_ns":percentile(values, .99),
            "max_ns":max(values) if values else None}


def analyze(rows):
    enter = {}
    jack_cycle = {}
    slot = defaultdict(deque)
    publish = {}
    prepared = defaultdict(deque)
    sent = defaultdict(deque)
    elapsed = defaultdict(list)
    gap_run = longest_gap_run = 0
    last_exit = None
    over_period = over_two = over_100ms = 0
    period_ns = 512 * 1_000_000_000 // 48_000
    for row in rows:
        if row.get("event") != "rpi1_phase":
            continue
        kind = row.get("phase")
        callback = row.get("callback_sequence")
        position = row.get("bridge_position")
        stamp = row["monotonic_ns"]
        key = (callback, position)
        if kind == "callback_enter":
            enter[callback] = stamp
        elif kind == "callback_exit":
            if callback in enter and stamp >= enter[callback]:
                duration = stamp - enter.pop(callback)
                elapsed["callback_duration_ns"].append(duration)
                over_period += duration > period_ns
                over_two += duration > 2 * period_ns
                over_100ms += duration > 100_000_000
            missing = row.get("value_2", 0)
            if missing:
                gap_run = gap_run + 1 if last_exit is None or callback == last_exit + 1 else 1
                longest_gap_run = max(longest_gap_run, gap_run)
            else:
                gap_run = 0
            last_exit = callback
        elif kind == "jack_cycle" and row.get("detail") == 0:
            expected, actual = row.get("value_1"), row.get("value_2")
            if isinstance(expected, int) and isinstance(actual, int) and actual >= expected:
                jack_cycle[callback] = actual
                elapsed["jack_entry_lateness_ns"].append((actual - expected) * 1000)
        elif kind == "jack_next" and callback in jack_cycle:
            next_usec = row.get("value_1")
            if isinstance(next_usec, int):
                elapsed["jack_time_remaining_ns"].append(
                    max(0, next_usec - jack_cycle.pop(callback)) * 1000)
        elif kind == "request_slot_inspect":
            slot[key].append(stamp)
        elif kind == "request_published":
            if slot[key]:
                elapsed["request_publication_ns"].append(stamp - slot[key].popleft())
            publish[row.get("value_1")] = stamp
        elif kind == "worker_request_observed":
            published = publish.pop(row.get("value_1"), None)
            if published is not None and stamp >= published:
                elapsed["worker_queue_delay_ns"].append(stamp - published)
        elif kind == "worker_prepared":
            prepared[key].append(stamp)
        elif kind == "worker_sent":
            if prepared[key]:
                elapsed["worker_send_ns"].append(stamp - prepared[key].popleft())
            sent[key].append(stamp)
        elif kind == "worker_replied":
            if sent[key]:
                elapsed["host_reply_ns"].append(stamp - sent[key].popleft())
            duration = row.get("value_2")
            if isinstance(duration, int) and duration > 0:
                elapsed["windows_process_ns"].append(duration)
    return {"phase_distributions":{key:distribution(value)
            for key, value in elapsed.items()},
            "callback_over_one_period":over_period,
            "callback_over_two_periods":over_two,
            "callback_over_100ms":over_100ms,
            "longest_consecutive_missing_callbacks":longest_gap_run,
            "phase_events":sum(row.get("event") == "rpi1_phase" for row in rows),
            "phase_drops":{str(producer):max(counts) for producer, counts in
                ((index, [row.get("count", 0) for row in rows
                          if row.get("event") == "rpi1_phase_dropped"
                          and row.get("producer") == index]) for index in (0, 1))
                if counts}}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--phase", required=True, type=Path)
    parser.add_argument("--health", type=Path)
    parser.add_argument("--xdamage", type=Path)
    parser.add_argument("--native", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    arguments = parser.parse_args()
    if arguments.output.exists():
        parser.error("output already exists")
    rows = []
    for path in (arguments.phase, arguments.health, arguments.xdamage, arguments.native):
        if path:
            rows.extend(records(path))
    rows.sort(key=lambda item:item["monotonic_ns"])
    summary = analyze(rows)
    fd = os.open(arguments.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w") as stream:
        for row in rows:
            stream.write(json.dumps(row, separators=(",", ":")) + "\n")
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
