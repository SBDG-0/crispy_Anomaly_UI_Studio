param(
    [string]$Tag = '',
    [string]$OutputDirectory = 'release-artifacts',
    [switch]$NotesOnly
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = Split-Path $PSScriptRoot -Parent
Push-Location $repo
try {
    $manifest = Get-Content Cargo.toml -Raw
    $package = [regex]::Match($manifest, '(?ms)^\[package\]\s*$(?<body>.*?)(?=^\[|\z)').Groups['body'].Value
    $versionText = [regex]::Match($package, '(?m)^version\s*=\s*"(?<version>\d+\.\d+\.\d+)"').Groups['version'].Value
    if (!$versionText) { throw 'Could not read the application version from Cargo.toml.' }
    $version = [version]$versionText
    if (!$Tag) { $Tag = "v$($version.Major).$($version.Minor)" }
    if ($Tag -notmatch '^v(\d+)\.(\d+)(?:\.(\d+))?$') { throw "Use a version tag such as v0.7 or v0.7.0; got '$Tag'." }
    $patch = if ($Matches[3]) { [int]$Matches[3] } else { 0 }
    if ([int]$Matches[1] -ne $version.Major -or [int]$Matches[2] -ne $version.Minor -or $patch -ne $version.Build) {
        throw "Tag '$Tag' does not match Cargo.toml version '$versionText'."
    }
    $readme = Get-Content README.md -Raw
    $featureSection = [regex]::Match($readme, '(?ms)^## Two editing modes\s*$(?<body>.*?)(?=^## |\z)').Groups['body'].Value
    $features = @([regex]::Matches($featureSection, '(?m)^- .+$') | ForEach-Object Value) -join "`n"
    if (!$features) { throw 'README.md must contain features under Two editing modes.' }
    $changelog = Get-Content CHANGELOG.md -Raw
    $entry = $null
    foreach ($candidate in [regex]::Matches($changelog, '(?ms)^## (?<version>\d+\.\d+(?:\.\d+)?)(?:[^\r\n]*)\r?\n(?<body>.*?)(?=^## |\z)')) {
        $entryVersion = $candidate.Groups['version'].Value
        if (($entryVersion -eq $versionText) -or ($version.Build -eq 0 -and $entryVersion -eq "$($version.Major).$($version.Minor)")) { $entry = $candidate.Groups['body'].Value.Trim(); break }
    }
    if (!$entry) { throw "CHANGELOG.md has no entry for '$versionText'." }
    New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
    $output = (Resolve-Path -LiteralPath $OutputDirectory).Path
    $notes = Join-Path $output 'release-notes.md'
    $executable = "Anomaly Studio v$($version.Major).$($version.Minor).exe"
    $zipName = "Anomaly-Studio-$Tag-windows-x64.zip"
    $title = "Anomaly Studio $Tag"
    $body = @"
## Features

$features

## Changelog

$entry

## Downloads

Download **$zipName** for the Windows x64 application, documentation and licenses. The standalone **$executable** is also available.
"@
    Set-Content -LiteralPath $notes -Value $body -Encoding utf8
    if (!$NotesOnly) {
        $exe = Join-Path dist $executable
        $required = @($exe, 'dist/README.md', 'dist/CHANGELOG.md', 'dist/LICENSE', 'dist/LICENSE-icons.txt', 'dist/LICENSE-layout.txt', 'dist/docs', 'dist/screenshots')
        foreach ($item in $required) { if (!(Test-Path -LiteralPath $item)) { throw "Build output is missing: $item. Run ./build.ps1 -Test first." } }
        Copy-Item -LiteralPath $exe -Destination (Join-Path $output $executable) -Force
        $zip = Join-Path $output $zipName
        # Explicit package inputs exclude local settings, recovery data and unrelated dist files.
        Compress-Archive -LiteralPath $required -DestinationPath $zip -Force
        $hashes = foreach ($asset in @((Join-Path $output $executable), $zip)) {
            $hash = Get-FileHash -LiteralPath $asset -Algorithm SHA256
            "$($hash.Hash.ToLowerInvariant())  $(Split-Path $asset -Leaf)"
        }
        $checksums = Join-Path $output 'SHA256SUMS.txt'
        Set-Content -LiteralPath $checksums -Value $hashes -Encoding utf8
        [ordered]@{tag=$Tag; title=$title; notes=$notes; assets=@((Join-Path $output $executable), $zip, $checksums)} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'release.json') -Encoding utf8
    }
    Write-Host "Prepared Features and Changelog release notes for $Tag."
} finally { Pop-Location }
