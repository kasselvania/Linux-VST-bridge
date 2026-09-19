#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <msi.h>
#include <msiquery.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;

static bool plain_component(const std::wstring &value) {
    if (value.empty() || value == L"." || value == L"..") return false;
    for (wchar_t c : value) {
        if (!(c == L'_' || (c >= L'0' && c <= L'9') ||
              (c >= L'A' && c <= L'Z') || (c >= L'a' && c <= L'z'))) return false;
    }
    return true;
}

static bool exact_directory(const fs::path &path) {
    std::error_code ec;
    return fs::is_directory(path, ec) && !ec && !fs::is_symlink(path, ec) && !ec;
}

static bool get_string(MSIHANDLE record, UINT field, std::wstring &out) {
    DWORD length = 0;
    WCHAR marker = 0;
    UINT rc = MsiRecordGetStringW(record, field, &marker, &length);
    if (rc != ERROR_MORE_DATA && rc != ERROR_SUCCESS) return false;
    std::vector<WCHAR> buffer(static_cast<size_t>(length) + 1);
    DWORD capacity = length + 1;
    rc = MsiRecordGetStringW(record, field, buffer.data(), &capacity);
    if (rc != ERROR_SUCCESS) return false;
    out.assign(buffer.data(), capacity);
    return true;
}

static bool enumerate_tables(MSIHANDLE database, std::vector<std::wstring> &tables) {
    MSIHANDLE view = 0;
    if (MsiDatabaseOpenViewW(database, L"SELECT `Name` FROM `_Tables`", &view) != ERROR_SUCCESS)
        return false;
    if (MsiViewExecute(view, 0) != ERROR_SUCCESS) {
        MsiCloseHandle(view);
        return false;
    }
    for (;;) {
        MSIHANDLE row = 0;
        UINT rc = MsiViewFetch(view, &row);
        if (rc == ERROR_NO_MORE_ITEMS) break;
        if (rc != ERROR_SUCCESS) {
            MsiCloseHandle(view);
            return false;
        }
        std::wstring name;
        bool ok = get_string(row, 1, name) && plain_component(name);
        MsiCloseHandle(row);
        if (!ok) {
            MsiCloseHandle(view);
            return false;
        }
        tables.push_back(name);
    }
    MsiCloseHandle(view);
    std::sort(tables.begin(), tables.end());
    return std::adjacent_find(tables.begin(), tables.end()) == tables.end();
}

int wmain(int argc, wchar_t **argv) {
    if (argc != 3) return 64;
    fs::path database_path(argv[1]);
    fs::path output(argv[2]);
    std::error_code ec;
    if (!fs::is_regular_file(database_path, ec) || ec || fs::is_symlink(database_path, ec) || ec ||
        !exact_directory(output)) return 65;
    if (!fs::is_empty(output, ec) || ec) return 66;

    MSIHANDLE database = 0;
    if (MsiOpenDatabaseW(database_path.c_str(), MSIDBOPEN_READONLY, &database) != ERROR_SUCCESS)
        return 67;
    std::vector<std::wstring> tables;
    if (!enumerate_tables(database, tables)) {
        MsiCloseHandle(database);
        return 68;
    }
    for (const auto &name : tables) {
        std::wstring file = name + L".idt";
        if (MsiDatabaseExportW(database, name.c_str(), output.c_str(), file.c_str()) != ERROR_SUCCESS) {
            MsiCloseHandle(database);
            return 69;
        }
    }
    MsiCloseHandle(database);

    std::ofstream roster(output / "_k8i1-tables.txt", std::ios::binary | std::ios::trunc);
    if (!roster) return 70;
    const unsigned char bom[] = {0xff, 0xfe};
    roster.write(reinterpret_cast<const char *>(bom), sizeof(bom));
    for (const auto &name : tables) {
        roster.write(reinterpret_cast<const char *>(name.data()),
                     static_cast<std::streamsize>(name.size() * sizeof(wchar_t)));
        const wchar_t newline = L'\n';
        roster.write(reinterpret_cast<const char *>(&newline), sizeof(newline));
    }
    roster.flush();
    return roster.good() ? 0 : 71;
}
