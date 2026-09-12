# NetFlow Studio - Administrator Launcher
# مشغّل NetFlow Studio كمسؤول مع درايفر النواة WinDivert

$exePath = "C:\Users\amjad\.cargo_target\netflow_studio\release\netflow-studio.exe"
$binDir = "C:\Users\amjad\.cargo_target\netflow_studio\release"

# 1. إغلاق أي نسخة سابقة لضمان عدم وجود قفل على قاعدة البيانات أو ملفات التعريف
Get-Process -Name "netflow-studio" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# 2. التأكد من وجود ملفات درايفر WinDivert بجانب الملف التنفيذي
Copy-Item "$PSScriptRoot\src-tauri\bin\WinDivert*" -Destination $binDir -Force -ErrorAction SilentlyContinue

# 3. تنظيف ملفات القفل الخاصة بـ WebView2
$wv2AdminLock = "$env:LOCALAPPDATA\com.netflowstudio.desktop\webview_admin\EBWebView\lockfile"
if (Test-Path $wv2AdminLock) {
    Remove-Item $wv2AdminLock -Force -ErrorAction SilentlyContinue
}

Write-Host "=================================================" -ForegroundColor Cyan
Write-Host " NetFlow Studio - تشغيل التطبيق كمسؤول مع درايفر النواة" -ForegroundColor Green
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "مسار الملف التنفيذي: $exePath" -ForegroundColor Gray

Start-Process -FilePath $exePath -Verb RunAs
Write-Host "تم إرسال طلب التشغيل كمسؤول بنجاح!" -ForegroundColor Green
