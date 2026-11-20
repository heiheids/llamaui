// 测试聊天页：通过面板代理调用 llama-server，避免跨域
// 支持图片：点按钮选择、Ctrl+V 粘贴、拖拽进输入框
// Test chat page: calls llama-server through the panel proxy (no CORS issues).
// Images can be picked, pasted, or dropped into the composer.

import { api } from './api.js';
import { setBundle, t } from './i18n.js';

const history = [];
let configured = null;
/** 待发送的图片（data URL 数组） */
const pending = [];
/** 图片长边上限，过大先缩放，避免请求体爆掉 */
const MAX_EDGE = 1568;

function el(id) {
  return document.getElementById(id);
}

function addMessage(role, text, kind, images) {
  const box = document.createElement('div');
  box.className = 'msg ' + (kind || role);
  const tag = document.createElement('span');
  tag.className = 'role';
  tag.textContent = role;
  box.appendChild(tag);
  if (images && images.length) {
    const wrap = document.createElement('div');
    wrap.className = 'msg-images';
    images.forEach((src) => {
      const img = document.createElement('img');
      img.src = src;
      img.alt = 'image';
      wrap.appendChild(img);
    });
    box.appendChild(wrap);
  }
  if (text) box.appendChild(document.createTextNode(text));
  el('chat-log').appendChild(box);
  el('chat-log').scrollTop = el('chat-log').scrollHeight;
  return box;
}

function renderPhase(st) {
  const pill = el('chat-phase');
  pill.dataset.phase = st.phase || 'stopped';
  pill.textContent = t('status.' + (st.phase || 'stopped'));
}

/** 把 File 读成 data URL，并按长边缩放 */
function fileToDataUrl(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error(t('err.network')));
    reader.onload = () => {
      const src = String(reader.result || '');
      const img = new Image();
      img.onerror = () => resolve(src);
      img.onload = () => {
        const scale = Math.min(1, MAX_EDGE / Math.max(img.width, img.height));
        if (scale >= 1) {
          resolve(src);
          return;
        }
        const w = Math.round(img.width * scale);
        const h = Math.round(img.height * scale);
        const canvas = document.createElement('canvas');
        canvas.width = w;
        canvas.height = h;
        canvas.getContext('2d').drawImage(img, 0, 0, w, h);
        try {
          resolve(canvas.toDataURL('image/jpeg', 0.9));
        } catch (e) {
          resolve(src);
        }
      };
      img.src = src;
    };
    reader.readAsDataURL(file);
  });
}

async function addImages(files) {
  const list = Array.from(files || []).filter((f) => f && f.type && f.type.startsWith('image/'));
  if (!list.length) return;
  for (const f of list) {
    try {
      const url = await fileToDataUrl(f);
      pending.push(url);
    } catch (e) { /* 单张失败不影响其它 */ }
  }
  renderPending();
}

function renderPending() {
  const strip = el('attach-strip');
  strip.innerHTML = '';
  if (!pending.length) {
    strip.classList.add('hidden');
    return;
  }
  strip.classList.remove('hidden');
  pending.forEach((src, i) => {
    const cell = document.createElement('div');
    cell.className = 'attach-item';
    const img = document.createElement('img');
    img.src = src;
    cell.appendChild(img);
    const del = document.createElement('button');
    del.type = 'button';
    del.className = 'attach-del';
    del.textContent = '×';
    del.title = t('chat.removeImage');
    del.addEventListener('click', () => {
      pending.splice(i, 1);
      renderPending();
    });
    cell.appendChild(del);
    strip.appendChild(cell);
  });
}

function canUseVision() {
  return !!(configured && configured.vision);
}

