# Run from PowerShell. -WhatIf previews rebuildable output removal.
[CmdletBinding(SupportsShouldProcess = $true)]
param([string[]]$KeepBuild = @('release'))

$ErrorActionPreference = 'Stop'
$waypointRoot = (Resolve-Path -LiteralPath $PSScriptRoot).Path
$waypointTarget = Join-Path $waypointRoot 'src-tauri\target'
if (-not (Test-Path -LiteralPath (Join-Path $waypointRoot 'src-tauri\Cargo.toml'))) {
    throw 'Run this script from the Waypoint project directory.'
}

# Preserve the current release and every running client's executable and installer.
$waypointRetained = @{}
foreach ($waypointProcess in @(Get-Process waypoint -ErrorAction SilentlyContinue)) {
    if ($waypointProcess.Path -and $waypointProcess.Path.StartsWith($waypointTarget + '\', [StringComparison]::OrdinalIgnoreCase)) {
        $waypointRetained[$waypointProcess.Path] = $true
    }
}
foreach ($waypointBuild in $KeepBuild) {
    if ($waypointBuild -notmatch '^[a-zA-Z0-9_-]+$') { throw "Invalid build name: $waypointBuild" }
    $waypointRelease = if ($waypointBuild -eq 'release') { Join-Path $waypointTarget 'release' } else { Join-Path $waypointTarget "$waypointBuild\release" }
    $waypointRetained[(Join-Path $waypointRelease 'waypoint.exe')] = $true
}
foreach ($waypointExe in @($waypointRetained.Keys)) {
    foreach ($waypointInstaller in @(Get-ChildItem -LiteralPath (Join-Path (Split-Path $waypointExe) 'bundle\nsis') -Filter '*-setup.exe' -File -ErrorAction SilentlyContinue)) {
        $waypointRetained[$waypointInstaller.FullName] = $true
    }
}
# Only the payload and its source signature are build inputs; native object
# files, dependency caches and copied sources can be regenerated.
foreach ($waypointNativeFile in @('payload.tar.gz', 'signature')) {
    $waypointRetained[(Join-Path $waypointTarget "scaling\$waypointNativeFile")] = $true
}
$waypointHashes = @{}
foreach ($waypointFile in $waypointRetained.Keys) {
    if (Test-Path -LiteralPath $waypointFile -PathType Leaf) { $waypointHashes[$waypointFile] = (Get-FileHash -LiteralPath $waypointFile).Hash }
}

$waypointCandidates = [Collections.Generic.List[string]]::new()
function Select-WaypointOutput([string]$OutputPath) {
    if ($waypointRetained.ContainsKey($OutputPath)) { return }
    $waypointHasRetained = @($waypointRetained.Keys | Where-Object { $_.StartsWith($OutputPath + '\', [StringComparison]::OrdinalIgnoreCase) }).Count -gt 0
    if ($waypointHasRetained) {
        foreach ($waypointChild in Get-ChildItem -LiteralPath $OutputPath -Force) { Select-WaypointOutput $waypointChild.FullName }
    } else { $waypointCandidates.Add($OutputPath) }
}
if (Test-Path -LiteralPath $waypointTarget) {
    foreach ($waypointChild in Get-ChildItem -LiteralPath $waypointTarget -Force) {
        Select-WaypointOutput $waypointChild.FullName
    }
}
$waypointBytes = 0L
$waypointSizes = @{}
foreach ($waypointPath in $waypointCandidates) {
    $waypointResolved = (Resolve-Path -LiteralPath $waypointPath).Path
    if (-not $waypointResolved.StartsWith($waypointTarget + '\', [StringComparison]::OrdinalIgnoreCase)) { throw "Outside target directory: $waypointResolved" }
    $waypointAncestor = Get-Item -LiteralPath $waypointResolved -Force
    if ($waypointAncestor.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Output is a link: $waypointResolved" }
    if (-not $waypointAncestor.PSIsContainer) { $waypointAncestor = $waypointAncestor.Directory }
    while ($waypointAncestor.FullName -ne $waypointRoot) {
        if ($waypointAncestor.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Directory link requires manual inspection: $($waypointAncestor.FullName)" }
        $waypointAncestor = $waypointAncestor.Parent
        if ($null -eq $waypointAncestor) { throw 'Project boundary check failed.' }
    }
    $waypointEntries = @(Get-Item -LiteralPath $waypointResolved -Force)
    if ($waypointEntries[0].PSIsContainer) { $waypointEntries += @(Get-ChildItem -LiteralPath $waypointResolved -Recurse -Force) }
    if ($waypointEntries | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint }) { throw "Output contains a link: $waypointResolved" }
    foreach ($waypointLock in @($waypointEntries | Where-Object { $_.Name -eq '.cargo-lock' -and -not $_.PSIsContainer })) {
        try {
            $waypointLockStream = [IO.File]::Open($waypointLock.FullName, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
            $waypointLockStream.Dispose()
        } catch { throw "Finish compilation before cleaning this build: $($waypointLock.DirectoryName)" }
    }
    $waypointSizes[$waypointResolved] = ($waypointEntries | Where-Object { -not $_.PSIsContainer } | Measure-Object -Property Length -Sum).Sum
}
foreach ($waypointPath in $waypointCandidates) {
    $waypointResolved = (Resolve-Path -LiteralPath $waypointPath).Path
    if ($PSCmdlet.ShouldProcess($waypointResolved, 'Delete rebuildable output')) {
        $waypointBytes += $waypointSizes[$waypointResolved]
        Remove-Item -LiteralPath $waypointResolved -Recurse -Force
    }
}
foreach ($waypointFile in $waypointHashes.Keys) {
    if ((Get-FileHash -LiteralPath $waypointFile).Hash -ne $waypointHashes[$waypointFile]) { throw "Retained artifact changed: $waypointFile" }
}
[pscustomobject]@{ RemovedBytes = $waypointBytes; MatchedPaths = $waypointCandidates.Count; RetainedArtifacts = $waypointHashes.Count }
