<p align="center">
  <img src="public/logo.svg" width="120" height="120" alt="NetFlow Studio Logo" />
</p>

<h1 align="center">NetFlow Studio | نت فلو ستوديو</h1>

<p align="center">
  <strong>Production-Grade Bandwidth Controller, Network Traffic Shaper & Floating Speed Widget for Windows</strong><br>
  <strong>مراقب متقدم لحركة الشبكة، محدد النطاق الترددي للبرامج، وودجت عائم ذكي لشريط مهام ويندوز</strong>
</p>

<p align="center">
  <a href="https://github.com/Tubba-Soft"><img src="https://img.shields.io/badge/Organization-Tubba--Soft-blue?style=for-the-badge&logo=github" alt="Tubba-Soft" /></a>
  <a href="#"><img src="https://img.shields.io/badge/Author-Amjad%20Alwan-teal?style=for-the-badge" alt="Amjad Alwan" /></a>
  <a href="#"><img src="https://img.shields.io/badge/Tauri-v2.2-24C8D5?style=for-the-badge&logo=tauri&logoColor=white" alt="Tauri v2" /></a>
  <a href="#"><img src="https://img.shields.io/badge/Rust-2021%20Edition-black?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" /></a>
  <a href="#"><img src="https://img.shields.io/badge/React-18%20%2B%20Vite-61DAFB?style=for-the-badge&logo=react&logoColor=black" alt="React" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-emerald?style=for-the-badge" alt="License MIT" /></a>
</p>

<p align="center">
  <a href="#-العربية"><strong>العربية</strong></a> •
  <a href="#-english"><strong>English</strong></a>
</p>

---

<div align="center">

> 💡 **تطوير فريق تبع سوفت (Tubba-Soft Team) — المطور الرئيسي: أمجد علوان (Amjad Alwan)**  
> مشروع مفتوح المصدر بالكامل (Open Source) مصمم لتقديم أقصى درجات التحكم في شبكة نظام Windows مع تجربة مستخدم عصرية وسريعة.

</div>

---

# 🇸🇦 العربية

## 🌟 نظرة عامة على المشروع

**NetFlow Studio** هو تطبيق مكتبي متكامل فائق الأداء والسرعة، يُنافس ويتفوق على برامج إدارة الشبكات الكلاسيكية مثل *NetLimiter* و *GlassWire*. يجمع التطبيق بين قوة محرك **Rust** عبر طبقة النواة (Kernel Space) باستخدام مشغّل **WinDivert** الرقمي المعتمد، ومرونة واجهة **Tauri v2 + React 18 + TailwindCSS**.

يمنحك التطبيق سيطرة مطلقة ولحظية على كل برنامج وكل اتصال شبكي يمر عبر حاسوبك، مع ودجت عائم أنيق لشريط المهام يراقب السرعات الحية في كل ثانية.

---

## ✨ أبرز الميزات التقنية

### 1. مراقبة دقيقة لاتصالات الشبكة (Deep 5-Tuple Dissection)
* **شجرة العمليات الهيكلية**: تصنيف هرمي دقيق (`اسم البرنامج .exe ➔ المنافذ والاتصالات النشطة`).
* **استخراج أسماء النطاقات (TLS SNI Extraction)**: قراءة اسم النطاق فورياً من حزم `ClientHello` على المنفذ 443 (مثل `youtube.com`، `discord.gg`، `github.com`) دون الحاجة لفك التشفير.
* **إسناد منافذ UDP اللحظية (Ephemeral Ports)**: كاش عالي السرعة (`DashMap`) يربط اتصالات الألعاب (مثل CS:GO، Apex، Discord Voice) بالبرنامج الفعلي بدقة 100%.

### 2. محدد النطاق الترددي وخوارزمية تشكيل البيانات (Token Bucket Shaper)
* **تحديد السرعة (Throttling)**: إمكانية تحديد سقف أقصى للتحميل أو الرفع لأي برنامج أو اتصال محدد (بدقة الكيلوبايت أو الميجابايت في الثانية).
* **حظر الاتصال الفوري (Firewall Drop)**: قطع سيل البيانات في اتجاه التحميل أو الرفع بنقرة زر واحدة دون إعادة حقن الحزم.
* **قواعد دائمة**: حفظ إعدادات الحظر والتحديد في قاعدة بيانات **SQLite** مدمجة يتم استرجاعها تلقائياً عند تشغيل الجهاز.

