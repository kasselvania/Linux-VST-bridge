#!/usr/bin/env python3
"""One UIO1 automatic drag and page click on the sealed UIR1 candidate.
Reuses the calibrated UIO1 fixture plan; no host-driven parameter probe, audio
campaign, UIA, Mac input, or retry. Inspect the exact private pre-action image
when preparing this plan; refuse changed geometry through Capture.
"""
import json
import pathlib
import signal
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent/'uio1'))
from launch import Context
from capture import Capture


def main():
    c = Context(pathlib.Path(sys.argv[1]), '41727475415649534B61743150726F63',
                pathlib.Path(sys.argv[2]), uir1=True)
    out = c.package/'uir1-confirmation'
    out.mkdir(mode=0o700)
    process, rows, snapshot = c.census(out/'census.log')
    capture = Capture(c, out, process, rows, snapshot)
    capture.result['scope'] = 'UIR1: idle diagnostic bracket, one Macro 1 drag, one page click; no host parameter update'
    signal.signal(signal.SIGTERM, lambda *_: setattr(capture, 'stop_requested', True))
    signal.signal(signal.SIGINT, lambda *_: setattr(capture, 'stop_requested', True))
    try:
        capture.run(refine=True)
    except BaseException as error:
        capture.result['failure_class'] = type(error).__name__
        capture.result['failure'] = str(error)
        raise
    finally:
        capture.close()
    print(json.dumps(dict(actions_complete=True, win32_records=len(capture.windows),
                          gui_records=len(capture.gestures), frames=len(capture.frames),
                          cleanup=capture.result.get('helper_cleanup'))))


if __name__ == '__main__':
    main()
