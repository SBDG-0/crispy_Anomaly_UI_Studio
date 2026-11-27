param([switch]$Test, [switch]$Offline)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath "$PSScriptRoot/.toolchain/cargo/bin/cargo.exe") {
    $env:RUSTUP_HOME = "$PSScriptRoot/.toolchain/rustup"
    $env:CARGO_HOME = "$PSScriptRoot/.toolchain/cargo"
    $env:PATH = "$PSScriptRoot/.toolchain/cargo/bin;$PSScriptRoot/.toolchain/w64devkit/bin;$env:PATH"
    $env:RUSTFLAGS = '-C link-self-contained=yes'
}
Push-Location $PSScriptRoot
try {
    $flags = @('--locked')
    if ($Offline) { $flags += '--offline' }
    if ($Test) {
        cargo test --workspace @flags
        if ($LASTEXITCODE -ne 0) { throw 'Tests failed.' }
    }
    cargo build --release -p anomaly-studio @flags
    if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
    $metadata = cargo metadata --no-deps --format-version 1 @flags | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Could not read build metadata.' }
    New-Item -ItemType Directory -Force -Path 'dist' | Out-Null
    $package = $metadata.packages | Where-Object { $_.name -eq 'anomaly-studio' } | Select-Object -First 1
    $version = [version]$package.version
    $outputName = "Anomaly Studio v$($version.Major).$($version.Minor).exe"
    Copy-Item -LiteralPath (Join-Path $metadata.target_directory 'release/anomaly-studio.exe') -Destination (Join-Path 'dist' $outputName)
    Copy-Item -LiteralPath 'docs' -Destination 'dist' -Recurse -Force
    Copy-Item -LiteralPath 'CHANGELOG.md' -Destination 'dist/CHANGELOG.md'
    Copy-Item -LiteralPath 'LICENSE' -Destination 'dist/LICENSE'
    $guide = (Get-Content -LiteralPath 'README.md' -Raw).Replace('(editors/icons/LICENSE)', '(LICENSE-icons.txt)').Replace('(editors/layout/LICENSE)', '(LICENSE-layout.txt)')
    $guide = $guide.Replace('made with ❤️ by SavageBulldogOG', '[Back to main page](../README.md)' + [Environment]::NewLine + [Environment]::NewLine + 'made with ❤️ by SavageBulldogOG')
    Set-Content -LiteralPath 'dist/README.md' -Value $guide -Encoding utf8
    Copy-Item -LiteralPath 'editors/icons/LICENSE' -Destination 'dist/LICENSE-icons.txt'
    Copy-Item -LiteralPath 'editors/layout/LICENSE' -Destination 'dist/LICENSE-layout.txt'
} finally { Pop-Location }
