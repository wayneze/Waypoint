$ErrorActionPreference = 'Stop'
$nativeSource = $PSScriptRoot
$tauriRoot = [IO.Path]::GetFullPath((Join-Path $nativeSource '../..'))
$nativeCache = Join-Path $tauriRoot 'target/scaling'
New-Item -ItemType Directory -Path $nativeCache -Force | Out-Null
$inputFiles = Get-ChildItem -LiteralPath $nativeSource -Recurse -File | Sort-Object FullName
$signature = ($inputFiles | ForEach-Object { (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }) -join ''
$signaturePath = Join-Path $nativeCache 'signature'
$payload = Join-Path $nativeCache 'payload.tar.gz'
# Compiler flags affect paths embedded in assertions and must invalidate the cache.
$signature += $env:CL + $env:WAYPOINT_NATIVE_CXXFLAGS
if ((Test-Path $signaturePath) -and (Test-Path $payload) -and [IO.File]::ReadAllText($signaturePath) -eq $signature) { exit 0 }

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vsRoot = & $vswhere -latest -version '[17.0,18.0)' -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (!$vsRoot) { throw '缩放源构建需要 Visual Studio 2022 C++ Build Tools' }
$msbuild = Join-Path $vsRoot 'MSBuild/Current/Bin/MSBuild.exe'
$cmakeBin = Join-Path $vsRoot 'Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin'
$sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin/10.0.26100.0/x64'
if (!(Test-Path "$sdkBin/makepri.exe")) { throw '缩放源构建需要 Windows SDK 10.0.26100.0' }
$pythonCommand = Get-Command py,python,python3 -ErrorAction SilentlyContinue |
  Where-Object { $_.Source -notlike '*WindowsApps*' } | Select-Object -First 1
if (!$pythonCommand) { throw '缩放源构建需要 Python 3，请安装 Python 并将 py 或 python 加入 PATH' }
$python = if ($pythonCommand.Name -eq 'py.exe' -or $pythonCommand.Name -eq 'py') {
  & $pythonCommand.Source -3 -c 'import sys; assert sys.version_info.major == 3; print(sys.executable)'
} else {
  & $pythonCommand.Source -c 'import sys; assert sys.version_info.major == 3; print(sys.executable)'
}
if ($LASTEXITCODE -or !$python -or !(Test-Path -LiteralPath $python)) { throw '无法找到可用的 Python 3' }

Copy-Item -Path "$nativeSource/vendor/*" -Destination $nativeCache -Recurse -Force
$packages = @(
  @{Id='Microsoft.Windows.CppWinRT'; Version='2.0.250303.1'},
  @{Id='Microsoft.Windows.ImplementationLibrary'; Version='1.0.250325.1'}
)
foreach ($package in $packages) {
  $destination = Join-Path $nativeCache "packages/$($package.Id).$($package.Version)"
  if (!(Test-Path "$destination/build")) {
    $id = $package.Id.ToLowerInvariant(); $version = $package.Version
    $zip = Join-Path $nativeCache "$id.zip"
    Invoke-WebRequest -Uri "https://api.nuget.org/v3-flatcontainer/$id/$version/$id.$version.nupkg" -OutFile $zip
    Expand-Archive -LiteralPath $zip -DestinationPath $destination -Force
  }
}
$pythonTools = Join-Path $nativeCache 'python-tools'
if (!(Test-Path "$pythonTools/conan")) {
  & $python -m pip install --target $pythonTools 'conan==2.22.1' --quiet --disable-pip-version-check
  if ($LASTEXITCODE) { throw 'Conan 本地依赖恢复失败' }
}
$conanLauncher = Join-Path $nativeCache 'conan-local.py'
$launcher = @'
import os, sys
from pathlib import Path
root = Path(__file__).resolve().parent
sys.path.insert(0, str(root / "python-tools"))
os.environ["CONAN_HOME"] = str(root / "conan-cache")
os.environ["PATH"] = sys.argv.pop(1) + os.pathsep + os.environ["PATH"]
if len(sys.argv) > 1 and sys.argv[1] == "install" and os.environ.get("WAYPOINT_NATIVE_CXXFLAGS"):
    sys.argv.extend(["-c", "tools.build:cxxflags=" + os.environ["WAYPOINT_NATIVE_CXXFLAGS"]])
from conan.cli.cli import main
sys.exit(main(sys.argv[1:]))
'@
[IO.File]::WriteAllText($conanLauncher, $launcher)
# Always resolve dependencies after a signature change. Conan's package cache
# reuses unchanged inputs, while cxxflags participate in the package identity.
& $python $conanLauncher $cmakeBin install "$nativeCache/src/Magpie/conanfile.txt" "-pr:a=$nativeCache/src/_ConanDeps/msvc.profile" --output-folder "$nativeCache/obj/x64/Release/_ConanDeps/Magpie" -s build_type=Release -s arch=x86_64 -c tools.build:jobs=4 --build=missing
if ($LASTEXITCODE) { throw '缩放 C++ 依赖构建失败' }
& $msbuild "$nativeCache/src/Magpie.Core/Magpie.Core.vcxproj" /t:Rebuild /p:Configuration=Release /p:Platform=x64 "/p:SolutionDir=$nativeCache\" /m:4 /nologo /verbosity:minimal
if ($LASTEXITCODE) { throw 'Magpie.Core 构建失败' }
& $msbuild "$nativeSource/host.vcxproj" /t:Rebuild /p:Configuration=Release /p:Platform=x64 "/p:NativeCache=$nativeCache" /nologo /verbosity:minimal
if ($LASTEXITCODE) { throw 'Waypoint 缩放宿主构建失败' }

$runtime = Join-Path $nativeCache 'runtime'
foreach ($language in @(@{Name='en-US'; Source='en-US'}, @{Name='zh-Hans'; Source='zh-Hans'})) {
  $strings = Join-Path $runtime "Strings/$($language.Name)"
  New-Item -ItemType Directory -Path $strings -Force | Out-Null
  Copy-Item -LiteralPath "$nativeSource/vendor/src/Magpie/Resources.language-$($language.Source).resw" -Destination "$strings/Resources.resw" -Force
}
$priConfig = Join-Path $nativeCache 'pri.xml'
& "$sdkBin/makepri.exe" createconfig /cf $priConfig /dq en-US /o | Out-Null
$config = [IO.File]::ReadAllText($priConfig).Replace('type="resw" convertDotsToSlashes="true" initialPath=""','type="resw" convertDotsToSlashes="true" initialPath="Magpie"')
# Index only the language tree; folder traversal is required to discover its RESW files.
$config = $config -replace '<indexer-config type="PRI"\s*/>', ''
[IO.File]::WriteAllText($priConfig, $config)
& "$sdkBin/makepri.exe" new /pr "$runtime/Strings" /cf $priConfig /in WaypointScaling /of "$runtime/resources.pri" /o | Out-Null
if ($LASTEXITCODE) { throw '缩放资源包生成失败' }
New-Item -ItemType Directory -Path "$runtime/effects" -Force | Out-Null
Copy-Item -Path "$nativeSource/vendor/src/Effects/*" -Destination "$runtime/effects" -Recurse -Force
Copy-Item -LiteralPath "$nativeSource/LICENSE","$nativeSource/NOTICE.md" -Destination $runtime -Force
$packScript = Join-Path $nativeCache 'pack.py'
[IO.File]::WriteAllText($packScript, @'
import tarfile, sys
import os
from pathlib import Path
root = Path(sys.argv[1])
home = str(Path.home())
for binary in root.glob("*.exe") if os.environ.get("WAYPOINT_NATIVE_CXXFLAGS") else []:
    data = binary.read_bytes()
    for prefix in [home, home.replace("\\", "/")]:
        if any(prefix.encode(encoding) in data for encoding in ["utf-8", "utf-16le"]):
            raise RuntimeError("Native payload contains a local build path")
with tarfile.open(sys.argv[2], "w:gz") as archive:
    for name in ["waypoint-scaling.exe", "resources.pri", "effects", "LICENSE", "NOTICE.md"]:
        archive.add(root / name, arcname=name)
'@)
& $python $packScript $runtime $payload
if ($LASTEXITCODE) { throw '缩放组件打包失败' }
[IO.File]::WriteAllText($signaturePath, $signature)
Write-Output "Scaling payload: $payload"
