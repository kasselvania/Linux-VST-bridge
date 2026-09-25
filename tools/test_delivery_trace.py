#!/usr/bin/env python3
"""Exercise the actual fixed Windows history storage without a plug-in."""
import json
import os
import pathlib
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]
HEADER = ROOT / 'windows-factory-probe/source/delivery_trace.h'
SHIM = r'''
#include <cstdint>
using DWORD=unsigned long;
using ULONG64=unsigned long long;
using HANDLE=void*;
struct FILETIME { unsigned long dwLowDateTime=0,dwHighDateTime=0; };
struct LARGE_INTEGER { long long QuadPart; };
DWORD GetEnvironmentVariableW(const wchar_t*,wchar_t*,DWORD){return 0;}
bool QueryPerformanceCounter(LARGE_INTEGER* t){static long long clock=0;t->QuadPart=++clock;return true;}
bool QueryPerformanceFrequency(LARGE_INTEGER* t){t->QuadPart=1000;return true;}
HANDLE GetCurrentThread(){return nullptr;}
HANDLE GetCurrentProcess(){return nullptr;}
bool GetThreadTimes(HANDLE,FILETIME*,FILETIME*,FILETIME*,FILETIME*){return false;}
bool GetProcessTimes(HANDLE,FILETIME*,FILETIME*,FILETIME*,FILETIME*){return false;}
bool QueryThreadCycleTime(HANDLE,ULONG64*){return false;}
'''
TEST = r'''
#include <cassert>
int main(){
 using linux_vst_bridge::wf0::DeliveryTrace;
 DeliveryTrace d;d.enabled=true;d.frequency=1000;
 for(int i=0;i<1000;++i)d.complete();
 assert(d.count==0&&d.available==0);
 d.armed=true;
 for(uint64_t i=0;i<2000;++i){
  d.current={};d.current.sequence=i;
  d.current.at[0]=i*100;
  d.current.at[1]=i*100+((i==80||i%200==0)?20:2);
  d.current.at[7]=d.current.at[1]+2;
  if(i==80||i==180){
   d.current.note_on_count=1;d.current.event_count=1;
   d.current.note_pitch=i==80?60:64;d.current.note_channel=0;
   d.current.before.thread_user=10;d.current.after.thread_user=50;
  }
  d.complete();
  assert(d.count<=d.retained.size());
 }
 // First trigger at zero has no prior records. Subsequent windows contain the
 // available predecessors and 33 current/following records; the cap never grows.
 assert(d.retained[0].sequence==0);
 assert(d.retained[32].sequence==32);
 assert(d.retained[33].sequence==16);
 assert(d.triggers==4&&d.count==324);
 assert(d.following==0);
 assert(d.note_count==2&&d.note_rows[0].sequence==80&&d.note_rows[1].sequence==180);
 linux_vst_bridge::wf0::EventWriter events(200000);
 d.dump(events);
 d.dump(events); // exactly once
 assert(events.sequence()==d.count+3);
}
'''

def main():
    with tempfile.TemporaryDirectory(prefix='ap10-history-') as directory:
        root=pathlib.Path(directory);unit=root/'history.cpp';binary=root/('history.exe' if os.name=='nt' else 'history')
        header=HEADER.read_text()
        if os.name!='nt':header=header.replace('#include <windows.h>',SHIM)
        (root/'history.h').write_text(header)
        unit.write_text('#include "history.h"\n'+TEST)
        include=ROOT/'windows-factory-probe/include'
        command=(['cl','/nologo','/std:c++20','/EHsc','/DNOMINMAX',f'/I{include}',str(unit),f'/Fe:{binary}'] if os.name=='nt' else ['c++','-std=c++20','-Wall','-Wextra','-Werror','-I',str(include),str(unit),'-o',str(binary)])
        subprocess.run(command,cwd=root,check=True)
        output=subprocess.check_output([str(binary)],text=True)
        def unique(pairs):
            result={}
            for key,value in pairs:
                assert key not in result, f'duplicate protocol field {key}'
                result[key]=value
            return result
        rows=[json.loads(line,object_pairs_hook=unique) for line in output.splitlines()]
        assert [row['sequence'] for row in rows]==list(range(1,328))
        assert rows[0]['retained']==324
        assert rows[1]['request_sequence']==0
        assert all(row['state']=='ap10_windows_request' for row in rows[1:-2])
        assert [row['state'] for row in rows[-2:]]==['fn1_note_process']*2
        assert [row['pitch'] for row in rows[-2:]]==[60,64]
        assert [row['caller_user_100ns'] for row in rows[-2:]]==[40,40]
    print('Fixed Windows history bounds and startup filtering passed')

if __name__=='__main__':main()
