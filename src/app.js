let appSettings = {
  language: "ru",
  theme: "lavender",
  scan_timeout_ms: 450,
  http_timeout_ms: 2500,
  concurrency: 128,
  monitor_interval_sec: 15,
  only_success_miners: true,
  show_extra_pools: false,
  demo_mode: true,
  antminer_user: "root",
  antminer_pass: "root",
  whatsminer_user: "admin",
  whatsminer_pass: "admin",
  avalon_user: "root",
  avalon_pass: "root",
  ip_ranges: [
    { id: "r1", range: "192.168.1.1-254", comment: "LAN Segment 1", enabled: true }
  ],
  column_widths: {}
};

let miners = [];
let selectedIps = new Set();
let statusFilter = "all";
let dashModelFilter = "all";
let sortField = "ip";
let sortAsc = true;
let monitorTimer = null;
let progressPollTimer = null;
let sessionHashrateHistory = [];
let minerTelemetryHistory = {};
let activeDetailMinerIp = null;
let editingRangeId = null;

let btcNetwork = {
  price: 100000,
  hashrateHs: 1e21,
  difficulty: 1.2e14,
  blockHeight: 920000,
  updatedAt: null,
  live: false
};
let statsLoading = false;

async function invokeBackend(cmd, args = {}) {
  if (window.__TAURI__ && window.__TAURI__.core) {
    return await window.__TAURI__.core.invoke(cmd, args);
  }
  return await browserFallbackMock(cmd, args);
}

async function browserFallbackMock(cmd, args) {
  await new Promise(r => setTimeout(r, 180));
  if (cmd === "get_app_settings") return appSettings;
  if (cmd === "save_app_settings") {
    appSettings = args.settings;
    return "OK";
  }
  if (cmd === "auto_detect_subnets") {
    return [
      { id: "auto_1", range: "192.168.1.1-254", comment: "Авто (192.168.1.50)", enabled: true },
      { id: "auto_2", range: "192.168.0.1-254", comment: "Авто (192.168.0.12)", enabled: false }
    ];
  }
  if (cmd === "scan_miners") {
    return getBrowserDemoMiners();
  }
  if (cmd === "get_scan_progress") {
    return { active: false, scanned: 254, total: 254, found: 6 };
  }
  if (cmd === "ping_miner") {
    return "2 ms (TCP 4028)";
  }
  if (cmd === "get_event_logs") {
    return [
      { timestamp: new Date().toLocaleTimeString(), level: "info", title: "Сканирование завершено", details: "Найдено 7 ASIC устройств" }
    ];
  }
  if (cmd === "read_log_file") {
    return "[INFO] ASIC Pulse log4rs initialized.\n[INFO] Subnet scan 192.168.1.1-254 finished.";
  }
  if (cmd === "batch_power_control") {
    return `Режим питания '${args.payload?.work_mode}' применен к ${args.payload?.ips?.length || 0} ASIC`;
  }
  if (cmd === "batch_set_frequency") {
    return `Частота ${args.payload?.freq_mhz} MHz применена к ${args.payload?.ips?.length || 0} ASIC`;
  }
  if (cmd === "batch_configure_pools") {
    return `Пулы применены к ${args.payload?.ips?.length || 0} майнерам`;
  }
  return "OK";
}

function getBrowserDemoMiners() {
  return [
    {
      ip: "192.168.1.101", status: "online", miner_type: "Antminer S21 Pro (Vnish)", working_mode: "Normal",
      hashrate_rt: 236.4, hashrate_avg: 234.8, temp_max: 66, temp_min: 57, fans: [5100, 5140, 5080, 5120],
      fan_speed_str: "5100 / 5140", elapsed_sec: 1572480, elapsed_str: "18d 04h 48m",
      pool1: "stratum+tcp://btc.viabtc.io:3333", worker1: "north_farm.1x101",
      pool2: "stratum+tcp://btc.f2pool.com:1314", worker2: "north_farm.1x101",
      pool3: "stratum+tcp://btc.foundryusa.com:3333", worker3: "north_farm.1x101",
      hw_errors: 12, rejected_pct: 0.08, firmware_ver: "Vnish 1.2.6-S21", mac_addr: "02:42:AC:11:01:65",
      voltage_v: 13.9, freq_mhz: 675, power_w: 3510, blink_led: false,
      boards: [
        { index: 0, chips: 114, freq_mhz: 675, voltage_v: 13.9, temp_board: 57, temp_chip: 66, hashrate_ths: 78.9, hw_errors: 4 },
        { index: 1, chips: 114, freq_mhz: 675, voltage_v: 13.9, temp_board: 58, temp_chip: 65, hashrate_ths: 78.8, hw_errors: 5 },
        { index: 2, chips: 114, freq_mhz: 675, voltage_v: 13.9, temp_board: 57, temp_chip: 64, hashrate_ths: 78.7, hw_errors: 3 }
      ]
    },
    {
      ip: "192.168.1.108", status: "hightemp", miner_type: "WhatsMiner M60S", working_mode: "High",
      hashrate_rt: 186.5, hashrate_avg: 185.1, temp_max: 88, temp_min: 74, fans: [6680, 6720],
      fan_speed_str: "6680 / 6720", elapsed_sec: 386400, elapsed_str: "4d 11h 20m",
      pool1: "stratum+tcp://btc.f2pool.com:1314", worker1: "north_farm.1x108",
      pool2: "", worker2: "", pool3: "", worker3: "",
      hw_errors: 84, rejected_pct: 0.34, firmware_ver: "20260118.22.REL", mac_addr: "02:42:AC:11:01:6C",
      voltage_v: 14.5, freq_mhz: 740, power_w: 3440, blink_led: true,
      boards: [
        { index: 0, chips: 132, freq_mhz: 740, voltage_v: 14.5, temp_board: 75, temp_chip: 88, hashrate_ths: 62.2, hw_errors: 41 },
        { index: 1, chips: 132, freq_mhz: 740, voltage_v: 14.5, temp_board: 74, temp_chip: 86, hashrate_ths: 62.1, hw_errors: 22 },
        { index: 2, chips: 132, freq_mhz: 740, voltage_v: 14.5, temp_board: 74, temp_chip: 85, hashrate_ths: 62.2, hw_errors: 21 }
      ]
    },
    {
      ip: "192.168.1.124", status: "lowhash", miner_type: "Antminer T21", working_mode: "Normal",
      hashrate_rt: 124.2, hashrate_avg: 188.0, temp_max: 76, temp_min: 63, fans: [5620, 5680, 5600, 5640],
      fan_speed_str: "5620 / 5680", elapsed_sec: 98400, elapsed_str: "1d 03h 20m",
      pool1: "stratum+tcp://btc.foundryusa.com:3333", worker1: "north_farm.1x124",
      pool2: "", worker2: "", pool3: "", worker3: "",
      hw_errors: 412, rejected_pct: 1.45, firmware_ver: "Bitmain-T21-Rel", mac_addr: "02:42:AC:11:01:7C",
      voltage_v: 13.5, freq_mhz: 610, power_w: 2560, blink_led: false,
      boards: []
    }
  ];
}

