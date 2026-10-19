"""Real ELF negative cases for the replacement-driver admission check."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

import compatibility

@unittest.skipUnless(shutil.which('readelf') and shutil.which('gcc'), 'Linux ELF toolchain required')
class CompatibilityTest(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name)
        self.original=self.compile('original', 'extern int puts(const char *); int surface[2]; int call(void) { return puts("ok"); }')
    def compile(self,name,source,soname='driver.so',extra=()):
        c=self.root/(name+'.c');c.write_text(source)
        so=self.root/(name+'.so')
        subprocess.run(['gcc','-shared','-fPIC','-Wl,-soname,'+soname,str(c),'-o',str(so),*extra],check=True,capture_output=True)
        return so
    def test_different_implementation_same_interface(self):
        other=self.compile('changed','extern int puts(const char *); int surface[2]; int call(void) { return puts("different body") + 3; }')
        compatibility.compare(self.original,other)
    def test_changed_soname(self):
        other=self.compile('changed','extern int puts(const char *); int surface[2]; int call(void) { return puts("ok"); }',soname='wrong.so')
        with self.assertRaisesRegex(ValueError,'dynamic'):compatibility.compare(self.original,other)
    def test_changed_export(self):
        other=self.compile('changed','extern int puts(const char *); int renamed[2]; int call(void) { return puts("ok"); }')
        with self.assertRaisesRegex(ValueError,'exports'):compatibility.compare(self.original,other)
    def test_changed_exported_data_size(self):
        other=self.compile('changed','extern int puts(const char *); int surface[3]; int call(void) { return puts("ok"); }')
        with self.assertRaisesRegex(ValueError,'exports'):compatibility.compare(self.original,other)
    def test_added_import(self):
        other=self.compile('changed','extern int puts(const char *); extern int new_requirement(void); int surface[2]; int call(void) { return puts("ok") + new_requirement(); }')
        with self.assertRaisesRegex(ValueError,'imports'):compatibility.compare(self.original,other)
    def test_changed_required_symbol_version(self):
        version=self.root/'versions.map'
        source='extern int requirement(void); int surface[2]; int call(void) { return requirement(); }'
        def dependency(label):
            version.write_text(label + ' { global: requirement; };')
            self.compile('dependency','int requirement(void) { return 1; }',
                         soname='dependency.so',extra=('-Wl,--version-script='+str(version),))
        dependency('REQUIRED_1')
        old=self.compile('version_old',source,extra=(str(self.root/'dependency.so'),))
        dependency('REQUIRED_2')
        new=self.compile('version_new',source,extra=(str(self.root/'dependency.so'),))
        with self.assertRaisesRegex(ValueError,'imports'):compatibility.compare(old,new)
    def test_added_dependency(self):
        self.compile('dependency','int dependency(void) { return 1; }',soname='dependency.so')
        other=self.compile('changed','extern int puts(const char *); int surface[2]; int call(void) { return puts("ok"); }',extra=('-Wl,--no-as-needed',str(self.root/'dependency.so')))
        with self.assertRaisesRegex(ValueError,'dynamic'):compatibility.compare(self.original,other)
    def test_wrong_architecture(self):
        other=self.root/'wrong.so';data=bytearray(self.original.read_bytes());data[18:20]=(183).to_bytes(2,'little');other.write_bytes(data)
        with self.assertRaisesRegex(ValueError,'x86-64'):compatibility.compare(self.original,other)
    def test_added_dynamic_library(self):
        other=self.compile('changed','extern int puts(const char *); int surface[2]; int call(void) { return puts("libunexpected.so.1"); }')
        with self.assertRaisesRegex(ValueError,'library_names'):compatibility.compare(self.original,other)

if __name__=='__main__':unittest.main()
