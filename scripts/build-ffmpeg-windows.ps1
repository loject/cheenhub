$ErrorActionPreference = 'Stop'

if ($env:VSCMD_ARG_TGT_ARCH -notin @('x64', 'amd64') -or -not $env:VCToolsInstallDir) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
        throw 'FFmpeg requires Visual Studio 2022 with the MSVC x64 C++ build tools.'
    }
    $vs = & $vswhere -latest -products '*' -version '[17.0,18.0)' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $vs) {
        throw 'FFmpeg requires Visual Studio 2022 with the MSVC x64 C++ build tools.'
    }
    & (Join-Path $vs 'Common7\Tools\Launch-VsDevShell.ps1') -SkipAutomaticLocation -Arch amd64 -HostArch amd64
}

$msysRoot = $env:MSYS2_LOCATION
if (-not $msysRoot) {
    $msysRoot = 'C:\msys64'
}
$msysBin = Join-Path $msysRoot 'usr\bin'
$missing = @('cygpath', 'sh', 'make', 'pkgconf', 'nasm') | Where-Object {
    -not (Test-Path -LiteralPath (Join-Path $msysBin "$_.exe") -PathType Leaf)
}
if ($missing) {
    throw "FFmpeg requires MSYS2 tools in $msysBin (missing: $($missing -join ', ')). Set MSYS2_LOCATION to the MSYS2 root and install make pkgconf nasm diffutils perl xz with pacman."
}

$msvcBin = Split-Path -Parent (Get-Command cl.exe -CommandType Application | Select-Object -First 1).Source
$env:Path = "$msvcBin;$msysBin;$env:Path"
Write-Host "[FFmpeg] MSYS2: $msysBin"
Write-Host "[FFmpeg] MSVC: $msvcBin"

& cargo run --locked -p xtask -- build-ffmpeg
exit $LASTEXITCODE
