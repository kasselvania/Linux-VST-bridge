/* Source-owned x64 scalar/upper-lane fixture for the pinned ARM64EC FEX A/B. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <windows.h>

typedef void (*case_fn)(const uint32_t *, const uint32_t *, uint32_t *, uint32_t *);
typedef void (*double_case_fn)(const uint64_t *, const uint64_t *, uint64_t *, uint64_t *);

/* Loading YMM first makes legacy SSE lane retention and AVX high-half zeroing
 * visible in the stored result. The live form stores its first source after
 * the scalar operation. */
#define SCALAR_CASES(name, sse, avx)                                         \
  __attribute__((noinline)) static void name##_legacy(                       \
      const uint32_t *a, const uint32_t *b, uint32_t *out, uint32_t *live) { \
    (void)live;                                                               \
    __asm__ __volatile__("vmovups (%[a]), %%ymm0\n\t"                        \
                         "vmovups (%[b]), %%ymm1\n\t"                        \
                         sse " %%xmm1, %%xmm0\n\t"                           \
                         "vmovups %%ymm0, (%[out])\n\t"                     \
                         : : [a] "r"(a), [b] "r"(b), [out] "r"(out)             \
                         : "xmm0", "xmm1", "memory");                         \
  }                                                                           \
  __attribute__((noinline)) static void name##_alias(                        \
      const uint32_t *a, const uint32_t *b, uint32_t *out, uint32_t *live) { \
    (void)live;                                                               \
    __asm__ __volatile__("vmovups (%[a]), %%ymm0\n\t"                        \
                         "vmovups (%[b]), %%ymm1\n\t"                        \
                         avx " %%xmm1, %%xmm0, %%xmm0\n\t"                 \
                         "vmovups %%ymm0, (%[out])\n\t"                     \
                         : : [a] "r"(a), [b] "r"(b), [out] "r"(out)             \
                         : "xmm0", "xmm1", "memory");                         \
  }                                                                           \
  __attribute__((noinline)) static void name##_live(                         \
      const uint32_t *a, const uint32_t *b, uint32_t *out, uint32_t *live) { \
    __asm__ __volatile__("vmovups (%[a]), %%ymm0\n\t"                        \
                         "vmovups (%[b]), %%ymm1\n\t"                        \
                         avx " %%xmm1, %%xmm0, %%xmm2\n\t"                 \
                         "vmovups %%ymm2, (%[out])\n\t"                     \
                         "vmovups %%ymm0, (%[live])\n\t"                    \
                         : : [a] "r"(a), [b] "r"(b), [out] "r"(out),            \
                             [live] "r"(live)                                  \
                         : "xmm0", "xmm1", "xmm2", "memory");                 \
  }

SCALAR_CASES(add, "addss", "vaddss")
SCALAR_CASES(sub, "subss", "vsubss")
SCALAR_CASES(mul, "mulss", "vmulss")
SCALAR_CASES(div, "divss", "vdivss")

#define DOUBLE_CASES(name, sse, avx)                                          \
  __attribute__((noinline)) static void name##_legacy(                       \
      const uint64_t *a, const uint64_t *b, uint64_t *out, uint64_t *live) { \
    (void)live;                                                               \
    __asm__ __volatile__("vmovups (%[a]), %%ymm0\n\t"                        \
                         "vmovups (%[b]), %%ymm1\n\t"                        \
                         sse " %%xmm1, %%xmm0\n\t"                           \
                         "vmovups %%ymm0, (%[out])\n\t"                     \
                         : : [a] "r"(a), [b] "r"(b), [out] "r"(out)             \
                         : "xmm0", "xmm1", "memory");                         \
  }                                                                           \
  __attribute__((noinline)) static void name##_alias(                        \
      const uint64_t *a, const uint64_t *b, uint64_t *out, uint64_t *live) { \
    (void)live;                                                               \
    __asm__ __volatile__("vmovups (%[a]), %%ymm0\n\t"                        \
                         "vmovups (%[b]), %%ymm1\n\t"                        \
                         avx " %%xmm1, %%xmm0, %%xmm0\n\t"                 \
                         "vmovups %%ymm0, (%[out])\n\t"                     \
                         : : [a] "r"(a), [b] "r"(b), [out] "r"(out)             \
                         : "xmm0", "xmm1", "memory");                         \
  }                                                                           \
  __attribute__((noinline)) static void name##_live(                         \
      const uint64_t *a, const uint64_t *b, uint64_t *out, uint64_t *live) { \
    __asm__ __volatile__("vmovups (%[a]), %%ymm0\n\t"                        \
                         "vmovups (%[b]), %%ymm1\n\t"                        \
                         avx " %%xmm1, %%xmm0, %%xmm2\n\t"                 \
                         "vmovups %%ymm2, (%[out])\n\t"                     \
                         "vmovups %%ymm0, (%[live])\n\t"                    \
                         : : [a] "r"(a), [b] "r"(b), [out] "r"(out),            \
                             [live] "r"(live)                                  \
                         : "xmm0", "xmm1", "xmm2", "memory");                 \
  }

