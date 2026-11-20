// 前端 API 封装：统一处理 JSON 请求与错误文案
// Front-end API wrapper: unified JSON requests and error messages.

import { t } from './i18n.js';

async function request(method, path, body) {
  const opts = { method, headers: {} };
  if (body !== undefined) {
    opts.headers['Content-Type'] = 'application/json';
    opts.body = JSON.stringify(body);
  }
  let resp;
  try {
    resp = await fetch(path, opts);
  } catch (e) {
    throw new Error(t('err.network'));
  }
  let data = null;
  try {
    data = await resp.json();
  } catch (e) {
    throw new Error(t('err.network'));
  }
  if (!data || data.ok !== true) {
    const code = data && data.code ? data.code : 'err.badPayload';
    const msg = data && data.message ? data.message : t(code);
    throw new Error(msg);
  }
  return data.data;
}

export const api = {
  bootstrap: () => request('GET', '/api/bootstrap'),
  status: (since) => request('GET', `/api/status?since=${since || 0}`),
  models: () => request('GET', '/api/models'),
  importPaths: (paths) => request('POST', '/api/models/import', { paths }),
  scan: () => request('POST', '/api/models/scan', {}),
  removeModel: (id) => request('POST', '/api/models/remove', { id }),
  selectModel: (id) => request('POST', '/api/models/select', { id }),
  saveConfig: (patch) => request('POST', '/api/config', patch),
  newKey: () => request('POST', '/api/keys/new', {}),
  start: (id) => request('POST', '/api/server/start', { id: id || '' }),
  stop: () => request('POST', '/api/server/stop', {}),
  log: (since) => request('GET', `/api/log?since=${since || 0}`),
  clearLog: () => request('POST', '/api/log/clear', {}),
  preview: () => request('GET', '/api/preview'),
  chat: (payload) => request('POST', '/api/chat', payload),
};
