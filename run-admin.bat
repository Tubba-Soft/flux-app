@echo off
title Flux - Launching as Administrator...
echo ========================================================
echo  Flux - تشغيل كمسؤول مع تفعيل درايفر WinDivert
echo ========================================================
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run-admin.ps1"
pause