DOUBLE_CASES(dadd, "addsd", "vaddsd")
DOUBLE_CASES(dsub, "subsd", "vsubsd")
DOUBLE_CASES(dmul, "mulsd", "vmulsd")
DOUBLE_CASES(ddiv, "divsd", "vdivsd")

/* The ten extra vectors remain live across the scalar add. This intentionally
 * puts pressure on the JIT register allocator without depending on a vendor
 * address or any FEX-specific instruction. */
__attribute__((noinline)) static void pressured_add(
    const uint32_t *a, const uint32_t *b, const uint32_t *extra,
    uint32_t *out, uint32_t *live, uint32_t *extra_after) {
  __asm__ __volatile__(
      "vmovups (%[a]), %%ymm0\n\t"
      "vmovups (%[b]), %%ymm1\n\t"
      "vmovups 0(%[extra]), %%ymm3\n\t"
      "vmovups 32(%[extra]), %%ymm4\n\t"
      "vmovups 64(%[extra]), %%ymm5\n\t"
      "vmovups 96(%[extra]), %%ymm6\n\t"
      "vmovups 128(%[extra]), %%ymm7\n\t"
      "vmovups 160(%[extra]), %%ymm8\n\t"
      "vmovups 192(%[extra]), %%ymm9\n\t"
      "vmovups 224(%[extra]), %%ymm10\n\t"
      "vmovups 256(%[extra]), %%ymm11\n\t"
      "vmovups 288(%[extra]), %%ymm12\n\t"
      "vaddss %%xmm1, %%xmm0, %%xmm2\n\t"
      "vmovups %%ymm2, (%[out])\n\t"
      "vmovups %%ymm0, (%[live])\n\t"
      "vmovups %%ymm3, 0(%[after])\n\t"
      "vmovups %%ymm4, 32(%[after])\n\t"
      "vmovups %%ymm5, 64(%[after])\n\t"
      "vmovups %%ymm6, 96(%[after])\n\t"
      "vmovups %%ymm7, 128(%[after])\n\t"
      "vmovups %%ymm8, 160(%[after])\n\t"
      "vmovups %%ymm9, 192(%[after])\n\t"
      "vmovups %%ymm10, 224(%[after])\n\t"
      "vmovups %%ymm11, 256(%[after])\n\t"
      "vmovups %%ymm12, 288(%[after])\n\t"
      : : [a] "r"(a), [b] "r"(b), [extra] "r"(extra), [out] "r"(out),
          [live] "r"(live), [after] "r"(extra_after)
      : "xmm0", "xmm1", "xmm2", "xmm3", "xmm4", "xmm5", "xmm6", "xmm7",
        "xmm8", "xmm9", "xmm10", "xmm11", "xmm12", "memory");
}

static int check(const char *label, const uint32_t *actual,
                 const uint32_t *expected, unsigned count) {
  for (unsigned lane = 0; lane < count; ++lane) {
    if (actual[lane] != expected[lane]) {
      fprintf(stderr, "%s lane %u: got %08x expected %08x\n", label, lane,
              actual[lane], expected[lane]);
      return 1;
    }
  }
  return 0;
}

static int is_nan_bits(uint32_t bits) {
  return (bits & 0x7f800000U) == 0x7f800000U &&
         (bits & 0x007fffffU) != 0;
}

static int check_double(const char *label, const uint64_t *actual,
                        const uint64_t *expected, unsigned count) {
  for (unsigned lane = 0; lane < count; ++lane) {
    if (actual[lane] != expected[lane]) {
      fprintf(stderr, "%s lane %u: got %016llx expected %016llx\n", label,
              lane, (unsigned long long)actual[lane],
              (unsigned long long)expected[lane]);
      return 1;
    }
  }
  return 0;
}

