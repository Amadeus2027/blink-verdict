param([string]$Executable = "$PSScriptRoot/../../target/release/blink-verdict.exe", [string]$Output = "$PSScriptRoot/../../showcase-frames")
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class NativeDemo {
 [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left,Top,Right,Bottom; }
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
 public static IntPtr FindByClass(string cls) { return FindWindow(cls,null); }
 [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr hwnd,int id);
 [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageW")] public static extern IntPtr TextMessage(IntPtr hwnd,uint msg,IntPtr wp,string text);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr hwnd,StringBuilder value,int count);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out Rect rect);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd,IntPtr dc,uint flags);
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int cx,int cy,uint flags);
 [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] public static extern int AddFontResourceEx(string path,uint flags,IntPtr reserved);
}
'@
New-Item -ItemType Directory -Force $Output | Out-Null
$Output = (Resolve-Path $Output).Path
$font = Join-Path $env:RUNNER_TEMP 'NotoSansSC.ttf'
Invoke-WebRequest 'https://raw.githubusercontent.com/google/fonts/main/ofl/notosanssc/NotoSansSC%5Bwght%5D.ttf' -OutFile $font
if ([NativeDemo]::AddFontResourceEx($font,0,[IntPtr]::Zero) -eq 0) { throw 'Could not register the demonstration font.' }
# Isolated synthetic preferences. No credentials, screenshots, provider or model calls.
$demoData = Join-Path $env:RUNNER_TEMP 'blink-verdict-showcase-data'
New-Item -ItemType Directory -Force "$demoData/BlinkVerdict" | Out-Null
@{ mode='Fullscreen'; provider='Jev'; prompt='根据当前局面选择适合的下一步行动。证据不足时放弃判断。'; options=@('YES','NO'); theme=@{font='Noto Sans SC'; widget_duration_ms=0; text_color='D3E8FF'; background_color='18212F'} } | ConvertTo-Json -Depth 4 | Set-Content "$demoData/BlinkVerdict/config.json" -Encoding utf8NoBOM
$env:LOCALAPPDATA = $demoData
$env:RUST_BACKTRACE = '1'
$app = Start-Process -FilePath (Resolve-Path $Executable) -PassThru -RedirectStandardError "$Output/native-stderr.log" -RedirectStandardOutput "$Output/native-stdout.log"
function Send-Command([IntPtr]$Window,[int]$Id) { [void][NativeDemo]::SendMessage($Window,0x111,[IntPtr]$Id,[IntPtr]::Zero) }
function Set-Text([IntPtr]$Window,[int]$Id,[string]$Text) { [void][NativeDemo]::TextMessage([NativeDemo]::GetDlgItem($Window,$Id),0xC,[IntPtr]::Zero,$Text) }
function Capture([IntPtr]$Window,[string]$Name) {
 Write-Host "Capturing $Name"
 $rect = New-Object NativeDemo+Rect
 if (-not [NativeDemo]::GetWindowRect($Window,[ref]$rect)) {
  $app.Refresh()
  Get-Content "$Output/native-stderr.log" -ErrorAction SilentlyContinue | Write-Host
  Get-Content "$Output/native-trace.log" -ErrorAction SilentlyContinue | Write-Host
  Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=(Get-Date).AddMinutes(-2)} -ErrorAction SilentlyContinue | Where-Object {$_.Message -match 'blink-verdict'} | Select-Object -First 2 -ExpandProperty Message | Write-Host
  throw "No window rectangle for $Name; process exited=$($app.HasExited), exit code=$($app.ExitCode)."
 }
 $bitmap = New-Object System.Drawing.Bitmap ($rect.Right-$rect.Left),($rect.Bottom-$rect.Top),([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
 $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
 $dc = $graphics.GetHdc()
 try {
  if (-not [NativeDemo]::PrintWindow($Window,$dc,2)) { throw "PrintWindow failed: $Name" }
 } finally { $graphics.ReleaseHdc($dc); $graphics.Dispose() }
 $bitmap.Save((Join-Path $Output "$Name.png"),[System.Drawing.Imaging.ImageFormat]::Png)
 $bitmap.Dispose()
}
try {
 $hostWindow = [IntPtr]::Zero
 for ($i=0;$i -lt 100;$i++) { $hostWindow=[NativeDemo]::FindByClass('BlinkVerdict.Host'); if ($hostWindow -ne [IntPtr]::Zero) {break}; Start-Sleep -Milliseconds 100 }
 if ($hostWindow -eq [IntPtr]::Zero) {
  $app.Refresh()
  if ($app.HasExited) { throw ('Native process exited with code {0} (0x{1:X8}).' -f $app.ExitCode,($app.ExitCode -band 0xffffffffL)) }
  throw "Native process $($app.Id) is running but its host window was not found."
 }
 $debugger=Start-Process -FilePath "$env:RUNNER_TEMP/native-trace.exe" -ArgumentList "$($app.Id) `"$Output/native-trace.log`"" -PassThru
 Start-Sleep -Milliseconds 500
 # Same entry point as an explicit tray click; no capture/decision shortcut is sent.
 [void][NativeDemo]::SendMessage($hostWindow,0x8001,[IntPtr]::Zero,[IntPtr]0x202)
 $settings=[NativeDemo]::FindByClass('BlinkVerdict.Settings')
 if ($settings -eq [IntPtr]::Zero) {throw 'Settings window did not open.'}
 [void][NativeDemo]::SetWindowPos($settings,[IntPtr]::Zero,40,40,0,0,0x15)
 Start-Sleep -Milliseconds 400
 Capture $settings '01-decision'
 # Incomplete input must survive navigation and must not prevent opening another page.
 Set-Text $settings 301 'Ctrl+'
 Send-Command $settings 211
 Start-Sleep -Milliseconds 180
 Capture $settings '02-models'
 Set-Text $settings 314 'showcase-not-a-real-api-key'
 Set-Text $settings 312 'http://127.0.0.1:8000/v1'
 $keyValue=New-Object System.Text.StringBuilder 100
 [void][NativeDemo]::GetWindowText([NativeDemo]::GetDlgItem($settings,314),$keyValue,100)
 if ($keyValue.Length -ne 0) {throw 'Changing endpoint retained an unsaved key.'}
 Send-Command $settings 210
 $value=New-Object System.Text.StringBuilder 100
 [void][NativeDemo]::GetWindowText([NativeDemo]::GetDlgItem($settings,301),$value,100)
 if ($value.ToString() -ne 'Ctrl+') {throw 'Page navigation lost the unfinished shortcut.'}
 # Preview must remain available despite unfinished model/shortcut fields.
 Send-Command $settings 212
 Start-Sleep -Milliseconds 180
 Capture $settings '03-appearance'
 Send-Command $settings 213
 Start-Sleep -Milliseconds 180
 Capture $settings '04-wordart'
 Send-Command $settings 214
 Start-Sleep -Milliseconds 180
 Capture $settings '05-widget-layout'
 # Preview a widget using the current draft, without applying its mode to disk.
 Send-Command $settings 210
 [void][NativeDemo]::SendMessage([NativeDemo]::GetDlgItem($settings,300),0x14E,[IntPtr]1,[IntPtr]::Zero)
 Send-Command $settings 201
 $overlay=[NativeDemo]::FindByClass('BlinkVerdict.Overlay')
 for ($frame=0;$frame -lt 24;$frame++) { Capture $overlay ('widget-{0:D3}' -f $frame); Start-Sleep -Milliseconds 60 }
 $disk=Get-Content "$demoData/BlinkVerdict/config.json" -Raw | ConvertFrom-Json
 if ($disk.mode -ne 'Fullscreen') {throw 'Preview unexpectedly persisted the draft.'}
 Capture $settings '06-finish'
 @{
   kind='Windows native application capture, editorial presentation';
   source_commit=$env:GITHUB_SHA; os=[Environment]::OSVersion.VersionString;
   font='Noto Sans SC (downloaded for capture only; not shipped with the application)';
   capture='Win32 PrintWindow; composited desktop blur is not guaranteed in offscreen capture';
   provider_requests=0; credentials_used=$false;
   checks=@('unfinished field survives page navigation','preview does not save draft mode','changing endpoint clears unsaved key');
 } | ConvertTo-Json -Depth 4 | Set-Content "$Output/provenance.json" -Encoding utf8NoBOM
} finally { if (-not $app.HasExited) {Stop-Process -Id $app.Id -Force} }
