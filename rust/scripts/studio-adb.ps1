# Sobe o adb na porta 5037 (a do Android Studio) e lista os aparelhos.
# Rode isto e depois ligue o aparelho no MultiDroid.
# Detalhe: docs/arquitetura/ADR-001-nucleo-nativo-e-android-studio.md
$ErrorActionPreference = "Stop"

$nome = "adb.exe"
$cands = @()
if ($env:MULTIDROID_ADB) { $cands += $env:MULTIDROID_ADB }
if ($env:ANDROID_HOME) { $cands += (Join-Path $env:ANDROID_HOME "platform-tools\$nome") }
if ($env:ANDROID_SDK_ROOT) { $cands += (Join-Path $env:ANDROID_SDK_ROOT "platform-tools\$nome") }
if ($env:LOCALAPPDATA) {
    $cands += (Join-Path $env:LOCALAPPDATA "MultiDroid\sdk\platform-tools\$nome")
    $cands += (Join-Path $env:LOCALAPPDATA "Android\Sdk\platform-tools\$nome")
}

$adb = $cands | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
if (-not $adb) {
    $noPath = Get-Command adb -ErrorAction SilentlyContinue
    if ($noPath) { $adb = $noPath.Source }
}
if (-not $adb) {
    Write-Error "Não achei o adb. Baixe as platform-tools no MultiDroid (Imagens) ou instale o Android Studio."
}

Remove-Item Env:ANDROID_ADB_SERVER_PORT -ErrorAction SilentlyContinue
Remove-Item Env:ADB_SERVER_SOCKET -ErrorAction SilentlyContinue
& $adb start-server
Write-Output "adb: $adb"
Write-Output ""
& $adb devices -l
Write-Output @"

No Android Studio, o aparelho do MultiDroid entra na lista do Run com o
serial emulator-5600, emulator-5602, ... (a porta de console). Escolha esse
aparelho. Não crie um AVD no Studio para isso.

Se a lista vier sem o emulator-*, ligue o aparelho depois deste script.
Se ele já estava ligado, desligue e ligue de novo.

O guest no Windows continua em WHPX. Este script só anuncia o aparelho ao adb.
O Studio e o MultiDroid usam o mesmo servidor, na porta 5037.
Não defina ANDROID_ADB_SERVER_PORT.
"@
