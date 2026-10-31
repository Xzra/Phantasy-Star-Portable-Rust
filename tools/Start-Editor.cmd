@echo off
setlocal
set "PSP_PROJECT=%~f1"
if "%~1"=="" set "PSP_PROJECT=%~dp0projects\starter\project.json"
cd /d "%~dp0"
if not exist "%~dp0engine-editor.exe" (
  echo engine-editor.exe is missing. Extract the complete engine ZIP first.
  pause
  exit /b 1
)
start "" "%~dp0engine-editor.exe" --project "%PSP_PROJECT%"
