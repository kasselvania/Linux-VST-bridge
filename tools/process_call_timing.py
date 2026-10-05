"""Bounded offline reader for native process-call observation schema1.

Retain the source binary: every call is present there, including refused/unwound
calls and actual zero/partial N. Summaries do not establish audio deadlines.
"""
import argparse
import collections
import hashlib
import json
import math
import pathlib
import struct

SUMMARY = struct.Struct('<6I11Q')
RECORD = struct.Struct('<II6QdqII6iII')
CAPACITY = 262144
SUMMARY_FIELDS = ('schema size capacity namespace_pid record_size flags instance offered retained '
                  'capacity_dropped contention_dropped allocation_dropped invalid_clocks '
                  'invalid_identity after_seal unfinished_writers sequence_overflow').split()
FIELDS = ('schema size instance sequence entry_ns return_ns backend_handle configuration sample_rate '
          'project_samples namespace_pid namespace_tid frames mode precision maximum phase sdk_result '
          'valid outcome').split()
Call = collections.namedtuple('Call', FIELDS)


def read(path):
    path = pathlib.Path(path)
    extent = path.stat().st_size
    if extent < 128 or extent > 128 + CAPACITY * RECORD.size:
        raise ValueError('observation extent outside prepared capacity')
    with path.open('rb') as stream:
        if stream.read(16) != b'LVBPC001' + struct.pack('<II', 128, RECORD.size):
            raise ValueError('observation header/encoding')
        summary = dict(zip(SUMMARY_FIELDS, SUMMARY.unpack(stream.read(SUMMARY.size))))
        if (summary['schema'] != 1 or summary['size'] != SUMMARY.size or
                summary['record_size'] != RECORD.size or summary['capacity'] > CAPACITY or
                summary['retained'] > summary['capacity'] or summary['flags'] & ~15 or
                summary['flags'] & 5 != 5 or summary['unfinished_writers'] or
                extent != 128 + summary['retained'] * RECORD.size):
            raise ValueError('observation summary/extent/ownership')
        rows = []
        for _ in range(summary['retained']):
            row = Call(*RECORD.unpack(stream.read(RECORD.size)))
            if (row.schema != 1 or row.size != RECORD.size or row.instance != summary['instance'] or
                    row.namespace_pid != summary['namespace_pid'] or row.valid & ~255 or
                    row.outcome not in (1, 2) or bool(row.valid & 64) != (row.outcome == 1)):
                raise ValueError('observation record/schema/identity/outcome')
            rows.append(row)
    if len({row.sequence for row in rows}) != len(rows):
        if not summary['sequence_overflow']:
            raise ValueError('unreported entry sequence collision')
    if summary['flags'] & 8:
        losses = ('capacity_dropped contention_dropped allocation_dropped invalid_clocks '
                  'invalid_identity after_seal unfinished_writers sequence_overflow').split()
        if (not summary['flags'] & 2 or not summary['instance'] or
                summary['offered'] != len(rows) or any(summary[key] for key in losses) or
                sorted(row.sequence for row in rows) != list(range(len(rows))) or
                any(row.valid & 39 != 39 or not row.namespace_tid or not row.entry_ns or
                    row.return_ns < row.entry_ns for row in rows)):
            raise ValueError('false complete-population claim')
    return summary, rows


