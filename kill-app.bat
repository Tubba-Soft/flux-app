@echo off
title NetFlow Studio - تحرير الملفات وايقاف الدرايفر

:: فحص صلاحيات المسؤول، واذا لم تكن متوفرة يطلبها تلقائيا من الويندوز (UAC Prompt)
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo جاري طلب صلاحيات المسؤول من الويندوز...
    powershell -Command "Start-Process cmd -ArgumentList '/c \"\"%~f0\"\"' -Verb RunAs"
    exit /b
)

color 0A
echo ========================================================
echo   NetFlow Studio - تحرير ملفات البناء وايقاف WinDivert
echo ========================================================
echo.

echo [1/2] اغلاق اي نسخة شغالة من التطبيق...
taskkill /F /IM netflow-studio.exe >nul 2>&1

echo [2/2] ايقاف خدمة تعريف النواة WinDivert...
net stop WinDivert >nul 2>&1
sc stop WinDivert >nul 2>&1

echo.
echo ========================================================
echo   [OK] تم ايقاف الدرايفر وتحرير جميع الملفات المقفولة!
echo   الان يمكنك تشغيل: npm run tauri build
echo ========================================================
echo.
pause
