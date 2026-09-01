@echo off
setlocal EnableExtensions EnableDelayedExpansion

if "%~1"=="" exit /b 64
if "%~2"=="" exit /b 64
if "%~3"=="" exit /b 64
if "%~4"=="" exit /b 64

set "WR0_NONCE_VALUE=%~1"
set "WR0_RUN_VALUE=%~2"
set "WR0_CONTRACT_DIGEST=%~3"
set "WR0_WORKLOAD_SHA256=%~4"

if not "!WR0_NONCE_VALUE:~32,1!"=="" exit /b 65
if "!WR0_NONCE_VALUE:~31,1!"=="" exit /b 65
for /f "delims=0123456789abcdef" %%H in ("!WR0_NONCE_VALUE!") do exit /b 65

if not "!WR0_RUN_VALUE!"=="1" if not "!WR0_RUN_VALUE!"=="2" if not "!WR0_RUN_VALUE!"=="37" exit /b 66

if not "!WR0_CONTRACT_DIGEST:~64,1!"=="" exit /b 67
if "!WR0_CONTRACT_DIGEST:~63,1!"=="" exit /b 67
for /f "delims=0123456789abcdef" %%H in ("!WR0_CONTRACT_DIGEST!") do exit /b 67

if not "!WR0_WORKLOAD_SHA256:~64,1!"=="" exit /b 68
if "!WR0_WORKLOAD_SHA256:~63,1!"=="" exit /b 68
for /f "delims=0123456789abcdef" %%H in ("!WR0_WORKLOAD_SHA256!") do exit /b 68

set "WR0_ARCH_VALUE=unknown"
if /I "%PROCESSOR_ARCHITECTURE%"=="AMD64" set "WR0_ARCH_VALUE=x86_64"
if /I "%PROCESSOR_ARCHITECTURE%"=="ARM64" set "WR0_ARCH_VALUE=arm64"
if /I "%PROCESSOR_ARCHITECTURE%"=="x86" set "WR0_ARCH_VALUE=x86"
if "!WR0_ARCH_VALUE!"=="unknown" exit /b 69

set "WR0_STATE_DIR=%USERPROFILE%\AppData\Local\LinuxVSTBridge\WR0"
set "WR0_HANDSHAKE_DIR=!WR0_STATE_DIR!\handshake"
set "WR0_READY_FILE=!WR0_HANDSHAKE_DIR!\ready-!WR0_NONCE_VALUE!.txt"
set "WR0_READY_TEMP=!WR0_HANDSHAKE_DIR!\ready-!WR0_NONCE_VALUE!.tmp"
set "WR0_GATE_FILE=!WR0_HANDSHAKE_DIR!\gate-!WR0_NONCE_VALUE!.txt"

if not exist "!WR0_HANDSHAKE_DIR!" mkdir "!WR0_HANDSHAKE_DIR!"
if errorlevel 1 exit /b 70
if exist "!WR0_READY_FILE!" exit /b 73
if exist "!WR0_READY_TEMP!" exit /b 73
if exist "!WR0_GATE_FILE!" exit /b 74

>"!WR0_READY_TEMP!" echo WR0_HANDSHAKE_SCHEMA=LINUX_VST_BRIDGE_WR0_HANDSHAKE_V1
>>"!WR0_READY_TEMP!" echo WR0_NONCE=!WR0_NONCE_VALUE!
>>"!WR0_READY_TEMP!" echo WR0_RUN=!WR0_RUN_VALUE!
>>"!WR0_READY_TEMP!" echo WR0_ARCH=!WR0_ARCH_VALUE!
if errorlevel 1 goto wr0_ready_write_failed
move /y "!WR0_READY_TEMP!" "!WR0_READY_FILE!" >nul
if errorlevel 1 goto wr0_ready_write_failed

echo WR0_MAGIC=LINUX_VST_BRIDGE_WR0_V1
echo WR0_NONCE=!WR0_NONCE_VALUE!
echo WR0_RUN=!WR0_RUN_VALUE!
echo WR0_ARCH=!WR0_ARCH_VALUE!
echo WR0_READY=waiting_for_supervisor

for /L %%I in (1,1,200000000) do if exist "!WR0_GATE_FILE!" goto wr0_gate_found
del /q "!WR0_READY_FILE!" >nul 2>nul
echo WR0_HANDSHAKE_ERROR=gate_wait_exhausted
exit /b 75

