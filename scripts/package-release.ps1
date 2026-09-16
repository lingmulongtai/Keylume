param([string]$OutputDirectory = 'artifacts/release')
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
$root = Split-Path $PSScriptRoot -Parent
Set-Location -LiteralPath $root
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected a numeric release version' }
$tauriVersion = (Get-Content src-tauri/tauri.conf.json -Raw | ConvertFrom-Json).version
$cargoVersion = (Select-String -Path src-tauri/Cargo.toml -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
if ($version -ne $tauriVersion -or $version -ne $cargoVersion) { throw 'Version mismatch' }
$output = [IO.Path]::GetFullPath((Join-Path $root $OutputDirectory))
New-Item -ItemType Directory -Path $output -Force | Out-Null
if (@(Get-ChildItem -LiteralPath $output -File).Count) { throw 'Release output must be empty; use a new directory' }
$portable = Join-Path ([IO.Path]::GetTempPath()) ('keylume-portable-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $portable | Out-Null
Copy-Item -LiteralPath src-tauri/target/release/keylume.exe -Destination (Join-Path $portable 'Keylume.exe')
Copy-Item -LiteralPath README.md,THIRD_PARTY_NOTICES.md -Destination $portable
Copy-Item -LiteralPath licenses,docs -Destination $portable -Recurse
$installer = "Keylume_${version}_x64-setup.exe"
Copy-Item -LiteralPath "src-tauri/target/release/bundle/nsis/$installer" -Destination $output
$zip = "Keylume_${version}_windows-x64.zip"
Compress-Archive -Path (Join-Path $portable '*') -DestinationPath (Join-Path $output $zip)

$report = Join-Path $portable 'native-smoke.json'
$process = Start-Process -FilePath (Join-Path $portable 'Keylume.exe') -ArgumentList @('--tray','--self-test',('"{0}"' -f $report)) -WindowStyle Hidden -PassThru
if (-not $process.WaitForExit(60000)) {
    Stop-Process -Id $process.Id
    throw 'Native self-test timed out'
}
if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $report)) { throw 'Native self-test failed to produce a report' }
$smoke = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
if (-not $smoke.passed -or $smoke.checks.Count -ne 7 -or $smoke.hardwareTested) { throw "Native self-test failed: $($smoke.error)" }
@{ passed = $smoke.passed; checks = $smoke.checks; hardwareTested = $false } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output 'verification.json') -Encoding utf8NoBOM
$sha = (git rev-parse HEAD).Trim()
@{
    version = $version
    sourceCommit = $sha
    platform = 'windows-x86_64'
    signed = $false
    rust = (rustc --version)
    node = (node --version)
    workflowRun = $(if ($env:GITHUB_RUN_ID) { "https://github.com/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID" } else { $null })
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'build-provenance.json') -Encoding utf8NoBOM
$checksums = Get-ChildItem -LiteralPath $output -File | Sort-Object Name | ForEach-Object {
    '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.Name
}
[IO.File]::WriteAllText((Join-Path $output 'SHA256SUMS'), ($checksums -join "`n") + "`n")
Write-Output "Packaged Keylume $version from $sha; all seven native checks passed."
