#!/bin/sh
# Launched only by the private headless KWin session.
exec /usr/bin/python3 "$(dirname "$0")/session.py" "$UIR1_ROOT" "$UIR1_PACKAGE"
