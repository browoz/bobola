param(
  [Parameter(Mandatory = $true)][string]$RootPids,
  [Parameter(Mandatory = $true)][string]$ProfileRoots,
  [Parameter(Mandatory = $true)][int]$DurationSeconds,
  [Parameter(Mandatory = $true)][int]$IntervalMilliseconds,
  [Parameter(Mandatory = $true)][string]$OutputPath,
  [string]$AllowedNames = 'service-workspace.exe,msedgewebview2.exe,Ferdium.exe,Rambox.exe'
)

$ErrorActionPreference = 'Stop'
# ProfileRoots and AllowedNames remain accepted for older callers. Ownership comes from
# explicit root PIDs and their descendants, never a machine-wide executable-name match.
$targetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $env:TEMP 'service-workspace-target' }
$samplerPath = Join-Path $targetRoot 'debug\resource-sampler.exe'
if (-not (Test-Path -LiteralPath $samplerPath)) { throw 'Build the native sampler: node scripts/rust.mjs build -p service-workspace --bin resource-sampler' }
if (-not (Test-Path -LiteralPath (Split-Path -Parent $OutputPath))) { throw 'Sampler output directory is missing.' }
$rootPidValues = @($RootPids.Split(',') | ForEach-Object { [int]::Parse($_) })
$samples = [System.Collections.Generic.List[object]]::new()
$deadline = [DateTime]::UtcNow.AddSeconds($DurationSeconds)
while ([DateTime]::UtcNow -lt $deadline) {
  $result = & $samplerPath @rootPidValues
  if ($LASTEXITCODE -ne 0) { throw "Native resource sampler failed: $result" }
  $sample = $result | ConvertFrom-Json
  $rows = @($sample.processes)
  $private = ($rows | Measure-Object privateBytes -Sum).Sum
  $working = ($rows | Measure-Object workingSetBytes -Sum).Sum
  $cpu = ($rows | Measure-Object cpuSeconds -Sum).Sum
  $samples.Add([pscustomobject]@{
    utc = [DateTime]::UtcNow.ToString('o')
    scope = 'owned-process-tree'
    processCount = $rows.Count
    unavailable = $sample.unavailable
    workingSetBytes = $working
    privateBytes = $private
    cpuSeconds = $cpu
    processes = $rows
  })
  Start-Sleep -Milliseconds $IntervalMilliseconds
}
ConvertTo-Json -InputObject @($samples) -Depth 6 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
