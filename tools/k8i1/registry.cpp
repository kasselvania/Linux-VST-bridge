// K8I1 fixed registry helper. It can touch only Kontakt 8 product state and
// the one per-application Wine MSI override; no arbitrary registry surface.
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
#include <array>
#include <cstdio>
#include <string>
#include <vector>

static bool hex(const std::wstring& s,size_t n){if(s.size()!=n)return false;for(auto c:s)if(!((c>=L'0'&&c<=L'9')||(c>=L'a'&&c<=L'f')))return false;return true;}
static bool decode(const std::wstring& in,std::wstring& out){
    if(in.size()%4)return false;out.clear();out.reserve(in.size()/4);
    auto d=[](wchar_t c)->int{if(c>=L'0'&&c<=L'9')return c-L'0';if(c>=L'a'&&c<=L'f')return c-L'a'+10;return -1;};
    for(size_t i=0;i<in.size();i+=4){int a=d(in[i]),b=d(in[i+1]),c=d(in[i+2]),e=d(in[i+3]);if(a<0||b<0||c<0||e<0)return false;out.push_back(wchar_t((a<<12)|(b<<8)|(c<<4)|e));}
    return true;
}
static std::wstring encode(const wchar_t* data,size_t count){
    static constexpr wchar_t h[]=L"0123456789abcdef";std::wstring out;out.reserve(count*4);
    for(size_t i=0;i<count;i++){unsigned v=data[i];out.push_back(h[(v>>12)&15]);out.push_back(h[(v>>8)&15]);out.push_back(h[(v>>4)&15]);out.push_back(h[v&15]);}return out;
}
static bool allowed(const std::wstring& hive,const std::wstring& key,const std::wstring& name){
    const std::wstring override=L"Software\\Wine\\AppDefaults\\Kontakt 8 Setup PC.exe\\DllOverrides";
    const std::wstring product=L"Software\\Native Instruments\\Kontakt 8";
    if(hive==L"HKCU"&&key==override&&name==L"msi")return true;
    if(hive!=L"HKLM"||key!=product)return false;
    return name==L"InstallDir"||name==L"ContentDir"||name==L"ContentVersion"||name==L"InstallVST364Dir";
}
static void result(const std::wstring& op,const std::wstring& nonce,const std::wstring& action,DWORD status,bool present,DWORD type,const std::wstring& value){
    std::fwprintf(stdout,L"K8I1_REGISTRY_RESULT_V1 %ls %ls %ls %lu %u %lu %ls\n",op.c_str(),nonce.c_str(),action.c_str(),status,present?1u:0u,type,encode(value.data(),value.size()).c_str());std::fflush(stdout);
}
int wmain(int argc,wchar_t** argv){
    if(argc==2&&std::wstring(argv[1])==L"--self-test")return allowed(L"HKCU",L"Software\\Wine\\AppDefaults\\Kontakt 8 Setup PC.exe\\DllOverrides",L"msi")&&allowed(L"HKLM",L"Software\\Native Instruments\\Kontakt 8",L"InstallDir")&&!allowed(L"HKLM",L"Software\\Native Instruments\\Kontakt 8\\Licensing",L"Token")&&!allowed(L"HKLM",L"System\\Services",L"x")?0:1;
    if(argc!=2)return 120;
    HANDLE f=CreateFileW(argv[1],GENERIC_READ,FILE_SHARE_READ,nullptr,OPEN_EXISTING,FILE_FLAG_OPEN_REPARSE_POINT,nullptr);if(f==INVALID_HANDLE_VALUE)return 121;
    BY_HANDLE_FILE_INFORMATION md{};std::array<wchar_t,32768> buf{};DWORD bytes=0;
    bool ok=GetFileInformationByHandle(f,&md)&&!(md.dwFileAttributes&(FILE_ATTRIBUTE_DIRECTORY|FILE_ATTRIBUTE_REPARSE_POINT))&&md.nNumberOfLinks==1&&md.nFileSizeHigh==0&&md.nFileSizeLow<sizeof(buf)&&md.nFileSizeLow%2==0&&ReadFile(f,buf.data(),md.nFileSizeLow,&bytes,nullptr)&&bytes==md.nFileSizeLow;CloseHandle(f);if(!ok)return 122;
    std::wstring all(buf.data(),bytes/2);std::vector<std::wstring> lines;size_t pos=0;
    for(;;){auto end=all.find(L'\n',pos);if(end==std::wstring::npos)break;auto line=all.substr(pos,end-pos);if(!line.empty()&&line.back()==L'\r')line.pop_back();lines.push_back(line);pos=end+1;}
    if(pos!=all.size()||lines.size()!=9||lines[0]!=L"K8I1_REGISTRY_V1"||!hex(lines[1],32)||!hex(lines[2],64)||
       (lines[3]!=L"read"&&lines[3]!=L"write"&&lines[3]!=L"delete"))return 123;
    std::wstring hive,key,name,value;if(!decode(lines[4],hive)||!decode(lines[5],key)||!decode(lines[6],name)||!decode(lines[8],value)||!allowed(hive,key,name))return 124;
    DWORD type=lines[7]==L"REG_SZ"?REG_SZ:lines[7]==L"REG_DWORD"?REG_DWORD:lines[7]==L"none"?REG_NONE:~0u;if(type==~0u)return 125;
    HKEY root=hive==L"HKCU"?HKEY_CURRENT_USER:hive==L"HKLM"?HKEY_LOCAL_MACHINE:nullptr;if(!root)return 126;
    HKEY opened{};REGSAM rights=KEY_WOW64_64KEY|(lines[3]==L"read"?KEY_QUERY_VALUE:KEY_QUERY_VALUE|KEY_SET_VALUE);
    LONG status=lines[3]==L"write"?RegCreateKeyExW(root,key.c_str(),0,nullptr,0,rights,nullptr,&opened,nullptr):RegOpenKeyExW(root,key.c_str(),0,rights,&opened);
    if(status==ERROR_FILE_NOT_FOUND&&lines[3]!=L"write"){result(lines[1],lines[2],lines[3],0,false,REG_NONE,L"");return 0;}
    if(status!=ERROR_SUCCESS){result(lines[1],lines[2],lines[3],status,false,REG_NONE,L"");return 1;}
    if(lines[3]==L"read"){
        DWORD actual=0,size=0;status=RegQueryValueExW(opened,name.c_str(),nullptr,&actual,nullptr,&size);
        if(status==ERROR_FILE_NOT_FOUND){RegCloseKey(opened);result(lines[1],lines[2],lines[3],0,false,REG_NONE,L"");return 0;}
        if(status!=ERROR_SUCCESS||size>65536||actual!=REG_SZ&&actual!=REG_DWORD){RegCloseKey(opened);result(lines[1],lines[2],lines[3],status,false,actual,L"");return 1;}
        std::vector<unsigned char> data(size+sizeof(wchar_t));status=RegQueryValueExW(opened,name.c_str(),nullptr,&actual,data.data(),&size);RegCloseKey(opened);
        if(status!=ERROR_SUCCESS){result(lines[1],lines[2],lines[3],status,false,actual,L"");return 1;}
        std::wstring text;if(actual==REG_SZ){if(size<sizeof(wchar_t)||size%sizeof(wchar_t)){result(lines[1],lines[2],lines[3],ERROR_INVALID_DATA,false,actual,L"");return 1;}auto chars=reinterpret_cast<wchar_t*>(data.data());size_t count=size/sizeof(wchar_t);if(count&&chars[count-1]==0)--count;text.assign(chars,count);}else{if(size!=4){result(lines[1],lines[2],lines[3],ERROR_INVALID_DATA,false,actual,L"");return 1;}text=std::to_wstring(*reinterpret_cast<DWORD*>(data.data()));}
        result(lines[1],lines[2],lines[3],0,true,actual,text);return 0;
    }
    if(lines[3]==L"delete")status=RegDeleteValueW(opened,name.c_str());
    else if(type==REG_DWORD){wchar_t* end=nullptr;unsigned long parsed=wcstoul(value.c_str(),&end,10);status=end&&!*end?RegSetValueExW(opened,name.c_str(),0,REG_DWORD,reinterpret_cast<const BYTE*>(&parsed),sizeof(parsed)):ERROR_INVALID_DATA;}
    else status=RegSetValueExW(opened,name.c_str(),0,REG_SZ,reinterpret_cast<const BYTE*>(value.c_str()),DWORD((value.size()+1)*sizeof(wchar_t)));
    if(status==ERROR_FILE_NOT_FOUND&&lines[3]==L"delete")status=ERROR_SUCCESS;RegCloseKey(opened);result(lines[1],lines[2],lines[3],status,false,REG_NONE,L"");return status==ERROR_SUCCESS?0:1;
}
