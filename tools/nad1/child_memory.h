// Source-owned reproduction of ordinary suspended-child PEB access.
// Uses documented process ownership and public Windows structures, no vendor code.
#include <winternl.h>
#include <cstdio>
static bool child_memory_probe(const wchar_t* receipt) {
 wchar_t image[32768]{};if(!GetModuleFileNameW(nullptr,image,32768))return false;
 std::wstring cmd=L"\"";cmd+=image;cmd+=L"\" --memory-child";
 STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION pi{};
 BOOL created=CreateProcessW(image,cmd.data(),nullptr,nullptr,FALSE,CREATE_SUSPENDED,nullptr,nullptr,&si,&pi);
 DWORD create_error=created?0:GetLastError(),read_error=0,write_error=0;LONG query_status=-1;ULONG returned=0;SIZE_T read=0,written=0;BOOL got=FALSE,put=FALSE,retired=FALSE;
 PROCESS_BASIC_INFORMATION info{};ULONG_PTR prior=0,zero=0;
 if(created){
  using Query=LONG (WINAPI*)(HANDLE,PROCESSINFOCLASS,PVOID,ULONG,PULONG);
  auto query=reinterpret_cast<Query>(GetProcAddress(GetModuleHandleW(L"ntdll.dll"),"NtQueryInformationProcess"));
  if(query)query_status=query(pi.hProcess,ProcessBasicInformation,&info,sizeof(info),&returned);
  if(query_status>=0 && returned==sizeof(info) && info.PebBaseAddress){
   // x64 PEB pShimData offset, the exact public Chromium launch-error-72 seam.
   auto address=reinterpret_cast<BYTE*>(info.PebBaseAddress)+0x2d8;
   got=ReadProcessMemory(pi.hProcess,address,&prior,sizeof(prior),&read);read_error=got?0:GetLastError();
   put=WriteProcessMemory(pi.hProcess,address,&zero,sizeof(zero),&written);write_error=put?0:GetLastError();
  }
  TerminateProcess(pi.hProcess,0);retired=WaitForSingleObject(pi.hProcess,5000)==WAIT_OBJECT_0;CloseHandle(pi.hThread);CloseHandle(pi.hProcess);
 }
 bool ok=created&&query_status>=0&&returned==sizeof(info)&&got&&read==sizeof(prior)&&put&&written==sizeof(zero)&&retired;
 char row[700];int n=std::snprintf(row,sizeof(row),"{\"created\":%s,\"create_error\":%lu,\"query_status\":%ld,\"query_bytes\":%lu,\"expected_query_bytes\":%zu,\"read_ok\":%s,\"read_error\":%lu,\"read_bytes\":%zu,\"write_ok\":%s,\"write_error\":%lu,\"written_bytes\":%zu,\"child_retired\":%s,\"passed\":%s}\n",created?"true":"false",create_error,query_status,returned,sizeof(info),got?"true":"false",read_error,read,put?"true":"false",write_error,written,retired?"true":"false",ok?"true":"false");
 HANDLE f=CreateFileW(receipt,GENERIC_WRITE,0,nullptr,CREATE_NEW,FILE_ATTRIBUTE_NORMAL,nullptr);if(f==INVALID_HANDLE_VALUE)return false;DWORD bytes=0;bool saved=WriteFile(f,row,n,&bytes,nullptr)&&bytes==static_cast<DWORD>(n);CloseHandle(f);return ok&&saved;
}
