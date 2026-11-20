// 多语言运行时：文案全部来自 /api/bootstrap 语言包，页面不硬编码
// i18n runtime: all copy comes from the bootstrap language bundle.

let strings = {};
let current = 'en-US';

export function setBundle(bundle) {
  strings = (bundle && bundle.strings) || {};
  current = (bundle && bundle.lang) || 'en-US';
  document.documentElement.lang = current;
}

export function lang() {
  return current;
}

export function t(key, fallback) {
  const v = get(key);
  if (v !== undefined) return v;
  return fallback !== undefined ? fallback : key;
}

function get(key) {
  let cur = strings;
  for (const part of key.split('.')) {
    if (cur == null || typeof cur !== 'object') return undefined;
    cur = cur[part];
  }
  return typeof cur === 'string' ? cur : undefined;
}

/** 应用 data-i18n / data-i18n-placeholder 等静态文案 */
export function applyStatic(root) {
  const scope = root || document;
  scope.querySelectorAll('[data-i18n]').forEach((el) => {
    const key = el.getAttribute('data-i18n');
    const val = get(key);
    if (val !== undefined) el.textContent = val;
  });
  scope.querySelectorAll('[data-i18n-placeholder]').forEach((el) => {
    const val = get(el.getAttribute('data-i18n-placeholder'));
    if (val !== undefined) el.placeholder = val;
  });
  scope.querySelectorAll('[data-i18n-title]').forEach((el) => {
    const val = get(el.getAttribute('data-i18n-title'));
    if (val !== undefined) el.title = val;
  });
}
