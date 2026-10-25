param([string]$Output="$PSScriptRoot/../../release-evidence")
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $Output | Out-Null
$Output = (Resolve-Path $Output).Path
$work = Join-Path $env:RUNNER_TEMP 'blink-llama-release'
New-Item -ItemType Directory -Force $work | Out-Null
$archive = Join-Path $work 'llama.zip'
$model = Join-Path $work 'stories260K.gguf'
# Official public test artifacts; pinned independently of the moving latest tag.
$binaryUrl = 'https://github.com/ggml-org/llama.cpp/releases/download/b11669/llama-b11669-bin-win-cpu-x64.zip'
$modelUrl = 'https://huggingface.co/ggml-org/models/resolve/499bc8821c6b12b4e53c5bffcb21ec206f212d81/tinyllamas/stories260K.gguf'
Invoke-WebRequest $binaryUrl -OutFile $archive
Invoke-WebRequest $modelUrl -OutFile $model
$binaryHash = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLower()
$modelHash = (Get-FileHash $model -Algorithm SHA256).Hash.ToLower()
if ($binaryHash -ne '37b3efcf962847ca869745fd4ccf7f5e7e8dacfeff9fa3523b4ca8cdc66d632b') {throw 'llama binary checksum mismatch'}
if ($modelHash -ne '270cba1bd5109f42d03350f60406024560464db173c0e387d91f0426d3bd256d') {throw 'test model checksum mismatch'}
Expand-Archive $archive -DestinationPath "$work/bin" -Force
$executable = (Get-ChildItem "$work/bin" -Filter llama-server.exe -Recurse | Select-Object -First 1).FullName
if (-not $executable) {throw 'llama-server executable missing'}
$listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
$listener.Start(); $port = $listener.LocalEndpoint.Port; $listener.Stop()
$server = $null
try {
 $server = Start-Process $executable -WorkingDirectory (Split-Path $executable) -ArgumentList @('-m',"`"$model`"",'--alias','blink-release','--host','127.0.0.1','--port',"$port",'--chat-template','chatml','-c','2048','-t','2','--api-key','synthetic-release-key') -PassThru -RedirectStandardOutput "$Output/llama-stdout.log" -RedirectStandardError "$Output/llama-stderr.log"
 $deadline = (Get-Date).AddSeconds(60)
 while ($true) {
  $server.Refresh()
  if ($server.HasExited) {Get-Content "$Output/llama-stderr.log" -Tail 30; throw 'llama-server exited before readiness'}
  try { $health = Invoke-RestMethod "http://127.0.0.1:$port/health" -TimeoutSec 2; if ($health.status -eq 'ok') {break} } catch {}
  if ((Get-Date) -gt $deadline) {throw 'llama-server readiness timeout'}
  Start-Sleep -Milliseconds 200
 }
 $env:BLINK_LLAMA_TEST_ENDPOINT = "http://127.0.0.1:$port/v1"
 cargo test --locked --test release_llamacpp -- --ignored
 if ($LASTEXITCODE) {throw 'real llama-server smoke failed'}
 @{source_commit=$env:GITHUB_SHA;llama_tag='b11669';binary_sha256=$binaryHash;model='ggml-org/models tinyllamas/stories260K.gguf';model_revision='499bc8821c6b12b4e53c5bffcb21ec206f212d81';model_sha256=$modelHash;checks=@('two nonstreaming schema-constrained requests through production WinHTTP','missing API key rejected');limits='Tiny public text-only test weights; no model quality, user GGUF, vision, GPU or latency certification.'} | ConvertTo-Json -Depth 4 | Set-Content "$Output/llama.json" -Encoding utf8NoBOM
} finally {
 Remove-Item Env:BLINK_LLAMA_TEST_ENDPOINT -ErrorAction SilentlyContinue
 if ($server -and -not $server.HasExited) {Stop-Process -Id $server.Id -Force}
}
