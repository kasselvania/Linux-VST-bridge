"""MF1 focus selection refuses unowned/ambiguous/recycled main identities.

No X server is opened for any rejected process identity.
"""
import unittest
from unittest.mock import patch
import session

class Scope:
    def __init__(self, members): self.records=members
    def members(self):return self.records

class OperatorFocus(unittest.TestCase):
    def test_only_unique_registered_main_reaches_the_x11_boundary(self):
        app={'executable':{'path':'exact-main','sha256':'a'}}
        for records,images in [([],[]),([{'pid':1}],[]),([{'pid':1},{'pid':2}],[{'path':'exact-main','sha256':'a'}]),([{'pid':1}],[{'path':'exact-main','sha256':'different'}])]:
            with self.subTest(records=records,images=images),patch.object(session,'vendor_process_metadata',return_value={'registered_images':images}),patch.object(session.ctypes,'CDLL',side_effect=AssertionError('must not open X11')):
                with self.assertRaises(RuntimeError):session.vendor_focus(Scope(records),app)

if __name__=='__main__':unittest.main()
