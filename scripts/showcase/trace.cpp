// Temporary recording diagnostics, never built into or shipped with BlinkVerdict.
#include <windows.h>
#include <dbghelp.h>
#include <cstdio>
#include <cstdlib>
#pragma comment(lib,"dbghelp.lib")
int wmain(int argc,wchar_t** argv) {
 if(argc!=3)return 2;
 DWORD pid=wcstoul(argv[1],nullptr,10);
 FILE* log=nullptr;_wfopen_s(&log,argv[2],L"w");if(!log)return 3;
 HANDLE process=OpenProcess(PROCESS_ALL_ACCESS,FALSE,pid);
 if(!process||!DebugActiveProcess(pid)){fprintf(log,"Attach failed: %lu\n",GetLastError());fclose(log);return 4;}
 DebugSetProcessKillOnExit(FALSE);
 SymSetOptions(SYMOPT_LOAD_LINES|SYMOPT_UNDNAME|SYMOPT_DEFERRED_LOADS);
 SymInitialize(process,nullptr,TRUE);
 fprintf(log,"Attached to %lu\n",pid);fflush(log);
 DEBUG_EVENT event{};
 while(WaitForDebugEvent(&event,60000)) {
  DWORD action=DBG_CONTINUE;
  if(event.dwDebugEventCode==EXCEPTION_DEBUG_EVENT) {
   const auto& ex=event.u.Exception;
   const DWORD code=ex.ExceptionRecord.ExceptionCode;
   if(code!=EXCEPTION_BREAKPOINT && code!=EXCEPTION_SINGLE_STEP) {
    fprintf(log,"Exception 0x%08lX at %p, first chance %lu\n",code,ex.ExceptionRecord.ExceptionAddress,ex.dwFirstChance);
    HANDLE thread=OpenThread(THREAD_GET_CONTEXT|THREAD_QUERY_INFORMATION,FALSE,event.dwThreadId);
    CONTEXT context{};context.ContextFlags=CONTEXT_FULL;
    if(thread&&GetThreadContext(thread,&context)) {
     STACKFRAME64 frame{};frame.AddrPC.Offset=context.Rip;frame.AddrFrame.Offset=context.Rbp;frame.AddrStack.Offset=context.Rsp;
     frame.AddrPC.Mode=frame.AddrFrame.Mode=frame.AddrStack.Mode=AddrModeFlat;
     for(int i=0;i<36;i++) {
      char storage[sizeof(SYMBOL_INFO)+1024]{};auto* symbol=reinterpret_cast<SYMBOL_INFO*>(storage);
      symbol->SizeOfStruct=sizeof(SYMBOL_INFO);symbol->MaxNameLen=1023;DWORD64 displacement=0;
      if(SymFromAddr(process,frame.AddrPC.Offset,&displacement,symbol))fprintf(log,"  %s + 0x%llX",symbol->Name,displacement);
      else fprintf(log,"  0x%llX",frame.AddrPC.Offset);
      IMAGEHLP_LINE64 line{};line.SizeOfStruct=sizeof(line);DWORD offset=0;
      if(SymGetLineFromAddr64(process,frame.AddrPC.Offset,&offset,&line))fprintf(log," (%s:%lu)",line.FileName,line.LineNumber);
      fprintf(log,"\n");
      if(!StackWalk64(IMAGE_FILE_MACHINE_AMD64,process,thread,&frame,&context,nullptr,SymFunctionTableAccess64,SymGetModuleBase64,nullptr))break;
     }
    }
    if(thread)CloseHandle(thread);
    fflush(log);action=DBG_EXCEPTION_NOT_HANDLED;
   }
  }
  ContinueDebugEvent(event.dwProcessId,event.dwThreadId,action);
  if(event.dwDebugEventCode==EXIT_PROCESS_DEBUG_EVENT)break;
 }
 SymCleanup(process);CloseHandle(process);fclose(log);return 0;
}