function switchScreen(screenId, btnEl) {
  document.querySelectorAll(".screen-view").forEach(s => s.classList.remove("active"));
  document.getElementById("screen-" + screenId).classList.add("active");
  document.querySelectorAll(".rail-item").forEach(b => b.classList.remove("active"));
  if (btnEl) btnEl.classList.add("active");
  if (screenId === "dashboard") {
    renderDashboard();
  } else if (screenId === "tools") {
    renderToolsScreen();
    if (!btcNetwork.live && !statsLoading) loadBtcNetworkStats();
  } else if (screenId === "logs") {
    loadEventLogsAndFile();
  }
}

function setTheme(themeName, dotEl) {
  document.body.setAttribute("data-theme", themeName);
  appSettings.theme = themeName;
  localStorage.setItem("asic_pulse_theme", themeName);
  document.querySelectorAll(".theme-dot").forEach(d => d.classList.remove("active"));
  if (dotEl) dotEl.classList.add("active");
  else {
    const targetDot = document.querySelector(`.theme-dot[data-theme-btn="${themeName}"]`);
    if (targetDot) targetDot.classList.add("active");
  }
  drawSessionChart();
}

function toggleLanguage() {
  currentLang = currentLang === "ru" ? "en" : "ru";
  appSettings.language = currentLang;
  localStorage.setItem("asic_pulse_lang", currentLang);
  applyTranslations();
  renderIpRanges();
  renderMinersTable();
  renderDashboard();
  renderToolsScreen();
}

function toggleDemoMode() {
  appSettings.demo_mode = !appSettings.demo_mode;
  updateDemoToggleUI();
  persistSettingsSilently();
  startScan();
}

function updateDemoToggleUI() {
  const btn = document.getElementById("demoToggleBtn");
  const txt = document.getElementById("demoToggleText");
  btn.classList.toggle("active", appSettings.demo_mode);
  txt.textContent = appSettings.demo_mode ? t("demoMode") : t("realNetwork");
}

function renderIpRanges() {
  const container = document.getElementById("rangesList");
  container.innerHTML = "";
  appSettings.ip_ranges.forEach(item => {
    const chip = document.createElement("div");
    chip.className = "range-chip " + (item.enabled ? "enabled" : "");
    chip.innerHTML = `
      <input type="checkbox" ${item.enabled ? "checked" : ""} onchange="toggleRangeEnabled('${item.id}', this.checked)" />
      <span class="mono" onclick="openRangeModal('${item.id}')" style="cursor:pointer">${item.range}</span>
      <span style="opacity:0.8;font-size:12px" onclick="openRangeModal('${item.id}')">${item.comment || ""}</span>
      <button onclick="removeRange('${item.id}')" title="${t('deleteBtn')}">
        <span class="material-symbols-rounded" style="font-size:16px">close</span>
      </button>
    `;
    container.appendChild(chip);
  });
}

function toggleRangeEnabled(id, checked) {
  const r = appSettings.ip_ranges.find(x => x.id === id);
  if (r) r.enabled = checked;
  renderIpRanges();
  persistSettingsSilently();
}

function removeRange(id) {
  appSettings.ip_ranges = appSettings.ip_ranges.filter(x => x.id !== id);
  renderIpRanges();
  persistSettingsSilently();
}

function openRangeModal(id = null) {
  editingRangeId = id;
  const existing = appSettings.ip_ranges.find(x => x.id === id);
  document.getElementById("rangeInputVal").value = existing ? existing.range : "192.168.1.1-254";
  document.getElementById("rangeCommentVal").value = existing ? existing.comment : "";
  document.getElementById("rangeModal").classList.add("open");
}

function closeRangeModal() {
  document.getElementById("rangeModal").classList.remove("open");
}

function saveRangeFromModal() {
  const rangeStr = document.getElementById("rangeInputVal").value.trim();
  const commentStr = document.getElementById("rangeCommentVal").value.trim();
  if (!rangeStr) return;
  if (editingRangeId) {
    const item = appSettings.ip_ranges.find(x => x.id === editingRangeId);
    if (item) {
      item.range = rangeStr;
      item.comment = commentStr;
    }
  } else {
    appSettings.ip_ranges.push({
      id: "r_" + Date.now(),
      range: rangeStr,
      comment: commentStr,
      enabled: true
    });
  }
  closeRangeModal();
  renderIpRanges();
  persistSettingsSilently();
}

async function runAutoImportSubnets() {
  try {
    const detected = await invokeBackend("auto_detect_subnets");
    if (detected && detected.length) {
      detected.forEach(d => {
        if (!appSettings.ip_ranges.some(r => r.range === d.range)) {
          appSettings.ip_ranges.push(d);
        }
      });
      renderIpRanges();
      persistSettingsSilently();
      showToast(`${t("autoImportBtn")}: +${detected.length}`);
    }
  } catch (e) {
    showToast("Auto-import error: " + e);
  }
}

async function startScan() {
  const scanBtn = document.getElementById("scanMainBtn");
  const cancelBtn = document.getElementById("cancelScanBtn");
  const progWrap = document.getElementById("scanProgressWrap");
  const progFill = document.getElementById("scanProgressFill");
  const progText = document.getElementById("scanProgressText");

  scanBtn.disabled = true;
  cancelBtn.style.display = "inline-flex";
  progWrap.classList.add("visible");
  progFill.style.width = "10%";
  progText.textContent = `${t("scanningProgress")}...`;

  progressPollTimer = setInterval(async () => {
    try {
      const p = await invokeBackend("get_scan_progress");
      if (p && p.total > 0) {
        const pct = Math.min(100, Math.round((p.scanned / p.total) * 100));
        progFill.style.width = pct + "%";
        progText.textContent = `${t("scanningProgress")}: ${p.scanned} / ${p.total} (ASIC: ${p.found})`;
      }
    } catch (_) {}
  }, 100);

  try {
    miners = await invokeBackend("scan_miners", {
      ranges: appSettings.ip_ranges,
      onlySuccess: appSettings.only_success_miners,
      demoMode: appSettings.demo_mode
    });
    recordSessionPoint();
    renderMinersTable();
    renderDashboard();
  } catch (err) {
    showToast("Scan Error: " + err);
  } finally {
    clearInterval(progressPollTimer);
    scanBtn.disabled = false;
    cancelBtn.style.display = "none";
    progWrap.classList.remove("visible");
  }
}

async function stopScan() {
  await invokeBackend("cancel_scan");
}

