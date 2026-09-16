// Source-owned Windows API measurements. Output is a fixture oracle, not vendor
// role/ownership authority. The production supervisor observes roots and scripts.
#include <windows.h>
#include <cstdio>
#include <string>
#include <vector>
#include <array>
static std::wstring image(){wchar_t p[32768]{};DWORD n=GetModuleFileNameW(nullptr,p,32768);return {p,n};}
static unsigned long long born(HANDLE h){FILETIME c{},e{},k{},u{};if(!GetProcessTimes(h,&c,&e,&k,&u))return 0;return (static_cast<unsigned long long>(c.dwHighDateTime)<<32)|c.dwLowDateTime;}
static bool alive(HANDLE h){return WaitForSingleObject(h,0)==WAIT_TIMEOUT;}
static bool ignore_close=false;
static LRESULT CALLBACK window(HWND h,UINT m,WPARAM w,LPARAM l){if(m==WM_CLOSE){if(!ignore_close)DestroyWindow(h);return 0;}if(m==WM_DESTROY){PostQuitMessage(0);return 0;}return DefWindowProcW(h,m,w,l);}
static PROCESS_INFORMATION spawn(const std::wstring& args,const std::wstring& exe=L""){
 std::wstring cmd=L"\""+(exe.empty()?image():exe)+L"\" "+args;STARTUPINFOW s{};s.cb=sizeof(s);PROCESS_INFORMATION p{};
 if(!CreateProcessW(nullptr,cmd.data(),nullptr,nullptr,FALSE,0,nullptr,nullptr,&s,&p))return {};CloseHandle(p.hThread);return p;
}
static DWORD join(PROCESS_INFORMATION p,DWORD wait=15000){if(!p.hProcess)return 250;if(WaitForSingleObject(p.hProcess,wait)!=WAIT_OBJECT_0)return 251;DWORD code=252;GetExitCodeProcess(p.hProcess,&code);CloseHandle(p.hProcess);return code;}
struct Find {DWORD pid;HWND hwnd;unsigned count;};
static BOOL CALLBACK enumerate(HWND hwnd,LPARAM v){auto f=reinterpret_cast<Find*>(v);DWORD pid=0;GetWindowThreadProcessId(hwnd,&pid);wchar_t type[80]{};if(pid==f->pid&&GetClassNameW(hwnd,type,80)&&std::wstring(type)==L"IS2SourceOwned"){f->hwnd=hwnd;f->count++;}return TRUE;}
static HWND find(DWORD pid){Find f{pid,nullptr,0};EnumWindows(enumerate,reinterpret_cast<LPARAM>(&f));return f.count==1?f.hwnd:nullptr;}
static void emit(const std::string& test,const char* stage,DWORD pid,unsigned long long creation,int count,const char* outcome,DWORD status=0){
 std::printf("IS2_API_V1 case=%s stage=%s pid=%lu generation=%llu matches=%d outcome=%s status=%lu tick=%llu\n",test.c_str(),stage,pid,creation,count,outcome,status,GetTickCount64());std::fflush(stdout);
}
static bool close_exact(const std::string& test,PROCESS_INFORMATION p,HWND hwnd,unsigned long long expected,bool denied){
 DWORD owner=0;if(hwnd)GetWindowThreadProcessId(hwnd,&owner);
 if(p.dwProcessId==GetCurrentProcessId()||!p.hProcess||born(p.hProcess)!=expected||!alive(p.hProcess)||owner!=p.dwProcessId){emit(test,"close",p.dwProcessId,expected,-1,"refused_identity",0);return false;}
 if(denied){HANDLE limited=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,p.dwProcessId);BOOL result=limited?TerminateProcess(limited,29):FALSE;DWORD error=result?0:GetLastError();if(limited)CloseHandle(limited);emit(test,"close",p.dwProcessId,expected,1,result?"unexpected_success":"access_denied",error);return result!=FALSE;}
 SetLastError(0);BOOL sent=PostMessageW(hwnd,WM_CLOSE,0,0);DWORD error=sent?0:GetLastError();emit(test,"close",p.dwProcessId,expected,1,sent?"request_accepted":"request_refused",error);return sent!=FALSE;
}
static std::string case_file(){FILE* f=nullptr;_wfopen_s(&f,(image()+L".case").c_str(),L"rb");if(!f)return {};char b[80]{};auto n=fread(b,1,79,f);fclose(f);std::string s(b,n);while(!s.empty()&&(s.back()=='\n'||s.back()=='\r'))s.pop_back();return s;}
int wmain(int argc,wchar_t** argv){
 if(argc==2&&std::wstring(argv[1])==L"--self-test")return born(GetCurrentProcess())?0:1;
 if(argc==3&&std::wstring(argv[1])==L"--app"){
  ignore_close=std::wstring(argv[2])==L"ignore";WNDCLASSW c{};c.lpfnWndProc=window;c.hInstance=GetModuleHandleW(nullptr);c.lpszClassName=L"IS2SourceOwned";if(!RegisterClassW(&c))return 241;
  HWND h=CreateWindowW(c.lpszClassName,L"IS2 source-owned",WS_OVERLAPPED,0,0,100,100,nullptr,nullptr,c.hInstance,nullptr);if(!h)return 242;
  MSG m{};while(GetMessageW(&m,nullptr,0,0)>0){TranslateMessage(&m);DispatchMessageW(&m);}return 0;
 }
 if(argc==3&&std::wstring(argv[1])==L"--wrap"){auto n=wcstoul(argv[2],nullptr,10);return static_cast<int>(join(spawn(n?L"--wrap "+std::to_wstring(n-1):L"--payload")));}
 if(argc==5&&std::wstring(argv[1])==L"--adapter"){
  auto n=wcstoul(argv[2],nullptr,10);if(n>3)return 248;
  std::wstring args=n?L"--adapter "+std::to_wstring(n-1)+L" \""+argv[3]+L"\" \""+argv[4]+L"\"":L"\""+std::wstring(argv[4])+L"\"";
  return static_cast<int>(join(spawn(args,n?L"":std::wstring(argv[3]))));
 }
 std::string test=case_file();if(test.empty())return 240;
 emit(test,"ready",GetCurrentProcessId(),born(GetCurrentProcess()),-1,"source_owned");
 if(argc==2&&std::wstring(argv[1])==L"--payload")return 0;
 if(test=="wrapper1"&&argc==1)return static_cast<int>(join(spawn(L"--wrap 0")));
 if(test=="wrapper3"&&argc==1)return static_cast<int>(join(spawn(L"--wrap 2")));
 if(test=="exit23")return 23;
 if(test=="contract"){
  wchar_t cwd[32768]{};GetCurrentDirectoryW(32768,cwd);
  FILE* file=nullptr;_wfopen_s(&file,(image()+L".contract").c_str(),L"wb");if(!file)return 246;
  std::wstring data=L"cwd="+std::wstring(cwd)+L"\n";
  for(auto key:{L"HOME",L"USER",L"TEMP",L"PATH",L"WINEDLLOVERRIDES"}){wchar_t v[32768]{};GetEnvironmentVariableW(key,v,32768);data+=std::wstring(key)+L"="+v+L"\n";}
  HANDLE token=nullptr;TOKEN_ELEVATION elevation{};DWORD length=0;
  if(OpenProcessToken(GetCurrentProcess(),TOKEN_QUERY,&token)){GetTokenInformation(token,TokenElevation,&elevation,sizeof(elevation),&length);CloseHandle(token);}
  data+=L"elevated="+std::to_wstring(elevation.TokenIsElevated)+L"\n";fwrite(data.data(),sizeof(wchar_t),data.size(),file);fclose(file);
  std::printf("IS2_CONTRACT retained=private\n");std::fflush(stdout);return 0;
 }
 if(test=="powershell"){
  std::vector<std::wstring> scripts={L"[IO.File]::WriteAllText('C:\\IS2-script-sentinel.txt', 'source_owned'); exit 37",L"if (Get-Command Get-CimInstance -ErrorAction SilentlyContinue) { exit 0 } else { exit 1 }",L"if ((Get-CimInstance -ClassName Win32_Process | ? {$_.Path -and $_.Path.StartsWith('C:\\IS2-No-Application-Here', 'CurrentCultureIgnoreCase')}).Count -gt 0) { exit 0 } else { exit 1 }",L"Get-CimInstance -ClassName Win32_Process | ? {$_.Path -and $_.Path.StartsWith('C:\\IS2-No-Application-Here', 'CurrentCultureIgnoreCase')} | % { Stop-Process -Id $_.ProcessId -Force }"};
  wchar_t sys[32768]{};GetSystemDirectoryW(sys,32768);std::wstring ps=std::wstring(sys)+L"\\WindowsPowerShell\\v1.0\\powershell.exe";
  unsigned idx=0;for(auto& script:scripts){auto child=spawn(L"-C \""+script+L"\"",ps);DWORD pid=child.dwProcessId;auto generation=child.hProcess?born(child.hProcess):0;DWORD code=join(child);emit(test,"script",pid,generation,-1,("probe"+std::to_string(idx++)).c_str(),code);}
  emit(test,"sentinel",0,0,GetFileAttributesW(L"C:\\IS2-script-sentinel.txt")!=INVALID_FILE_ATTRIBUTES?1:0,"script_file_side_effect");return 0;
 }
 if(test=="no_app"||test=="stale_mutex"){
  HANDLE mutex=test=="stale_mutex"?CreateMutexW(nullptr,FALSE,L"Local\\IS2SourceOwnedStale"):nullptr;
  emit(test,"presence",0,0,0,mutex?"mutex_without_process":"absent");emit(test,"close",0,0,0,"not_requested");emit(test,"recheck",0,0,0,"absent");if(mutex)CloseHandle(mutex);return 0;
 }
 if(test=="self"){
  emit(test,"presence",GetCurrentProcessId(),born(GetCurrentProcess()),1,"self_match");PROCESS_INFORMATION self{};self.dwProcessId=GetCurrentProcessId();self.hProcess=GetCurrentProcess();close_exact(test,self,nullptr,born(self.hProcess),false);return 0;
 }
 bool ignore=test=="ignores"||test=="helper_zero"||test=="cancel_after"||test=="cleanup_preserves";
 auto child=spawn(ignore?L"--app ignore":L"--app cooperative");if(!child.hProcess)return 243;
 auto generation=born(child.hProcess);HWND hwnd=nullptr;
 for(unsigned k=0;k<200&&!hwnd;k++){hwnd=find(child.dwProcessId);Sleep(10);}
 if(!hwnd){TerminateProcess(child.hProcess,244);join(child);return 244;}
 PROCESS_INFORMATION unrelated{};if(test=="same_name"){unrelated=spawn(L"--app ignore");if(!unrelated.hProcess)return 245;}
 emit(test,"presence",child.dwProcessId,generation,1,"exact_handle_and_window_owner");
 if(test=="exit_between"||test=="stale_window"){PostMessageW(hwnd,WM_CLOSE,0,0);WaitForSingleObject(child.hProcess,5000);}
 if(test=="helper_zero")emit(test,"close",child.dwProcessId,generation,1,"helper_zero_no_close",0);
 else close_exact(test,child,hwnd,test=="stale_pid"?generation+1:generation,test=="access_denied");
 WaitForSingleObject(child.hProcess,500);bool remains=alive(child.hProcess);
 emit(test,"recheck",child.dwProcessId,generation,remains?1:0,remains?"still_present":"exited");
 if(test=="stale_recheck")emit(test,"cached_recheck",child.dwProcessId,generation,1,"rejected_stale_snapshot");
 if(unrelated.hProcess){emit(test,"unrelated",unrelated.dwProcessId,born(unrelated.hProcess),alive(unrelated.hProcess)?1:0,"preserved");TerminateProcess(unrelated.hProcess,0);join(unrelated);}
 if(test=="cancel_after"){emit(test,"holding",child.dwProcessId,generation,remains?1:0,"await_owned_cancellation");Sleep(180000);}
 if(remains)TerminateProcess(child.hProcess,0);join(child);
 emit(test,"cleanup",0,0,0,"source_owned_children_retired");return 0;
}
