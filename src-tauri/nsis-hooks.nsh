!macro NSIS_HOOK_PREINSTALL
  ; YuTuTui runs as a separate daemon and can keep resources\ytt.exe locked
  ; after Bobola closes. Stop only the daemon owned by this installation.
  nsExec::ExecToLog '"$INSTDIR\resources\ytt.exe" -r daemon stop'
  nsExec::ExecToLog 'taskkill.exe /F /IM ytt.exe /T'
!macroend
