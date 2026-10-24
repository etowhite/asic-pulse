# ASIC Pulse

**A local-first control room for ASIC mining fleets.**

ASIC Pulse is a standalone Windows desktop application for discovering, monitoring, and managing ASIC miners on a local network. It brings subnet scanning, live fleet telemetry, batch pool and power configuration, board-level diagnostics, firmware workflow, and Bitcoin profitability analytics into one workspace — without a cloud account, an external dashboard, or an agent installed on the miners.

[English](#english) · [Русский](#russian)

---

<a id="english"></a>
## English

### What ASIC Pulse does

ASIC Pulse is built for people who run miners on their own network. Point it at one or more IPv4 segments; it probes every address, identifies compatible devices, and builds a live inventory of the fleet. From that inventory you can inspect individual boards, compare real-time and average hashrate, change pools and worker names in bulk, switch power profiles, tune frequency and voltage, reboot or blink selected devices, and run a firmware workflow — all from one window.

The application is deliberately local. It talks directly to miners over TCP port 4028 and their HTTP CGI endpoints. There is no account, no cloud relay, no external monitoring agent, and no telemetry upload. The only optional fleet-related request is to mempool.space for live Bitcoin network statistics. The interface also loads its display and icon fonts from Google Fonts; if those requests are blocked, the layout falls back to local fonts.

ASIC Pulse is designed for mixed, real-world fleets. Antminer, WhatsMiner, and Avalon devices can sit in the same table; the app normalizes what each device exposes into a common inventory while keeping vendor-specific controls available where the firmware supports them.

### At a glance

| Area | What you get |
|---|---|
| Purpose | Local discovery, monitoring, and batch control of ASIC miners |
| Network model | Direct LAN access; no cloud relay, account, or miner-side agent |
| Discovery | IPv4 ranges, automatic subnet detection, CSV/TXT import |
| Protocols | CGMiner/BMMiner API on TCP 4028; HTTP Digest CGI |
| Miner families | Antminer, WhatsMiner, Avalon, and other CGMiner-compatible devices |
| Fleet operations | Pools and workers, pool difficulty, power modes, frequency/voltage, reboot, LED locate, firmware workflow |
| Telemetry | Hashrate, temperatures, fans, uptime, pools, workers, hardware errors, rejection rate, boards and chips |
| Analytics | Profitability calculator and Bitcoin network statistics from mempool.space |
| Output | CSV export, standalone HTML report, rotating operation log |
| Interface | English and Russian, four color themes, simulated demo fleet |
| Platform | Windows x64, WebView2, built with Tauri and Rust |

### Design principles

- **Local-first by design.** Miners are queried directly. Configuration, logs, and reports stay on the machine.
- **Fleet-scale, not one device at a time.** Selection, batch dialogs, and apply-to-all actions are part of the primary workflow.
- **Operational signal over raw API output.** Discovery responses are normalized into a readable inventory with status, telemetry, and actions.
- **Explicit targets.** The selection is always visible, and every batch dialog states whether it applies to selected miners or the whole fleet.
- **Bilingual and themeable.** The entire workspace can switch between English and Russian and between four color themes without restarting.

### Workspace at a glance

| Screen | Purpose |
|---|---|
| **Scanner** | Define address ranges, discover miners, inspect the fleet table, and launch batch actions |
| **Dashboard** | See fleet KPIs, session hashrate history, anomalies, and miner cards |
| **Tools** | Estimate mining economics and follow Bitcoin network conditions |
| **Settings** | Tune scan behavior, timeouts, concurrency, monitoring, and vendor credentials |
| **Event Log** | Review operations, warnings, and the raw rotating log file |

A persistent top bar provides theme selection, language switching, demo/live mode, monitoring, and scan/cancel controls. When miners are selected, a floating action bar appears with the most common batch operations.

On launch, ASIC Pulse performs an initial scan using the saved ranges and the current mode. The first run starts in demo mode so the interface is usable without hardware.

### Scanner

#### Address ranges

The scanner works with IPv4 ranges. Each range has its own comment/location, enabled state, and delete action, and all ranges are persisted between launches.

- **Automatic subnet detection** reads the Windows network configuration and adds one local subnet per detected adapter, excluding loopback and link-local addresses.
- **Manual ranges** can be a single address (`192.168.1.101`) or a last-octet sweep inside one subnet (`192.168.1.1-254`, `192.168.1.10-192.168.1.200`).
- **CSV/TXT import** accepts one entry per line in the form `range,comment`. The file is loaded into the import editor first, so it can be reviewed or edited before it is applied.
- **Enable/disable** lets you keep a large address plan while scanning only the segments you need.

Ranges are swept inside a single /24 at a time. IPv6 is not supported.

#### Discovery

Scanning is parallel and bounded by a semaphore:

- **Concurrency:** 8 to 512 simultaneous probes, configurable in Settings.
- **Port 4028 timeout:** 150 to 5000 ms per probe.
- **HTTP CGI timeout:** configured separately for web-interface fallbacks.
- **Live progress:** the scanner reports how many addresses have been checked and how many ASIC devices have been found.
- **Cancellation:** an in-progress scan can be stopped without closing the app.

For every address, ASIC Pulse first sends a combined `summary+pools+stats+version` request to the CGMiner/BMMiner API on port 4028, then retries with `summary` if needed. If the API does not answer, it falls back to the HTTP Digest CGI interface used by many Antminer web panels.

The resulting inventory can include model, real-time and average hashrate, temperatures, fan speed, uptime, active pools and workers, hardware errors, rejection rate, firmware string, MAC address, voltage, frequency, power draw, and per-board chip telemetry when the device exposes it.

Two table options keep large scans manageable:

- **Only found miners** hides offline rows unless the Offline filter is selected.
- **Show pools 2/3** adds the secondary and tertiary pool/worker columns.

#### Fleet table

The scanner table is the operational center of the app. It shows:

- selection checkbox, IP address, status, model, working mode;
- real-time and average hashrate;
- maximum temperature, fan speed, and uptime;
- pool 1 and worker 1, with optional pool/worker columns for pools 2 and 3.

The table supports:

- **Status filters:** All, Normal, Anomalies, Offline.
- **Search** across IP address, model, worker, pool, and working mode.
- **Column sorting** by IP (numeric order), status, model, mode, hashrate, temperature, fans, uptime, and pools.
- **Row selection** by click or checkbox, select-all for the currently visible rows, and a clear-selection action.
- **Double-click** on a row to open the full miner detail view.

Status values are **Online**, **Overheat**, **Low Hash**, and **Offline**. Hot temperatures and low hashrate are highlighted both in the table and on the dashboard. The KPI strip above the table summarizes total real-time/average hashrate, online/total miners, average and maximum temperature, and the number of active anomalies.

### Dashboard

The dashboard is the fleet-health view:

- **KPI cards** repeat the key numbers: total hashrate, online miners, average temperature, and anomalies.
- **Session chart** plots the last 30 total-hashrate samples collected during scans and monitoring ticks.
- **Anomaly panel** lists every miner that is not in the normal state; each entry opens the miner detail view.
- **Fleet cards** show IP, model, temperature, real-time hashrate, and working mode. Cards can be filtered by All, Antminer, WhatsMiner, or Avalon.
- **HTML report** creates a standalone snapshot of the current fleet and opens it in the default browser.

Charts are session-scoped. ASIC Pulse does not maintain a historical time-series database.

### Miner detail

Double-click any row or dashboard card to open the detail view:

- **Identity:** IP, model, MAC address, firmware string, working mode, and power draw.
- **Live KPIs:** real-time hashrate, maximum chip temperature, fan RPM, hardware errors, and rejection percentage.
- **Hashboard table:** board index, chip count, frequency, voltage, board temperature, chip temperature, board hashrate, and hardware errors.
- **Temperature history:** a small chart of recent samples for that miner.
- **Actions:** open the miner's web UI, ping the device, copy its IP or worker name, jump to pool configuration, or reboot it.

Telemetry coverage is firmware-dependent. Some devices expose full board-level data; others return only summary values. The detail view shows what is available and reports when no board data was provided.

### Batch operations

Batch actions are available from the floating selection bar and from the scanner toolbar. Dialogs that support both scopes offer **Apply to selected** and **Apply to all**.

#### Pools and workers

The pool dialog configures up to three Stratum pools:

- pool URL and port for each of the three pools;
- a shared worker prefix (subaccount) and pool password;
- an optional pool difficulty (`diff`) applied to each non-empty pool; empty or zero means automatic difficulty;
- a worker suffix mode.

| Suffix mode | Result for worker prefix `north_farm` and miner `192.168.1.101` |
|---|---|
| IP-based | `north_farm.1x101` |
| No suffix | `north_farm` |
| Empty worker | blank worker name |
| Custom | `north_farm.rack01` |

Pool changes are sent through the Antminer-compatible HTTP Digest CGI endpoint, first as a modern JSON request and then as a legacy form-encoded request for older firmware. The app reports how many devices acknowledged the configuration and writes the result to the event log. Other vendors may require their own web interface or firmware that accepts the same API.

#### Power and frequency

The power dialog combines a working-mode preset with optional manual frequency and voltage control.

| Mode | Intent |
|---|---|
| Normal | Standard vendor operating profile |
| LPM | Low-power mode |
| Enhanced LPM | Deeper power saving |
| Overclock | Higher hashrate profile |
| Underclock | Lower frequency for efficiency |
| Sleep | Low-activity/sleep state |

Model-family presets are provided for Antminer S21/T21/S19, Antminer S9/T9, WhatsMiner M50/M60, and Avalon A6/A7/A8. Changing the family or mode updates the suggested frequency.

Manual control adds:

- **Frequency:** 100 to 1500 MHz.
- **Voltage:** optional, in volts.

When manual frequency is enabled, ASIC Pulse applies the working mode and then the frequency/voltage step. For Antminer it reads the current configuration and writes it back with the new frequency so pools and unrelated settings are not discarded. For WhatsMiner it uses the `set.miner.freq` command on port 4028. For Avalon and other CGMiner-compatible devices it falls back to the `ascset` command. Voltage is written through the Antminer configuration path; other vendors may ignore it.

Frequency and voltage control depends on the model and firmware. Stock vendor firmware may ignore these fields; custom firmware and supported WhatsMiner/Avalon builds are more likely to accept them. Use vendor specifications and change one miner first.

#### Reboot, LED locate, and firmware

- **Reboot** targets the selected miners or the whole fleet. The app first calls the Antminer reboot CGI endpoint and falls back to the `restart` command on port 4028 for Avalon-style devices.
- **LED locate** asks selected miners to blink their locate LED, making it easier to find a physical device in a rack.
- **Firmware workflow** accepts a firmware image, targets selected miners (or the whole fleet if nothing is selected), and shows a per-device result list. The upgrade is performed through the vendor's firmware endpoint; image compatibility, validation, and rollback remain the vendor's responsibility.

Reboot, power-mode changes, frequency changes, and firmware operations interrupt mining. Use them deliberately, especially on production hardware.

### Tools

The Tools screen combines a profitability calculator with live Bitcoin network context.

#### Profitability calculator

Inputs:

- fleet hashrate in TH/s;
- power draw in watts;
- electricity tariff in USD per kWh;
- pool fee in percent;
- BTC price in USD;
- network hashrate in EH/s.

Outputs:

- BTC mined per day;
- revenue per day;
- electricity cost per day;
- profit per day;
- profit per month (30 days);
- margin;
- break-even BTC price.

The calculation uses 144 blocks per day, a block reward derived from the current block height (halving-aware), and the entered pool fee:

```text
btc_per_day   = (fleet_ths / network_ehs) × 10^-6 × 144 × block_reward × (1 − pool_fee)
revenue       = btc_per_day × btc_price
power_cost    = (power_w / 1000) × 24 × tariff
profit        = revenue − power_cost
margin        = profit / revenue × 100
break_even    = power_cost / btc_per_day
```

**Use Fleet Data** fills hashrate and power from the current inventory and, when live network data is available, also fills BTC price and network hashrate. Calculator inputs are stored locally and restored on the next launch.

The result is an estimate. It does not model future difficulty changes, hardware failures, uptime losses, or pool luck.

#### Bitcoin network

The Tools screen loads data from mempool.space:

- BTC/USD price;
- network hashrate;
- network difficulty;
- current block height;
- current block reward;
- estimated time and date of the next halving.

A live/offline indicator and update timestamp show whether the last request succeeded. If the API is unreachable, the app keeps a fallback dataset so the calculator and screen remain usable. **Refresh Data** requests a new snapshot.

### Settings

Settings control scan behavior, credentials, and monitoring:

| Setting | Purpose |
|---|---|
| Port 4028 scan timeout | Per-address API probe timeout (clamped to 150–5000 ms) |
| HTTP CGI timeout | Timeout for web-interface requests |
| Concurrent scan threads | Semaphore limit for parallel probes (clamped to 8–512) |
| Monitor auto-refresh interval | How often monitoring repeats (minimum 3 seconds) |
| Antminer credentials | Username and password for HTTP Digest/CGI |
| WhatsMiner credentials | Username and password for vendor web/API access |
| Avalon credentials | Username and password for HTTP Digest/CGI |

Settings are saved to a TOML configuration file under `%APPDATA%\ASICPulse\`. Theme, language, demo/live mode, table preferences, address ranges, and credentials are all persisted there (theme and language also use WebView local storage).

### Event log and diagnostics

The Event Log screen provides two views:

- **Event history:** the latest 300 operations and warnings, newest first, with title, details, and timestamp. A badge on the navigation rail shows the current anomaly count.
- **Raw log:** the contents of the rotating operation log, with a refresh button and a shortcut to open the log folder.

Logged events include scans, cancellations, settings changes, pool configuration, power/frequency changes, reboots, LED locate, firmware runs, CSV/HTML exports, and errors. The log file rotates at 2 MB and keeps five archives.

### Reports and local data

| Data | Location |
|---|---|
| Configuration | `%APPDATA%\ASICPulse\config.toml` |
| Operation log | `%APPDATA%\ASICPulse\logs\asic_pulse.log` |
| Log archives | `%APPDATA%\ASICPulse\logs\` |
| CSV and HTML exports | `%APPDATA%\ASICPulse\exports\` |
| Calculator inputs | WebView local storage for the app |

**CSV export** writes the current fleet snapshot with IP, status, model, working mode, real-time and average hashrate, maximum temperature, fan speed, uptime, and all three pool/worker pairs. The exports folder opens automatically after the file is written.

**HTML report** creates a standalone dark-themed page with the report date and a table of IP, status, model, hashrate, temperature, pool 1, and worker 1. It also opens automatically in the default browser.

ASIC Pulse does not use a database. Fleet data and charts exist for the current session; exports are snapshots, not history.

### Supported equipment and protocols

| Device family | Discovery | Management coverage |
|---|---|---|
| **Antminer** (including legacy S7/S9/T9 and newer S19/S21/T21 families) | Port 4028 CGMiner API and HTTP Digest CGI fallback | Pool/worker configuration, power modes, frequency/voltage, reboot, LED locate, firmware endpoint |
| **WhatsMiner** M-series | Port 4028 CGMiner-compatible API | Discovery and telemetry; frequency through `set.miner.freq`; other operations depend on the exposed vendor interface |
| **Avalon** A6/A7/A8 | Port 4028; CGI where supported | Discovery and telemetry; frequency through `ascset`; reboot through the port 4028 `restart` fallback; other operations depend on firmware |
| **Other CGMiner-compatible ASIC** | Port 4028 `summary`/`pools`/`stats`/`version` | Discovery and monitoring; frequency may work through the `ascset` fallback |

Vendor APIs differ by model and firmware. Treat this matrix as a guide, not a guarantee. The app reports acknowledged operations, but it cannot make a device support an endpoint it does not expose.

### Demo mode

Demo mode is enabled on first launch. It generates a simulated mixed fleet, including normal, overheating, and low-hashrate miners, and runs an eight-step scan animation. Batch operations are acknowledged without touching the network, which makes the app safe to explore, demonstrate, or use for training.

Use the **Demo Rig / Live Network** control in the top bar to switch modes. Switching to live mode immediately starts a scan of the configured ranges. The browser preview of the interface also uses a demo fallback when native Tauri commands are unavailable.

### Interface and personalization

- **Four themes:** Lavender, Blue, Mint, and Peach. The selected theme applies to charts, cards, tables, dialogs, and status colors.
- **Two languages:** English and Russian, switchable at runtime. The choice is persisted and the interface, charts, and tools are re-rendered immediately.
- **Material You-inspired layout:** navigation rail, rounded surfaces, tonal cards, pill controls, and a floating selection bar.
- **Desktop-first layout:** the window opens at 1440×880 and is constrained to a minimum of 1080×680.
- **Inline feedback:** snackbar messages confirm scans, copies, saves, exports, and batch results; the event log keeps the durable record.

### Privacy and security

- **No account and no cloud.** ASIC Pulse does not require a login, does not upload fleet data, and does not include analytics or telemetry.
- **Limited outbound requests.** The Tools screen requests public Bitcoin statistics from mempool.space. The interface also loads web fonts and icon fonts from Google Fonts. Neither request includes miner data, but both disclose the machine's IP address to the respective service. Block them at the firewall if that is not acceptable; the app remains usable with fallback fonts and offline Bitcoin data.
- **Local credentials.** Miner usernames and passwords are stored in the local TOML configuration in plain text. Protect the Windows account and the machine; use a dedicated operator account where possible.
- **Local HTTP.** Miner CGI endpoints are contacted over plain HTTP on the local network. There is no TLS or certificate validation. Run the app on a trusted, segmented network.
- **No application-level access control.** Anyone with access to the desktop session can operate the fleet.
- **Destructive operations are possible.** Power modes, frequency/voltage changes, reboots, and firmware operations can interrupt or damage hardware. Verify vendor limits, test on one miner, and keep a recovery plan.
- **Unsigned build.** The executable is not code-signed, so Windows may show a SmartScreen warning. Compare its SHA-256 hash with the value published on the release page.
- **Logs contain operational data.** IP addresses, pool URLs, and worker names may appear in logs and exports. Redact them before sharing.

### Requirements

- Windows x64.
- Microsoft Edge WebView2 Runtime. It is included with current Windows releases; if it is missing, install the Evergreen runtime from Microsoft.
- A network route to the miners, with TCP port 4028 and the vendor HTTP interface reachable.
- Internet access is optional: it enables live mempool.space statistics and Google Fonts. Without it, the app uses fallback fonts and offline network data on an isolated LAN.
- No administrator privileges are required for normal operation.

### Getting started

1. Download the executable from the [Releases](https://github.com/etowhite/asic-pulse/releases) page.
2. Run it. Because the build is unsigned, confirm the SHA-256 hash if Windows shows a warning.
3. Explore the demo fleet first; no miners are contacted in demo mode.
4. Switch the top-bar control from **Demo Rig** to **Live Network**.
5. Open **Settings** and enter the credentials used by your Antminer, WhatsMiner, and Avalon devices.
6. Return to **Scanner**, use **Auto-Import Subnets**, add a range manually, or import a CSV/TXT list.
7. Start a scan, review the discovered fleet, and select the miners you want to manage.
8. Use the floating action bar for pools, power, LED locate, or reboot. Use the scanner toolbar for exports and fleet-wide actions.
9. Open **Tools** to estimate profitability with live network data.

### Build from source

Prerequisites:

- Rust toolchain with the MSVC target;
- Visual Studio C++ Build Tools (or equivalent MSVC linker);
- WebView2 runtime;
- Git.

```powershell
git clone https://github.com/etowhite/asic-pulse.git
cd asic-pulse
cd src-tauri
cargo build --release
```

The executable is written to `src-tauri\target\release\asic-pulse-m3.exe`.

For a quick frontend preview without the native shell:

```powershell
cd src
python -m http.server 8765
```

Open `http://127.0.0.1:8765/`. The browser build uses demo data and browser fallbacks; native scanning, file exports, and miner operations require the Tauri application.

### Project layout

```text
.
├─ src/                     # WebView frontend
│  ├─ index.html            # Screens, dialogs, and controls
│  ├─ app.js                # UI logic, charts, calculator, exports
│  ├─ i18n.js               # English and Russian strings
│  └─ styles.css            # Material You-inspired themes
├─ src-tauri/
│  ├─ src/lib.rs            # Tauri commands, discovery, protocols, storage
│  ├─ src/main.rs           # Desktop entry point
│  ├─ capabilities/         # Tauri permissions
│  ├─ icons/                # Application icon
│  ├─ Cargo.toml            # Rust dependencies and build profile
│  └─ tauri.conf.json       # Window and bundle configuration
└─ README.md
```

### Troubleshooting

| Symptom | What to check |
|---|---|
| No miners found | Is the range enabled? Is the subnet correct? Is TCP 4028 reachable? Is the scan timeout too low? Are vendor credentials correct? |
| Web UI opens but the app cannot authenticate | Verify the per-vendor username/password in Settings. Some firmware uses different CGI credentials or disables the API. |
| Frequency or voltage is ignored | Stock firmware may not accept manual values. Try a supported power preset, or use custom firmware/WhatsMiner/Avalon paths that expose the control. |
| Only demo miners appear | The top bar is still in **Demo Rig** mode. Switch to **Live Network**. |
| Live network statistics are offline | Check internet access, DNS, and access to mempool.space. The calculator still works with fallback data. |
| Exports are hard to find | Open `%APPDATA%\ASICPulse\exports\`; the folder also opens automatically after an export. |
| The app does not start | Install or repair the WebView2 runtime, then relaunch. |
| Windows warns about the executable | The build is unsigned. Verify the published SHA-256 hash before running it. |
| LED locate or reboot has no effect | The vendor firmware may not expose the corresponding endpoint. Check the event log for the acknowledgement count. |
| The table is empty after a scan | Check the status filter, search text, and the **Only found miners** toggle. |

### Scope and limitations

- IPv4 only; each range is swept inside one /24.
- Scanning is on demand. There is no background service, scheduler, or monitoring while the app is closed.
- Charts and telemetry history are session-scoped; there is no historical database.
- Alerts are shown in the app and event log only. There are no email, webhook, Telegram, or push integrations.
- Single-user desktop model; no roles, remote access, or multi-operator audit trail.
- Vendor coverage varies by model and firmware. Some management operations are best-effort.
- Firmware images are not bundled, downloaded, or signature-verified by the app.
- The firmware workflow calls the vendor upgrade endpoint; it is not a universal flasher and does not provide automatic rollback.
- No pool payout accounting, wallet integration, or tax reporting.
- No IPv6, mDNS auto-discovery, Linux, or macOS build.
- Local HTTP only; do not expose miner management interfaces to untrusted networks.
- Temperature and low-hashrate thresholds are presentation rules, not configurable alarm policies.

### Feedback

Use the [issue tracker](https://github.com/etowhite/asic-pulse/issues) for bug reports and feature requests. Include the miner model, firmware string, network layout, steps to reproduce, and relevant log lines. Remove credentials, pool passwords, wallet addresses, and other sensitive data before posting.

ASIC Pulse is an independent desktop project. The source code is published in this repository.

---

<a id="russian"></a>
## Русский

### Что делает ASIC Pulse

ASIC Pulse — автономное приложение для Windows, предназначенное для поиска, мониторинга и управления ASIC-майнерами в локальной сети. Оно объединяет сканирование подсетей, телеметрию парка в реальном времени, групповую настройку пулов и питания, диагностику хеш-плат, работу с прошивками и расчёт доходности Bitcoin в одном рабочем пространстве — без облачного аккаунта, внешнего дашборда и агента, устанавливаемого на майнеры.

Приложение создано для тех, кто обслуживает майнеры в собственной сети. Укажите один или несколько диапазонов IPv4 — ASIC Pulse проверит адреса, определит совместимые устройства и построит актуальный список парка. Из этого списка можно открыть карточку отдельного майнера, сравнить мгновенный и средний хешрейт, массово сменить пулы и имена воркеров, переключить профиль питания, настроить частоту и напряжение, перезагрузить или подсветить выбранные устройства и запустить работу с прошивкой — всё в одном окне.

Приложение принципиально локальное. Оно обращается к майнерам напрямую по TCP-порту 4028 и через их HTTP CGI-интерфейсы. Здесь нет аккаунта, облачного посредника, внешнего агента мониторинга и отправки телеметрии. Единственный необязательный запрос, связанный с парком, — публичная статистика сети Bitcoin с mempool.space. Кроме того, интерфейс загружает шрифты и иконки с Google Fonts; если эти запросы заблокированы, используются локальные шрифты.

ASIC Pulse рассчитан на смешанные парки. Antminer, WhatsMiner и Avalon могут находиться в одной таблице: приложение приводит данные разных устройств к общему формату, сохраняя доступ к управлению, которое поддерживает конкретная прошивка.

### Кратко

| Область | Что получает пользователь |
|---|---|
| Назначение | Локальный поиск, мониторинг и групповое управление ASIC-майнерами |
| Сетевая модель | Прямой доступ по локальной сети; без облака, аккаунта и агента на майнере |
| Обнаружение | Диапазоны IPv4, автоопределение подсетей, импорт CSV/TXT |
| Протоколы | CGMiner/BMMiner API на TCP 4028; HTTP Digest CGI |
| Семейства | Antminer, WhatsMiner, Avalon и другие CGMiner-совместимые устройства |
| Групповые операции | Пулы и воркеры, сложность пула, режимы питания, частота/напряжение, перезагрузка, LED-поиск, работа с прошивкой |
| Телеметрия | Хешрейт, температуры, вентиляторы, время работы, пулы, воркеры, ошибки оборудования, доля отклонённых шар, платы и чипы |
| Аналитика | Калькулятор доходности и статистика сети Bitcoin с mempool.space |
| Результаты | Экспорт CSV, автономный HTML-отчёт, ротация журнала операций |
| Интерфейс | Английский и русский языки, четыре цветовые темы, демонстрационный парк |
| Платформа | Windows x64, WebView2, Tauri и Rust |

### Принципы

- **Локальность прежде всего.** Майнеры опрашиваются напрямую. Конфигурация, журналы и отчёты остаются на компьютере.
- **Парк, а не одно устройство.** Выделение, групповые диалоги и действия «применить ко всем» встроены в основной сценарий.
- **Понятный сигнал вместо сырого API.** Ответы обнаружения превращаются в читаемый список со статусами, телеметрией и действиями.
- **Явные цели операций.** Выделение всегда видно, а каждый групповой диалог показывает, применяется действие к выбранным майнерам или ко всему парку.
- **Два языка и настраиваемый вид.** Рабочее пространство переключается между английским и русским и между четырьмя темами без перезапуска.

### Рабочее пространство

| Экран | Назначение |
|---|---|
| **Сканер** | Диапазоны адресов, поиск майнеров, таблица парка и запуск групповых действий |
| **Дашборд** | KPI парка, история хешрейта за сессию, аномалии и карточки майнеров |
| **Инструменты** | Оценка доходности майнинга и состояние сети Bitcoin |
| **Настройки** | Параметры сканирования, таймауты, параллельность, мониторинг и доступы к майнерам |
| **Журнал событий** | История операций, предупреждения и сырой файл журнала |

Постоянная верхняя панель содержит выбор темы, переключение языка, режим «демо/реальная сеть», мониторинг и кнопки запуска/отмены сканирования. При выделении майнеров появляется плавающая панель с основными групповыми действиями.

При запуске ASIC Pulse сразу выполняет сканирование с сохранёнными диапазонами и текущими настройками. Первый запуск происходит в демо-режиме, поэтому интерфейс можно изучить без оборудования.

### Сканер

#### Диапазоны адресов

Сканер работает с IPv4. У каждого диапазона есть комментарий/расположение, состояние включения и кнопка удаления; все диапазоны сохраняются между запусками.

- **Автоопределение подсетей** читает сетевую конфигурацию Windows и добавляет по одной локальной подсети для каждого адаптера, исключая loopback и link-local адреса.
- **Ручные диапазоны** — одиночный адрес (`192.168.1.101`) или перебор последнего октета в пределах одной подсети (`192.168.1.1-254`, `192.168.1.10-192.168.1.200`).
- **Импорт CSV/TXT** принимает по одной строке в формате `диапазон,комментарий`. Файл сначала загружается в редактор импорта, поэтому его можно проверить или исправить до применения.
- **Включение и отключение** позволяют хранить большой план адресов, проверяя только нужные сегменты.

Диапазон всегда перебирается в пределах одной сети /24. IPv6 не поддерживается.

#### Обнаружение

Сканирование выполняется параллельно и ограничено семафором:

- **Параллельность:** от 8 до 512 одновременных проверок, настраивается в «Настройках».
- **Таймаут порта 4028:** от 150 до 5000 мс на проверку.
- **Таймаут HTTP CGI:** настраивается отдельно для резервного опроса веб-интерфейса.
- **Живой прогресс:** сканер показывает, сколько адресов проверено и сколько ASIC найдено.
- **Отмена:** начатое сканирование можно остановить, не закрывая приложение.

Для каждого адреса ASIC Pulse сначала отправляет в CGMiner/BMMiner API на порту 4028 объединённый запрос `summary+pools+stats+version`, затем при необходимости повторяет его с `summary`. Если API не отвечает, используется HTTP Digest CGI — веб-интерфейс, который есть у многих панелей Antminer.

В итоговый список могут попасть модель, мгновенный и средний хешрейт, температуры, скорость вентиляторов, время работы, активные пулы и воркеры, ошибки оборудования, доля отклонённых шар, строка прошивки, MAC-адрес, напряжение, частота, потребление и телеметрия по платам и чипам — если устройство её отдаёт.

Два переключателя помогают работать с большими парками:

- **Только найденные майнеры** скрывает офлайн-строки, если не выбран фильтр «Офлайн».
- **Показывать пулы 2/3** добавляет столбцы второго и третьего пула/воркера.

#### Таблица парка

Таблица сканера — операционный центр приложения. В ней отображаются:

- флажок выбора, IP-адрес, статус, модель и режим работы;
- мгновенный и средний хешрейт;
- максимальная температура, скорость вентиляторов и время работы;
- пул 1 и воркер 1, а также пулы и воркеры 2/3 при включённых дополнительных столбцах.

Таблица поддерживает:

- **Фильтры статуса:** «Все», «В норме», «Аномалии», «Офлайн».
- **Поиск** по IP, модели, воркеру, пулу и режиму работы.
- **Сортировку** по IP (в числовом порядке), статусу, модели, режиму, хешрейту, температуре, вентиляторам, времени работы и пулам.
- **Выделение** кликом или флажком, выбор всех видимых строк и снятие выделения.
- **Двойной клик** по строке открывает подробную карточку майнера.

Статусы: **В сети**, **Перегрев**, **Низкий хешрейт** и **Офлайн**. Высокая температура и низкий хешрейт подсвечиваются в таблице и на дашборде. Полоса KPI над таблицей показывает суммарный мгновенный/средний хешрейт, количество майнеров онлайн/всего, среднюю и максимальную температуру и число активных аномалий.

### Дашборд

Дашборд предназначен для быстрой оценки здоровья парка:

- **KPI-карточки** повторяют ключевые показатели: суммарный хешрейт, майнеры онлайн, среднюю температуру и аномалии.
- **График сессии** строит последние 30 замеров суммарного хешрейта, собранных во время сканирований и тиков мониторинга.
- **Панель аномалий** перечисляет всех майнеров не в нормальном состоянии; запись открывает подробную карточку.
- **Карточки парка** показывают IP, модель, температуру, мгновенный хешрейт и режим работы. Их можно фильтровать по всем, Antminer, WhatsMiner или Avalon.
- **HTML-отчёт** создаёт автономный снимок текущего парка и открывает его в браузере по умолчанию.

Графики существуют в рамках сессии. ASIC Pulse не ведёт базу исторических временных рядов.

### Карточка майнера

Двойной клик по строке или карточке дашборда открывает подробный просмотр:

- **Идентификация:** IP, модель, MAC-адрес, строка прошивки, режим работы и потребление.
- **Живые KPI:** мгновенный хешрейт, максимальная температура чипов, обороты вентиляторов, ошибки оборудования и доля отклонённых шар.
- **Таблица хеш-плат:** номер платы, число чипов, частота, напряжение, температура платы, температура чипов, хешрейт платы и ошибки оборудования.
- **История температуры:** небольшой график последних замеров конкретного майнера.
- **Действия:** открыть веб-интерфейс майнера, пропинговать устройство, скопировать IP или имя воркера, перейти к настройке пулов или перезагрузить майнер.

Полнота телеметрии зависит от прошивки. Одни устройства отдают данные по платам и чипам, другие — только сводные значения. Карточка показывает то, что доступно, и сообщает, если детальных данных по платам нет.

### Групповые операции

Групповые действия доступны на плавающей панели выделения и на панели инструментов сканера. Диалоги, поддерживающие оба режима, предлагают **«Применить к выбранным»** и **«Применить ко всем»**.

#### Пулы и воркеры

Диалог настройки пулов задаёт до трёх Stratum-пулов:

- URL и порт для каждого из трёх пулов;
- общий префикс воркера (субаккаунт) и пароль пула;
- необязательную сложность пула (`diff`) для каждого непустого пула; пустое значение или ноль означают автоматическую сложность;
- режим суффикса воркера.

| Режим суффикса | Результат для префикса `north_farm` и майнера `192.168.1.101` |
|---|---|
| По IP | `north_farm.1x101` |
| Без суффикса | `north_farm` |
| Пустой воркер | пустое имя воркера |
| Свой суффикс | `north_farm.rack01` |

Изменения пулов отправляются через HTTP Digest CGI-интерфейс, совместимый с Antminer: сначала современным JSON-запросом, затем — устаревшим form-urlencoded запросом для старой прошивки. Приложение сообщает, сколько устройств подтвердили настройку, и записывает результат в журнал событий. Другим производителям может понадобиться собственный веб-интерфейс или прошивка, принимающая тот же API.

#### Питание и частоты

Диалог питания объединяет профиль работы и необязательную ручную настройку частоты и напряжения.

| Режим | Назначение |
|---|---|
| Normal | Штатный профиль производителя |
| LPM | Режим пониженного энергопотребления |
| Enhanced LPM | Более глубокое энергосбережение |
| Overclock | Профиль повышенного хешрейта |
| Underclock | Пониженная частота для эффективности |
| Sleep | Малоактивный/спящий режим |

Предусмотрены пресеты для семейств Antminer S21/T21/S19, Antminer S9/T9, WhatsMiner M50/M60 и Avalon A6/A7/A8. При смене семейства или режима предлагаемая частота пересчитывается.

Ручное управление добавляет:

- **Частоту:** от 100 до 1500 МГц.
- **Напряжение:** необязательно, в вольтах.

Если ручная частота включена, ASIC Pulse сначала применяет режим работы, а затем шаг частоты/напряжения. Для Antminer приложение читает текущую конфигурацию и записывает её обратно с новой частотой, чтобы не потерять пулы и другие настройки. Для WhatsMiner используется команда `set.miner.freq` на порту 4028. Для Avalon и других CGMiner-совместимых устройств применяется резервная команда `ascset`. Напряжение записывается через конфигурацию Antminer; другие производители могут его игнорировать.

Управление частотой и напряжением зависит от модели и прошивки. Штатная прошивка производителя может игнорировать эти поля; кастомные прошивки и поддерживаемые сборки WhatsMiner/Avalon принимают их чаще. Сверяйтесь со спецификацией производителя и меняйте настройки сначала на одном майнере.

#### Перезагрузка, LED-поиск и прошивка

- **Перезагрузка** применяется к выбранным майнерам или ко всему парку. Приложение сначала вызывает CGI-перезагрузку Antminer, а затем резервную команду `restart` на порту 4028 для устройств типа Avalon.
- **LED-поиск** просит выбранные майнеры мигнуть локализующим светодиодом, чтобы найти устройство в стойке.
- **Работа с прошивкой** принимает файл образа, применяется к выбранным майнерам (или ко всему парку, если ничего не выбрано) и показывает результат по каждому устройству. Обновление выполняется через endpoint прошивки производителя; совместимость образа, проверка и откат остаются ответственностью производителя.

Перезагрузка, смена режима питания, частоты и прошивка прерывают майнинг. Применяйте их осознанно, особенно на рабочем оборудовании.

### Инструменты

Экран «Инструменты» объединяет калькулятор доходности и актуальный контекст сети Bitcoin.

#### Калькулятор доходности

Входные данные:

- хешрейт парка в TH/s;
- потребление в ваттах;
- тариф на электроэнергию в USD за кВт·ч;
- комиссия пула в процентах;
- курс BTC в USD;
- хешрейт сети в EH/s.

Результаты:

- добыча BTC в день;
- доход в день;
- стоимость электроэнергии в день;
- прибыль в день;
- прибыль в месяц (30 дней);
- маржа;
- безубыточный курс BTC.

Расчёт использует 144 блока в день, награду за блок, зависящую от текущей высоты (с учётом халвингов), и введённую комиссию пула:

```text
btc_per_day   = (fleet_ths / network_ehs) × 10^-6 × 144 × block_reward × (1 − pool_fee)
revenue       = btc_per_day × btc_price
power_cost    = (power_w / 1000) × 24 × tariff
profit        = revenue − power_cost
margin        = profit / revenue × 100
break_even    = power_cost / btc_per_day
```

**«Взять данные парка»** подставляет хешрейт и потребление из текущего списка, а при наличии живых данных сети — также курс BTC и хешрейт сети. Введённые значения сохраняются локально и восстанавливаются при следующем запуске.

Результат ориентировочный. Он не учитывает будущие изменения сложности, отказы оборудования, простои и удачу пула.

#### Сеть Bitcoin

Экран загружает данные с mempool.space:

- курс BTC/USD;
- хешрейт сети;
- сложность сети;
- текущую высоту блока;
- текущую награду за блок;
- примерное время и дату следующего халвинга.

Индикатор «онлайн/офлайн» и время обновления показывают, удался ли последний запрос. Если API недоступен, приложение сохраняет резервный набор данных, чтобы калькулятор и экран оставались работоспособными. Кнопка **«Обновить данные»** запрашивает новый снимок.

### Настройки

Настройки управляют сканированием, доступами и мониторингом:

| Параметр | Назначение |
|---|---|
| Таймаут сканирования порта 4028 | Таймаут проверки одного адреса (ограничен 150–5000 мс) |
| Таймаут HTTP CGI | Таймаут запросов к веб-интерфейсу |
| Число параллельных проверок | Ограничение семафора для одновременных проверок (8–512) |
| Интервал автообновления мониторинга | Как часто повторяется мониторинг (минимум 3 секунды) |
| Доступы Antminer | Логин и пароль для HTTP Digest/CGI |
| Доступы WhatsMiner | Логин и пароль для веб-интерфейса/API производителя |
| Доступы Avalon | Логин и пароль для HTTP Digest/CGI |

Настройки сохраняются в TOML-файл в `%APPDATA%\ASICPulse\`. Тема, язык, режим «демо/реальная сеть», параметры таблицы, диапазоны адресов и доступы хранятся там же (тема и язык дополнительно используют локальное хранилище WebView).

### Журнал событий

Экран журнала предлагает два представления:

- **История событий:** последние 300 операций и предупреждений, сначала новые, с заголовком, подробностями и временем. Значок на боковой панели показывает текущее число аномалий.
- **Сырой журнал:** содержимое файла операций с кнопкой обновления и переходом в папку журналов.

В журнал попадают сканирования, отмены, изменения настроек, настройка пулов, смена питания/частоты, перезагрузки, LED-поиск, работа с прошивкой, экспорт CSV/HTML и ошибки. Файл журнала ротируется при достижении 2 МБ и хранит пять архивов.

### Отчеты и локальные данные

| Данные | Расположение |
|---|---|
| Конфигурация | `%APPDATA%\ASICPulse\config.toml` |
| Журнал операций | `%APPDATA%\ASICPulse\logs\asic_pulse.log` |
| Архивы журнала | `%APPDATA%\ASICPulse\logs\` |
| Экспорт CSV и HTML | `%APPDATA%\ASICPulse\exports\` |
| Данные калькулятора | Локальное хранилище WebView приложения |

**Экспорт CSV** записывает текущий снимок парка: IP, статус, модель, режим работы, мгновенный и средний хешрейт, максимальную температуру, скорость вентиляторов, время работы и все три пары «пул/воркер». После записи папка экспорта открывается автоматически.

**HTML-отчёт** создаёт автономную страницу в тёмном оформлении с датой отчёта и таблицей IP, статуса, модели, хешрейта, температуры, пула 1 и воркера 1. Она также автоматически открывается в браузере по умолчанию.

ASIC Pulse не использует базу данных. Данные парка и графики существуют в текущей сессии; экспорт — это снимок, а не история.

### Поддерживаемое оборудование и протоколы

| Семейство | Обнаружение | Покрытие управления |
|---|---|---|
| **Antminer** (включая старые S7/S9/T9 и новые S19/S21/T21) | CGMiner API на порту 4028 и резервный HTTP Digest CGI | Настройка пулов/воркеров, режимы питания, частота/напряжение, перезагрузка, LED-поиск, endpoint прошивки |
| **WhatsMiner** серии M | CGMiner-совместимый API на порту 4028 | Обнаружение и телеметрия; частота через `set.miner.freq`; остальные операции зависят от веб-интерфейса производителя |
| **Avalon** A6/A7/A8 | Порт 4028; CGI там, где поддерживается | Обнаружение и телеметрия; частота через `ascset`; перезагрузка через резервную команду `restart` на порту 4028; остальное зависит от прошивки |
| **Другие CGMiner-совместимые ASIC** | `summary`/`pools`/`stats`/`version` на порту 4028 | Обнаружение и мониторинг; частота может работать через резервную команду `ascset` |

API производителей различаются в зависимости от модели и прошивки. Воспринимайте таблицу как ориентир, а не гарантию. Приложение сообщает о подтверждённых операциях, но не может заставить устройство поддерживать endpoint, которого у него нет.

### Демо-режим

Демо-режим включён при первом запуске. Он создаёт имитацию смешанного парка, включая нормальные, перегретые майнеры и устройства с низким хешрейтом, и запускает восьмишаговую анимацию сканирования. Групповые операции подтверждаются без обращения к сети, поэтому приложение безопасно изучать, демонстрировать или использовать для обучения.

Переключатель **«Демо-парк / Реальная сеть»** находится в верхней панели. Переход в реальный режим сразу запускает сканирование настроенных диапазонов. Браузерный просмотр интерфейса также использует демонстрационный резерв, когда нативные команды Tauri недоступны.

### Интерфейс

- **Четыре темы:** Lavender, Blue, Mint и Peach. Выбранная тема применяется к графикам, карточкам, таблицам, диалогам и цветам статусов.
- **Два языка:** английский и русский с переключением на ходу. Выбор сохраняется, интерфейс, графики и инструменты обновляются сразу.
- **Оформление в духе Material You:** боковая навигация, скруглённые поверхности, тональные карточки, pill-кнопки и плавающая панель выделения.
- **Настольная компоновка:** окно открывается размером 1440×880 и ограничено минимальным размером 1080×680.
- **Обратная связь на месте:** snackbar подтверждает сканирования, копирование, сохранения, экспорт и результаты групповых операций; журнал событий хранит постоянную запись.

### Приватность и безопасность

- **Без аккаунта и облака.** ASIC Pulse не требует входа, не выгружает данные парка и не содержит аналитики или телеметрии.
- **Ограниченные внешние запросы.** Экран «Инструменты» запрашивает публичную статистику Bitcoin с mempool.space. Интерфейс также загружает шрифты и иконки с Google Fonts. Данные майнеров не передаются, но оба запроса раскрывают IP-адрес машины соответствующему сервису. При необходимости заблокируйте их на межсетевом экране; приложение продолжит работать с резервными шрифтами и офлайн-данными Bitcoin.
- **Локальные доступы.** Логины и пароли майнеров хранятся в локальной TOML-конфигурации в открытом виде. Защитите учётную запись Windows и компьютер; по возможности используйте отдельную учётную запись оператора.
- **Локальный HTTP.** CGI-интерфейсы майнеров опрашиваются по обычному HTTP в локальной сети. Здесь нет TLS и проверки сертификатов. Запускайте приложение в доверенной сегментированной сети.
- **Нет управления доступом на уровне приложения.** Любой, у кого есть доступ к сеансу рабочего стола, может управлять парком.
- **Возможны разрушительные операции.** Режимы питания, изменение частоты/напряжения, перезагрузка и прошивка могут прервать работу или повредить оборудование. Проверяйте ограничения производителя, тестируйте на одном майнере и держите план восстановления.
- **Сборка без подписи.** Исполняемый файл не подписан кодом, поэтому Windows может показать предупреждение SmartScreen. Сравните SHA-256 со значением, опубликованным на странице релиза.
- **Журналы содержат рабочие данные.** В журналах и экспорте могут встречаться IP-адреса, URL пулов и имена воркеров. Удаляйте их перед публикацией.

### Требования

- Windows x64.
- Среда выполнения Microsoft Edge WebView2. Она входит в актуальные выпуски Windows; если её нет, установите Evergreen Runtime от Microsoft.
- Сетевой маршрут до майнеров, доступность TCP-порта 4028 и HTTP-интерфейса производителя.
- Интернет не обязателен: он включает живую статистику mempool.space и загрузку шрифтов Google. Без него приложение использует резервные шрифты и офлайн-данные сети в изолированной локальной сети.
- Для обычной работы права администратора не требуются.

### Первый запуск

1. Скачайте исполняемый файл со страницы [Releases](https://github.com/etowhite/asic-pulse/releases).
2. Запустите его. Сборка не подписана, поэтому при предупреждении Windows сверьте хеш SHA-256.
3. Сначала изучите демонстрационный парк: в демо-режиме майнеры не опрашиваются.
4. Переключите верхнюю кнопку с **«Демо-парк»** на **«Реальная сеть»**.
5. Откройте **«Настройки»** и укажите доступы, используемые вашими Antminer, WhatsMiner и Avalon.
6. Вернитесь в **«Сканер»**, нажмите **«Автоимпорт подсетей»**, добавьте диапазон вручную или импортируйте список CSV/TXT.
7. Запустите сканирование, проверьте найденный парк и выделите майнеры для управления.
8. Используйте плавающую панель для пулов, питания, LED-поиска или перезагрузки. Панель инструментов сканера предназначена для экспорта и действий над всем парком.
9. Откройте **«Инструменты»**, чтобы оценить доходность с живыми данными сети.

### Сборка из исходников

Требования:

- инструментарий Rust с целью MSVC;
- Visual Studio C++ Build Tools (или совместимый компоновщик MSVC);
- среда WebView2;
- Git.

```powershell
git clone https://github.com/etowhite/asic-pulse.git
cd asic-pulse
cd src-tauri
cargo build --release
```

Исполняемый файл появится в `src-tauri\target\release\asic-pulse-m3.exe`.

Для быстрого просмотра интерфейса без нативной оболочки:

```powershell
cd src
python -m http.server 8765
```

Откройте `http://127.0.0.1:8765/`. Браузерный просмотр использует демо-данные и резервные обработчики; нативное сканирование, экспорт файлов и операции с майнерами требуют приложения Tauri.

### Структура проекта

```text
.
├─ src/                     # Frontend в WebView
│  ├─ index.html            # Экраны, диалоги и элементы управления
│  ├─ app.js                # Логика интерфейса, графики, калькулятор, экспорт
│  ├─ i18n.js               # Строки на английском и русском
│  └─ styles.css            # Темы в духе Material You
├─ src-tauri/
│  ├─ src/lib.rs            # Команды Tauri, обнаружение, протоколы, хранение
│  ├─ src/main.rs           # Точка входа приложения
│  ├─ capabilities/         # Разрешения Tauri
│  ├─ icons/                # Иконка приложения
│  ├─ Cargo.toml            # Зависимости Rust и профиль сборки
│  └─ tauri.conf.json       # Конфигурация окна и сборки
└─ README.md
```

### Диагностика

| Симптом | Что проверить |
|---|---|
| Майнеры не найдены | Включён ли диапазон? Верна ли подсеть? Доступен ли TCP 4028? Не слишком ли мал таймаут? Верны ли доступы? |
| Веб-интерфейс открывается, но приложение не авторизуется | Проверьте логин/пароль производителя в «Настройках». Некоторые прошивки используют другие данные CGI или отключают API. |
| Частота или напряжение игнорируются | Штатная прошивка может не принимать ручные значения. Попробуйте поддерживаемый профиль питания или кастомную прошивку/путь WhatsMiner/Avalon, где управление доступно. |
| Показываются только демо-майнеры | Верхняя панель всё ещё в режиме **«Демо-парк»**. Переключитесь на **«Реальная сеть»**. |
| Статистика сети недоступна | Проверьте интернет, DNS и доступ к mempool.space. Калькулятор продолжит работать на резервных данных. |
| Не удаётся найти экспорт | Откройте `%APPDATA%\ASICPulse\exports\`; папка также открывается автоматически после экспорта. |
| Приложение не запускается | Установите или восстановите WebView2 и запустите снова. |
| Windows предупреждает о файле | Сборка не подписана. Перед запуском сверьте опубликованный SHA-256. |
| LED-поиск или перезагрузка не работают | Прошивка производителя может не поддерживать нужный endpoint. Проверьте в журнале число подтверждений. |
| После сканирования таблица пуста | Проверьте фильтр статуса, строку поиска и переключатель **«Только найденные майнеры»**. |

### Ограничения

- Только IPv4; каждый диапазон перебирается в пределах одной сети /24.
- Сканирование выполняется по запросу. Нет фоновой службы, планировщика и мониторинга при закрытом приложении.
- Графики и история телеметрии существуют в рамках сессии; исторической базы данных нет.
- Оповещения показываются только в приложении и журнале. Нет интеграций с email, webhook, Telegram или push.
- Однопользовательская настольная модель; нет ролей, удалённого доступа и общего аудита действий.
- Покрытие производителей зависит от модели и прошивки. Часть операций управления выполняется по мере поддержки.
- Образы прошивок не поставляются, не скачиваются и не проверяются по подписи приложением.
- Работа с прошивкой вызывает endpoint производителя; это не универсальный прошивальщик и не автоматический откат.
- Нет учёта выплат пула, интеграции с кошельком и налоговой отчётности.
- Нет IPv6, mDNS-автообнаружения, сборок для Linux и macOS.
- Только локальный HTTP; не выставляйте интерфейсы управления майнерами в недоверенные сети.
- Пороги температуры и низкого хешрейта — правила отображения, а не настраиваемая политика оповещений.

### Обратная связь

Для сообщений об ошибках и предложений используйте [issue tracker](https://github.com/etowhite/asic-pulse/issues). Укажите модель майнера, строку прошивки, схему сети, шаги воспроизведения и относящиеся к делу строки журнала. Перед публикацией удалите логины, пароли пулов, адреса кошельков и другие чувствительные данные.

ASIC Pulse — самостоятельный настольный проект. Исходный код опубликован в этом репозитории.
