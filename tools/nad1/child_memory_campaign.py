"""One disposable cold-versus-service-first child memory comparison."""
from application_campaign import run
if __name__=='__main__':
 import sys
 run(*sys.argv[1:],cases=('child_memory',))
