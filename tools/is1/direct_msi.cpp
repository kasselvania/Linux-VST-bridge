// Build an account-free empty MSI database using the Windows Installer API.
#include <windows.h>
#include <msi.h>
#include <msiquery.h>
#include <cstdio>
static UINT sql(MSIHANDLE db,const wchar_t* query) {
    MSIHANDLE view=0;UINT result=MsiDatabaseOpenViewW(db,query,&view);
    if(result==ERROR_SUCCESS)result=MsiViewExecute(view,0);
    if(view)MsiCloseHandle(view);return result;
}
int wmain(int argc,wchar_t** argv) {
    if(argc!=2)return 1;MSIHANDLE db=0;
    if(MsiOpenDatabaseW(argv[1],MSIDBOPEN_CREATE,&db)!=ERROR_SUCCESS)return 2;
    const wchar_t* queries[]={
      L"CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, `Value` CHAR(0) LOCALIZABLE PRIMARY KEY `Property`)",
      L"INSERT INTO `Property` (`Property`,`Value`) VALUES ('ProductCode','{1F993747-12F1-4AB8-97A0-1E08BA68E661}')",
      L"INSERT INTO `Property` (`Property`,`Value`) VALUES ('ProductName','IS1 source-owned MSI')",
      L"INSERT INTO `Property` (`Property`,`Value`) VALUES ('ProductVersion','1.0.0')",
      L"INSERT INTO `Property` (`Property`,`Value`) VALUES ('Manufacturer','Linux VST Bridge fixture')",
      L"INSERT INTO `Property` (`Property`,`Value`) VALUES ('ProductLanguage','1033')",
      L"INSERT INTO `Property` (`Property`,`Value`) VALUES ('ALLUSERS','1')",
      L"CREATE TABLE `Directory` (`Directory` CHAR(72) NOT NULL, `Directory_Parent` CHAR(72), `DefaultDir` CHAR(255) NOT NULL LOCALIZABLE PRIMARY KEY `Directory`)",
      L"INSERT INTO `Directory` (`Directory`,`DefaultDir`) VALUES ('TARGETDIR','SourceDir')",
      L"CREATE TABLE `InstallExecuteSequence` (`Action` CHAR(72) NOT NULL, `Condition` CHAR(255), `Sequence` SHORT PRIMARY KEY `Action`)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('CostInitialize',800)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('FileCost',900)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('CostFinalize',1000)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('InstallValidate',1400)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('InstallInitialize',1500)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('RegisterProduct',6100)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('PublishProduct',6400)",
      L"INSERT INTO `InstallExecuteSequence` (`Action`,`Sequence`) VALUES ('InstallFinalize',6600)"
    };
    for(auto query:queries){UINT result=sql(db,query);if(result){std::printf("MSI fixture SQL status %u\n",result);MsiCloseHandle(db);return 3;}}
    MSIHANDLE info=0;UINT code=MsiGetSummaryInformationW(db,nullptr,5,&info);
    if(!code)code=MsiSummaryInfoSetPropertyW(info,7,VT_LPSTR,0,nullptr,L"x64;1033");
    if(!code)code=MsiSummaryInfoSetPropertyW(info,9,VT_LPSTR,0,nullptr,L"{D04F0A26-9F76-44C2-B842-FA1CC8365158}");
    if(!code)code=MsiSummaryInfoSetPropertyW(info,14,VT_I4,200,nullptr,nullptr);
    if(!code)code=MsiSummaryInfoSetPropertyW(info,15,VT_I4,0,nullptr,nullptr);
    if(!code)code=MsiSummaryInfoPersist(info);
    if(info)MsiCloseHandle(info);
    if(!code)code=MsiDatabaseCommit(db);MsiCloseHandle(db);return code?4:0;
}
