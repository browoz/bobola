; Migration for installations that bundled the retired native Music companion.
; Limit process cleanup to the executable paths owned by this installation.
!macro BOBOLA_REMOVE_LEGACY_MUSIC
  Push $0
  Push $1
  ; NSIS is a 32-bit process: use native PowerShell to inspect 64-bit player paths.
  StrCpy $1 "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe"
  IfFileExists "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" 0 +2
    StrCpy $1 "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
  System::Call 'Kernel32::SetEnvironmentVariable(t "BOBOLA_LEGACY_INSTALL", t "$INSTDIR") i.r0'
  nsExec::ExecToLog `"$1" -NoProfile -NonInteractive -Command "$$root = $$env:BOBOLA_LEGACY_INSTALL; $$targets = @((Join-Path $$root 'resources\ytt.exe'), (Join-Path $$root 'resources\resources\ytt.exe')); Get-Process ytt -ErrorAction SilentlyContinue | Where-Object { $$targets -contains $$_.Path } | Stop-Process -Force -ErrorAction Stop"`
  Pop $0
  Delete "$INSTDIR\resources\ytt.exe"
  Delete "$INSTDIR\resources\YUTUTUI_LICENSE.md"
  Delete "$INSTDIR\resources\resources\ytt.exe"
  Delete "$INSTDIR\resources\resources\YUTUTUI_LICENSE.md"
  RMDir "$INSTDIR\resources\resources"
  RMDir "$INSTDIR\resources"
  Pop $1
  Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro BOBOLA_REMOVE_LEGACY_MUSIC
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro BOBOLA_REMOVE_LEGACY_MUSIC
!macroend
