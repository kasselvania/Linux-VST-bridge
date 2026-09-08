#!/usr/bin/env python3
"""Test bounded owner-thread automation delivery and a drained state barrier."""
import pathlib, subprocess, tempfile
ROOT = pathlib.Path(__file__).resolve().parents[1]
TEST = r'''
#include "controller_updates.h"
#include <cassert>
#include <limits>
#include <thread>
using linux_vst_bridge::wf0::ControllerUpdates;
int main() {
    ControllerUpdates q;
    uint32_t ids[]{70, 1, 99999};
    assert(q.configure(ids));
    assert(!q.publish(2, .5));
    assert(!q.publish(1, std::numeric_limits<double>::quiet_NaN()));
    assert(q.publish(1, 0)); assert(q.publish(1, 1));
    assert(q.publish(99999, .25));
    int calls = 0;
    assert(q.drain([&](uint32_t id, double v) {
        ++calls; assert((id==1 && v==1) || (id==99999 && v==.25)); return true;
    }));
    assert(calls==2);
    assert(q.drain([&](uint32_t, double) { assert(false); return true; }));
    std::atomic<bool> done{false}; double last = 0;
    std::thread producer([&] {
        for(int i=0;i<=100000;++i) assert(q.publish(70,double(i)/100000));
        done.store(true,std::memory_order_release);
    });
    auto apply=[&](uint32_t id,double value) {assert(id==70 && value>=last);last=value;return true;};
    while(!done.load(std::memory_order_acquire)) assert(q.drain(apply));
    producer.join(); assert(q.drain(apply)); assert(last==1);
    assert(q.publish(1,.4)); assert(!q.drain([](uint32_t,double){return false;}));
    ControllerUpdates bad; uint32_t duplicates[]{1,1}; assert(!bad.configure(duplicates));
}
'''
with tempfile.TemporaryDirectory(prefix='ap10-controller-') as d:
    p = pathlib.Path(d); (p/'test.cpp').write_text(TEST)
    subprocess.run(['c++', '-std=c++20', '-pthread', '-Wall', '-Wextra', '-Werror',
                    '-I'+str(ROOT/'windows-factory-probe/source'), str(p/'test.cpp'),
                    '-o', str(p/'test')], check=True)
    subprocess.run([str(p/'test')], check=True)
print('Controller updates preserve final values across concurrent drain and state barrier')
