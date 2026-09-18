"""Compile the same bounded observation law used by the Windows adapter."""
import pathlib,shutil,subprocess,tempfile,unittest
class StopLogic(unittest.TestCase):
 def test_generated_scm_states_and_generation_waits(self):
  compiler=shutil.which('c++') or shutil.which('g++')
  self.assertIsNotNone(compiler,'C++ compiler required for the source-owned stop law')
  with tempfile.TemporaryDirectory() as tmp:
   exe=pathlib.Path(tmp)/'stop-test'
   subprocess.run([compiler,'-std=c++20','-Wall','-Wextra','-Werror',str(pathlib.Path(__file__).with_name('stop_logic_test.cpp')),'-o',str(exe)],check=True,timeout=30)
   subprocess.run([str(exe)],check=True,timeout=5)
