@echo off
setlocal
set "PSP_MODEL_APP=phantasy-star.exe"
call "%~dp0Load-Dagan.cmd"
exit /b %errorlevel%
