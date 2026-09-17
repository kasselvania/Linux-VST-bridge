"""Three disposable recovery operations; no real daemon or Native Access launch."""
from owner_campaign import run
if __name__=='__main__':
 import sys
 run(*sys.argv[1:],cases=('recovery','recovery_not_ready','recovery_cancel'))
