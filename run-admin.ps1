# Flux - Administrator Launcher
# مشغّل Flux كمسؤول مع درايفر النواة WinDivert

$targetLocations = @(
    "C:\Program Files\Flux\flux.exe",
    "$PSScriptRoot\src-tauri\target\release\flux.exe",
    "C:\Users\amjad\.cargo_target\netflow_studio\release\flux.exe",
    "C:\Users\amjad\.cargo_target\netflow_studio\release\netflow-studio.exe"
)

$exePath = ""
foreach ($loc in $targetLocations) {
    if (Test-Path $loc) {
        $exePath = $loc
        break
    }
}
if (-not $exePath) {
    $exePath = "$PSScriptRoot\src-tauri\target\release\flux.exe"
}
$binDir = Split-Path -Parent $exePath

# 1. إغلاق أي نسخة سابقة لضمان عدم وجود قفل على قاعدة البيانات أو ملفات التعريف
Get-Process -Name "flux" -ErrorAction SilentlyContinue | Stop-Process -Force
Get-Process -Name "netflow-studio" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

# 2. التأكد من وجود ملفات درايفر WinDivert بجانب الملف التنفيذي
if (Test-Path $binDir) {
    Copy-Item "$PSScriptRoot\src-tauri\bin\WinDivert*" -Destination $binDir -Force -ErrorAction SilentlyContinue
}

# 3. تنظيف ملفات القفل الخاصة بـ WebView2
$wv2AdminLock = "$env:LOCALAPPDATA\com.tubbasoft.flux\webview_admin\EBWebView\lockfile"
if (Test-Path $wv2AdminLock) {
    Remove-Item $wv2AdminLock -Force -ErrorAction SilentlyContinue
}

# 4. إضافة استثناء Windows Defender لمنع الحذف الخاطئ لملفات WinDivert
Write-Host ""
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host " Flux - إعداد استثناء Windows Defender" -ForegroundColor Yellow
Write-Host "=================================================" -ForegroundColor Cyan

$exclusionPaths = @($binDir, "$PSScriptRoot\src-tauri\bin", "C:\Program Files\Flux")
foreach ($path in $exclusionPaths) {
    if (Test-Path $path) {
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
}

Write-Host ""
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host " Flux - تشغيل التطبيق كمسؤول مع درايفر النواة" -ForegroundColor Green
Write-Host "=================================================" -ForegroundColor Cyan
Write-Host "مسار الملف التنفيذي: $exePath" -ForegroundColor Gray

Start-Process -FilePath $exePath -Verb RunAs
Write-Host "تم إرسال طلب التشغيل كمسؤول بنجاح!" -ForegroundColor Green
