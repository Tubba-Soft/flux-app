@echo off
title NetFlow Studio - Launching as Administrator...
echo ========================================================
echo  NetFlow Studio - تشغيل كمسؤول مع تفعيل درايفر WinDivert
echo ========================================================
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-admin.ps1"
pause
