// 左侧模型库：列表渲染、导入、扫描、选择、移除
// Left model library: rendering, import, scan, select, remove.

import { api } from './api.js';
import { t } from './i18n.js';

export const models = {
  items: [],
  selected: '',
};

let listEl = null;
let countEl = null;
let onChange = null;

export function initModels(options) {
  listEl = document.getElementById('model-list');
  countEl = document.getElementById('model-count');
  onChange = options && options.onChange;
}

export function setModels(items, selected) {
  models.items = items || [];
  models.selected = selected || '';
  render();
}

export function currentModel() {
  return models.items.find((m) => m.id === models.selected) || null;
}

function render() {
  if (!listEl) return;
  listEl.innerHTML = '';
  if (countEl) countEl.textContent = String(models.items.length);

  if (models.items.length === 0) {
    const li = document.createElement('li');
    li.className = 'empty';
    li.textContent = t('label.emptyModels');
    listEl.appendChild(li);
    return;
  }

  for (const m of models.items) {
    const li = document.createElement('li');
    li.className = m.id === models.selected ? 'selected' : '';
    li.title = m.path;

    const name = document.createElement('span');
    name.className = 'mname';
    name.textContent = m.name;
    li.appendChild(name);

    const size = document.createElement('span');
    size.className = 'msize';
    size.textContent = m.sizeText;
    li.appendChild(size);

    const del = document.createElement('button');
    del.className = 'mremove';
    del.textContent = '✕';
    del.title = t('button.remove');
    del.addEventListener('click', async (ev) => {
      ev.stopPropagation();
      if (!confirm(t('label.confirmRemove'))) return;
      try {
        const items = await api.removeModel(m.id);
        const data = await api.models();
        setModels(items, data.selected);
        if (onChange) onChange();
      } catch (e) {
        alert(e.message);
      }
    });
    li.appendChild(del);

    li.addEventListener('click', async () => {
      try {
        await api.selectModel(m.id);
        models.selected = m.id;
        render();
        if (onChange) onChange();
      } catch (e) {
        alert(e.message);
      }
    });

    listEl.appendChild(li);
  }
}

export async function refreshModels() {
  const data = await api.models();
  setModels(data.models, data.selected);
  return data;
}

export async function importPath(pathText) {
  const paths = String(pathText || '')
    .split(/[\r\n;]+/)
    .map((s) => s.trim())
    .filter(Boolean);
  if (paths.length === 0) return 0;
  const res = await api.importPaths(paths);
  const data = await api.models();
  setModels(data.models, data.selected);
  return res.added || 0;
}

export async function scanDirs() {
  const res = await api.scan();
  const data = await api.models();
  setModels(data.models, data.selected);
  return res.added || 0;
}
