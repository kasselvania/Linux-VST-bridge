# Extend the retained Debian builder inside a disposable product container.
# No host package installation; preserve its older glibc deployment baseline.
FROM lvb-beta-debian-builder:13.7-rust1.95
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev libxcursor-dev libasound2-dev libpulse-dev libwayland-dev \
    && rm -rf /var/lib/apt/lists/*