function recordSessionPoint() {
  const totalRt = miners.reduce((acc, m) => acc + (m.hashrate_rt || 0), 0);
  const label = new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  sessionHashrateHistory.push({ time: label, ths: Number(totalRt.toFixed(1)) });
  if (sessionHashrateHistory.length > 30) sessionHashrateHistory.shift();

  miners.forEach(m => {
    if (!minerTelemetryHistory[m.ip]) minerTelemetryHistory[m.ip] = [];
    minerTelemetryHistory[m.ip].push({
      time: label,
      ths: m.hashrate_rt,
      temp: m.temp_max
    });
    if (minerTelemetryHistory[m.ip].length > 20) minerTelemetryHistory[m.ip].shift();
  });

  const alertsCount = miners.filter(m => m.status !== "online").length;
  const badge = document.getElementById("logsAlertBadge");
  if (badge) {
    badge.style.display = alertsCount > 0 ? "block" : "none";
    badge.textContent = alertsCount;
  }
}

function toggleMonitor() {
  const btn = document.getElementById("monitorBtn");
  const label = document.getElementById("monitorBtnLabel");
  if (monitorTimer) {
    clearInterval(monitorTimer);
    monitorTimer = null;
    btn.classList.remove("active");
    label.textContent = t("monitorBtn");
  } else {
    const sec = Math.max(3, Number(appSettings.monitor_interval_sec) || 15);
    btn.classList.add("active");
    label.textContent = `${t("monitorActive")} (${sec}s)`;
    monitorTimer = setInterval(() => {
      if (appSettings.demo_mode && miners.length) {
        miners.forEach(m => {
          if (m.status !== "offline") {
            const delta = (Math.random() - 0.48) * 1.6;
            m.hashrate_rt = Math.max(0, Number((m.hashrate_rt + delta).toFixed(1)));
          }
        });
        recordSessionPoint();
        renderMinersTable();
        renderDashboard();
      } else {
        startScan();
      }
    }, sec * 1000);
  }
}

function sortByColumn(field) {
  if (sortField === field) {
    sortAsc = !sortAsc;
  } else {
    sortField = field;
    sortAsc = true;
  }
  renderMinersTable();
}

function ipToNum(ip) {
  return ip.split(".").reduce((acc, oct) => (acc << 8) + ( parseInt(oct, 10) || 0 ), 0) >>> 0;
}

function getFilteredAndSortedMiners() {
  const q = (document.getElementById("searchInput")?.value || "").toLowerCase().trim();
  const filtered = miners.filter(m => {
    if (appSettings.only_success_miners && m.status === "offline" && statusFilter !== "offline") return false;
    if (statusFilter === "online" && m.status !== "online") return false;
    if (statusFilter === "alerts" && (m.status === "online" || m.status === "offline")) return false;
    if (statusFilter === "offline" && m.status !== "offline") return false;
    if (q) {
      const hay = `${m.ip} ${m.miner_type} ${m.worker1} ${m.pool1} ${m.working_mode}`.toLowerCase();
      if (!hay.includes(q)) return false;
    }
    return true;
  });

  filtered.sort((a, b) => {
    let va = a[sortField];
    let vb = b[sortField];
    if (sortField === "ip") {
      va = ipToNum(a.ip);
      vb = ipToNum(b.ip);
    } else if (typeof va === "string") {
      va = va.toLowerCase();
      vb = (vb || "").toLowerCase();
    }
    if (va < vb) return sortAsc ? -1 : 1;
    if (va > vb) return sortAsc ? 1 : -1;
    return 0;
  });

  return filtered;
}

function renderMinersTable() {
  updateKpiSummary();
  const tbody = document.getElementById("minersTableBody");
  const extraCols = document.querySelectorAll(".extra-pool-col");
  extraCols.forEach(c => {
    c.style.display = appSettings.show_extra_pools ? "table-cell" : "none";
  });

  const rows = getFilteredAndSortedMiners();
  tbody.innerHTML = "";

  if (rows.length === 0) {
    tbody.innerHTML = `
      <tr>
        <td colspan="15" style="text-align:center;padding:38px;color:var(--md-text-sub)">
          <div style="font-size:16px;font-weight:700;color:var(--md-text);margin-bottom:6px">${t("emptyTableTitle")}</div>
          <div>${t("emptyTableDesc")}</div>
        </td>
      </tr>
    `;
    updateFloatingBar();
    return;
  }

  rows.forEach(m => {
    const isSel = selectedIps.has(m.ip);
    const tr = document.createElement("tr");
    tr.className = (isSel ? "selected " : "") + (m.status === "hightemp" || m.status === "lowhash" ? "row-anomaly" : "");
    tr.onclick = (e) => {
      if (e.target.tagName === "INPUT" || e.target.closest("button")) return;
      if (selectedIps.has(m.ip)) selectedIps.delete(m.ip);
      else selectedIps.add(m.ip);
      renderMinersTable();
    };
    tr.ondblclick = () => openMinerDetailModal(m.ip);

    const statusBadge = getStatusPillHtml(m.status);
    const tempClass = m.temp_max === 0 ? "pill-offline" : m.temp_max >= 85 ? "pill-coral" : m.temp_max >= 70 ? "pill-yellow" : "pill-mint";
    const hashClass = m.status === "lowhash" ? "pill-coral" : m.status === "offline" ? "pill-offline" : "pill-mint";
    const extraDisp = appSettings.show_extra_pools ? "table-cell" : "none";

    tr.innerHTML = `
      <td><input type="checkbox" ${isSel ? "checked" : ""} onchange="toggleSelectMiner('${m.ip}', this.checked)" /></td>
      <td class="mono" style="font-weight:700">${m.ip}</td>
      <td>${statusBadge}</td>
      <td style="font-weight:600">${m.miner_type}</td>
      <td><span class="metric-pill">${m.working_mode}</span></td>
      <td><span class="metric-pill ${hashClass}">${m.hashrate_rt.toFixed(1)} TH/s</span></td>
      <td class="mono">${m.hashrate_avg.toFixed(1)} TH/s</td>
      <td><span class="metric-pill ${tempClass}">${m.temp_max} °C</span></td>
      <td class="mono">${m.fan_speed_str} RPM</td>
      <td class="mono">${m.elapsed_str}</td>
      <td style="max-width:190px;overflow:hidden;text-overflow:ellipsis">${m.pool1}</td>
      <td style="color:var(--md-primary);font-weight:600">${m.worker1}</td>
      <td style="display:${extraDisp}">${m.pool2 || "—"}</td>
      <td style="display:${extraDisp}">${m.worker2 || "—"}</td>
      <td style="display:${extraDisp}">${m.pool3 || "—"}</td>
      <td style="display:${extraDisp}">${m.worker3 || "—"}</td>
    `;
    tbody.appendChild(tr);
  });

  updateFloatingBar();
}

function getStatusPillHtml(st) {
  if (st === "online") return `<span class="metric-pill pill-mint">${t("statusOnline")}</span>`;
  if (st === "hightemp") return `<span class="metric-pill pill-coral">${t("statusHighTemp")}</span>`;
  if (st === "lowhash") return `<span class="metric-pill pill-coral">${t("statusLowHash")}</span>`;
  return `<span class="metric-pill pill-offline">${t("statusOffline")}</span>`;
}

