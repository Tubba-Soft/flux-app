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

# 4. إضافة استثناء Windows Defender لمنع الحذف الخاطئ لملفات WinDivert
#    (يتطلب صلاحيات مسؤول - يتم تشغيل هذا السكريبت كمسؤول أصلاً)
Write-Host ""
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host " NetFlow Studio - إعداد استثناء Windows Defender" -ForegroundColor Yellow
Write-Host "=================================================" -ForegroundColor Cyan

$exclusionPaths = @($binDir, "$PSScriptRoot\src-tauri\bin")
foreach ($path in $exclusionPaths) {
    try {
        $existing = (Get-MpPreference).ExclusionPath
        if ($existing -and $existing -contains $path) {
            Write-Host "  [OK] الاستثناء موجود مسبقاً: $path" -ForegroundColor DarkGray
        } else {
            Add-MpPreference -ExclusionPath $path -ErrorAction Stop
            Write-Host "  [+] تم إضافة استثناء Defender: $path" -ForegroundColor Green
        }
    } catch {
        Write-Host "  [!] لم يتم إضافة الاستثناء (غير حرج): $path" -ForegroundColor DarkYellow
        Write-Host "      $($_.Exception.Message)" -ForegroundColor DarkGray
    }
}

Write-Host ""
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host " NetFlow Studio - تشغيل التطبيق كمسؤول مع درايفر النواة" -ForegroundColor Green
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "مسار الملف التنفيذي: $exePath" -ForegroundColor Gray

Start-Process -FilePath $exePath -Verb RunAs
Write-Host "تم إرسال طلب التشغيل كمسؤول بنجاح!" -ForegroundColor Green
