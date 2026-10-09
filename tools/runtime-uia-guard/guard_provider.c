/*
 * UI Automation argument and valid-provider regression tests.
 * Copyright 2026 Linux VST Bridge contributors.
 * Added 2026-10-09.
 *
 * This library is free software; you can redistribute it and/or modify it
 * under the terms of the GNU Lesser General Public License as published by
 * the Free Software Foundation; either version 2.1 of the License, or
 * (at your option) any later version.
 * This library is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
 * FITNESS FOR A PARTICULAR PURPOSE. See the GNU Lesser General Public License
 * for more details. You should have received a copy of the GNU Lesser General
 * Public License along with this library; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin St, Fifth Floor, Boston, MA 02110-1301, USA.
 */
#define COBJMACROS
#include <windows.h>
#include <uiautomation.h>
#include "wine/test.h"

static LONG references = 1;
static unsigned int options_calls;

static HRESULT WINAPI query_interface(IRawElementProviderSimple *iface, REFIID iid, void **out)
{
    *out = NULL;
    if (!IsEqualIID(iid, &IID_IUnknown) && !IsEqualIID(iid, &IID_IRawElementProviderSimple))
        return E_NOINTERFACE;
    *out = iface;
    IRawElementProviderSimple_AddRef(iface);
    return S_OK;
}

static ULONG WINAPI add_ref(IRawElementProviderSimple *iface)
{
    return InterlockedIncrement(&references);
}

static ULONG WINAPI release(IRawElementProviderSimple *iface)
{
    return InterlockedDecrement(&references);
}

static HRESULT WINAPI get_options(IRawElementProviderSimple *iface, enum ProviderOptions *out)
{
    ++options_calls;
    *out = ProviderOptions_ServerSideProvider;
    return S_OK;
}

static HRESULT WINAPI get_pattern(IRawElementProviderSimple *iface, PATTERNID id, IUnknown **out)
{
    *out = NULL;
    return S_OK;
}

static HRESULT WINAPI get_property(IRawElementProviderSimple *iface, PROPERTYID id, VARIANT *out)
{
    VariantInit(out);
    if (id == UIA_NamePropertyId)
    {
        V_VT(out) = VT_BSTR;
        V_BSTR(out) = SysAllocString(L"guard valid provider");
        if (!V_BSTR(out)) return E_OUTOFMEMORY;
    }
    return S_OK;
}

static HRESULT WINAPI get_host(IRawElementProviderSimple *iface, IRawElementProviderSimple **out)
{
    *out = NULL;
    return S_OK;
}

static const IRawElementProviderSimpleVtbl provider_vtbl = {
    query_interface, add_ref, release, get_options, get_pattern, get_property, get_host
};
static IRawElementProviderSimple provider = { &provider_vtbl };

START_TEST(guard_provider)
{
    HRESULT (WINAPI *disconnect)(IRawElementProviderSimple *);
    HMODULE module = LoadLibraryA("uiautomationcore.dll");
    HUIANODE node = (HUIANODE)0xdeadbeef;
    VARIANT value;
    HRESULT hr;
    LRESULT result;
    char **argv;
    int argc = winetest_get_mainargs(&argv);
    const char *selection = argc == 3 ? argv[2] : "all";
    BOOL all = !strcmp(selection, "all");

    ok(!!module, "UI Automation library failed to load\n");
    if (!module) return;
    hr = CoInitializeEx(NULL, COINIT_MULTITHREADED);
    ok(SUCCEEDED(hr), "COM initialization returned %#lx\n", hr);
    if (FAILED(hr)) { FreeLibrary(module); return; }

    ok(all || !strcmp(selection, "null-provider") || !strcmp(selection, "null-return") || !strcmp(selection, "null-output")
        || !strcmp(selection, "valid"), "Unknown case %s\n", selection);
    /* The original DLL reproduces the observed null-provider fault first. */
    if (all || !strcmp(selection, "null-provider"))
    {
        hr = UiaNodeFromProvider(NULL, &node);
        ok(hr == E_INVALIDARG, "Null provider returned %#lx\n", hr);
        disconnect = (void *)GetProcAddress(module, "UiaDisconnectProvider");
        ok(!!disconnect, "DisconnectProvider export missing\n");
        if (disconnect)
        {
            hr = disconnect(NULL);
            ok(hr == E_INVALIDARG, "DisconnectProvider returned %#lx\n", hr);
        }
    }
    if (all || !strcmp(selection, "null-return"))
    {
        result = UiaReturnRawElementProvider(NULL, 0, UiaRootObjectId, NULL);
        ok(!result, "ReturnRawElementProvider returned %Ix\n", result);
    }
    /* Independently exercise the output check with a real nonnull provider. */
    if (all || !strcmp(selection, "null-output"))
    {
        hr = UiaNodeFromProvider(&provider, NULL);
        ok(hr == E_INVALIDARG, "Null output returned %#lx\n", hr);
        ok(!options_calls, "Null output called the provider\n");
    }

    /* Keep the successful UIA node/property/reference path operational. */
    if (all || !strcmp(selection, "valid"))
    {
        hr = UiaNodeFromProvider(&provider, &node);
        ok(hr == S_OK && !!node, "Valid provider returned %#lx / %p\n", hr, node);
        if (hr == S_OK && node)
        {
            VariantInit(&value);
            hr = UiaGetPropertyValue(node, UIA_NamePropertyId, &value);
            ok(hr == S_OK, "Property read returned %#lx\n", hr);
            ok(V_VT(&value) == VT_BSTR, "Property type was %u\n", V_VT(&value));
            if (V_VT(&value) == VT_BSTR)
                ok(!lstrcmpW(V_BSTR(&value), L"guard valid provider"), "Property value changed\n");
            VariantClear(&value);
            ok(UiaNodeRelease(node), "Node release failed\n");
        }
        ok(options_calls > 0, "Valid provider was not consulted\n");
        ok(references == 1, "Provider reference count was %ld\n", references);
    }

    CoUninitialize();
    FreeLibrary(module);
}
