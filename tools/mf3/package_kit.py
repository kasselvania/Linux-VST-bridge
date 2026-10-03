#!/usr/bin/env python3
"""Maintainer entry point for the reusable-engine kit.

Retained source-build recipes remain readable for recovery. New releases use
one engine built here; they never require a compiler on the user's host.
"""
from prebuilt_kit import main

if __name__ == "__main__":
    main()
