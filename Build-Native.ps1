#requires -Version 7.0
param(
    [Parameter(Mandatory)][string]$ClientRoot,
    [switch]$SkipClient,
    [switch]$PortableOnly,
    [string]$HorizonLauncher,
    [string]$HorizonVersion
)
$ErrorActionPreference = 'Stop'
$taskBuild = Join-Path ([IO.Path]::GetFullPath($ClientRoot)) 'tools\Build-TurtleLauncher.ps1'
if (!(Test-Path -LiteralPath $taskBuild)) { throw 'ClientRoot must point to TurtleClient-Mod on branch codex/rust-26.3.' }
$taskOptions = @{LauncherRoot=$PSScriptRoot;SkipClient=$SkipClient;PortableOnly=$PortableOnly}
if ($HorizonLauncher) { $taskOptions.HorizonLauncher = $HorizonLauncher }
if ($HorizonVersion) { $taskOptions.HorizonVersion = $HorizonVersion }
& $taskBuild @taskOptions
