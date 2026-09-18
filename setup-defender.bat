@echo off
title Flux - Antivirus Setup
echo.
echo =========================================================
echo   Flux - Windows Defender Setup
echo   اعداد استثناء ويندوز ديفندر لمنع الحذف الخاطئ
echo =========================================================
echo.
echo This script adds a Windows Defender exclusion for the
echo Flux folder to prevent false positive deletion
echo of the WinDivert network driver files.
echo.
echo يقوم هذا السكريبت بإضافة استثناء في ويندوز ديفندر
echo لمنع حذف ملفات درايفر الشبكة WinDivert بالخطأ
echo.
echo =========================================================
echo.

:: Check for admin privileges
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo [!] This script requires Administrator privileges.
    echo [!] هذا السكريبت يتطلب صلاحيات مسؤول.
    echo.
    echo Right-click this file and select "Run as administrator"
    echo انقر بالزر الايمن واختر "تشغيل كمسؤول"
    echo.
    pause
    exit /b 1
)

:: Add exclusion for the current directory (where the app is extracted)
powershell -NoProfile -ExecutionPolicy Bypass -Command ^
  "try { " ^
  "  $appDir = '%~dp0'.TrimEnd('\'); " ^
  "  Write-Host ''; " ^
  "  Write-Host 'Adding Windows Defender exclusion...' -ForegroundColor Cyan; " ^
  "  Write-Host 'Directory: ' $appDir -ForegroundColor Gray; " ^
  "  Add-MpPreference -ExclusionPath $appDir -ErrorAction Stop; " ^
  "  Write-Host ''; " ^
  "  Write-Host '[OK] Exclusion added successfully!' -ForegroundColor Green; " ^
  "  Write-Host '[OK] تم اضافة الاستثناء بنجاح!' -ForegroundColor Green; " ^
  "  Write-Host ''; " ^
  "  Write-Host 'You can now run Flux without antivirus interference.' -ForegroundColor White; " ^
  "  Write-Host 'يمكنك الآن تشغيل البرنامج بدون تدخل مكافح الفيروسات' -ForegroundColor White; " ^
  "} catch { " ^
  "  Write-Host ''; " ^
  "  Write-Host '[ERROR] Failed to add exclusion:' -ForegroundColor Red; " ^
  "  Write-Host $_.Exception.Message -ForegroundColor Red; " ^
  "  Write-Host ''; " ^
  "  Write-Host 'Please add the exclusion manually:' -ForegroundColor Yellow; " ^
  "  Write-Host '1. Open Windows Security' -ForegroundColor White; " ^
  "  Write-Host '2. Virus and threat protection > Manage settings' -ForegroundColor White; " ^
  "  Write-Host '3. Exclusions > Add or remove exclusions' -ForegroundColor White; " ^
  "  Write-Host '4. Add folder > Select:' $appDir -ForegroundColor White; " ^
  "}"

echo.
pause