async function send() {
  const input = el('chat-input');
  const text = input.value.trim();
  if (!text && !pending.length) return;
  if (!configured) {
    addMessage('system', t('chat.empty'), 'error');
    return;
  }
  // 没开识图时不要偷偷丢图；明确提示并只发文字
  let images = pending.slice();
  if (images.length && !canUseVision()) {
    addMessage('system', t('chat.visionOff'), 'error');
    images = [];
  }
  const sentText = text;
  input.value = '';
  pending.length = 0;
  renderPending();

  addMessage('user', sentText, undefined, images);
  if (images.length) {
    const parts = [];
    if (sentText) parts.push({ type: 'text', text: sentText });
    images.forEach((url) => parts.push({ type: 'image_url', image_url: { url } }));
    history.push({ role: 'user', content: parts });
  } else {
    history.push({ role: 'user', content: sentText });
  }

  const status = el('chat-status');
  status.textContent = t('chat.waiting');
  el('btn-send').disabled = true;
  try {
    const res = await api.chat({
      messages: history,
      temperature: configured.sampling && configured.sampling.temp,
      top_p: configured.sampling && configured.sampling.topP,
    });
    const choice = res.choices && res.choices[0];
    const content = (choice && choice.message && choice.message.content) || '';
    addMessage('assistant', content);
    history.push({ role: 'assistant', content });
    status.textContent = '';
  } catch (e) {
    addMessage('system', e.message, 'error');
    status.textContent = e.message;
  } finally {
    el('btn-send').disabled = false;
  }
}

async function main() {
  const boot = await api.bootstrap();
  setBundle(boot.i18n);
  configured = boot.config;
  document.title = t('chat.title');
  el('chat-title').textContent = t('chat.title');
  el('chat-back').textContent = t('button.back');
  el('btn-send').textContent = t('chat.send');
  el('btn-clear').textContent = t('chat.clear');
  el('btn-image').textContent = t('chat.addImage');
  el('chat-input').placeholder = t('chat.placeholder') + '\n' + t('chat.imageHint');

  const selected = (boot.models || []).find((m) => m.id === boot.selected);
  el('chat-model').textContent = selected ? `${t('chat.model')}: ${selected.name}` : t('status.noModel');
  renderPhase(boot.status);

  el('composer').addEventListener('submit', (ev) => {
    ev.preventDefault();
    send();
  });
  el('chat-input').addEventListener('keydown', (ev) => {
    if ((ev.ctrlKey || ev.metaKey) && ev.key === 'Enter') {
      ev.preventDefault();
      send();
    }
  });
  el('btn-clear').addEventListener('click', () => {
    history.length = 0;
    pending.length = 0;
    renderPending();
    el('chat-log').innerHTML = '';
    el('chat-status').textContent = '';
  });

  // 选择图片
  el('btn-image').addEventListener('click', () => el('file-input').click());
  el('file-input').addEventListener('change', async (ev) => {
    await addImages(ev.target.files);
    ev.target.value = '';
  });

  // 粘贴图片（Ctrl+V）
  document.addEventListener('paste', async (ev) => {
    const items = (ev.clipboardData && ev.clipboardData.items) || [];
    const files = [];
    for (const it of items) {
      if (it.kind === 'file' && it.type.startsWith('image/')) {
        const f = it.getAsFile();
        if (f) files.push(f);
      }
    }
    if (files.length) {
      ev.preventDefault();
      await addImages(files);
    }
  });

  // 拖拽进窗口
  const composer = el('composer');
  composer.addEventListener('dragover', (ev) => {
    ev.preventDefault();
    composer.classList.add('drop-active');
  });
  composer.addEventListener('dragleave', () => composer.classList.remove('drop-active'));
  composer.addEventListener('drop', async (ev) => {
    ev.preventDefault();
    composer.classList.remove('drop-active');
    if (ev.dataTransfer && ev.dataTransfer.files) {
      await addImages(ev.dataTransfer.files);
    }
  });

  setInterval(async () => {
    try {
      const data = await api.status(0);
      renderPhase(data.status);
      el('chat-phase').dataset.phase = data.status.phase;
    } catch (e) { /* 忽略 */ }
  }, 3000);
}

main();