function updateKpiSummary() {
  const onlineList = miners.filter(m => m.status !== "offline");
  const rtSum = onlineList.reduce((a, m) => a + m.hashrate_rt, 0);
  const avgSum = onlineList.reduce((a, m) => a + m.hashrate_avg, 0);
  const avgTemp = onlineList.length ? Math.round(onlineList.reduce((a, m) => a + m.temp_max, 0) / onlineList.length) : 0;
  const maxTemp = onlineList.length ? Math.max(...onlineList.map(m => m.temp_max)) : 0;
  const anomalies = miners.filter(m => m.status !== "online").length;

  const setTxt = (id, val) => {
    const el = document.getElementById(id);
    if (el) el.textContent = val;
  };

  setTxt("kpiHashVal", `${rtSum.toFixed(1)} / ${avgSum.toFixed(1)} TH/s`);
  setTxt("kpiCountVal", `${onlineList.length} / ${miners.length} ASIC`);
  setTxt("kpiTempVal", `${avgTemp} °C (max ${maxTemp} °C)`);
  setTxt("kpiAlertVal", `${anomalies}`);

  setTxt("dashKpiHash", `${rtSum.toFixed(1)} TH/s`);
  setTxt("dashKpiCount", `${onlineList.length} / ${miners.length}`);
  setTxt("dashKpiTemp", `${avgTemp} °C`);
  setTxt("dashKpiAlerts", `${anomalies}`);
}

function toggleSelectMiner(ip, checked) {
  if (checked) selectedIps.add(ip);
  else selectedIps.delete(ip);
  renderMinersTable();
}

function toggleSelectAllTable(checked) {
  const visible = getFilteredAndSortedMiners();
  if (checked) visible.forEach(m => selectedIps.add(m.ip));
  else selectedIps.clear();
  renderMinersTable();
}

function clearSelection() {
  selectedIps.clear();
  renderMinersTable();
}

function updateFloatingBar() {
  const bar = document.getElementById("floatingBar");
  const badge = document.getElementById("selectedBadge");
  badge.textContent = `${t("selectedCount")}: ${selectedIps.size} ASIC`;
  bar.classList.toggle("visible", selectedIps.size > 0);
}

function setStatusFilter(filter, btnEl) {
  statusFilter = filter;
  document.querySelectorAll(".scanner-filter-chip").forEach(b => b.classList.remove("active"));
  if (btnEl) btnEl.classList.add("active");
  renderMinersTable();
}

/* --- Dashboard Rendering & Canvas Chart --- */
function renderDashboard() {
  updateKpiSummary();
  drawSessionChart();

  const anomBox = document.getElementById("dashAnomaliesList");
  const anomalies = miners.filter(m => m.status !== "online");
  if (anomalies.length === 0) {
    anomBox.innerHTML = `<div style="color:var(--md-mint);padding:12px 0;font-weight:600">${t("noAnomaliesText")}</div>`;
  } else {
    anomBox.innerHTML = anomalies.map(m => `
      <div style="display:flex;justify-content:space-between;align-items:center;padding:10px 14px;background:var(--md-surface-high);border-radius:16px;margin-bottom:8px;cursor:pointer" onclick="openMinerDetailModal('${m.ip}')">
        <div>
          <div class="mono" style="font-weight:700">${m.ip} • ${m.miner_type}</div>
          <div style="font-size:12px;color:var(--md-text-sub)">TH/s: ${m.hashrate_rt.toFixed(1)} | Temp: ${m.temp_max} °C</div>
        </div>
        ${getStatusPillHtml(m.status)}
      </div>
    `).join("");
  }

  const grid = document.getElementById("dashMinersGrid");
  const filtered = miners.filter(m => {
    if (dashModelFilter === "all") return true;
    return m.miner_type.toLowerCase().includes(dashModelFilter);
  });

  grid.innerHTML = filtered.map(m => {
    const tempPill = m.status === "offline"
      ? `<span class="metric-pill pill-offline">Offline</span>`
      : m.temp_max >= 85
        ? `<span class="metric-pill pill-coral">${m.temp_max} °C</span>`
        : m.temp_max >= 70
          ? `<span class="metric-pill pill-yellow">${m.temp_max} °C</span>`
          : `<span class="metric-pill pill-mint">${m.temp_max} °C</span>`;

    return `
      <div class="dash-miner-card ${m.status === 'offline' ? 'offline' : ''}" ondblclick="openMinerDetailModal('${m.ip}')" onclick="openMinerDetailModal('${m.ip}')">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:10px">
          <span class="mono" style="font-size:15px;font-weight:700">${m.ip}</span>
          ${tempPill}
        </div>
        <div style="font-size:12.5px;color:var(--md-text-sub);margin-bottom:12px">${m.miner_type}</div>
        <div style="display:flex;justify-content:space-between;align-items:baseline">
          <span style="font-size:22px;font-weight:700;color:var(--md-primary)">${m.hashrate_rt.toFixed(1)} <small style="font-size:12px">TH/s</small></span>
          <span style="font-size:12px;color:var(--md-text-sub)">${m.working_mode}</span>
        </div>
      </div>
    `;
  }).join("");
}

function setDashModelFilter(modelKey, btnEl) {
  dashModelFilter = modelKey;
  document.querySelectorAll(".dash-model-chip").forEach(b => b.classList.remove("active"));
  if (btnEl) btnEl.classList.add("active");
  renderDashboard();
}

