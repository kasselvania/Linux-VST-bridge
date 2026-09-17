"""One sealed delayed application case; no vendor or installed entry."""
from application_campaign import run
if __name__=='__main__':
 import sys
 run(*sys.argv[1:],cases=('lifetime',))
