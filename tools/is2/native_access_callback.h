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
    HANDLE image,primary,input,child=INVALID_HANDLE_VALUE;
    std::wstring path,op,policy;
    std::vector<unsigned char> pending;
    unsigned count=0;ULONGLONG deadline=0,partial_since=0;bool closed=false;
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
    NativeAccessCallback(HANDLE f,HANDLE p,const std::wstring& name,const std::wstring& operation,const std::wstring& mode):image(f),primary(p),input(GetStdHandle(STD_INPUT_HANDLE)),path(name),op(operation),policy(mode){}
    void tick(){
        if(child!=INVALID_HANDLE_VALUE){
            DWORD status=WaitForSingleObject(child,0);
            if(status==WAIT_OBJECT_0){DWORD code=1;GetExitCodeProcess(child,&code);CloseHandle(child);child=INVALID_HANDLE_VALUE;reply(code==0?0:1);}
            else if(GetTickCount64()>=deadline){TerminateProcess(child,130);WaitForSingleObject(child,5000);CloseHandle(child);child=INVALID_HANDLE_VALUE;reply(2);}
            return;
        }
        if(closed)return;
        DWORD available=0;
        if(!PeekNamedPipe(input,nullptr,0,nullptr,&available,nullptr)){closed=true;return;}
        if(available){
            std::array<unsigned char,2052> b{};DWORD n=0;
            if(!ReadFile(input,b.data(),(std::min)(available,static_cast<DWORD>(b.size())),&n,nullptr)){closed=true;return;}
            if(pending.empty())partial_since=GetTickCount64();
            pending.insert(pending.end(),b.begin(),b.begin()+n);SecureZeroMemory(b.data(),b.size());
            if(pending.size()>2052){closed=true;SecureZeroMemory(pending.data(),pending.size());pending.clear();return;}
        }
        if(pending.size()>=4){
            unsigned n=static_cast<unsigned>(pending[0])|(static_cast<unsigned>(pending[1])<<8)|(static_cast<unsigned>(pending[2])<<16)|(static_cast<unsigned>(pending[3])<<24);
            if(n>2048||n<=14||pending.size()>n+4||count>=4){closed=true;SecureZeroMemory(pending.data(),pending.size());pending.clear();return;}
            if(pending.size()==n+4){++count;std::string uri(pending.begin()+4,pending.end());SecureZeroMemory(pending.data(),pending.size());pending.clear();launch(uri);SecureZeroMemory(uri.data(),uri.size());return;}
        }
        if(!pending.empty()&&GetTickCount64()-partial_since>5000){closed=true;SecureZeroMemory(pending.data(),pending.size());pending.clear();}
    }
    ~NativeAccessCallback(){if(child!=INVALID_HANDLE_VALUE){TerminateProcess(child,130);WaitForSingleObject(child,5000);CloseHandle(child);}if(!pending.empty())SecureZeroMemory(pending.data(),pending.size());}
};
