$ErrorActionPreference = 'Stop'

if (-not $env:LIBCLANG_PATH) {
    $llvmBin = Join-Path $env:ProgramFiles 'LLVM\bin'
    if (Test-Path -LiteralPath (Join-Path $llvmBin 'libclang.dll')) {
        $env:LIBCLANG_PATH = $llvmBin
    }
}
if ($env:LIBCLANG_PATH) {
    Write-Host "[Dioxus] libclang: $env:LIBCLANG_PATH"
}
# bindgen обрабатывает заголовки FFmpeg в основном потоке build script.
$env:_LINK_ = "$env:_LINK_ /STACK:33554432".Trim()
& dx @args
exit $LASTEXITCODE
