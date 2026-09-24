"""Reduce the bounded vendor-quantum comparison using existing audio/CPU readers."""
import argparse
import json
from pathlib import Path
from trace_analysis import audio, common_cpu, WINDOWS
from fex_analysis import held_process_cpu, zero_runs


def reduce(directory):
    raw=json.loads((directory/'comparison.json').read_text())
    out={'comparison':raw['comparison'],'errors':raw['errors'],
         'preference_restored_exact':raw.get('preference_restored_exact'),
         'guard_final':raw.get('guard_final'),'conditions':[],
         'limitations':[
             'One ordered 256/512/256 comparison; not randomized or sustained qualification.',
             'The same binary pair uses map v2 capacity512 in every condition. Default installed map v1 is unchanged.',
             'CPU windows use graph-ready stdout observation and interpolation; exact audio windows use capture frames.',
             'Counter deltas span the whole capture and control responses, not just the held chord. Completed frames count successful processing responses, not timely delivery.',
             'Process CPU sums surviving threads in the owned cohort. Short-lived threads may be absent; whole-cohort CPU is reported separately.',
             'Phase tracing is off, other existing transport and Windows diagnostics remain enabled. No scheduler wait or per-call DSP timing attribution.',
             'Exact zero can include preset behavior or onset latency. Report transport missing frames separately. No subjective listening or battery energy measurement.'
         ]}
    for c in raw['conditions']:
        session=c.get('session',{})
        row={'condition':c['condition'],'quantum':c.get('quantum'),
             'processing_readback':c.get('processing_readback'),
             'trials':[], 'restored_identity_matches':c.get('restored_identity')==raw['original_identity'],
             'restored_master_confirmed':any('normalized=0.48033079504966736 ' in v for v in c.get('restored_master',[])),
             'session':{k:session[k] for k in ['ready','ready_seconds','elapsed_seconds','exit_code','clean_shutdown'] if k in session},
             'drain_checks':[{'outstanding_frames_estimate':r['outstanding_frames_estimate'],'request_elapsed_ns':r['request_elapsed_ns']} for r in c.get('drain_checks',[])]}
        row['session'].update(temperature_peak_c=max((s['temperature_c'] for s in session.get('samples',[])),default=None),
            throttled_flags=sorted(set(s['throttled'] for s in session.get('samples',[]))),
            graph_restored=bool(session.get('jack_graph_after')) and not any('lvb-' in s or s.startswith(' ') for s in session['jack_graph_after'].splitlines()))
        setup=directory/(c['condition']+'-windows-setup.json')
        if setup.exists():row['windows_setup']=json.loads(setup.read_text())
        for t in c['trials']:
            trial={'name':t['name'],'completed':'after' in t}
            row['trials'].append(trial)
            if 'after' not in t:continue
            delta={k:t['after'][k]-t['before'][k] for k in ['bridge_processed','bridge_processed_frames','missing_frames','gaps','midi_accepted','xruns','process_failures','delivered_frames']}
            trial.update(counter_delta=delta,fixture_exit=t['fixture_exit'],
                completed_frames_per_call=delta['bridge_processed_frames']/delta['bridge_processed'] if delta['bridge_processed'] else None,
                actual_quantum_before=t['before'].get('processing_quantum'),actual_quantum_after=t['after'].get('processing_quantum'))
            capture=directory/(c['condition']+'-'+t['name']+'.f32le')
            trial['audio']=audio(capture);trial['zero_runs']=zero_runs(capture)
            anchor=t['graph_observed_ns']
            trial['common_wall_windows']={name:common_cpu(t['samples'],anchor+int(a*1e9),anchor+int(b*1e9)) for name,(a,b) in WINDOWS.items()}
            trial.update(held_process_cpu(t,c['pigments_pid']))
        out['conditions'].append(row)
    return out


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('directory',type=Path);args=parser.parse_args()
    result=reduce(args.directory)
    (args.directory/'reduced-quantum.json').write_text(json.dumps(result,indent=2)+'\n')
    for c in result['conditions']:
        for t in c['trials']:
            if not t['completed']:continue
            print(c['condition'],t['name'],json.dumps({'frames_per_call':t['completed_frames_per_call'],
                'counters':t['counter_delta'],'audio_held':t['audio']['windows']['held_including_attack'],
                'cpu':t['held_process_cpu_surviving_threads_one_core_percent']}))