function drawSessionChart() {
  const canvas = document.getElementById("sessionHashChart");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.width = canvas.clientWidth || 640;
  const h = canvas.height = 210;
  ctx.clearRect(0, 0, w, h);

  const primaryColor = getComputedStyle(document.body).getPropertyValue("--md-primary").trim() || "#D0BCFF";
  const points = sessionHashrateHistory.length ? sessionHashrateHistory : [{ time: "Now", ths: 0 }];

  const vals = points.map(p => p.ths);
  const minV = Math.max(0, Math.min(...vals) * 0.9);
  const maxV = Math.max(...vals, 10) * 1.05;

  ctx.strokeStyle = "rgba(255,255,255,0.07)";
  ctx.lineWidth = 1;
  for (let i = 1; i <= 4; i++) {
    const y = (h / 5) * i;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(w, y);
    ctx.stroke();
  }

  ctx.beginPath();
  points.forEach((pt, idx) => {
    const x = points.length === 1 ? w / 2 : (idx / (points.length - 1)) * (w - 40) + 20;
    const y = h - 28 - ((pt.ths - minV) / (maxV - minV || 1)) * (h - 52);
    if (idx === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.strokeStyle = primaryColor;
  ctx.lineWidth = 3;
  ctx.stroke();

  points.forEach((pt, idx) => {
    const x = points.length === 1 ? w / 2 : (idx / (points.length - 1)) * (w - 40) + 20;
    const y = h - 28 - ((pt.ths - minV) / (maxV - minV || 1)) * (h - 52);
    ctx.fillStyle = primaryColor;
    ctx.beginPath();
    ctx.arc(x, y, 4.5, 0, Math.PI * 2);
    ctx.fill();
  });
}

/* --- Double-Click Machine Telemetry Modal --- */
function openMinerDetailModal(ip) {
  const m = miners.find(x => x.ip === ip);
  if (!m) return;
  activeDetailMinerIp = ip;
  document.getElementById("detailMinerIp").textContent = `${m.ip} — ${m.miner_type}`;
  document.getElementById("detailMinerSub").textContent = `MAC: ${m.mac_addr} • FW: ${m.firmware_ver} • Mode: ${m.working_mode} • Power: ${m.power_w} W`;

  document.getElementById("detailHashRt").textContent = `${m.hashrate_rt.toFixed(2)} TH/s`;
  document.getElementById("detailTempMax").textContent = `${m.temp_max} °C`;
  document.getElementById("detailFans").textContent = `${m.fan_speed_str} RPM`;
  document.getElementById("detailHwErr").textContent = `${m.hw_errors} (${m.rejected_pct}% rej)`;

  const boardsBody = document.getElementById("detailBoardsBody");
  if (!m.boards || m.boards.length === 0) {
    boardsBody.innerHTML = `<tr><td colspan="8" style="text-align:center;color:var(--md-text-sub)">Нет детальной информации по платам</td></tr>`;
  } else {
    boardsBody.innerHTML = m.boards.map(b => `
      <tr>
        <td class="mono">#${b.index}</td>
        <td class="mono">${b.chips}</td>
        <td class="mono">${b.freq_mhz} MHz</td>
        <td class="mono">${b.voltage_v.toFixed(2)} V</td>
        <td><span class="metric-pill">${b.temp_board} °C</span></td>
        <td><span class="metric-pill ${b.temp_chip >= 85 ? 'pill-coral' : 'pill-mint'}">${b.temp_chip} °C</span></td>
        <td class="mono" style="font-weight:700">${b.hashrate_ths.toFixed(1)} TH/s</td>
        <td class="mono">${b.hw_errors}</td>
      </tr>
    `).join("");
  }

  document.getElementById("minerDetailModal").classList.add("open");
  drawMinerMiniChart(ip);
}

function closeMinerDetailModal() {
  document.getElementById("minerDetailModal").classList.remove("open");
}

function drawMinerMiniChart(ip) {
  const canvas = document.getElementById("minerMiniChart");
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  const w = canvas.width = canvas.clientWidth || 760;
  const h = canvas.height = 110;
  ctx.clearRect(0, 0, w, h);

  const hist = minerTelemetryHistory[ip] || [{ ths: 100, temp: 65 }];
  const primary = getComputedStyle(document.body).getPropertyValue("--md-primary").trim() || "#D0BCFF";

  ctx.beginPath();
  hist.forEach((pt, i) => {
    const x = hist.length === 1 ? w / 2 : (i / (hist.length - 1)) * (w - 30) + 15;
    const y = h - 20 - (Math.min(pt.temp, 100) / 100) * (h - 36);
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.strokeStyle = primary;
  ctx.lineWidth = 2.5;
  ctx.stroke();
}

async function pingActiveMiner() {
  if (!activeDetailMinerIp) return;
  const res = await invokeBackend("ping_miner", { ip: activeDetailMinerIp });
  showToast(`Ping ${activeDetailMinerIp}: ${res}`);
}

async function openActiveMinerWeb() {
  if (!activeDetailMinerIp) return;
  await invokeBackend("open_external_url", { url: `http://${activeDetailMinerIp}` });
}

function copyActiveMinerField(field) {
  const m = miners.find(x => x.ip === activeDetailMinerIp);
  if (!m) return;
  const text = field === "ip" ? m.ip : m.worker1;
  navigator.clipboard.writeText(text);
  showToast(`${t("toastCopied")}: ${text}`);
}

/* --- Batch Pool Config, Power Control, Reboot & Firmware --- */
function openPoolsModal() {
  document.getElementById("poolsModal").classList.add("open");
}
function closePoolsModal() {
  document.getElementById("poolsModal").classList.remove("open");
}

async function submitBatchPools(allMiners) {
  const targetIps = allMiners ? miners.map(m => m.ip) : Array.from(selectedIps);
  if (targetIps.length === 0) {
    showToast("Выберите хотя бы один майнер");
    return;
  }
  const payload = {
    ips: targetIps,
    pool1_url: document.getElementById("pool1Url").value.trim(),
    pool1_user: document.getElementById("poolSubaccount").value.trim(),
    pool1_pw: document.getElementById("poolPassword").value.trim() || "123",
    pool2_url: document.getElementById("pool2Url").value.trim(),
    pool2_user: document.getElementById("poolSubaccount").value.trim(),
    pool2_pw: document.getElementById("poolPassword").value.trim() || "123",
    pool3_url: document.getElementById("pool3Url").value.trim(),
    pool3_user: document.getElementById("poolSubaccount").value.trim(),
    pool3_pw: document.getElementById("poolPassword").value.trim() || "123",
    suffix_mode: document.getElementById("suffixModeSelect").value,
    custom_suffix: document.getElementById("customSuffixInput").value.trim(),
    difficulty: Math.round(Number(document.getElementById("poolDifficulty").value)) || null
  };

  miners.forEach(m => {
    if (targetIps.includes(m.ip)) {
      m.pool1 = payload.pool1_url;
      const octs = m.ip.split(".");
      const sfx = payload.suffix_mode === "ip" ? `.${octs[2]}x${octs[3]}` : payload.suffix_mode === "custom" ? `.${payload.custom_suffix}` : "";
      m.worker1 = payload.suffix_mode === "empty" ? "" : `${payload.pool1_user}${sfx}`;
    }
  });

  const msg = await invokeBackend("batch_configure_pools", { payload });
  closePoolsModal();
  renderMinersTable();
  showToast(msg);
}

function openPowerModal() {
  const model = document.getElementById("powerModelSelect").value;
  model && syncPowerFreqPreset();
  document.getElementById("powerModal").classList.add("open");
}
function closePowerModal() {
  document.getElementById("powerModal").classList.remove("open");
}

function syncPowerFreqPreset() {
  const presets = {
    "Antminer S21 / T21 / S19": 675,
    "Antminer S9 / T9 Legacy": 500,
    "WhatsMiner M50 / M60": 740,
    "Avalon A6 / A7 / A8": 600
  };
  const factors = {
    normal: 1,
    overclock: 1.05,
    underclock: 0.9,
    lpm: 0.85,
    elpm: 0.75,
    sleep: 0.5
  };
  const base = presets[document.getElementById("powerModelSelect").value] || 675;
  const mode = document.getElementById("powerOptionSelect").value;
  const freq = Math.round((base * (factors[mode] ?? 1)) / 5) * 5;
  document.getElementById("powerFreqInput").value = Math.max(100, freq);
}

function toggleCustomFreqFields() {
  const checked = document.getElementById("powerCustomFreqChk").checked;
  document.getElementById("powerFreqFields").style.display = checked ? "grid" : "none";
  if (checked) syncPowerFreqPreset();
}

async function submitPowerControl(allMiners) {
  const targetIps = allMiners ? miners.map(m => m.ip) : Array.from(selectedIps);
  if (!targetIps.length) return;
  const workMode = document.getElementById("powerOptionSelect").value;
  const modelFamily = document.getElementById("powerModelSelect").value;
  const customFreq = document.getElementById("powerCustomFreqChk").checked;
  const freqMhz = Math.round(Number(document.getElementById("powerFreqInput").value)) || 0;
  const voltageRaw = document.getElementById("powerVoltageInput").value.trim();
  const voltageV = voltageRaw ? Number(voltageRaw) : null;

  if (customFreq && (freqMhz < 100 || freqMhz > 1500)) {
    showToast("Частота должна быть в диапазоне 100–1500 MHz");
    return;
  }

  miners.forEach(m => {
    if (targetIps.includes(m.ip)) {
      m.working_mode = workMode.toUpperCase();
      if (customFreq) {
        m.freq_mhz = freqMhz;
        if (voltageV) m.voltage_v = voltageV;
        (m.boards || []).forEach(b => {
          b.freq_mhz = freqMhz;
          if (voltageV) b.voltage_v = voltageV;
        });
      }
    }
  });

  let msg = await invokeBackend("batch_power_control", {
    payload: { ips: targetIps, model_family: modelFamily, work_mode: workMode }
  });

  if (customFreq) {
    const freqMsg = await invokeBackend("batch_set_frequency", {
      payload: { ips: targetIps, model_family: modelFamily, freq_mhz: freqMhz, voltage_v: voltageV }
    });
    msg = `${msg} • ${freqMsg}`;
  }

  closePowerModal();
  renderMinersTable();
  renderDashboard();
  showToast(msg);
}

async function triggerBatchReboot(allMiners) {
  const targetIps = allMiners ? miners.map(m => m.ip) : Array.from(selectedIps);
  if (!targetIps.length) return;
  const msg = await invokeBackend("batch_reboot", { ips: targetIps });
  showToast(msg);
}

async function triggerBatchLed() {
  const targetIps = Array.from(selectedIps);
  if (!targetIps.length) return;
  miners.forEach(m => {
    if (selectedIps.has(m.ip)) m.blink_led = !m.blink_led;
  });
  const msg = await invokeBackend("toggle_led_locate", { ips: targetIps, enable: true });
  renderMinersTable();
  showToast(msg);
}

function openFirmwareModal() {
  document.getElementById("fwResultsList").innerHTML = "";
  document.getElementById("firmwareModal").classList.add("open");
}
function closeFirmwareModal() {
  document.getElementById("firmwareModal").classList.remove("open");
}

async function runFirmwareUpgrade() {
  const fileInput = document.getElementById("fwFileInput");
  const fwName = fileInput.files?.[0]?.name || "Antminer-S21-Vnish-1.2.6.tar.gz";
  const targetIps = selectedIps.size > 0 ? Array.from(selectedIps) : miners.map(m => m.ip);
  const resBox = document.getElementById("fwResultsList");
  resBox.innerHTML = `<div style="color:var(--md-primary);margin-top:10px">Отправка образа ${fwName} на ${targetIps.length} майнеров...</div>`;

  const results = await invokeBackend("batch_firmware_upgrade", { ips: targetIps, firmwareName: fwName });
  if (Array.isArray(results)) {
    resBox.innerHTML = results.map(r => `
      <div style="display:flex;justify-content:space-between;padding:6px 10px;background:var(--md-surface);border-radius:10px;margin-top:6px">
        <span class="mono">${r.ip}</span>
        <span style="color:${r.success ? 'var(--md-mint)' : 'var(--md-coral)'}">${r.message}</span>
      </div>
    `).join("");
  }
}

/* --- Export CSV, Import IPs, Export HTML Report --- */
async function exportTableToCsv() {
  const path = await invokeBackend("export_miners_csv", { miners });
  showToast(`CSV: ${path}`);
}

async function exportDashboardHtml() {
  const rowsHtml = miners.map(m => `
    <tr>
      <td>${m.ip}</td><td>${m.status}</td><td>${m.miner_type}</td>
      <td>${m.hashrate_rt.toFixed(1)} TH/s</td><td>${m.temp_max} °C</td>
      <td>${m.pool1}</td><td>${m.worker1}</td>
    </tr>
  `).join("");

  const doc = `<!DOCTYPE html><html lang="ru"><head><meta charset="UTF-8"><title>ASIC Pulse Report</title>
  <style>body{background:#141118;color:#E6E1E9;font-family:sans-serif;padding:32px}table{width:100%;border-collapse:collapse;margin-top:20px}th,td{padding:10px;border-bottom:1px solid #362F40;text-align:left}th{color:#D0BCFF}</style></head>
  <body><h1>ASIC Pulse — Сводный отчёт по парку майнеров</h1><p>Дата: ${new Date().toLocaleString()}</p>
  <table><thead><tr><th>IP</th><th>Status</th><th>Model</th><th>Hashrate</th><th>Temp</th><th>Pool 1</th><th>Worker</th></tr></thead><tbody>${rowsHtml}</tbody></table></body></html>`;

  const path = await invokeBackend("export_html_report", { htmlContent: doc });
  showToast(`HTML: ${path}`);
}

function openImportModal() {
  document.getElementById("importModal").classList.add("open");
}
function closeImportModal() {
  document.getElementById("importModal").classList.remove("open");
}

function applyImportedIps() {
  const raw = document.getElementById("importTextarea").value.trim();
  if (!raw) return;
  raw.split(/\r?\n/).forEach((line, idx) => {
    const parts = line.split(",");
    const range = (parts[0] || "").trim();
    const comment = (parts[1] || "Импорт").trim();
    if (range) {
      appSettings.ip_ranges.push({
        id: "imp_" + Date.now() + "_" + idx,
        range,
        comment,
        enabled: true
      });
    }
  });
  closeImportModal();
  renderIpRanges();
  persistSettingsSilently();
}

function loadImportFile(input) {
  const file = input.files?.[0];
  if (!file) return;
  const reader = new FileReader();
  reader.onload = e => {
    const text = String(e.target.result || "").trim();
    if (text) {
      document.getElementById("importTextarea").value = text;
      showToast(`${file.name}: ${text.split(/\r?\n/).length} строк`);
    }
  };
  reader.readAsText(file, "UTF-8");
}

/* --- Tools: profitability calculator & Bitcoin network --- */
function fmtUsd(value, digits = 0) {
  const sign = value < 0 ? "-" : "";
  return `${sign}$${Math.abs(value).toLocaleString("en-US", {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits
  })}`;
}

function blockRewardAt(height) {
  return 50 / Math.pow(2, Math.floor(height / 210000));
}

function nextHalvingHeight(height) {
  return Math.ceil((height + 1) / 210000) * 210000;
}

function restoreCalculatorInputs() {
  try {
    const saved = JSON.parse(localStorage.getItem("asic_pulse_calc") || "{}");
    ["calcHashrate", "calcPower", "calcTariff", "calcPoolFee", "calcBtcPrice", "calcNetworkHash"].forEach(id => {
      const el = document.getElementById(id);
      if (el && saved[id] !== undefined && Number.isFinite(Number(saved[id]))) {
        el.value = saved[id];
      }
    });
  } catch (_) {}
}

function recalcProfitability() {
  const val = id => Number(document.getElementById(id)?.value) || 0;
  const hashrate = val("calcHashrate");
  const power = val("calcPower");
  const tariff = val("calcTariff");
  const fee = val("calcPoolFee");
  const btcPrice = val("calcBtcPrice");
  const netHashEh = val("calcNetworkHash");

  const setText = (id, text, tone) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.textContent = text;
    el.classList.remove("positive", "negative");
    if (tone) el.classList.add(tone);
  };

  if (hashrate <= 0 || netHashEh <= 0) {
    ["calcBtcDay", "calcRevenueDay", "calcCostDay", "calcProfitDay", "calcProfitMonth", "calcMargin", "calcBreakEven"]
      .forEach(id => setText(id, "—"));
  } else {
    const netHashHs = netHashEh * 1e18;
    const reward = blockRewardAt(btcNetwork.blockHeight);
    const btcDay = (hashrate * 1e12 / netHashHs) * 144 * reward * (1 - fee / 100);
    const revenue = btcDay * btcPrice;
    const cost = (power / 1000) * 24 * tariff;
    const profit = revenue - cost;
    const margin = revenue > 0 ? (profit / revenue) * 100 : 0;
    const breakEven = btcDay > 0 ? cost / btcDay : 0;

    setText("calcBtcDay", btcDay >= 0.000001 ? btcDay.toFixed(6) : btcDay.toFixed(8));
    setText("calcRevenueDay", fmtUsd(revenue, 2));
    setText("calcCostDay", fmtUsd(cost, 2));
    setText("calcProfitDay", fmtUsd(profit, 2), profit >= 0 ? "positive" : "negative");
    setText("calcProfitMonth", fmtUsd(profit * 30, 0), profit >= 0 ? "positive" : "negative");
    setText("calcMargin", `${margin.toFixed(1)}%`, margin >= 0 ? "positive" : "negative");
    setText("calcBreakEven", btcDay > 0 ? fmtUsd(breakEven) : "—");
  }

  try {
    const state = {};
    ["calcHashrate", "calcPower", "calcTariff", "calcPoolFee", "calcBtcPrice", "calcNetworkHash"].forEach(id => {
      state[id] = document.getElementById(id)?.value;
    });
    localStorage.setItem("asic_pulse_calc", JSON.stringify(state));
  } catch (_) {}
}

function fillCalculatorFromFleet(silent = false) {
  if (!miners.length) {
    if (!silent) showToast(t("emptyTableTitle"));
    return;
  }
  const totalHash = miners.reduce((acc, m) => acc + (Number(m.hashrate_rt) || 0), 0);
  const totalPower = miners.reduce((acc, m) => acc + (Number(m.power_w) || 0), 0)
    || Math.round(totalHash * 18);
  document.getElementById("calcHashrate").value = totalHash.toFixed(1);
  document.getElementById("calcPower").value = totalPower;
  if (btcNetwork.live) {
    document.getElementById("calcBtcPrice").value = Math.round(btcNetwork.price);
    document.getElementById("calcNetworkHash").value = Math.round(btcNetwork.hashrateHs / 1e18);
  }
  recalcProfitability();
  if (!silent) showToast(t("toastFleetFilled"));
}

function renderToolsScreen() {
  const priceEl = document.getElementById("toolsKpiPrice");
  if (!priceEl) return;

  const height = btcNetwork.blockHeight;
  const reward = blockRewardAt(height);
  const nextHeight = nextHalvingHeight(height);
  const blocksLeft = Math.max(0, nextHeight - height);
  const daysLeft = blocksLeft / 144;
  const halvingDate = new Date(Date.now() + daysLeft * 86400000);
  const hashEh = btcNetwork.hashrateHs / 1e18;
  const difficultyT = btcNetwork.difficulty / 1e12;

  priceEl.textContent = fmtUsd(btcNetwork.price);
  document.getElementById("toolsKpiPriceSub").textContent =
    btcNetwork.live ? `${t("toolsLive")} • mempool.space` : t("toolsOffline");
  document.getElementById("toolsKpiHash").textContent =
    `${hashEh.toLocaleString("en-US", { maximumFractionDigits: 1 })}`;
  document.getElementById("toolsKpiDiff").textContent =
    `${difficultyT.toLocaleString("en-US", { maximumFractionDigits: 1 })}`;

  document.getElementById("toolsKpiHalving").textContent =
    daysLeft > 365 ? `${(daysLeft / 365).toFixed(1)} ${currentLang === "ru" ? "г." : "y"}` : `${Math.round(daysLeft)} ${t("toolsDays")}`;
  document.getElementById("toolsKpiHalvingSub").textContent =
    `${blocksLeft.toLocaleString("en-US")} ${currentLang === "ru" ? "блоков" : "blocks"} • ${halvingDate.toLocaleDateString(currentLang === "ru" ? "ru-RU" : "en-US")}`;

  const rows = [
    [t("toolsBlockHeight"), height.toLocaleString("en-US")],
    [t("toolsBlockReward"), `${reward} BTC`],
    [t("toolsNetworkHashrate"), `${hashEh.toLocaleString("en-US", { maximumFractionDigits: 1 })} EH/s`],
    [t("toolsDifficulty"), `${difficultyT.toLocaleString("en-US", { maximumFractionDigits: 1 })} T`],
    [t("toolsHalvingDate"), `${halvingDate.toLocaleDateString(currentLang === "ru" ? "ru-RU" : "en-US")} (${blocksLeft.toLocaleString("en-US")} ${currentLang === "ru" ? "блоков" : "blocks"})`]
  ];
  document.getElementById("netStatsList").innerHTML = rows.map(([label, value]) => `
    <div class="net-row">
      <span class="net-label">${label}</span>
      <span class="net-value mono">${value}</span>
    </div>
  `).join("");

  const updated = document.getElementById("toolsUpdated");
  updated.textContent = btcNetwork.live && btcNetwork.updatedAt
    ? `${t("toolsUpdatedAt")}: ${btcNetwork.updatedAt.toLocaleTimeString()} • mempool.space`
    : `${t("toolsOffline")} • mempool.space`;

  recalcProfitability();
}

async function fetchWithTimeout(url, timeoutMs = 8000, asText = false) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const res = await fetch(url, { signal: controller.signal, cache: "no-store" });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return asText ? await res.text() : await res.json();
  } finally {
    clearTimeout(timer);
  }
}

async function loadBtcNetworkStats(force = false) {
  if (statsLoading) return;
  statsLoading = true;
  const results = await Promise.allSettled([
    fetchWithTimeout("https://mempool.space/api/v1/prices"),
    fetchWithTimeout("https://mempool.space/api/v1/mining/hashrate/3d"),
    fetchWithTimeout("https://mempool.space/api/blocks/tip/height", 8000, true)
  ]);

  let live = false;
  const [priceRes, hashRes, heightRes] = results;

  if (priceRes.status === "fulfilled" && Number(priceRes.value?.USD) > 0) {
    btcNetwork.price = Number(priceRes.value.USD);
    live = true;
  }
  if (hashRes.status === "fulfilled") {
    const hashList = Array.isArray(hashRes.value?.hashrates) ? hashRes.value.hashrates : [];
    const diffList = Array.isArray(hashRes.value?.difficulty) ? hashRes.value.difficulty : [];
    const lastAvg = hashList.length ? Number(hashList[hashList.length - 1]?.avgHashrate) : 0;
    const lastDiff = diffList.length ? Number(diffList[diffList.length - 1]?.difficulty) : 0;
    const currentHash = Number(hashRes.value?.currentHashrate) || lastAvg;
    const currentDiff = Number(hashRes.value?.currentDifficulty) || lastDiff;
    if (currentHash > 0) {
      btcNetwork.hashrateHs = currentHash;
      live = true;
    }
    if (currentDiff > 0) btcNetwork.difficulty = currentDiff;
  }
  if (heightRes.status === "fulfilled") {
    const height = Number(String(heightRes.value).trim());
    if (height > 0) {
      btcNetwork.blockHeight = height;
      live = true;
    }
  }

  btcNetwork.live = live;
  btcNetwork.updatedAt = new Date();
  statsLoading = false;

  if (live) {
    const priceInput = document.getElementById("calcBtcPrice");
    const hashInput = document.getElementById("calcNetworkHash");
    if (priceInput) priceInput.value = Math.round(btcNetwork.price);
    if (hashInput) hashInput.value = Math.round(btcNetwork.hashrateHs / 1e18);
  }

  renderToolsScreen();
  if (force) {
    showToast(live ? `${t("toolsLive")} • mempool.space` : t("toolsNoData"));
  }
}

/* --- Settings & Logs --- */
function populateSettingsForm() {
  document.getElementById("cfgScanTimeout").value = appSettings.scan_timeout_ms;
  document.getElementById("cfgHttpTimeout").value = appSettings.http_timeout_ms;
  document.getElementById("cfgConcurrency").value = appSettings.concurrency;
  document.getElementById("cfgMonitorInt").value = appSettings.monitor_interval_sec;
  document.getElementById("cfgAntUser").value = appSettings.antminer_user;
  document.getElementById("cfgAntPass").value = appSettings.antminer_pass;
  document.getElementById("cfgWhatsUser").value = appSettings.whatsminer_user;
  document.getElementById("cfgWhatsPass").value = appSettings.whatsminer_pass;
  document.getElementById("cfgAvaUser").value = appSettings.avalon_user;
  document.getElementById("cfgAvaPass").value = appSettings.avalon_pass;
  document.getElementById("onlySuccessChk").checked = appSettings.only_success_miners;
  document.getElementById("extraPoolsChk").checked = appSettings.show_extra_pools;
}

async function saveSettingsFromForm() {
  appSettings.scan_timeout_ms = Number(document.getElementById("cfgScanTimeout").value) || 450;
  appSettings.http_timeout_ms = Number(document.getElementById("cfgHttpTimeout").value) || 2500;
  appSettings.concurrency = Number(document.getElementById("cfgConcurrency").value) || 128;
  appSettings.monitor_interval_sec = Number(document.getElementById("cfgMonitorInt").value) || 15;
  appSettings.antminer_user = document.getElementById("cfgAntUser").value.trim() || "root";
  appSettings.antminer_pass = document.getElementById("cfgAntPass").value.trim() || "root";
  appSettings.whatsminer_user = document.getElementById("cfgWhatsUser").value.trim() || "admin";
  appSettings.whatsminer_pass = document.getElementById("cfgWhatsPass").value.trim() || "admin";
  appSettings.avalon_user = document.getElementById("cfgAvaUser").value.trim() || "root";
  appSettings.avalon_pass = document.getElementById("cfgAvaPass").value.trim() || "root";

  await invokeBackend("save_app_settings", { settings: appSettings });
  showToast(t("toastSaved"));
}

async function persistSettingsSilently() {
  try {
    await invokeBackend("save_app_settings", { settings: appSettings });
  } catch (_) {}
}

async function loadEventLogsAndFile() {
  const events = await invokeBackend("get_event_logs");
  const evContainer = document.getElementById("eventLogsContainer");
  if (Array.isArray(events)) {
    evContainer.innerHTML = events.map(ev => `
      <div style="display:flex;justify-content:space-between;padding:10px 14px;background:var(--md-surface-high);border-radius:14px;margin-bottom:8px">
        <div>
          <strong style="color:var(--md-primary)">${ev.title}</strong>
          <div style="font-size:12.5px;color:var(--md-text-sub)">${ev.details}</div>
        </div>
        <span class="mono" style="font-size:12px;color:var(--md-text-sub)">${ev.timestamp}</span>
      </div>
    `).join("");
  }
  const rawLog = await invokeBackend("read_log_file");
  document.getElementById("rawLogViewer").textContent = rawLog;
}

let toastTimeout;
function showToast(msg) {
  const bar = document.getElementById("snackbar");
  bar.textContent = msg;
  bar.classList.add("show");
  clearTimeout(toastTimeout);
  toastTimeout = setTimeout(() => bar.classList.remove("show"), 2800);
}

window.addEventListener("DOMContentLoaded", async () => {
  try {
    const loaded = await invokeBackend("get_app_settings");
    if (loaded) appSettings = loaded;
  } catch (_) {}

  currentLang = appSettings.language || "ru";
  setTheme(appSettings.theme || "lavender");
  applyTranslations();
  updateDemoToggleUI();
  populateSettingsForm();
  renderIpRanges();
  restoreCalculatorInputs();
  renderToolsScreen();
  loadBtcNetworkStats();
  await startScan();
  if (!(Number(document.getElementById("calcHashrate")?.value) > 0)) {
    fillCalculatorFromFleet(true);
  }
});