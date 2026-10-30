@echo off
setlocal
set "PSP_PREVIEW_DIR=%~dp0"
powershell.exe -NoLogo -NoProfile -Command "$ErrorActionPreference='Stop'; $root=$env:PSP_PREVIEW_DIR; $folder=Join-Path $root 'local'; New-Item -ItemType Directory -Force $folder | Out-Null; $file=Join-Path $folder 'dagan.ice'; if (!(Test-Path -LiteralPath $file)) { Write-Host 'Downloading the Classic Dagan model from the official PSO2 patch server...'; $temp=$file+'.download'; Invoke-WebRequest -UseBasicParsing -UserAgent 'AQUA_HTTP' -Uri 'https://download.pso2.jp/patch_prod/v74000_rc_83_masterbase/patches/data/win32/15d422cd829ec44a9584cf019d9f9014.pat' -OutFile $temp -TimeoutSec 60; if ((Get-FileHash -LiteralPath $temp -Algorithm SHA256).Hash.ToLowerInvariant() -ne '4f99143916c24cdbda9fc9be4fc4c8c2e268119f66269de37d13c177b9e76f96') { throw 'Downloaded file did not match the verified model archive.' }; Move-Item -LiteralPath $temp -Destination $file }; if ((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant() -ne '4f99143916c24cdbda9fc9be4fc4c8c2e268119f66269de37d13c177b9e76f96') { throw 'The cached model differs from the verified archive. Remove local\dagan.ice to download it again.' }; Write-Host 'Dagan model verified.'; if ($env:PSP_PREPARE_ONLY -ne '1') { & (Join-Path $root 'model-viewer.exe') $file; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE } }"
if errorlevel 1 (
  echo Model setup failed. You can also open your own supported ICE or AQP file.
  if not "%PSP_PREPARE_ONLY%"=="1" pause
  exit /b 1
)
