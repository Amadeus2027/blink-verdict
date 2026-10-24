$ErrorActionPreference = 'Stop'
# Compile the exact production ordering gate; no UWP install or model service.
$source = Get-Content "$PSScriptRoot/../../gamebar/DisplayOrder.cs" -Raw
$regression = Get-Content "$PSScriptRoot/GameBarOrderRegression.cs" -Raw
Add-Type -TypeDefinition ($source + "`n" + $regression)
[BlinkVerdict.GameBar.DisplayOrderRegression]::Run().GetAwaiter().GetResult()
Write-Host 'Game Bar ordering regression passed: delayed hide/show, old disconnect, async continuation, closed queued action and retry.'
