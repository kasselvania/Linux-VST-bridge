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

class BuildFactsTest(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup)
        self.root=Path(self.tmp.name)
        def put(path,text):
            p=self.root/path;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
        self.put=put
        put('obj64/include/config.h','#define HAVE_X11_EXTENSIONS_XINPUT2_H 1\n')
        put('obj64/Makefile','\n'.join(name+' = test' for name in ('CC','CFLAGS','CPPFLAGS','LDFLAGS','X_CFLAGS','X_LIBS')))
        put('wine/dlls/winex11.drv/mouse.c','#ifdef HAVE_X11_EXTENSIONS_XINPUT2_H\n#endif\n')
        for name,version in (('gdi',111),('vulkan',49),('opengl',38)):
            put('wine/include/wine/'+name+'_driver.h','#define WINE_'+name.upper()+'_DRIVER_VERSION '+str(version)+'\n')
    def test_retained_header_versions(self):
        self.assertEqual(compatibility.build_facts(self.root)['driver_versions']['WINE_GDI_DRIVER_VERSION'],111)
    def test_changed_interface_version(self):
        self.put('wine/include/wine/gdi_driver.h','#define WINE_GDI_DRIVER_VERSION 112\n')
        with self.assertRaisesRegex(ValueError,'interface header'):compatibility.build_facts(self.root)
    def test_missing_xinput_feature(self):
        self.put('obj64/include/config.h','/* XInput2 absent */\n')
        with self.assertRaisesRegex(ValueError,'Missing X11 build feature'):compatibility.build_facts(self.root)
    def test_disabled_xinput_feature(self):
        self.put('obj64/include/config.h','#define HAVE_X11_EXTENSIONS_XINPUT2_H 0\n')
        with self.assertRaisesRegex(ValueError,'Missing X11 build feature'):compatibility.build_facts(self.root)
    def test_new_wayland_conditional_requires_review(self):
        self.put('wine/dlls/winex11.drv/mouse.c','#ifdef HAVE_LIBWAYLAND_EGL\n#endif\n')
        self.put('obj64/include/config.h','#define HAVE_LIBWAYLAND_EGL 1\n')
        with self.assertRaisesRegex(ValueError,'Wayland/FFmpeg'):compatibility.build_facts(self.root)

if __name__=='__main__':unittest.main()
