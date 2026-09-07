#!/usr/bin/env python3
"""Exercise SDK input hints without rewriting host samples or hiding NaNs."""
import pathlib, subprocess, tempfile
ROOT = pathlib.Path(__file__).resolve().parents[1]
TEST = r'''
#include "input_silence.h"
#include <array>
#include <cassert>
#include <limits>
int main() {
  std::array<float,4> left{0.f, -0.f, std::bit_cast<float>(1u), 1e-7f};
  std::array<float,4> right{};
  const auto before = left;
  auto r = AP2::inputSilence(left.data(),right.data(),4,3);
  assert(r.flags==2 && r.nonzero==2 && r.first_bits==1u);
  for (int i=0;i<4;++i)
    assert(std::bit_cast<uint32_t>(left[i])==std::bit_cast<uint32_t>(before[i]));
  assert(AP2::inputSilence(right.data(),right.data(),4,3).flags==3);
  assert(AP2::inputSilence(left.data(),left.data(),4,0).nonzero==0);
  left[3]=0.25f;
  r=AP2::inputSilence(left.data(),right.data(),4,3);
  assert(r.flags==2 && r.finite_peak==0.25 && left[3]==0.25f);
  left[0]=std::numeric_limits<float>::quiet_NaN();
  r=AP2::inputSilence(left.data(),right.data(),4,3);
  assert(r.flags==2 && !std::isfinite(left[0])); // backend must still reject it
}
'''
with tempfile.TemporaryDirectory(prefix='ap10-silence-') as d:
    p=pathlib.Path(d); (p/'test.cpp').write_text(TEST)
    subprocess.run(['c++','-std=c++20','-Wall','-Wextra','-Werror',
                    '-I'+str(ROOT/'native-vst3-proxy/source'),str(p/'test.cpp'),
                    '-o',str(p/'test')],check=True)
    subprocess.run([str(p/'test')],check=True)
print('Input silence hints preserve near-zero, nonzero and invalid samples')