:wr0_gate_found
set "WR0_GATE_LINE_1=__WR0_MISSING__"
set "WR0_GATE_LINE_2=__WR0_MISSING__"
set "WR0_GATE_LINE_3=__WR0_MISSING__"
set "WR0_GATE_LINE_COUNT=0"
for /f "usebackq delims=" %%G in ("!WR0_GATE_FILE!") do (
  set /a WR0_GATE_LINE_COUNT+=1 >nul
  if !WR0_GATE_LINE_COUNT! EQU 1 set "WR0_GATE_LINE_1=%%G"
  if !WR0_GATE_LINE_COUNT! EQU 2 set "WR0_GATE_LINE_2=%%G"
  if !WR0_GATE_LINE_COUNT! EQU 3 set "WR0_GATE_LINE_3=%%G"
)
if not "!WR0_GATE_LINE_1!"=="WR0_HANDSHAKE_SCHEMA=LINUX_VST_BRIDGE_WR0_HANDSHAKE_V1" goto wr0_gate_schema_invalid
if not "!WR0_GATE_LINE_2!"=="WR0_NONCE=!WR0_NONCE_VALUE!" goto wr0_gate_nonce_invalid
if not "!WR0_GATE_LINE_3!"=="WR0_RUN=!WR0_RUN_VALUE!" goto wr0_gate_run_invalid
if not "!WR0_GATE_LINE_COUNT!"=="3" goto wr0_gate_extra_invalid

del /q "!WR0_READY_FILE!" >nul 2>nul
del /q "!WR0_GATE_FILE!" >nul 2>nul
if exist "!WR0_READY_FILE!" exit /b 77
if exist "!WR0_GATE_FILE!" exit /b 77
echo WR0_GATE=accepted

if /I "%~5"=="--exit-37" (
  echo WR0_EXIT=37
  exit /b 37
)
if not "%~5"=="" exit /b 78

set "WR0_RECEIPT_FILE=!WR0_STATE_DIR!\receipt.txt"
>"!WR0_RECEIPT_FILE!" echo WR0_RECEIPT_SCHEMA=LINUX_VST_BRIDGE_WR0_RECEIPT_V2
>>"!WR0_RECEIPT_FILE!" echo WR0_NONCE=!WR0_NONCE_VALUE!
>>"!WR0_RECEIPT_FILE!" echo WR0_RUN=!WR0_RUN_VALUE!
>>"!WR0_RECEIPT_FILE!" echo WR0_ARCH=!WR0_ARCH_VALUE!
>>"!WR0_RECEIPT_FILE!" echo WR0_CONTRACT_SOURCE_SCHEMA=linux-vst-bridge-wr0-contract-source/v1
>>"!WR0_RECEIPT_FILE!" echo WR0_CONTRACT_SOURCE_SHA256=!WR0_CONTRACT_DIGEST!
>>"!WR0_RECEIPT_FILE!" echo WR0_WORKLOAD_SHA256=!WR0_WORKLOAD_SHA256!
if errorlevel 1 exit /b 71

set "WR0_READBACK_SCHEMA="
set /p WR0_READBACK_SCHEMA=<"!WR0_RECEIPT_FILE!"
if not "!WR0_READBACK_SCHEMA!"=="WR0_RECEIPT_SCHEMA=LINUX_VST_BRIDGE_WR0_RECEIPT_V2" exit /b 72

echo WR0_RECEIPT=written_and_read
echo WR0_EXIT=0
exit /b 0

:wr0_ready_write_failed
del /q "!WR0_READY_TEMP!" >nul 2>nul
del /q "!WR0_READY_FILE!" >nul 2>nul
echo WR0_HANDSHAKE_ERROR=ready_write_failed
exit /b 76

:wr0_gate_schema_invalid
set "WR0_GATE_FAILURE=gate_schema_invalid"
goto wr0_gate_invalid

:wr0_gate_nonce_invalid
set "WR0_GATE_FAILURE=gate_nonce_invalid"
goto wr0_gate_invalid

:wr0_gate_run_invalid
set "WR0_GATE_FAILURE=gate_run_invalid"
goto wr0_gate_invalid

:wr0_gate_extra_invalid
set "WR0_GATE_FAILURE=gate_extra_invalid"
goto wr0_gate_invalid

:wr0_gate_invalid
del /q "!WR0_READY_FILE!" >nul 2>nul
del /q "!WR0_GATE_FILE!" >nul 2>nul
echo WR0_HANDSHAKE_ERROR=!WR0_GATE_FAILURE!
exit /b 79
