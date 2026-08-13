param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$desktop = Split-Path -Parent $PSScriptRoot
$root = Split-Path -Parent $desktop
$target = Join-Path $desktop "src-tauri\resources\qm"

if (Test-Path $target) {
    Remove-Item $target -Recurse -Force
}

New-Item -ItemType Directory -Force -Path $target | Out-Null

$entries = @(
    "src",
    "plugins\web-ui",
    "package.json",
    "package-lock.json",
    ".node-version",
    "node_modules"
)

foreach ($entry in $entries) {
    $source = Join-Path $root $entry
    if (-not (Test-Path $source)) {
        throw "Required QM runtime entry is missing: $source"
    }
    Copy-Item $source (Join-Path $target $entry) -Recurse -Force
}

Write-Output "QM runtime staged at $target"
