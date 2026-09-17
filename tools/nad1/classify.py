"""Closed dependency state law. A listener or helper exit is never readiness."""
FIELDS={'schema','complete','installer','daemon','registration','service','process','readiness','foreign'}
def classify(v):
    if not isinstance(v,dict) or set(v)!=FIELDS or type(v['schema']) is not int or v['schema']!=1:raise ValueError('nad1_state_schema')
    if type(v['complete']) is not bool:raise ValueError('nad1_complete_type')
    enums={'installer':{'absent','exact','ambiguous'},'daemon':{'absent','exact','ambiguous'},'registration':{'absent','exact','mismatch','unavailable'},'service':{'absent','stopped','running','unavailable'},'process':{'absent','exact','unavailable'},'readiness':{'unavailable','not_ready','exact'},'foreign':{'absent','exact','unavailable'}}
    for k,values in enums.items():
        if not isinstance(v[k],str) or v[k] not in values:raise ValueError('nad1_state_field')
    if not v['complete'] or v['installer']=='ambiguous' or v['daemon']=='ambiguous' or v['foreign']=='unavailable' or v['registration'] in ('mismatch','unavailable'):return 'NAD1_IDENTITY_UNRESOLVED'
    if v['foreign']=='exact':return 'NAD1_FOREIGN_DAEMON_CONFLICT'
    if v['daemon']=='absent':return 'NAD1_DEPENDENCY_ABSENT'
    if v['registration']=='absent':return 'NAD1_SERVICE_UNREGISTERED'
    if v['process']=='unavailable':return 'NAD1_IDENTITY_UNRESOLVED'
    if v['service']=='stopped' and v['process']=='absent':return 'NAD1_SERVICE_STOPPED'
    if v['service']=='running' and v['process']=='exact':
        return 'NAD1_DEPENDENCY_READY' if v['readiness']=='exact' else 'NAD1_SERVICE_RUNNING_NOT_READY'
    return 'NAD1_IDENTITY_UNRESOLVED'
