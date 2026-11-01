@echo off
setlocal
if exist "%~dp0..\Cargo.toml" (
  cd /d "%~dp0.."
) else (
  cd /d "%~dp0"
)
if not exist Cargo.toml (
  echo Put Build-Engine.cmd beside Cargo.toml in the source folder.
  pause
  exit /b 1
)
where cargo >nul 2>nul
if errorlevel 1 (
  echo Install Rust with rustup and the Visual Studio C++ build tools first.
  echo Read START-HERE.md and docs\engine-guide.md.
  pause
  exit /b 1
)
cargo build --release --locked --features viewer --bin phantasy-star --bin engine-editor --bin model-viewer --bin psp-client --bin unreal-export
if errorlevel 1 (
  echo Build failed. Read the first error above.
  pause
  exit /b 1
)
echo Build complete. Opening your source project's editor.
"target\release\engine-editor.exe" --project "projects\starter\project.json"
