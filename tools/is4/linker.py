#!/usr/bin/env python3
"""Fixed source-owned Linux helper link recipe; never installed."""
import os,sys
os.execv(os.environ['IS4_ZIG'],[os.environ['IS4_ZIG'],'cc','-target','x86_64-linux-gnu',*sys.argv[1:]])
