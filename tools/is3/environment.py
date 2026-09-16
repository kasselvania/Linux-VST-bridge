"""Private Windows environment receipt validation, never a loader conclusion."""
import re
P=re.compile(r'IS3_ENV_V1 mode=(baseline|unavailable|restored) origin=(expected|child) present=([01]) length=([0-9]+) sha256=([a-f0-9]{64}) duplicate_count=([0-9]+)')
def validate(lines):
    if len(lines)!=6:raise ValueError('environment_receipt_count')
    rows={}
    for line,expected in zip(lines,((m,o) for m in ('baseline','unavailable','restored') for o in ('expected','child'))):
        match=P.fullmatch(line)
        if not match or match.group(1,2)!=expected or len(line)>256:raise ValueError('environment_identity')
        m,o,p,n,h,d=match.groups()
        if int(d)!=0 or int(n)>32767 or (p=='0' and int(n)!=0):raise ValueError('environment_noncanonical')
        rows.setdefault(m,{})[o]={'present':p=='1','utf16_code_units':int(n),'sha256_utf16le':h,'duplicate_count':int(d)}
    for pair in rows.values():
        if pair['expected']!=pair['child']:raise ValueError('environment_delivery_mismatch')
    if rows['baseline']!=rows['restored']:raise ValueError('environment_not_restored')
    b=rows['baseline']['child'];u=rows['unavailable']['child']
    if not u['present'] or u['utf16_code_units']!=b['utf16_code_units']+(1 if b['utf16_code_units'] else 0)+len('powershell.exe=') or u['sha256_utf16le']==b['sha256_utf16le']:raise ValueError('environment_injection_missing')
    return {'schema':1,'authority':'windows_environment_delivery_only','rows':rows,'loader_authority':False}
