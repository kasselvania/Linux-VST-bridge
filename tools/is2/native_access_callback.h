// Operation-local opaque browser return. No URL is written to any diagnostic.
// Windows' ordinary second-instance command-line handoff remains vendor-owned.
static bool callback_uri(const std::string& value) {
    if(value.size()<=14||value.size()>2048||value.rfind("native-access:",0)!=0)return false;
    auto digit=[](unsigned char c)->int {if(c>='0'&&c<='9')return c-'0';if(c>='a'&&c<='f')return c-'a'+10;if(c>='A'&&c<='F')return c-'A'+10;return -1;};
    for(size_t i=14;i<value.size();++i){unsigned char c=value[i];
        if(c<0x21||c>0x7e||c=='"'||c=='\''||c=='\\'||c=='`'||c=='<'||c=='>')return false;
        if(c=='%'){if(i+2>=value.size())return false;int a=digit(value[i+1]),b=digit(value[i+2]);if(a<0||b<0||a*16+b<32||a*16+b==127)return false;i+=2;}
    }return true;
}
class NativeAccessCallback {
    // Wine can expose forwarded stdin as FILE_TYPE_CHAR. PeekNamedPipe is
    // therefore not an input authority. One bounded blocking reader accepts an
    // ASCII URI line from console or pipe, with echo disabled for console input.
    struct Reader {
        HANDLE input=INVALID_HANDLE_VALUE,ready=nullptr,consumed=nullptr;
        LONG stopping=0;unsigned status=0;DWORD error=0;
        std::string line;
    };
    static DWORD WINAPI read_input(void* context){
        auto* r=static_cast<Reader*>(context);
        while(!InterlockedCompareExchange(&r->stopping,0,0)){
            char c=0;DWORD n=0;
            if(!ReadFile(r->input,&c,1,&n,nullptr)||n!=1){r->error=GetLastError();r->status=2;SetEvent(r->ready);return 0;}
            if(c=='\n'){
                if(!r->line.empty()&&r->line.back()=='\r')r->line.pop_back();
                r->status=callback_uri(r->line)?0:1;SetEvent(r->ready);
                if(WaitForSingleObject(r->consumed,INFINITE)!=WAIT_OBJECT_0)return 0;
            }else{
                r->line+=c;
                if(r->line.size()>2049){r->status=1;SetEvent(r->ready);return 0;}
            }
        }return 0;
    }
    HANDLE image,primary,child=INVALID_HANDLE_VALUE,reader_thread=nullptr;
    Reader* reader=nullptr;
    std::wstring path,op,policy;
    unsigned count=0;ULONGLONG deadline=0;bool closed=false;
    void reply(unsigned result){std::fprintf(stdout,"NA_AUTH_V1 %ls %u %u\n",op.c_str(),count,result);std::fflush(stdout);}
    void launch(const std::string& uri){
        if(WaitForSingleObject(primary,0)!=WAIT_TIMEOUT||!callback_uri(uri)){reply(1);return;}
        // An explicit executable plus one quoted URI: never ShellExecute, registry
        // expansion, a shell, an alternate prefix or caller-selected flags.
        std::wstring command=L"\""+path+L"\"";
        if(policy==L"software_rendering")command+=L" --disable-gpu";
        command+=L" \""+std::wstring(uri.begin(),uri.end())+L"\"";
        STARTUPINFOW si{};si.cb=sizeof(si);PROCESS_INFORMATION pi{};
        bool created=CreateProcessW(path.c_str(),command.data(),nullptr,nullptr,FALSE,CREATE_SUSPENDED,nullptr,nullptr,&si,&pi)!=FALSE;
        SecureZeroMemory(command.data(),command.size()*sizeof(wchar_t));
        if(!created){reply(1);return;}
        std::array<wchar_t,32768> actual{};DWORD n=static_cast<DWORD>(actual.size());
        HANDLE opened=QueryFullProcessImageNameW(pi.hProcess,0,actual.data(),&n)?CreateFileW(actual.data(),GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr):INVALID_HANDLE_VALUE;
        bool exact=opened!=INVALID_HANDLE_VALUE&&same_file(image,opened)&&time_of(pi.hProcess)!=0&&WaitForSingleObject(primary,0)==WAIT_TIMEOUT;
        if(opened!=INVALID_HANDLE_VALUE)CloseHandle(opened);
        if(!exact||ResumeThread(pi.hThread)==static_cast<DWORD>(-1)){TerminateProcess(pi.hProcess,128);WaitForSingleObject(pi.hProcess,5000);CloseHandle(pi.hProcess);reply(1);}
        else{child=pi.hProcess;deadline=GetTickCount64()+15000;}
        CloseHandle(pi.hThread);
    }
public:
    NativeAccessCallback(HANDLE f,HANDLE p,const std::wstring& name,const std::wstring& operation,const std::wstring& mode):image(f),primary(p),path(name),op(operation),policy(mode){
        reader=new Reader;
        if(!DuplicateHandle(GetCurrentProcess(),GetStdHandle(STD_INPUT_HANDLE),GetCurrentProcess(),&reader->input,0,FALSE,DUPLICATE_SAME_ACCESS)){closed=true;return;}
        DWORD console=0;
        if(GetConsoleMode(reader->input,&console)&&!SetConsoleMode(reader->input,console&~ENABLE_ECHO_INPUT)){closed=true;return;}
        reader->ready=CreateEventW(nullptr,FALSE,FALSE,nullptr);reader->consumed=CreateEventW(nullptr,FALSE,FALSE,nullptr);
        if(!reader->ready||!reader->consumed){closed=true;return;}
        reader_thread=CreateThread(nullptr,0,read_input,reader,0,nullptr);if(!reader_thread)closed=true;
    }
    void tick(){
        if(child!=INVALID_HANDLE_VALUE){
            DWORD status=WaitForSingleObject(child,0);
            if(status==WAIT_OBJECT_0){DWORD code=1;GetExitCodeProcess(child,&code);CloseHandle(child);child=INVALID_HANDLE_VALUE;reply(code==0?0:1);}
            else if(GetTickCount64()>=deadline){TerminateProcess(child,130);WaitForSingleObject(child,5000);CloseHandle(child);child=INVALID_HANDLE_VALUE;reply(2);}
            return;
        }
        if(closed)return;
        if(!reader_thread||WaitForSingleObject(reader->ready,0)!=WAIT_OBJECT_0)return;
        if(reader->status!=0||count>=4){closed=true;reply(1);return;}
        ++count;std::string uri=reader->line;
        SecureZeroMemory(reader->line.data(),reader->line.size());reader->line.clear();
        launch(uri);SecureZeroMemory(uri.data(),uri.size());SetEvent(reader->consumed);
    }
    ~NativeAccessCallback(){
        if(child!=INVALID_HANDLE_VALUE){TerminateProcess(child,130);WaitForSingleObject(child,5000);CloseHandle(child);}
        if(reader){
            InterlockedExchange(&reader->stopping,1);if(reader->consumed)SetEvent(reader->consumed);
            if(reader_thread)CancelSynchronousIo(reader_thread);
            // Do not free a context still used by a blocked Wine reader. The
            // adapter exits immediately after this bounded join; ExitProcess
            // retires any remaining thread and its private handles/memory.
            bool joined=!reader_thread||WaitForSingleObject(reader_thread,1000)==WAIT_OBJECT_0;
            if(reader_thread)CloseHandle(reader_thread);
            if(joined){if(reader->input!=INVALID_HANDLE_VALUE)CloseHandle(reader->input);if(reader->ready)CloseHandle(reader->ready);if(reader->consumed)CloseHandle(reader->consumed);if(!reader->line.empty())SecureZeroMemory(reader->line.data(),reader->line.size());delete reader;}
        }
    }
};
