; ============================================================================
; NetFlow Studio - NSIS Installer Hooks (Professional Edition)
; ============================================================================
; Features:
; 1. Windows Defender exclusion (prevents false positive WinDivert detection)
; 2. Desktop shortcut with Admin elevation flag
; 3. Autostart scheduled task (runs as Admin, minimized to tray)
; ============================================================================

; ---------------------------------------------------------------------------
; NSIS_HOOK_POSTINSTALL - Runs after all files are installed
; ---------------------------------------------------------------------------
!macro NSIS_HOOK_POSTINSTALL

  ; ===== 1. Windows Defender Exclusion =====
  DetailPrint "Configuring Windows Defender exclusion..."
  FileOpen $0 "$PLUGINSDIR\defender_add.ps1" w
  FileWrite $0 "try {$\r$\n"
  FileWrite $0 "    Add-MpPreference -ExclusionPath '$INSTDIR' -ErrorAction Stop$\r$\n"
  FileWrite $0 "    Write-Host 'Defender exclusion added: $INSTDIR'$\r$\n"
  FileWrite $0 "} catch {$\r$\n"
  FileWrite $0 "    Write-Host 'Note: Could not add Defender exclusion (non-critical)'$\r$\n"
  FileWrite $0 "}$\r$\n"
  FileClose $0
  nsExec::ExecToLog "powershell.exe -NoProfile -ExecutionPolicy Bypass -File $\"$PLUGINSDIR\defender_add.ps1$\""
  Pop $0

  ; ===== 2. Desktop Shortcut =====
  DetailPrint "Creating Desktop shortcut..."
  CreateShortCut "$DESKTOP\NetFlow Studio.lnk" "$INSTDIR\NetFlow Studio.exe" "" "$INSTDIR\NetFlow Studio.exe" 0

  ; ===== 3. Autostart Scheduled Task (Admin + Minimized) =====
  DetailPrint "Registering autostart task..."
  FileOpen $0 "$PLUGINSDIR\autostart_setup.ps1" w
  FileWrite $0 "try {$\r$\n"
  FileWrite $0 "    $$exePath = '$INSTDIR\NetFlow Studio.exe'$\r$\n"
  FileWrite $0 "    $$action = New-ScheduledTaskAction -Execute $$exePath -Argument '--minimized'$\r$\n"
  FileWrite $0 "    $$trigger = New-ScheduledTaskTrigger -AtLogOn$\r$\n"
  FileWrite $0 "    $$principal = New-ScheduledTaskPrincipal -UserId (whoami) -RunLevel Highest -LogonType Interactive$\r$\n"
  FileWrite $0 "    $$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -ExecutionTimeLimit ([TimeSpan]::Zero)$\r$\n"
  FileWrite $0 "    Unregister-ScheduledTask -TaskName 'NetFlowStudio_Autostart' -Confirm:$$false -ErrorAction SilentlyContinue$\r$\n"
  FileWrite $0 "    Register-ScheduledTask -TaskName 'NetFlowStudio_Autostart' -Action $$action -Trigger $$trigger -Principal $$principal -Settings $$settings -Description 'NetFlow Studio - Auto-start minimized with admin privileges' -Force$\r$\n"
  FileWrite $0 "    Write-Host 'Autostart task registered successfully'$\r$\n"
  FileWrite $0 "} catch {$\r$\n"
  FileWrite $0 "    Write-Host 'Warning: Could not register autostart task'$\r$\n"
  FileWrite $0 "    Write-Host $$_.Exception.Message$\r$\n"
  FileWrite $0 "}$\r$\n"
  FileClose $0
  nsExec::ExecToLog "powershell.exe -NoProfile -ExecutionPolicy Bypass -File $\"$PLUGINSDIR\autostart_setup.ps1$\""
  Pop $0

  DetailPrint "NetFlow Studio installation complete!"
!macroend

; ---------------------------------------------------------------------------
; NSIS_HOOK_POSTUNINSTALL - Runs after all files are removed
; ---------------------------------------------------------------------------
!macro NSIS_HOOK_POSTUNINSTALL

  ; ===== 1. Remove Defender Exclusion =====
  DetailPrint "Removing Windows Defender exclusion..."
  FileOpen $0 "$PLUGINSDIR\defender_remove.ps1" w
  FileWrite $0 "try {$\r$\n"
  FileWrite $0 "    Remove-MpPreference -ExclusionPath '$INSTDIR' -ErrorAction SilentlyContinue$\r$\n"
  FileWrite $0 "} catch {}$\r$\n"
  FileClose $0
  nsExec::ExecToLog "powershell.exe -NoProfile -ExecutionPolicy Bypass -File $\"$PLUGINSDIR\defender_remove.ps1$\""
  Pop $0

  ; ===== 2. Remove Desktop Shortcut =====
  DetailPrint "Removing Desktop shortcut..."
  Delete "$DESKTOP\NetFlow Studio.lnk"

  ; ===== 3. Remove Autostart Task =====
  DetailPrint "Removing autostart task..."
  nsExec::ExecToLog 'schtasks /delete /tn "NetFlowStudio_Autostart" /f'
  Pop $0

  DetailPrint "NetFlow Studio uninstallation complete."
!macroend
