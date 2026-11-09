$ErrorActionPreference = 'Continue'
$env:RUSTUP_TOOLCHAIN = '1.97.1'
$env:LIBCLANG_PATH = 'C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\Llvm\x64\bin'
$env:PATH = 'C:\Program Files\Microsoft Visual Studio\18\Community\VC\Tools\Llvm\x64\bin;C:\Users\Cerbis\Documents\Default Project\nasm-temp\nasm-3.02;' + $env:PATH
Set-Location 'D:\Yappuccino'
Write-Output "=== start $(Get-Date -Format o) ==="
python scripts\prepare-windows-skia.py --jobs 16 --build 2>&1 | Tee-Object -FilePath 'D:\yappuccino-build.log'
Write-Output "=== exit $LASTEXITCODE $(Get-Date -Format o) ==="