def distribution(values):
    values = sorted(values)
    return dict(count=len(values), median_ns=values[len(values)//2],
                p99_ns=values[((len(values)-1)*99)//100], max_ns=values[-1]) if values else dict(count=0)


def export_custody(report, summary):
    """Existing lifecycle signal, not inferred durability from binary extent."""
    if report is None:
        return None
    report = pathlib.Path(report)
    if report.stat().st_size > 8*1024*1024:
        raise ValueError('lifecycle report outside bounded read allowance')
    matched = []
    with report.open() as stream:
        for line in stream:
            try:
                event = json.loads(line)
            except ValueError:
                continue
            if (isinstance(event, dict) and event.get('event') == 'native_process_call_summary' and
                    event.get('instance') == summary['instance'] and
                    event.get('namespace_pid') == summary['namespace_pid']):
                fields = ('flags capacity offered retained capacity_dropped contention_dropped '
                          'allocation_dropped invalid_clocks invalid_identity after_seal '
                          'unfinished_writers sequence_overflow').split()
                if any(event.get(key) != summary[key] for key in fields):
                    raise ValueError('lifecycle observation summary mismatch')
                status = event.get('export_status')
                if not isinstance(status, int) or isinstance(status, bool) or not 0 <= status <= 0xffffffff:
                    raise ValueError('lifecycle export status unavailable')
                matched.append(status == 0)
    if len(matched) > 1:
        raise ValueError('ambiguous lifecycle export custody')
    return matched[0] if matched else None


def analyze(summary, rows, start=None, end=None, export_confirmed=None):
    if (start is None) != (end is None) or (start is not None and end <= start):
        raise ValueError('paired increasing monotonic window required')
    selected = [row for row in rows if start is None or start <= row.entry_ns < end]
    valid = [row for row in selected if row.valid & 7 == 7 and row.entry_ns and row.return_ns >= row.entry_ns]
    cadence = []
    for row in valid:
        if row.frames > 0 and row.valid & 8 and math.isfinite(row.sample_rate) and row.sample_rate > 0:
            numerator, denominator = row.sample_rate.as_integer_ratio()
            cadence.append((row.sequence, (row.return_ns-row.entry_ns)*numerator > row.frames*1000000000*denominator))
    counts = collections.Counter(row.frames for row in selected)
    values = collections.defaultdict(list)
    for row in valid:
        values[row.frames].append(row.return_ns-row.entry_ns)
    groups = {str(n): dict(calls=counts[n], durations=distribution(values[n])) for n in sorted(counts)}
    population_complete = bool(summary['flags'] & 8)
    return dict(schema=1, record_population_complete=population_complete,
                export_confirmed=export_confirmed,
                dataset_complete=(export_confirmed if population_complete else False),
                retained_calls=len(rows), selected_entry_calls=len(selected), valid_duration_calls=len(valid),
                window=dict(start_ns=start, end_ns=end, membership='entry; crossing returns retained',
                            crossing_end_calls=sum(row.return_ns > end for row in selected) if end else 0),
                outcomes=dict(collections.Counter(str(row.outcome) for row in selected)),
                sdk_results=dict(collections.Counter(str(row.sdk_result) for row in selected if row.valid & 64)),
                durations=distribution([row.return_ns-row.entry_ns for row in valid]), actual_n=groups,
                cadence_comparable_calls=len(cadence), cadence_undefined_calls=len(selected)-len(cadence),
                cadence_exceeding_sequences=[sequence for sequence, exceeded in cadence if exceeded],
                method=dict(quantiles='sorted[count//2], sorted[floor((count-1)*0.99)]; no interpolation',
                            cadence='integer elapsed*rate_numerator>N*1e9*rate_denominator; not device deadline',
                            return_edge='after body/guard cleanup; publication and epilogue tail excluded',
                            observer='includes fixed calling-TID query and prepared capture work',
                            sequence='entry reservation order; not necessarily timestamp order under concurrency'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=pathlib.Path)
    parser.add_argument('--start-ns', type=int)
    parser.add_argument('--end-ns', type=int)
    parser.add_argument('--lifecycle-report', type=pathlib.Path)
    args = parser.parse_args()
    summary, rows = read(args.binary)
    confirmed = export_custody(args.lifecycle_report, summary)
    result = analyze(summary, rows, args.start_ns, args.end_ns, confirmed)
    result['source_sha256'] = hashlib.sha256(args.binary.read_bytes()).hexdigest()
    result['coverage'] = {key: value for key, value in summary.items() if key != 'namespace_pid'}
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