int main(int argc, char **argv) {
  static const struct {
    const char *name;
    case_fn legacy, alias, live;
    uint32_t simple, negative_zero_case;
  } ops[] = {
      {"add", add_legacy, add_alias, add_live, 0x41200000U, 0x40000000U},
      {"sub", sub_legacy, sub_alias, sub_live, 0x40c00000U, 0xc0000000U},
      {"mul", mul_legacy, mul_alias, mul_live, 0x41800000U, 0x80000000U},
      {"div", div_legacy, div_alias, div_live, 0x40800000U, 0x80000000U},
  };
  const uint32_t a[8] = {0x41000000U, 0xdeadbeefU, 0x12345678U,
                         0x89abcdefU, 0x2468ace0U, 0x13579bdfU,
                         0xfedcba98U, 0x01234567U};
  const uint32_t b[8] = {0x40000000U, 0x87654321U, 0xabcdef01U,
                         0x7fc00001U, 0xaaaaaaaaU, 0xbbbbbbbbU,
                         0xccccccccU, 0xddddddddU};
  uint32_t out[8], live[8], expected[8], edge_a[8];
  uint32_t extra[80], extra_after[80];
  static const struct {
    const char *name;
    double_case_fn legacy, alias, live;
    uint64_t simple;
  } double_ops[] = {
      {"dadd", dadd_legacy, dadd_alias, dadd_live, 0x4024000000000000ULL},
      {"dsub", dsub_legacy, dsub_alias, dsub_live, 0x4018000000000000ULL},
      {"dmul", dmul_legacy, dmul_alias, dmul_live, 0x4030000000000000ULL},
      {"ddiv", ddiv_legacy, ddiv_alias, ddiv_live, 0x4010000000000000ULL},
  };
  const uint64_t da[4] = {0x4020000000000000ULL, 0x123456789abcdef0ULL,
                          0xfedcba9876543210ULL, 0x0123456789abcdefULL};
  const uint64_t db[4] = {0x4000000000000000ULL, 0x7ff8000000000001ULL,
                          0xaaaaaaaaaaaaaaaaULL, 0xbbbbbbbbbbbbbbbbULL};
  uint64_t dout[4], dlive[4], dexpected[4];
  int failures = 0;

  for (unsigned op = 0; op < sizeof(ops) / sizeof(ops[0]); ++op) {
    memcpy(expected, a, sizeof expected);
    expected[0] = ops[op].simple;
    memset(out, 0xff, sizeof out);
    ops[op].legacy(a, b, out, live);
    failures += check(ops[op].name, out, expected, 8);

    memset(expected + 4, 0, 4 * sizeof(uint32_t));
    memset(out, 0xff, sizeof out);
    ops[op].alias(a, b, out, live);
    failures += check(ops[op].name, out, expected, 8);

    memset(out, 0xff, sizeof out);
    memset(live, 0xff, sizeof live);
    ops[op].live(a, b, out, live);
    failures += check(ops[op].name, out, expected, 8);
    failures += check("live first source", live, a, 8);

    memcpy(edge_a, a, sizeof edge_a);
    edge_a[0] = 0x80000000U; /* Signed zero with finite second source. */
    memset(out, 0xff, sizeof out);
    ops[op].live(edge_a, b, out, live);
    expected[0] = ops[op].negative_zero_case;
    failures += check("signed zero", out, expected, 8);

    edge_a[0] = 0x7f800000U; /* +infinity with finite second source. */
    ops[op].live(edge_a, b, out, live);
    expected[0] = 0x7f800000U;
    failures += check("infinity", out, expected, 8);

    edge_a[0] = 0x7fc00001U; /* NaN class; payload is not prescribed. */
    ops[op].live(edge_a, b, out, live);
    if (!is_nan_bits(out[0])) {
      fprintf(stderr, "%s did not produce NaN\n", ops[op].name);
      ++failures;
    }
    failures += check("NaN upper lanes", out + 1, expected + 1, 7);
  }

  for (unsigned op = 0; op < sizeof(double_ops) / sizeof(double_ops[0]); ++op) {
    memcpy(dexpected, da, sizeof dexpected);
    dexpected[0] = double_ops[op].simple;
    double_ops[op].legacy(da, db, dout, dlive);
    failures += check_double(double_ops[op].name, dout, dexpected, 4);
    dexpected[2] = dexpected[3] = 0;
    double_ops[op].alias(da, db, dout, dlive);
    failures += check_double(double_ops[op].name, dout, dexpected, 4);
    double_ops[op].live(da, db, dout, dlive);
    failures += check_double(double_ops[op].name, dout, dexpected, 4);
    failures += check_double("double live first source", dlive, da, 4);
  }

  for (unsigned i = 0; i < 80; ++i) extra[i] = 0x3f000000U + i;
  memset(out, 0xff, sizeof out);
  pressured_add(a, b, extra, out, live, extra_after);
  memcpy(expected, a, sizeof expected);
  expected[0] = 0x41200000U;
  memset(expected + 4, 0, 4 * sizeof(uint32_t));
  failures += check("register pressure result", out, expected, 8);
  failures += check("register pressure source", live, a, 8);
  failures += check("register pressure extras", extra_after, extra, 80);

  FILE *receipt = fopen("C:\\fex-scalar-result.txt", "w");
  if (receipt) {
    fprintf(receipt, "fixture_failures=%d\n", failures);
    fclose(receipt);
  } else {
    fprintf(stderr, "unable to write fixture receipt\n");
    ++failures;
  }
  if (!failures)
    puts("FEX_SCALAR_FIXTURE_PASS 4 operations x 2 widths legacy/alias/live/pressure/edges");
  fflush(stdout);
  if (argc == 2 && strcmp(argv[1], "--hold") == 0) Sleep(45000);
  return failures ? 1 : 0;
}
