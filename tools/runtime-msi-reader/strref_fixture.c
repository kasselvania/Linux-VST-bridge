/* Original, account-free MSI database regression. Copyright 2026 LVB contributors.
 * SPDX-License-Identifier: MIT
 *
 * A six-row/two-string-column table has 24 bytes with short references, also
 * divisible by the six-byte long-reference row width. Advertising long refs
 * without rewriting this table must not silently discard/misread its rows.
 * All files are created in the caller's disposable test directory.
 */
#define COBJMACROS
#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif
#include <windows.h>
#include <objbase.h>
#include <msi.h>
#include <msiquery.h>
#include <stdio.h>
#include <wchar.h>

static int failed;

static int check(int condition, const char *operation, unsigned long status)
{
    if (!condition) {
        fprintf(stderr, "%s failed: %#lx\n", operation, status);
        failed = 1;
    }
    return condition;
}

static int query(MSIHANDLE db, const WCHAR *sql)
{
    MSIHANDLE view = 0;
    UINT code = MsiDatabaseOpenViewW(db, sql, &view);
    if (!code) code = MsiViewExecute(view, 0);
    if (view) MsiCloseHandle(view);
    return check(code == ERROR_SUCCESS, "MSI query", code);
}

static int add_long_pool(MSIHANDLE db)
{
    MSIHANDLE view = 0, record = 0;
    WCHAR value[32];
    UINT code;
    unsigned int row, field;
    if (!query(db, L"CREATE TABLE `Pool` (`Key` CHAR(32) NOT NULL, `Value` CHAR(32) NOT NULL PRIMARY KEY `Key`)"))
        return 0;
    code = MsiDatabaseOpenViewW(db, L"SELECT `Key`,`Value` FROM `Pool`", &view);
    if (!check(!code, "open pool view", code)) goto done;
    code = MsiViewExecute(view, 0);
    if (!check(!code, "execute pool view", code)) goto done;
    record = MsiCreateRecord(2);
    if (!check(record != 0, "create pool record", 0)) goto done;
    /* Two distinct strings per row exceed the 65,535 short-reference limit.
     * Commit owns encoding of every table, including MSI metadata tables. */
    for (row = 0; row < 32768; ++row) {
        for (field = 1; field <= 2; ++field) {
            swprintf(value, 32, field == 1 ? L"S%05u" : L"T%05u", row);
            code = MsiRecordSetStringW(record, field, value);
            if (!check(!code, "set pool string", code)) goto done;
        }
        code = MsiViewModify(view, MSIMODIFY_INSERT, record);
        if (!check(!code, "insert pool row", code)) goto done;
    }
done:
    if (record) MsiCloseHandle(record);
    if (view) MsiCloseHandle(view);
    return !failed;
}

static int create_database(const WCHAR *path, int long_pool)
{
    MSIHANDLE db = 0;
    WCHAR sql[256];
    UINT code;
    unsigned int row;
    if (GetFileAttributesW(path) != INVALID_FILE_ATTRIBUTES)
        return check(0, "fixture already exists", ERROR_FILE_EXISTS);
    code = MsiOpenDatabaseW(path, MSIDBOPEN_CREATE, &db);
    if (!check(!code, "create database", code)) return 0;
    if (!query(db, L"CREATE TABLE `LVB` (`Key` CHAR(32) NOT NULL, `Value` CHAR(32) NOT NULL PRIMARY KEY `Key`)"))
        goto done;
    for (row = 0; row < 6; ++row) {
        swprintf(sql, 256, L"INSERT INTO `LVB` (`Key`,`Value`) VALUES ('K%u','V%u')", row, row);
        if (!query(db, sql)) goto done;
    }
    if (long_pool && !add_long_pool(db)) goto done;
    code = MsiDatabaseCommit(db);
    check(!code, "commit database", code);
done:
    MsiCloseHandle(db);
    return !failed;
}

