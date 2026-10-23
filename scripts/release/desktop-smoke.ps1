param([string]$Executable="$PSScriptRoot/../../target/release/blink-verdict.exe", [string]$Output="$PSScriptRoot/../../release-evidence")
$ErrorActionPreference='Stop'
# Dedicated synthetic receiver window, real SendInput, real capture and a loopback fixture.
# This is desktop evidence only: it does not emulate games or prove exclusive fullscreen.
Add-Type @'
using System;
using System.Text;
using System.Threading;
using System.Runtime.InteropServices;
public static class DesktopCheck {
 public static IntPtr Window,Edit,Button; public static int Clicks;
 delegate IntPtr Proc(IntPtr h,uint m,IntPtr w,IntPtr l);
 static Proc callback=WindowProc;
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct Class {
  public uint style; public Proc proc; public int cls,win; public IntPtr instance,icon,cursor,brush;
  public string menu,name;
 }
 [StructLayout(LayoutKind.Sequential)] struct Message { public IntPtr hwnd; public uint message; public UIntPtr wp; public IntPtr lp; public uint time; public int x,y; public uint extra; }
 [StructLayout(LayoutKind.Sequential)] struct Mouse { public int x,y; public uint data,flags,time; public UIntPtr extra; }
 [StructLayout(LayoutKind.Sequential)] struct Key { public ushort vk,scan; public uint flags,time; public UIntPtr extra; }
 [StructLayout(LayoutKind.Explicit)] struct Union { [FieldOffset(0)]public Mouse mouse; [FieldOffset(0)]public Key key; }
 [StructLayout(LayoutKind.Sequential)] struct Input { public uint type; public Union value; }
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern ushort RegisterClassW(ref Class cls);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern IntPtr GetModuleHandle(string name);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CreateWindowEx(uint ex,string cls,string name,uint style,int x,int y,int w,int h,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr param);
 [DllImport("user32.dll")] static extern bool GetMessage(out Message msg,IntPtr hwnd,uint min,uint max);
 [DllImport("user32.dll")] static extern bool TranslateMessage(ref Message msg);
 [DllImport("user32.dll")] static extern IntPtr DispatchMessage(ref Message msg);
 [DllImport("user32.dll")] static extern IntPtr DefWindowProc(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] static extern IntPtr SetFocus(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd,uint msg,IntPtr wp,IntPtr lp);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr h,StringBuilder text,int count);
 [DllImport("user32.dll")] static extern uint SendInput(uint n,Input[] inputs,int size);
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
 static IntPtr WindowProc(IntPtr h,uint m,IntPtr w,IntPtr l) {
  if(m==0x8001) { SetFocus(Edit); return IntPtr.Zero; }
  if(m==0x111 && (w.ToInt64()&0xffff)==2) Clicks++;
  return DefWindowProc(h,m,w,l);
 }
 public static void Start() {
  var t=new Thread(()=>{
   var cls=new Class { proc=callback,instance=GetModuleHandle(null),name="BlinkVerdict.ReleaseReceiver",brush=(IntPtr)6 };
   if(RegisterClassW(ref cls)==0)throw new Exception("Receiver registration failed");
   Window=CreateWindowEx(0,cls.name,"BlinkVerdict release input receiver",0x10CF0000,30,30,700,500,IntPtr.Zero,IntPtr.Zero,cls.instance,IntPtr.Zero);
   Edit=CreateWindowEx(0,"EDIT","",0x50800080,60,80,360,40,Window,(IntPtr)1,cls.instance,IntPtr.Zero);
   Button=CreateWindowEx(0,"BUTTON","Click-through receiver",0x50000000,60,150,220,40,Window,(IntPtr)2,cls.instance,IntPtr.Zero);
   Message msg; while(GetMessage(out msg,IntPtr.Zero,0,0)) {TranslateMessage(ref msg);DispatchMessage(ref msg);}
  });t.IsBackground=true;t.SetApartmentState(ApartmentState.STA);t.Start();
 }
 static Input K(ushort vk,uint flags=0,ushort scan=0) { return new Input { type=1,value=new Union { key=new Key {vk=vk,flags=flags,scan=scan} } }; }
 static void Send(Input[] inputs) { if(SendInput((uint)inputs.Length,inputs,Marshal.SizeOf<Input>())!=inputs.Length)throw new Exception("SendInput failed"); }
 public static void Hotkey() { Send(new[]{K(0x11),K(0x12),K(0x20),K(0x20,2),K(0x12,2),K(0x11,2)}); }
 public static void Type(char c) { Send(new[]{K(0,4,c),K(0,6,c)}); }
 public static string Text() {var s=new StringBuilder(512);GetWindowText(Edit,s,512);return s.ToString();}
 public static void Click() {SetCursorPos(150,220);Send(new[]{new Input{type=0,value=new Union{mouse=new Mouse{flags=2}}},new Input{type=0,value=new Union{mouse=new Mouse{flags=4}}}});}
 public static int ProcessId(IntPtr hwnd) { uint id;GetWindowThreadProcessId(hwnd,out id);return (int)id; }
}
'@
function Wait-Until([scriptblock]$Condition,[string]$Failure) {
 $deadline=(Get-Date).AddSeconds(12)
 while (-not (& $Condition)) { if ((Get-Date) -gt $deadline) { throw $Failure }; Start-Sleep -Milliseconds 80 }
}
if ([DesktopCheck]::FindWindow('BlinkVerdict.Host',$null) -ne [IntPtr]::Zero) {throw 'Close the existing instance before release validation.'}
New-Item -ItemType Directory -Force $Output | Out-Null
$Output=(Resolve-Path $Output).Path
$originalData=$env:LOCALAPPDATA
$data=Join-Path $env:RUNNER_TEMP 'blink-release-desktop'
New-Item -ItemType Directory -Force "$data/BlinkVerdict" | Out-Null
$state=Join-Path $Output 'fixture-state.json'
Remove-Item $state -ErrorAction SilentlyContinue
$server=$null;$app=$null
try {
 $server=Start-Process python -ArgumentList @("`"$PSScriptRoot/mock_service.py`"","`"$state`"") -PassThru -RedirectStandardError "$Output/fixture-stderr.txt"
 Wait-Until {Test-Path $state} 'Fixture did not start.'
 $port=(Get-Content $state -Raw|ConvertFrom-Json).port
 [DesktopCheck]::Start()
 Wait-Until {[DesktopCheck]::Edit -ne [IntPtr]::Zero} 'Receiver did not start.'
 $env:LOCALAPPDATA=$data
 $expected='';$requests=0;$samples=@{}
 foreach ($mode in @('Fullscreen','Widget')) {
  @{mode=$mode;provider='OpenAi';endpoint="http://127.0.0.1:$port/v1";model='synthetic-fixture';prompt='release-desktop-fixture';hotkey='Ctrl+Alt+Space';options=@('YES','NO');vision=$true;widget_x=60;widget_y=80;widget_width=480;widget_height=300;theme=@{animation='None';fullscreen_duration_ms=3000;widget_duration_ms=0}} | ConvertTo-Json -Depth 5 | Set-Content "$data/BlinkVerdict/config.json" -Encoding utf8NoBOM
  $app=Start-Process (Resolve-Path $Executable) -PassThru
  Wait-Until {[DesktopCheck]::FindWindow('BlinkVerdict.Host',$null) -ne [IntPtr]::Zero} 'Native host did not start.'
  $hostWindow=[DesktopCheck]::FindWindow('BlinkVerdict.Host',$null)
  $overlay=[DesktopCheck]::FindWindow('BlinkVerdict.Overlay',$null)
  if ([DesktopCheck]::ProcessId($hostWindow) -ne $app.Id) {throw 'Harness attached to another process.'}
  [void][DesktopCheck]::SendMessage($hostWindow,0x111,[IntPtr]108,[IntPtr]::Zero)
  if ($mode -eq 'Fullscreen') {$samples.fresh=& "$PSScriptRoot/../measure-idle.ps1" -Seconds 60}
  for ($i=0;$i -lt 3;$i++) {
   if (-not [DesktopCheck]::SetForegroundWindow([DesktopCheck]::Window)) {throw 'Cannot foreground the synthetic input receiver.'}
   [void][DesktopCheck]::SendMessage([DesktopCheck]::Window,0x8001,[IntPtr]::Zero,[IntPtr]::Zero)
   [DesktopCheck]::Type('a');$expected+='a'
   [DesktopCheck]::Hotkey();Start-Sleep -Milliseconds 50;[DesktopCheck]::Hotkey()
   $requests++
   Wait-Until {((Get-Content $state -Raw|ConvertFrom-Json).requests) -ge $requests} 'Shortcut did not send a captured image to the fixture.'
   Start-Sleep -Milliseconds 1000
   if (((Get-Content $state -Raw|ConvertFrom-Json).requests) -ne $requests) {throw 'Busy trigger sent a duplicate request.'}
   if (-not [DesktopCheck]::IsWindowVisible($overlay)) {throw 'No native result overlay.'}
   if ([DesktopCheck]::GetForegroundWindow() -ne [DesktopCheck]::Window) {throw 'Overlay stole the foreground window.'}
   [DesktopCheck]::Type('b');$expected+='b';Start-Sleep -Milliseconds 80
   if ([DesktopCheck]::Text() -ne $expected) {throw 'Keyboard input was lost during the overlay.'}
   $clicks=[DesktopCheck]::Clicks
   [DesktopCheck]::Click();Start-Sleep -Milliseconds 100
   if ([DesktopCheck]::Clicks -ne $clicks+1) {throw 'Mouse input did not pass through the overlay.'}
  }
  if ($mode -eq 'Widget') {$samples.widget=& "$PSScriptRoot/../measure-idle.ps1" -Seconds 60}
  [void][DesktopCheck]::SendMessage($hostWindow,0x111,[IntPtr]108,[IntPtr]::Zero)
  if ($mode -eq 'Fullscreen') {$samples.after_use=& "$PSScriptRoot/../measure-idle.ps1" -Seconds 60}
  [void][DesktopCheck]::SendMessage($hostWindow,0x111,[IntPtr]105,[IntPtr]::Zero)
  if (-not $app.WaitForExit(5000)) {throw 'Native host did not exit.'}
  if ($app.ExitCode -ne 0) {throw 'Native host exited unsuccessfully.'}
 }
 foreach ($name in @('fresh','after_use')) {
  if ($samples[$name].MeanPrivateWorkingSetMB -gt 30 -or $samples[$name].CPUPercentOfMachine -ge 0.1) {throw "Hidden host resource target exceeded: $name"}
 }
 @{source_commit=$env:GITHUB_SHA;os=[Environment]::OSVersion.VersionString;recording_mode=$false;provider='loopback OpenAI-compatible synthetic fixture';requests=$requests;checks=@('real shortcut and PNG capture in both modes','busy repeat suppressed','foreground preserved','keyboard input delivered','mouse clicks pass through','clean process exit');samples=$samples;limits='Synthetic desktop only; no game, real provider, OCR, multi-monitor or endurance certification.'} | ConvertTo-Json -Depth 8 | Set-Content "$Output/desktop.json" -Encoding utf8NoBOM
} finally {
 $env:LOCALAPPDATA=$originalData
 if ($app -and -not $app.HasExited) {Stop-Process -Id $app.Id -Force}
 if ($server -and -not $server.HasExited) {Stop-Process -Id $server.Id -Force}
}