### 3. ودجت شريط المهام العائم الذكي (Floating Speed Capsule Widget)
* **سحب وتحريك حر (Threshold-Based Drag)**: اسحب الودجت بحرية لأي مكان على الشاشة دون أن يتعارض مع النقر المزدوج لفتح التطبيق.
* **الشفافية الذكية عند الخمول (Smart Idle Transparency)**: عند ترك الودجت دون لمس الماوس لفترة محددة (مثلاً ثانيتين)، يتلاشى بلطف إلى نسبة شفافية ناعمة لعدم مضايقة المستخدم، وبمجرد اقتراب الماوس منه يصحو فوراً ساطعاً بنسبة 100%.
* **تثبيت في زوايا الشاشة (Docking Presets)**: إمكانية تثبيت الودجت في موضع دقيق (فوق شريط المهام، في المنتصف، أو الزوايا).
* **قائمة أوامر مدمجة بدون قص (Zero Menu Clipping)**: قائمة سريعة تنبثق داخل الكبسولة تتيح فتح التطبيق، التثبيت، أو الإلغاء دون أي اقتطاع بكسلي من ويندوز.
* **ثبات العرض والأرقام**: استخدام `min-w-[78px]` و `tabular-nums` لمنع أي اهتزاز في العرض عند تذبذب السرعات.

### 4. التشغيل التلقائي الهادئ في الخلفية (Silent Auto-Start)
* تكامل مباشر مع **Windows Task Scheduler** للتشغيل التلقائي عند تسجيل دخول المستخدم بأعلى صلاحيات كمسؤول (`Highest RunLevel`) **دون إظهار نافذة تأكيد UAC المزعجة**.
* أيقونة متطورة في صينية النظام (System Tray) مع خيارات فتح التطبيق، إظهار/إخفاء الودجت، وقراءة السرعات المباشرة في التلميح.

---

## 🛠️ متطلبات وطريقة البناء والتشغيل

### المتطلبات الأساسية:
1. نظام تشغيل **Windows 10 / 11 (x64)**.
2. بيئة **Node.js** (إصدار 18 أو أحدث) و **npm**.
3. بيئة **Rust** (إصدار 1.80+) مع أدوات MSVC C++.

### خطوات التثبيت والتشغيل:
```bash
# 1. استنساخ المستودع
git clone https://github.com/Tubba-Soft/netflow-studio.git
cd netflow-studio

# 2. تثبيت الحزم البرمجية
npm install

# 3. تشغيل وضع التطوير
npm run dev

# 4. بناء التطبيق النهائي للإنتاج (Release Build)
npm run build
cargo build --release --manifest-path src-tauri/Cargo.toml
```

---

# 🇬🇧 English

## 🌟 Executive Overview

**NetFlow Studio** is an open-source, high-performance network bandwidth controller, traffic shaper, and system monitor engineered for Windows. Built by the **Tubba-Soft Team** and architected by **Amjad Alwan**, it pairs a kernel-level packet inspection engine in **Rust** (via signed **WinDivert**) with a sleek, cyber-glassmorphic frontend in **Tauri v2 + React 18 + TailwindCSS**.

---

## ⚡ Key Highlights

* **Deep 5-Tuple Stream Inspection**: Real-time hierarchy mapping executables to active TCP/UDP sockets with instant bandwidth counters.
* **TLS SNI Domain Dissection**: Parses `ClientHello` packets on port 443 to reveal domains (e.g., `youtube.com`, `discord.gg`, `github.com`) without SSL termination.
* **Token Bucket Traffic Shaper**: Microsecond-precision rate-limiting queues and instant blocking for both inbound and outbound traffic.
* **Floating Desktop Speed Widget**:
  * Threshold-based dragging (differentiates between single clicks, double clicks, and drag gestures).
  * Auto-transparency on mouse idle (smoothly fades into the background and instantly wakes up on hover).
  * Zero-clipping inline capsule overlay menu.
  * Tabular-num layout stability preventing subpixel shifts.
* **Silent Task Scheduler Elevation**: Automatically launches with elevated administrator rights on Windows logon without repetitive UAC prompts.
* **Bilingual UI (Arabic RTL & English LTR)** with real-time dynamic switching.

---

## 🏗️ Architecture & Stack

| Layer | Technology | Purpose |
|---|---|---|
| **Kernel Driver** | WinDivert 2.2-A Signed | Packet capture, diversion, and kernel-level injection |
| **System Engine** | Rust 2021 + Windows-Sys | Token Bucket shaper, IP Helper socket resolution, IPC |
| **Persistence** | SQLite (`rusqlite`) | Process firewall rules, widget coordinates, telemetry stats |
| **Application GUI** | Tauri v2.2 + React 18 + Vite | Modern reactive interface with system tray integration |
| **Styling** | Tailwind CSS + Lucide Icons | Dark glassmorphism, responsive RTL/LTR typography |

---

## 💻 Building from Source

```powershell
# Clone the repository
git clone https://github.com/Tubba-Soft/netflow-studio.git
cd netflow-studio

# Install dependencies & build frontend
npm install
npm run build

# Compile release binary
cargo build --release --manifest-path src-tauri/Cargo.toml
```

Launch elevated using:
```powershell
.\run-admin.bat
```

---

## 👥 Credits & Authors

* **Organization**: [Tubba-Soft (فريق تبع سوفت)](https://github.com/Tubba-Soft)
* **Lead Developer & Architect**: **Amjad Alwan (أمجد علوان)**

---

## 📄 License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.

<p align="center">
  ⭐ <strong>If you like this project, please consider giving it a Star on GitHub!</strong> ⭐
</p>
