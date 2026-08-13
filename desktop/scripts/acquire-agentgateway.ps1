param(
    [string]$Version = "v1.4.1"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$desktop = Split-Path -Parent $PSScriptRoot
$binaryDir = Join-Path $desktop "src-tauri\binaries"
$resourceDir = Join-Path $desktop "src-tauri\resources\agentgateway"
$asset = "agentgateway-windows-amd64.exe"
$release = "https://github.com/agentgateway/agentgateway/releases/download/$Version"
$binary = Join-Path $binaryDir $asset
$checksum = "$binary.sha256"

$null = New-Item -ItemType Directory -Force -Path $binaryDir, $resourceDir
Invoke-WebRequest -Uri "$release/$asset" -OutFile $binary
Invoke-WebRequest -Uri "$release/$asset.sha256" -OutFile $checksum

$expected = (Get-Content $checksum -Raw).Split([char[]]" `t", [System.StringSplitOptions]::RemoveEmptyEntries)[0].Trim().ToLowerInvariant()
$actual = (Get-FileHash $binary -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $expected) {
    throw "AgentGateway checksum verification failed."
}

Copy-Item $binary (Join-Path $resourceDir "agentgateway.exe") -Force
Copy-Item $binary (Join-Path $binaryDir "agentgateway-x86_64-pc-windows-msvc.exe") -Force
Write-Output "AgentGateway $Version verified and staged."
