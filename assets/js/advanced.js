// 二级页面：高级与非常用设置
// Secondary page: advanced and rarely used settings.

import { api } from './api.js';
import { setBundle, t, applyStatic } from './i18n.js';
import { buildField, readValue, setPath, getPath, renderGroup } from './settings.js';

const state = { config: {}, schema: [], dirty: {} };

function el(id) {
  return document.getElementById(id);
}

function wireAppFields() {
  const box = el('fields-app');
  box.innerHTML = '';

  const defs = [
    { key: 'app.llamaExe', kind: 'text', label: 'label.llamaExe', hint: 'err.exeNotFound' },
    { key: 'app.panelPort', kind: 'int', label: 'label.panelPort', hint: '', min: 1024, max: 65535, step: 1 },
    { key: 'app.autoOpenBrowser', kind: 'bool', label: 'label.autoOpen', hint: '' },
    { key: 'app.lang', kind: 'select', label: 'label.language', hint: '', options: ['auto', 'zh-CN', 'en-US'] },
  ];

  for (const f of defs) {
    const field = Object.assign({ section: 'app' }, f);
    box.appendChild(
      buildField(field, getPath(state.config, field.key), (fl, value) => {
        setPath(state.config, fl.key, value);
        setPath(state.dirty, fl.key, value);
        el('adv-hint').textContent = '';
      })
    );
  }

  const dirs = getPath(state.config, 'app.scanDirs');
  el('adv-scan-dirs').value = Array.isArray(dirs) ? dirs.join(';') : '';
}

async function load() {
  const boot = await api.bootstrap();
  setBundle(boot.i18n);
  state.config = boot.config || {};
  state.schema = boot.schema || [];

  document.title = t('section.advanced');
  el('adv-title').textContent = t('app.title');
  el('adv-sub').textContent = t('app.subtitle');
  el('adv-back').textContent = t('button.back');
  el('adv-save').textContent = t('button.save');
  el('adv-exe-hint').textContent = t('label.scanDirs');
  el('adv-scan-save').textContent = t('button.save');
  applyStatic(document);

  renderGroup(el('fields-advanced'), state.schema, state.config, 'advanced', (field, value) => {
    setPath(state.config, field.key, value);
    setPath(state.dirty, field.key, value);
    el('adv-hint').textContent = '';
  });

  wireAppFields();
  applyStatic(document);
}

async function save() {
  const dirs = el('adv-scan-dirs').value
    .split(/[;\r\n]+/)
    .map((s) => s.trim())
    .filter(Boolean);
  const patch = Object.assign({}, state.dirty, { app: { scanDirs: dirs } });
  // 合并 app 分组中已修改的键
  const appPatch = { scanDirs: dirs };
  for (const key of Object.keys(state.dirty)) {
    if (key.startsWith('app.')) {
      appPatch[key.slice(4)] = state.dirty[key];
      delete patch[key];
    }
  }
  patch.app = Object.assign({}, patch.app, appPatch);

  try {
    const res = await api.saveConfig(patch);
    state.config = res.config;
    state.dirty = {};
    el('adv-hint').textContent = '✓ ' + t('button.save');
    setTimeout(() => { el('adv-hint').textContent = ''; }, 2000);
  } catch (e) {
    el('adv-hint').textContent = e.message;
  }
}

function main() {
  el('adv-back').addEventListener('click', async () => {
    try { await save(); } catch (e) { /* 忽略 */ }
    window.location.href = '/';
  });
  el('adv-save').addEventListener('click', save);
  el('adv-scan-save').addEventListener('click', save);
  load().catch((e) => {
    document.body.insertAdjacentHTML('afterbegin', `<pre style="color:#e06c75">${e.message}</pre>`);
  });
}

main();