static int check_encoding(const WCHAR *path, int long_pool, int advertise_long)
{
    /* MSI structured-storage stream names, including the table-name prefix. */
    static const WCHAR pool_name[] = {0x4840,0x3f3f,0x4577,0x446c,0x3e6a,0x44b2,0x482f,0};
    static const WCHAR table_name[] = {0x4840,0x3fd5,0x480b,0}; /* LVB */
    IStorage *storage = NULL;
    IStream *stream = NULL;
    WORD header[2];
    ULONG count = 0;
    LARGE_INTEGER zero;
    HRESULT hr;
    DWORD mode = advertise_long ? STGM_READWRITE : STGM_READ;
    zero.QuadPart = 0;
    hr = StgOpenStorage(path, NULL, STGM_DIRECT | mode | STGM_SHARE_EXCLUSIVE,
                        NULL, 0, &storage);
    if (!check(SUCCEEDED(hr), "open fixture storage", hr)) return 0;
    {
        STATSTG info;
        hr = IStorage_OpenStream(storage, table_name, NULL,
                                 mode | STGM_SHARE_EXCLUSIVE, 0, &stream);
        if (!check(SUCCEEDED(hr), "open LVB table", hr)) goto done;
        hr = IStream_Stat(stream, &info, STATFLAG_NONAME);
        if (!check(SUCCEEDED(hr) && info.cbSize.QuadPart == 12u * (long_pool ? 3u : 2u),
                   "actual reference width", hr)) goto done;
        IStream_Release(stream);
        stream = NULL;
    }
    hr = IStorage_OpenStream(storage, pool_name, NULL,
                             mode | STGM_SHARE_EXCLUSIVE, 0, &stream);
    if (!check(SUCCEEDED(hr), "open string pool", hr)) goto done;
    hr = IStream_Read(stream, header, sizeof(header), &count);
    if (!check(hr == S_OK && count == sizeof(header) && !!(header[1] & 0x8000) == !!long_pool,
               "actual string pool width", hr)) goto done;
    if (advertise_long) {
        header[1] |= 0x8000;
        hr = IStream_Seek(stream, zero, STREAM_SEEK_SET, NULL);
        if (!check(SUCCEEDED(hr), "seek string pool", hr)) goto done;
        hr = IStream_Write(stream, header, sizeof(header), &count);
        if (!check(hr == S_OK && count == sizeof(header), "write string pool header", hr)) goto done;
        hr = IStorage_Commit(storage, STGC_DEFAULT);
        check(SUCCEEDED(hr), "commit fixture storage", hr);
    }
done:
    if (stream) IStream_Release(stream);
    IStorage_Release(storage);
    return !failed;
}

static int verify_database(const WCHAR *path, const char *label)
{
    MSIHANDLE db = 0, view = 0, record = 0;
    WCHAR value[16], expected[16];
    DWORD length;
    UINT code;
    unsigned int row = 0, field;
    code = MsiOpenDatabaseW(path, MSIDBOPEN_READONLY, &db);
    if (!check(!code, "open verification database", code)) return 0;
    code = MsiDatabaseOpenViewW(db, L"SELECT `Key`,`Value` FROM `LVB` ORDER BY `Key`", &view);
    if (!check(!code, "open verification view", code)) goto done;
    code = MsiViewExecute(view, 0);
    if (!check(!code, "execute verification view", code)) goto done;
    while ((code = MsiViewFetch(view, &record)) == ERROR_SUCCESS) {
        if (!check(row < 6, "unexpected extra row", row)) goto done;
        for (field = 1; field <= 2; ++field) {
            length = 16;
            code = MsiRecordGetStringW(record, field, value, &length);
            swprintf(expected, 16, field == 1 ? L"K%u" : L"V%u", row);
            if (!check(!code && !wcscmp(value, expected), "exact row contents", code)) goto done;
        }
        MsiCloseHandle(record);
        record = 0;
        ++row;
    }
    check(code == ERROR_NO_MORE_ITEMS && row == 6, "exact row count", row);
    if (!failed) printf("%s: all six exact rows preserved\n", label);
done:
    if (record) MsiCloseHandle(record);
    if (view) MsiCloseHandle(view);
    MsiCloseHandle(db);
    return !failed;
}

int wmain(int argc, WCHAR **argv)
{
    static const WCHAR *names[] = {L"short", L"actual-long", L"advertised-long"};
    static const char *labels[] = {"ordinary short refs", "valid long refs", "ambiguous short refs"};
    WCHAR path[MAX_PATH];
    WCHAR module_path[MAX_PATH];
    DWORD length;
    HRESULT hr;
    unsigned int n;
    int any_failed = 0;
    if ((argc != 2 && argc != 3) || wcslen(argv[1]) + 40 >= MAX_PATH) return 2;
    if (argc == 3 && wcscmp(argv[2], L"--wait-for-readback")) return 2;
    {
        DWORD attributes = GetFileAttributesW(argv[1]);
        if (attributes == INVALID_FILE_ATTRIBUTES || !(attributes & FILE_ATTRIBUTE_DIRECTORY)) return 2;
    }
    hr = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    if (FAILED(hr)) return 2;
    length = GetModuleFileNameW(GetModuleHandleW(L"msi.dll"), module_path, MAX_PATH);
    if (!length || length >= MAX_PATH) { CoUninitialize(); return 2; }
    printf("Loaded MSI module: %ls\n", module_path);
    fflush(stdout);
    /* A fixture-only five-second window for the owner to hash /proc/PID/maps. */
    if (argc == 3) Sleep(5000);
    for (n = 0; n < 3; ++n) {
        failed = 0;
        swprintf(path, MAX_PATH, L"%ls\\lvb-strref-%ls.msi", argv[1], names[n]);
        if (create_database(path, n == 1) && check_encoding(path, n == 1, n == 2))
            verify_database(path, labels[n]);
        if (failed) {
            printf("%s: FAILED\n", labels[n]);
            any_failed = 1;
        }
    }
    CoUninitialize();
    return any_failed;
}
