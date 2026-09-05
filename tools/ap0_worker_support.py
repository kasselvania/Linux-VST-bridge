"""AP0-specific mode and stream admission over the existing supervisor."""
import time
import supervise as inherited
from ap0_contract import MODE, CALLS, normalize, validate_summary
from ap0_artifacts import verify_host_store

def verify_host(root, identity):
    return verify_host_store(root,identity,expected_branch='codex/ap0-offline-again-processing',expected_input_count=19)

def command_vector(environment,session,component_case,mode):
    if mode != MODE:
        raise RuntimeError('AP0 command mode differs')
    vector=inherited.command_vector(environment,session,component_case,inherited.PC0_MODE)
    # The AP0 registry extends only this closed mode. All identity, transport,
    # stage, environment and executable coordinates retain the existing builder.
    vector[vector.index('--mode')+1]=MODE
    return vector

class StreamState(inherited.StreamState):
    def __init__(self):
        super().__init__()
        self.ap0_call=None
    def accept(self, record):
        state=record.get('state')
        if record.get('event')=='lifecycle' and str(state).startswith('ap0_'):
            if state in {'ap0_call_started','ap0_process_started'}:
                if self.ap0_call is not None or self.in_flight is not None:
                    raise RuntimeError('AP0 overlapping call')
                if state=='ap0_call_started' and record.get('operation') not in CALLS:
                    raise RuntimeError('AP0 operation differs')
                self.ap0_call=record
                # Use the inherited bound for an in-flight processing operation.
                self.in_flight_at=time.monotonic()
            elif state in {'ap0_call_completed','ap0_process_completed'}:
                expected='ap0_call_started' if state=='ap0_call_completed' else 'ap0_process_started'
                if (self.ap0_call is None or self.ap0_call.get('state')!=expected
                        or any(record.get(k)!=self.ap0_call.get(k) for k in ('operation','block'))):
                    raise RuntimeError('AP0 unpaired completion')
                self.ap0_call=None;self.in_flight_at=None
            super().accept(record)
        elif record.get('event')=='host_callback' and self.ap0_call is not None:
            start=self.ap0_call
            if (start.get('operation') not in {'set_active_true','set_active_false'}
                    or record.get('thread_role')!='scanner_main_thread'
                    or record.get('origin')!='component'
                    or record.get('enclosing_attempt_sequence')!=start['sequence']
                    or record.get('enclosing_operation')!=start['operation']
                    or record.get('operation')!='createInstance'
                    or record.get('result_u32_hex')!='00000000'
                    or record.get('output_null') is not False
                    or record.get('sequence')!=len(self.records)+1):
                raise RuntimeError('AP0 activation callback differs')
            self.host_callback_count+=1
            if self.host_callback_count>64 or len(self.records)>=inherited.EVENT_CAP:
                raise RuntimeError('AP0 callback/event bound exceeded')
            self.records.append(record)
        else:
            super().accept(record)
