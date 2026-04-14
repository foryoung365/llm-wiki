@echo off
setlocal

if not "%LLMWIKI_BIN%"=="" (
  "%LLMWIKI_BIN%" %*
  exit /b %ERRORLEVEL%
)

if not "%LLMWIKI_INSTALL_PATH%"=="" if exist "%LLMWIKI_INSTALL_PATH%" (
  "%LLMWIKI_INSTALL_PATH%" %*
  exit /b %ERRORLEVEL%
)

if defined LOCALAPPDATA (
  set "LLMWIKI_SHARED=%LOCALAPPDATA%\llmwiki\bin\llmwiki.exe"
) else if defined USERPROFILE (
  set "LLMWIKI_SHARED=%USERPROFILE%\AppData\Local\llmwiki\bin\llmwiki.exe"
)

if defined LLMWIKI_SHARED if exist "%LLMWIKI_SHARED%" (
  "%LLMWIKI_SHARED%" %*
  exit /b %ERRORLEVEL%
)

where llmwiki >nul 2>nul
if %ERRORLEVEL% EQU 0 (
  llmwiki %*
  exit /b %ERRORLEVEL%
)

echo llmwiki is not installed in the shared location. Run `llmwiki install` first. 1>&2
exit /b 1
