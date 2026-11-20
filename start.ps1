# SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Bageo-Commercial
# SPDX-FileCopyrightText: h4lo-void <https://github.com/h4lo-void/bageo-call>

# Bageo - start the agent on Windows.
#
#   Right-click > "Run with PowerShell", or double-click start.cmd, or:
#   powershell -ExecutionPolicy Bypass -File .\start.ps1
#
# In a downloaded release (agent.exe and bageo-status.exe next to this script)
# it starts Bageo Status and ends. Bageo then runs in the background, with an
# icon in the system tray and no window. Bageo Status reads .env and opens the
# web console the first time. Use -Console to run Bageo in this window instead.
#
# With -Console or -Build, or when Bageo Status is not there, in order:
#   1. reads settings from .env next to this script (optional, see .env.example)
#   2. finds the agent program (a downloaded release, or builds it with Rust)
#   3. starts the agent and opens the web console
#   4. the first time the agent answers, puts a "Bageo Console" shortcut to the
#      web console on the desktop (-NoShortcut or BAGEO_NO_SHORTCUT=1: don't)
param(
    [switch]$Console,        # run Bageo in this window, not in the background
    [switch]$NoBrowser,      # do not open the web console
    [switch]$Yes,            # no longer needed; accepted so existing commands keep working
    [switch]$Build,          # build from source even when a program is already there
    [switch]$NoShortcut      # do not put a shortcut to the web console on the desktop
)
$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
Set-Location $root

function Say($msg) { Write-Host "[bageo] $msg" }

# A downloaded release: hand over to Bageo Status, which runs Bageo in the background.
$statusApp = Join-Path $root "bageo-status.exe"
if (-not $Console -and -not $Build -and (Test-Path (Join-Path $root "agent.exe")) -and (Test-Path $statusApp)) {
    Say "Bageo runs in the background. Its icon is in the system tray, next to the clock (-Console: run it in this window)."
    Start-Process -FilePath $statusApp -WorkingDirectory $root
    exit 0
}

# 1. .env -> environment (real environment variables win)
$envFile = Join-Path $root ".env"
if (Test-Path $envFile) {
    foreach ($line in Get-Content $envFile -Encoding UTF8) {
        $t = $line.Trim()
        if ($t -eq "" -or $t.StartsWith("#") -or -not $t.Contains("=")) { continue }
        $i = $t.IndexOf("=")
        $key = $t.Substring(0, $i).Trim()
        $val = $t.Substring($i + 1).Trim().Trim('"').Trim("'")
        if ($key -and -not (Test-Path "Env:$key")) { Set-Item -Path "Env:$key" -Value $val }
    }
}

# 2. the agent program
$candidates = @(
    (Join-Path $root "agent.exe"),
    (Join-Path $root "bin\agent.exe"),
    (Join-Path $root "target\release\agent.exe")
)
$agent = $null
if (-not $Build) { $agent = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1 }
if (-not $agent) {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Say "building the agent (first time takes a few minutes)..."
        & cargo build --release -p agent
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
        $agent = Join-Path $root "target\release\agent.exe"
    } else {
        Say "The agent program was not found."
        Say "Either download a release from the project's GitHub Releases page and unzip it here,"
        Say "or install Rust (https://rustup.rs) and run this script again to build it."
        exit 1
    }
}

# 3. run
$port = if ($env:AGENT_PORT) { $env:AGENT_PORT } else { "8787" }
$url = "http://localhost:$port/"
Say "starting $agent"
Say "web console: $url   (Ctrl+C here stops the agent)"
if (-not $NoBrowser) {
    Start-Job -ScriptBlock { param($u) Start-Sleep -Seconds 2; Start-Process $u } -ArgumentList $url | Out-Null
}

# 4. desktop shortcut to the web console, once the agent answers; kept when it exists
$noShortcutEnv = @("1", "true", "yes", "on") -contains ("$($env:BAGEO_NO_SHORTCUT)").ToLower()
if (-not $NoShortcut -and -not $noShortcutEnv) {
    # The name in the system's language (written as escapes: this file is read as ANSI).
    $label = switch -Regex ((Get-UICulture).Name) {
        "^zh" { [regex]::Unescape("Bageo \u63a7\u5236\u53f0"); break }
        "^ja" { [regex]::Unescape("Bageo \u30b3\u30f3\u30bd\u30fc\u30eb"); break }
        "^ko" { [regex]::Unescape("Bageo \ucf58\uc194"); break }
        "^es" { "Consola de Bageo"; break }
        "^fr" { "Console Bageo"; break }
        "^de" { "Bageo-Konsole"; break }
        "^pt" { "Console do Bageo"; break }
        default { "Bageo Console" }
    }
    $desktop = [Environment]::GetFolderPath("Desktop")
    if ($desktop) {
        $shortcut = Join-Path $desktop "$label.url"
        if (-not (Test-Path -LiteralPath $shortcut)) {
            # Ask the agent where it listens: 127.0.0.1 unless AGENT_HOST names one address.
            $probeHost = "127.0.0.1"
            $listenHost = "$env:AGENT_HOST".Trim()
            if ($listenHost -and $listenHost -ne "0.0.0.0" -and $listenHost -ne "::") {
                $probeHost = if ($listenHost.Contains(":")) { "[$listenHost]" } else { $listenHost }
            }
            Start-Job -ScriptBlock {
                param($probe, $file, $target)
                for ($i = 0; $i -lt 60; $i++) {
                    Start-Sleep -Seconds 2
                    try { Invoke-WebRequest -Uri $probe -UseBasicParsing -TimeoutSec 2 | Out-Null } catch { continue }
                    Set-Content -LiteralPath $file -Value "[InternetShortcut]`r`nURL=$target" -Encoding ASCII
                    break
                }
            } -ArgumentList "http://${probeHost}:$port/health", $shortcut, $url | Out-Null
        }
    }
}
& $agent
exit $LASTEXITCODE
