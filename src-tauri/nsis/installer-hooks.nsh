; ============================================================================
; Flux - NSIS Installer Hooks (Professional Edition)
; ============================================================================
; Features:
; 1. Pre-Install/Pre-Uninstall: Stops running exe and kernel driver automatically
; 2. Windows Defender exclusion (prevents false positive WinDivert detection)
; 3. Desktop shortcut with Admin elevation flag
; 4. Autostart scheduled task (runs as Admin, minimized to tray)
; ============================================================================

; ---------------------------------------------------------------------------
; NSIS_HOOK_PREINSTALL - Runs before files are copied/extracted
; ---------------------------------------------------------------------------
!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Closing any running Flux instances and stopping WinDivert driver..."
  nsExec::Exec 'taskkill /F /IM "flux.exe"'
  nsExec::Exec 'taskkill /F /IM "NetFlow Studio.exe"'
  nsExec::Exec 'taskkill /F /IM "netflow-studio.exe"'
  nsExec::Exec 'net stop WinDivert'
  nsExec::Exec 'sc stop WinDivert'
  Sleep 1000
!macroend

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
  CreateShortCut "$DESKTOP\Flux.lnk" "$INSTDIR\flux.exe" "" "$INSTDIR\flux.exe" 0
  ; Clean up any old shortcut
  Delete "$DESKTOP\NetFlow Studio.lnk"

  ; ===== 3. Autostart Scheduled Task (Admin + Minimized) =====
  DetailPrint "Registering autostart task..."
  FileOpen $0 "$PLUGINSDIR\autostart_setup.ps1" w
  FileWrite $0 "try {$\r$\n"
  FileWrite $0 "    $$exePath = '$INSTDIR\flux.exe'$\r$\n"
  FileWrite $0 "    $$workDir = '$INSTDIR'$\r$\n"
  FileWrite $0 "    $$action = New-ScheduledTaskAction -Execute $$exePath -Argument '--minimized' -WorkingDirectory $$workDir$\r$\n"
  FileWrite $0 "    $$trigger = New-ScheduledTaskTrigger -AtLogOn$\r$\n"
  FileWrite $0 "    $$principal = New-ScheduledTaskPrincipal -UserId (whoami) -RunLevel Highest -LogonType Interactive$\r$\n"
  FileWrite $0 "    $$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -ExecutionTimeLimit ([TimeSpan]::Zero)$\r$\n"
  FileWrite $0 "    Unregister-ScheduledTask -TaskName 'Flux_Autostart' -Confirm:$$false -ErrorAction SilentlyContinue$\r$\n"
  FileWrite $0 "    Unregister-ScheduledTask -TaskName 'NetFlowStudio_Autostart' -Confirm:$$false -ErrorAction SilentlyContinue$\r$\n"
  FileWrite $0 "    Register-ScheduledTask -TaskName 'Flux_Autostart' -Action $$action -Trigger $$trigger -Principal $$principal -Settings $$settings -Description 'Flux - Auto-start minimized with admin privileges' -Force$\r$\n"
  FileWrite $0 "    Write-Host 'Autostart task registered successfully'$\r$\n"
  FileWrite $0 "} catch {$\r$\n"
  FileWrite $0 "    Write-Host 'Warning: Could not register autostart task'$\r$\n"
  FileWrite $0 "    Write-Host $$_.Exception.Message$\r$\n"
  FileWrite $0 "}$\r$\n"
  FileClose $0
  nsExec::ExecToLog "powershell.exe -NoProfile -ExecutionPolicy Bypass -File $\"$PLUGINSDIR\autostart_setup.ps1$\""
  Pop $0

  DetailPrint "Flux installation complete!"
!macroend

; ---------------------------------------------------------------------------
; NSIS_HOOK_PREUNINSTALL - Runs before files are removed
; ---------------------------------------------------------------------------
!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Closing any running Flux instances and stopping WinDivert driver..."
  nsExec::Exec 'taskkill /F /IM "flux.exe"'
  nsExec::Exec 'taskkill /F /IM "NetFlow Studio.exe"'
  nsExec::Exec 'taskkill /F /IM "netflow-studio.exe"'
  nsExec::Exec 'net stop WinDivert'
  nsExec::Exec 'sc stop WinDivert'
  Sleep 1000
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
  Delete "$DESKTOP\Flux.lnk"
  Delete "$DESKTOP\NetFlow Studio.lnk"

  ; ===== 3. Remove Autostart Task =====
  DetailPrint "Removing autostart task..."
  nsExec::ExecToLog 'schtasks /delete /tn "Flux_Autostart" /f'
  nsExec::ExecToLog 'schtasks /delete /tn "NetFlowStudio_Autostart" /f'
  Pop $0

  DetailPrint "Flux uninstallation complete."
!macroend
