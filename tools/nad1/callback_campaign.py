"""Disposable opaque browser return through the real supervised Windows adapter."""
from application_campaign import run
if __name__=='__main__':
 import sys
 run(*sys.argv[1:],cases=('callback',))
