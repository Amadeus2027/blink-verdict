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
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint processId);
 [DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hwnd,IntPtr dc);
 [DllImport("gdi32.dll")] private static extern bool BitBlt(IntPtr target,int x,int y,int width,int height,IntPtr source,int sx,int sy,uint operation);
 public static bool CaptureDesktop(IntPtr target,int x,int y,int width,int height) {
  var screen=GetDC(IntPtr.Zero);
  try { return screen!=IntPtr.Zero && BitBlt(target,0,0,width,height,screen,x,y,0x40CC0020); }
  finally { if(screen!=IntPtr.Zero)ReleaseDC(IntPtr.Zero,screen); }
 }
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr hwnd,int id);
 [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageW")] public static extern IntPtr TextMessage(IntPtr hwnd,uint msg,IntPtr wp,string text);
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="SendMessageW")] public static extern IntPtr ReadTextMessage(IntPtr hwnd,uint msg,IntPtr count,[Out] StringBuilder value);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out Rect rect);
 [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr hwnd,uint attribute,out Rect value,int size);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr hwnd,IntPtr dc,uint flags);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] private static extern IntPtr CreateWindowEx(uint ex,string cls,string title,uint style,int x,int y,int width,int height,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr param);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] private static extern IntPtr GetModuleHandle(string name);
 [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
 [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hwnd,int command);
 [DllImport("user32.dll")] public static extern bool UpdateWindow(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr hwnd);
 [DllImport("gdi32.dll")] public static extern bool DeleteObject(IntPtr obj);
 public static IntPtr CreateStage() { return CreateWindowEx(0x08000088,"STATIC","BlinkVerdict recording stage",0x9000000E,0,0,GetSystemMetrics(0),GetSystemMetrics(1),IntPtr.Zero,IntPtr.Zero,GetModuleHandle(null),IntPtr.Zero); }
 [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int cx,int cy,uint flags);
 [DllImport("gdi32.dll",CharSet=CharSet.Unicode)] public static extern int AddFontResourceEx(string path,uint flags,IntPtr reserved);
}
'@
# Never attach this recorder to an existing user's resident instance.
if ([NativeDemo]::FindByClass('BlinkVerdict.Host') -ne [IntPtr]::Zero) { throw 'Close the existing BlinkVerdict instance before recording.' }
New-Item -ItemType Directory -Force $Output | Out-Null
$Output = (Resolve-Path $Output).Path
# Static OpenType family is visible to both GDI settings and DirectWrite overlay.
$font = Join-Path $env:RUNNER_TEMP 'NotoSansCJKsc-Regular.otf'
Invoke-WebRequest 'https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf' -OutFile $font
if ([NativeDemo]::AddFontResourceEx($font,0,[IntPtr]::Zero) -eq 0) { throw 'Could not register the demonstration font.' }
# Isolated synthetic preferences. No credentials, screenshots, provider or model calls.
$demoData = Join-Path $env:RUNNER_TEMP 'blink-verdict-showcase-data'
New-Item -ItemType Directory -Force "$demoData/BlinkVerdict" | Out-Null
@{ mode='Fullscreen'; provider='Jev'; prompt='根据当前局面选择适合的下一步行动。证据不足时放弃判断。'; options=@('YES','NO'); theme=@{font='Noto Sans CJK SC'; widget_duration_ms=0; text_color='D3E8FF'; background_color='18212F'} } | ConvertTo-Json -Depth 4 | Set-Content "$demoData/BlinkVerdict/config.json" -Encoding utf8NoBOM
$env:LOCALAPPDATA = $demoData
$env:RUST_BACKTRACE = '1'
$app = Start-Process -FilePath (Resolve-Path $Executable) -ArgumentList '--recordable-preview' -PassThru -RedirectStandardError "$Output/native-stderr.log" -RedirectStandardOutput "$Output/native-stdout.log"
function Send-Command([IntPtr]$Window,[int]$Id) { [void][NativeDemo]::SendMessage($Window,0x111,[IntPtr]$Id,[IntPtr]::Zero) }
function Set-Text([IntPtr]$Window,[int]$Id,[string]$Text) { [void][NativeDemo]::TextMessage([NativeDemo]::GetDlgItem($Window,$Id),0xC,[IntPtr]::Zero,$Text) }
function Capture([IntPtr]$Window,[string]$Name) {
 Write-Host "Capturing $Name"
 $rect = New-Object NativeDemo+Rect
 if (-not [NativeDemo]::GetWindowRect($Window,[ref]$rect)) {
  $app.Refresh()
  Get-Content "$Output/native-stderr.log" -ErrorAction SilentlyContinue | Write-Host
  Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=(Get-Date).AddMinutes(-2)} -ErrorAction SilentlyContinue | Where-Object {$_.Message -match 'blink-verdict'} | Select-Object -First 2 -ExpandProperty Message | Write-Host
  throw "No window rectangle for $Name; process exited=$($app.HasExited), exit code=$($app.ExitCode)."
 }
 if ($Name -notlike 'widget-*') {
  $visibleFrame=New-Object NativeDemo+Rect
  if ([NativeDemo]::DwmGetWindowAttribute($Window,9,[ref]$visibleFrame,16) -eq 0 -and $visibleFrame.Right -gt $visibleFrame.Left) { $rect=$visibleFrame }
 }
 $bitmap = New-Object System.Drawing.Bitmap ($rect.Right-$rect.Left),($rect.Bottom-$rect.Top),([System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
 $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
 $captured = $false
 if ($Name -like 'widget-*' -or [NativeDemo]::GetForegroundWindow() -eq $Window) {
  $desktopDc=$graphics.GetHdc()
  try { $captured=[NativeDemo]::CaptureDesktop($desktopDc,$rect.Left,$rect.Top,$bitmap.Width,$bitmap.Height) }
  finally { $graphics.ReleaseHdc($desktopDc) }
 }
 if (-not $captured) {
  $dc = $graphics.GetHdc()
  try {
   if (-not [NativeDemo]::PrintWindow($Window,$dc,2)) { throw "PrintWindow failed: $Name" }
  }
  finally { $graphics.ReleaseHdc($dc) }
 }
 $graphics.Dispose()
 if ($Name -eq 'widget-000') {
  $bright=$false
  for ($y=2; $y -lt $bitmap.Height -and -not $bright; $y+=4) { for ($x=2; $x -lt $bitmap.Width; $x+=4) { $pixel=$bitmap.GetPixel($x,$y); if ($pixel.R -gt 100 -or $pixel.G -gt 100 -or $pixel.B -gt 100) { $bright=$true;break } } }
  if (-not $bright) { throw 'Native overlay capture is blank; refusing to publish a black preview.' }
 }
 $bitmap.Save((Join-Path $Output "$Name.png"),[System.Drawing.Imaging.ImageFormat]::Png)
 $bitmap.Dispose()
}
$stage=[IntPtr]::Zero
$stageImage=[IntPtr]::Zero
try {
 $hostWindow = [IntPtr]::Zero
 for ($i=0;$i -lt 100;$i++) { $hostWindow=[NativeDemo]::FindByClass('BlinkVerdict.Host'); if ($hostWindow -ne [IntPtr]::Zero) {break}; Start-Sleep -Milliseconds 100 }
 if ($hostWindow -eq [IntPtr]::Zero) {
  $app.Refresh()
  if ($app.HasExited) { throw ('Native process exited with code {0} (0x{1:X8}).' -f $app.ExitCode,($app.ExitCode -band 0xffffffffL)) }
  throw "Native process $($app.Id) is running but its host window was not found."
 }
 [uint32]$owner=0
 [void][NativeDemo]::GetWindowThreadProcessId($hostWindow,[ref]$owner)
 if ($owner -ne $app.Id) {throw 'Host window belongs to a different process; recording stopped.'}
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
 [void][NativeDemo]::ReadTextMessage([NativeDemo]::GetDlgItem($settings,314),0xD,[IntPtr]100,$keyValue)
 if ($keyValue.Length -ne 0) {throw 'Changing endpoint retained an unsaved key.'}
 Send-Command $settings 210
 $value=New-Object System.Text.StringBuilder 100
 [void][NativeDemo]::ReadTextMessage([NativeDemo]::GetDlgItem($settings,301),0xD,[IntPtr]100,$value)
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
 Capture $settings '06-finish'
 # A plain editorial backdrop, created only by this recording script on its runner.
 $stage=[NativeDemo]::CreateStage()
 if ($stage -eq [IntPtr]::Zero) { throw 'Could not create the recording backdrop.' }
 $canvas=New-Object System.Drawing.Bitmap ([NativeDemo]::GetSystemMetrics(0)),([NativeDemo]::GetSystemMetrics(1))
 $paint=[System.Drawing.Graphics]::FromImage($canvas)
 $paint.Clear([System.Drawing.Color]::FromArgb(10,16,27));$paint.Dispose()
 $stageImage=$canvas.GetHbitmap();$canvas.Dispose()
 [void][NativeDemo]::SendMessage($stage,0x172,[IntPtr]::Zero,$stageImage)
 [void][NativeDemo]::UpdateWindow($stage)
 [void][NativeDemo]::ShowWindow($settings,0)
 Send-Command $settings 201
 Start-Sleep -Milliseconds 250
 $overlay=[NativeDemo]::FindByClass('BlinkVerdict.Overlay')
 if (-not [NativeDemo]::IsWindowVisible($overlay)) { throw 'Native widget preview was not visible.' }
 for ($frame=0;$frame -lt 24;$frame++) {
  if (-not [NativeDemo]::IsWindowVisible($overlay)) {break}
  $name=('widget-{0:D3}' -f $frame)
  Capture $overlay $name
  if (-not [NativeDemo]::IsWindowVisible($overlay)) {Remove-Item "$Output/$name.png";break}
  Start-Sleep -Milliseconds 50
 }
 $disk=Get-Content "$demoData/BlinkVerdict/config.json" -Raw | ConvertFrom-Json
 if ($disk.mode -ne 'Fullscreen') {throw 'Preview unexpectedly persisted the draft.'}
 @{
   kind='Windows native application capture, editorial presentation';
   source_commit=$env:GITHUB_SHA; os=[Environment]::OSVersion.VersionString;
   font='Noto Sans CJK SC (downloaded for capture only; not shipped with the application)';
   capture='Desktop crops of the native settings and actual visible overlay; PrintWindow fallback for settings. Synthetic solid stage behind the overlay.';
   provider_requests=0; credentials_used=$false; recording_mode='--recordable-preview permits recording and disables decision triggers';
   checks=@('unfinished field survives page navigation','preview does not save draft mode','changing endpoint clears unsaved key','native overlay preview visible and nonblank');
 } | ConvertTo-Json -Depth 4 | Set-Content "$Output/provenance.json" -Encoding utf8NoBOM
} finally {
 if ($stage -ne [IntPtr]::Zero) { [void][NativeDemo]::DestroyWindow($stage) }
 if ($stageImage -ne [IntPtr]::Zero) { [void][NativeDemo]::DeleteObject($stageImage) }
 if (-not $app.HasExited) {Stop-Process -Id $app.Id -Force}
}
