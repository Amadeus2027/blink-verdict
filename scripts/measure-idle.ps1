param([int]$Seconds = 60, [int]$ProcessId = 0)
$ErrorActionPreference = 'Stop'
if ($Seconds -lt 10) { throw 'Use at least 10 seconds; release records require 60.' }
if ($ProcessId -gt 0) {
 $app = Get-Process -Id $ProcessId
} else {
 $matches = @(Get-Process -Name 'blink-verdict')
 if ($matches.Count -ne 1) {throw 'Multiple app/worker processes; pass the resident host ProcessId explicitly.'}
 $app = $matches[0]
}
if (-not $app) { throw 'Start blink-verdict.exe and hide the overlay first.' }
# Perflib counter names are localized; Win32_PerfFormattedData works on localized Windows.
$startCpu = $app.TotalProcessorTime.TotalSeconds
$watch = [Diagnostics.Stopwatch]::StartNew()
$samples = @()
for ($i = 0; $i -lt $Seconds; $i++) {
    $sample = Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter "IDProcess=$($app.Id)"
    if ($null -eq $sample) { throw 'Host process exited during sampling.' }
    $samples += [double]$sample.WorkingSetPrivate
    Start-Sleep -Seconds 1
}
$app.Refresh()
$cpuPercent = 100 * ($app.TotalProcessorTime.TotalSeconds - $startCpu) / $watch.Elapsed.TotalSeconds / [Environment]::ProcessorCount
[pscustomobject]@{
    PID = $app.Id
    Windows = [Environment]::OSVersion.VersionString
    LogicalCPUs = [Environment]::ProcessorCount
    DurationSeconds = [math]::Round($watch.Elapsed.TotalSeconds, 2)
    MeanPrivateWorkingSetMB = [math]::Round(($samples | Measure-Object -Average).Average / 1MB, 2)
    PeakPrivateWorkingSetMB = [math]::Round(($samples | Measure-Object -Maximum).Maximum / 1MB, 2)
    CPUPercentOfMachine = [math]::Round($cpuPercent, 4)
    PrivateBytesMB = [math]::Round($app.PrivateMemorySize64 / 1MB, 2)
    HandleCount = $app.HandleCount
}
