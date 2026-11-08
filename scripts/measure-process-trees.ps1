param(
  [int[]]$RootIds = @(),
  [int]$Seconds = 60,
  [int]$IntervalSeconds = 3,
  [string]$OutputFile = 'output/benchmarks/live-process-samples.json'
)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class WorkspaceMemory {
  [StructLayout(LayoutKind.Sequential)] public struct Counters {
    public uint cb, faults;
    public UIntPtr peakWs, ws, peakPaged, paged, peakNonPaged, nonPaged, pagefile, peakPagefile, privateBytes, privateWs;
    public ulong sharedCommit;
  }
  public class Reading { public long PrivateWorkingSet, PrivateCommit, WorkingSet; public int Error; }
  [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr OpenProcess(uint access,bool inherit,uint pid);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
  [DllImport("psapi.dll",SetLastError=true)] static extern bool GetProcessMemoryInfo(IntPtr handle,ref Counters counters,uint size);
  public static Reading Read(uint pid) {
    var result = new Reading();
    var handle = OpenProcess(0x1000,false,pid);
    if(handle==IntPtr.Zero) { result.Error=Marshal.GetLastWin32Error(); return result; }
    try {
      var value=new Counters(); value.cb=(uint)Marshal.SizeOf(typeof(Counters));
      if(!GetProcessMemoryInfo(handle,ref value,value.cb)) { result.Error=Marshal.GetLastWin32Error(); return result; }
      result.PrivateWorkingSet=(long)value.privateWs.ToUInt64(); result.PrivateCommit=(long)value.privateBytes.ToUInt64(); result.WorkingSet=(long)value.ws.ToUInt64();
      return result;
    } finally { CloseHandle(handle); }
  }
}
'@
$logicalProcessors = [Environment]::ProcessorCount
$initial = @(Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,CreationDate)
if (!$RootIds.Count) { $RootIds = @($initial | Where-Object { $_.Name -in @('service-workspace.exe','Ferdium.exe') -and $_.ParentProcessId -notin @($initial | Where-Object {$_.Name -eq 'Ferdium.exe'} | Select-Object -ExpandProperty ProcessId) } | Select-Object -ExpandProperty ProcessId) }
$roots = @($initial | Where-Object { $_.ProcessId -in $RootIds })
if (!$roots.Count) { throw 'No benchmark roots were found.' }
$previous = @{}
$samples = [Collections.Generic.List[object]]::new()
$clock = [Diagnostics.Stopwatch]::StartNew()
while ($clock.Elapsed.TotalSeconds -lt $Seconds) {
  $processes = @(Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,CreationDate)
  $now = [DateTimeOffset]::UtcNow
  foreach ($root in $roots) {
    $identity = $processes | Where-Object { $_.ProcessId -eq $root.ProcessId -and $_.CreationDate -eq $root.CreationDate } | Select-Object -First 1
    $ids = [Collections.Generic.HashSet[int]]::new()
    if ($identity) {
      $null = $ids.Add([int]$root.ProcessId)
      do { $added = $false; foreach ($process in $processes) { $parent=$processes|Where-Object ProcessId -eq $process.ParentProcessId|Select-Object -First 1; if ($ids.Contains([int]$process.ParentProcessId) -and $parent -and $process.CreationDate -ge $parent.CreationDate) { if ($ids.Add([int]$process.ProcessId)) { $added = $true } } } } while ($added)
    }
    $rows = [Collections.Generic.List[object]]::new()
    foreach ($entry in $processes | Where-Object { $ids.Contains([int]$_.ProcessId) }) {
      $memory = [WorkspaceMemory]::Read([uint32]$entry.ProcessId)
      try {
        $process = Get-Process -Id $entry.ProcessId -ErrorAction Stop
        $ticks = $process.TotalProcessorTime.Ticks
        $process.Dispose()
        $key = '{0}:{1}' -f $entry.ProcessId,$entry.CreationDate.ToUniversalTime().Ticks
        $cpu = $null
        if ($previous.ContainsKey($key)) { $elapsed = ($now-$previous[$key].time).TotalSeconds; if ($elapsed -gt 0) { $cpu = [Math]::Max([double]0,[double](100*($ticks-$previous[$key].ticks)/10000000/$elapsed/$logicalProcessors)) } }
        $previous[$key] = @{time=$now;ticks=$ticks}
        $rows.Add([PSCustomObject]@{pid=[int]$entry.ProcessId;name=$entry.Name;private_working_set_bytes=$memory.PrivateWorkingSet;private_commit_bytes=$memory.PrivateCommit;working_set_bytes=$memory.WorkingSet;cpu_percent_machine=$cpu;error=$memory.Error})
      } catch { $rows.Add([PSCustomObject]@{pid=[int]$entry.ProcessId;name=$entry.Name;error='process-ended-during-sample'}) }
    }
    $valid = @($rows | Where-Object { $_.error -eq 0 })
    $samples.Add([PSCustomObject]@{utc=$now.ToString('o');elapsed_seconds=$clock.Elapsed.TotalSeconds;root_id=[int]$root.ProcessId;application=$root.Name;process_count=$rows.Count;measured_count=$valid.Count;private_working_set_bytes=($valid|Measure-Object private_working_set_bytes -Sum).Sum;private_commit_bytes=($valid|Measure-Object private_commit_bytes -Sum).Sum;working_set_sum_bytes=($valid|Measure-Object working_set_bytes -Sum).Sum;cpu_percent_machine=($valid|Measure-Object cpu_percent_machine -Sum).Sum;processes=@($rows)})
  }
  Start-Sleep -Seconds $IntervalSeconds
}
$outputPath = [IO.Path]::GetFullPath($OutputFile)
New-Item -ItemType Directory -Path (Split-Path -Parent $outputPath) -Force | Out-Null
@{schema=1;duration_seconds=$clock.Elapsed.TotalSeconds;interval_seconds=$IntervalSeconds;logical_processors=$logicalProcessors;metrics='Native PSAPI EX2 private resident working set and private commit; total working-set sum can double-count shared pages.';roots=@($roots);samples=@($samples)} | ConvertTo-Json -Depth 9 | Set-Content -LiteralPath $outputPath -Encoding UTF8
Write-Output $outputPath
