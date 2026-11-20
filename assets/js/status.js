// 顶部运行状态条：轮询 /api/status 并渲染
// Top status bar: polls /api/status and renders the running state.

import { api } from './api.js';
import { t } from './i18n.js';

const els = {};
let lastLogTotal = 0;
let onLines = null;
let timer = null;

export function initStatusBar(options) {
  onLines = options && options.onLines;
  els.pill = document.getElementById('status-pill');
  els.model = document.getElementById('status-model');
  els.host = document.getElementById('status-host');
  els.uptime = document.getElementById('status-uptime');
  els.endpoints = document.getElementById('api-endpoints');
  els.args = document.getElementById('args-preview');
  els.btnStart = document.getElementById('btn-start');
  els.btnStop = document.getElementById('btn-stop');
  els.preview = options && options.onPreview;
}

function fmtUptime(sec) {
  if (!sec || sec <= 0) return '—';
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  const s = sec % 60;
  return h > 0 ? `${h}h ${m}m` : `${m}m ${s}s`;
}

export function renderStatus(st, ctx) {
  if (!els.pill) return;
  const phase = st.phase || 'stopped';
  els.pill.dataset.phase = phase;
  els.pill.textContent = t('status.' + phase, phase);
  els.model.textContent = ctx && ctx.modelName ? ctx.modelName : t('status.noModel');
  els.host.textContent = `${st.host || '—'}:${st.port || ''}`;
  els.uptime.textContent = fmtUptime(st.uptime || 0);

  if (els.btnStart) {
    const busy = phase === 'starting';
    els.btnStart.disabled = phase === 'running' || phase === 'starting';
    els.btnStart.textContent = busy ? t('status.starting') : t('button.start');
  }
  if (els.btnStop) els.btnStop.disabled = phase === 'stopped';

  if (els.endpoints) {
    const ep = st.endpoints || {};
    els.endpoints.textContent = [
      `${t('status.endpoint')}: ${ep.base || '—'}`,
      `health: ${ep.health || '—'}`,
      `${t('status.pid')}: ${st.pid || '—'}`,
      st.error ? `${t('status.error')}: ${t(st.error, st.error)}` : '',
    ].filter(Boolean).join('\n');
  }
  if (els.args && st.argsPreview) els.args.textContent = st.argsPreview;
}

export function startPolling(getCtx) {
  stopPolling();
  const tick = async () => {
    try {
      const data = await api.status(lastLogTotal);
      lastLogTotal = data.total || lastLogTotal;
      renderStatus(data.status, getCtx && getCtx());
      if (onLines && data.lines && data.lines.length) onLines(data.lines);
      if (els.preview) els.preview();
    } catch (e) {
      /* 面板自身不可达时静默重试 */
    }
  };
  tick();
  timer = setInterval(tick, 2000);
}

export function stopPolling() {
  if (timer) clearInterval(timer);
  timer = null;
}
