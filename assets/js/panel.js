// 主面板装配：引导数据、字段渲染、按钮联动、语言切换
// Main panel wiring: bootstrap data, field rendering, buttons, language switch.

import { api } from './api.js';
import { setBundle, applyStatic, t, lang } from './i18n.js';
import { initStatusBar, renderStatus, startPolling } from './status.js';
import { initModels, setModels, currentModel, refreshModels, importPath, scanDirs, models } from './models.js';
import { renderGroup, setPath, keysToText, textToKeys } from './settings.js';

const state = {
  config: {},
  schema: [],
  dirty: {},
  languages: [],
};

const el = {};

function grab() {
  [
    'app-title', 'app-sub', 'app-footer', 'lang-select', 'btn-log', 'btn-advanced', 'btn-chat',
    'btn-import', 'btn-scan', 'btn-import-path', 'import-path', 'btn-start', 'btn-stop',
    'btn-save', 'save-hint', 'fields-common', 'fields-network', 'api-key-hint', 'api-endpoints',
    'lbl-model', 'lbl-host', 'lbl-uptime',
    'log-dialog', 'log-view', 'btn-log-close', 'btn-log-clear', 'model-count', 'status-model',
  ].forEach((id) => {
    el[id] = document.getElementById(id);
  });
}

function modelName() {
  const m = currentModel();
  return m ? m.name : '';
}

function applyStaticTexts() {
  const meta = (state.meta = state.meta || {});
  document.title = t('app.title');
  el['app-title'].textContent = t('app.title');
  el['app-sub'].textContent = t('app.subtitle');
  el['app-footer'].textContent = t('app.footer');
  el['lbl-model'].textContent = t('status.model');
  el['lbl-host'].textContent = t('status.host');
  el['lbl-uptime'].textContent = t('status.uptime');
  el['btn-log'].textContent = t('status.log');
  el['btn-advanced'].textContent = t('button.advanced');
  el['btn-chat'].textContent = t('button.chat');
  el['btn-import'].textContent = t('button.import');
  el['btn-scan'].textContent = t('button.scan');
  el['btn-import-path'].textContent = t('button.addPath');
  el['btn-start'].textContent = t('button.start');
  el['btn-stop'].textContent = t('button.stop');
  el['btn-save'].textContent = t('button.save');
  el['btn-log-close'].textContent = t('button.back');
  el['btn-log-clear'].textContent = t('button.clearLog');
  el['import-path'].placeholder = t('label.path');
  el['api-key-hint'].textContent = t('label.apiEnabledHint');
  document.getElementById('lbl-model').textContent = t('status.model');
  applyStatic(document);
  buildLangSelect();
}

function buildLangSelect() {
  const sel = el['lang-select'];
  sel.innerHTML = '';
  const auto = document.createElement('option');
  auto.value = 'auto';
  auto.textContent = t('label.language') + ': auto';
  sel.appendChild(auto);
  (state.languages || []).forEach((l) => {
    const o = document.createElement('option');
    o.value = l.code;
    o.textContent = l.name;
    sel.appendChild(o);
  });
  sel.value = (state.config.app && state.config.app.lang) || 'auto';
}

function fieldChanged(field, value) {
  setPath(state.config, field.key, value);
  setPath(state.dirty, field.key, value);
  el['save-hint'].textContent = '';
}

function renderSettings() {
  renderGroup(el['fields-common'], state.schema, state.config, 'common', fieldChanged);
  renderGroup(el['fields-network'], state.schema, state.config, 'network', fieldChanged);
}

function refreshPreview() {
  api.preview().then((d) => {
    const pre = document.getElementById('args-preview');
    if (pre) pre.textContent = d.args || '';
  }).catch(() => {});
}

async function loadAll(reloadLang) {
  const boot = await api.bootstrap();
  if (reloadLang || !state.schema.length) {
    state.schema = boot.schema || [];
    state.languages = boot.languages || [];
    setBundle(boot.i18n);
    applyStaticTexts();
  }
  state.config = boot.config || {};
  state.dirty = {};
  setModels(boot.models, boot.selected);
  renderSettings();
  renderStatus(boot.status, { modelName: modelName() });
}

async function saveConfig(extra) {
  const patch = Object.assign({}, state.dirty, extra || {});
  if (!Object.keys(patch).length) {
    el['save-hint'].textContent = '';
    return;
  }
  const res = await api.saveConfig(patch);
  state.config = res.config;
  state.dirty = {};
  el['save-hint'].textContent = '✓ ' + t('button.save');
  setTimeout(() => { el['save-hint'].textContent = ''; }, 2000);
  return res;
}

function openLog() {
  el['log-dialog'].showModal();
}

function wire() {
  el['btn-import'].addEventListener('click', async () => {
    const p = el['import-path'].value.trim();
    if (!p) { el['import-path'].focus(); return; }
    try {
      const n = await importPath(p);
      if (n === 0) alert(t('label.emptyModels'));
    } catch (e) { alert(e.message); }
  });

  el['btn-import-path'].addEventListener('click', () => el['btn-import'].click());

  el['btn-scan'].addEventListener('click', async () => {
    try {
      await scanDirs();
    } catch (e) { alert(e.message); }
  });

  el['btn-start'].addEventListener('click', async () => {
    try {
      await saveConfig();
      await api.start(models.selected);
      const s = await api.status(0);
      renderStatus(s.status, { modelName: modelName() });
    } catch (e) { alert(e.message); }
  });

  el['btn-stop'].addEventListener('click', async () => {
    try {
      await api.stop();
      const s = await api.status(0);
      renderStatus(s.status, { modelName: modelName() });
    } catch (e) { alert(e.message); }
  });

  el['btn-save'].addEventListener('click', async () => {
    try {
      await saveConfig();
      await refreshModels();
      renderStatus((await api.status(0)).status, { modelName: modelName() });
    } catch (e) { alert(e.message); }
  });

  el['btn-chat'].addEventListener('click', () => window.open('/chat.html', '_blank'));

  el['btn-advanced'].addEventListener('click', async () => {
    try { await saveConfig(); } catch (e) { /* 忽略，仍允许进入高级页 */ }
    window.location.href = '/advanced.html';
  });

  el['btn-log'].addEventListener('click', openLog);
  el['btn-log-close'].addEventListener('click', () => el['log-dialog'].close());
  el['btn-log-clear'].addEventListener('click', async () => {
    try { await api.clearLog(); el['log-view'].textContent = ''; } catch (e) { alert(e.message); }
  });

  el['lang-select'].addEventListener('change', async (ev) => {
    const value = ev.target.value;
    try {
      await api.saveConfig({ app: { lang: value } });
      await loadAll(true);
    } catch (e) { alert(e.message); }
  });
}

function appendLog(lines) {
  const view = el['log-view'];
  if (!view) return;
  const atBottom = view.scrollTop + view.clientHeight >= view.scrollHeight - 20;
  view.textContent += (view.textContent ? '\n' : '') + lines.join('\n');
  if (atBottom) view.scrollTop = view.scrollHeight;
}

async function main() {
  grab();
  wire();
  initModels({ onChange: () => renderStatus((state.lastStatus = state.lastStatus || {}), { modelName: modelName() }) });
  initStatusBar({ onLines: appendLog, onPreview: refreshPreview });
  try {
    await loadAll(true);
  } catch (e) {
    document.body.insertAdjacentHTML('afterbegin', `<pre style="color:#e06c75">${e.message}</pre>`);
    return;
  }
  startPolling(() => ({ modelName: modelName() }));
  refreshPreview();
}

main();
