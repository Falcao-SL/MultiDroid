$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$packages = Join-Path $root 'rust\target\packages'
$files = @(Get-ChildItem -LiteralPath $packages -Filter '*-setup.exe' -File)
if ($files.Count -ne 1) { throw "Expected exactly one NSIS installer, found $($files.Count)" }
if ($files[0].Length -lt 100000) { throw 'Installer is unexpectedly small' }
$out = Join-Path $root 'rust\out'
New-Item -ItemType Directory -Force -Path $out | Out-Null
$name = "MultiDroid-Preview-$($env:VERSION)-windows-x64-setup.exe"
Copy-Item -LiteralPath $files[0].FullName -Destination (Join-Path $out $name)
Write-Output "Validated Windows installer: $name"
