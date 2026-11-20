// 设置字段渲染：完全由后端 schema 驱动，标签取自语言包
// Settings renderer driven by the backend schema; labels come from locales.

import { t } from './i18n.js';

export function buildField(field, value, onInput) {
  const wrap = document.createElement('div');
  wrap.className = 'field' + (field.kind === 'multiline' ? ' wide' : '');

  const label = document.createElement('label');
  label.textContent = t(field.label);
  label.htmlFor = 'f_' + field.key;
  wrap.appendChild(label);

  let input;
  if (field.kind === 'bool') {
    input = document.createElement('input');
    input.type = 'checkbox';
    input.checked = !!value;
  } else if (field.kind === 'select') {
    input = document.createElement('select');
    (field.options || []).forEach((opt) => {
      const o = document.createElement('option');
      o.value = opt;
      o.textContent = opt;
      input.appendChild(o);
    });
    input.value = value != null ? String(value) : (field.options || [''])[0];
  } else if (field.kind === 'int' || field.kind === 'float') {
    input = document.createElement('input');
    input.type = 'number';
    if (field.min !== undefined) input.min = field.min;
    if (field.max !== undefined) input.max = field.max;
    if (field.step !== undefined) input.step = field.step;
    input.value = value != null ? String(value) : '';
  } else if (field.kind === 'multiline') {
    input = document.createElement('textarea');
    input.value = value != null ? String(value) : '';
  } else {
    input = document.createElement('input');
    input.type = 'text';
    input.value = value != null ? String(value) : '';
  }
  input.id = 'f_' + field.key;
  input.dataset.key = field.key;
  input.dataset.kind = field.kind;
  input.addEventListener('input', () => onInput(field, readValue(input, field)));
  input.addEventListener('change', () => onInput(field, readValue(input, field)));
  wrap.appendChild(input);

  if (field.hint) {
    const hint = document.createElement('span');
    hint.className = 'hint';
    hint.textContent = t(field.hint);
    wrap.appendChild(hint);
  }
  return wrap;
}

export function readValue(input, field) {
  if (field.kind === 'bool') return input.checked;
  if (field.kind === 'int') return parseInt(input.value || '0', 10) || 0;
  if (field.kind === 'float') {
    const v = parseFloat(input.value);
    return Number.isFinite(v) ? v : 0;
  }
  return input.value;
}

/** 把点分键写入嵌套对象 */
export function setPath(obj, key, value) {
  const parts = key.split('.');
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (typeof cur[parts[i]] !== 'object' || cur[parts[i]] === null) cur[parts[i]] = {};
    cur = cur[parts[i]];
  }
  cur[parts[parts.length - 1]] = value;
}

export function getPath(obj, key) {
  return key.split('.').reduce((acc, part) => (acc === undefined || acc === null ? undefined : acc[part]), obj);
}

/** 渲染指定分组的字段 */
export function renderGroup(container, schema, config, section, onInput) {
  container.innerHTML = '';
  schema
    .filter((f) => f.section === section)
    .forEach((f) => container.appendChild(buildField(f, getPath(config, f.key), onInput)));
}

/** 密钥文本 <-> 数组 */
export function keysToText(cfg) {
  const keys = getPath(cfg, 'network.apiKeys');
  return Array.isArray(keys) ? keys.join('\n') : '';
}

export function textToKeys(text) {
  return String(text || '')
    .split(/[\r\n,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}
