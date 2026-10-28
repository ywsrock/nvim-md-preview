const mermaid = window.mermaid;

// ── DOM refs ──────────────────────────────────────────────────
const $ = id => document.getElementById(id);
// /print で開かれたとき（PDF 用）に、サーバーが内容と出力の設定を埋め込む
const PRINT = window.__MDP_PRINT__ || null;

const contentEl   = $('content');
const titleEl     = $('title');
const statusDot   = $('statusDot');
const themeBtn    = $('themeBtn');
const skeleton    = $('loadingSkeleton');
const overlay     = $('overlay');
const ovPanzoom   = $('overlay-panzoom');
const ovInner     = $('overlay-panzoom-inner');

const MOON_SVG = '<path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>';
const SUN_SVG  = '<circle cx="12" cy="12" r="5" stroke="currentColor" stroke-width="1.5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>';
const COPY_ICON  = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><rect x="9" y="9" width="13" height="13" rx="2" stroke="currentColor" stroke-width="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" stroke="currentColor" stroke-width="2"/></svg>';
const CHECK_ICON = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path d="M20 6 9 17l-5-5" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';
const EXPAND_SVG = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none"><path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>';

// ── State ─────────────────────────────────────────────────────
let currentTheme = 'dark';
let mermaidCounter = 0;
const lastGoodSvg = {};
let ovSvgSource = '';
let ovBaseW = 0, ovBaseH = 0, ovZoom = 1, ovFitMode = 'width';
let ovDragging = false, ovStartX = 0, ovStartY = 0, ovStartL = 0, ovStartT = 0;

// ── Theme ─────────────────────────────────────────────────────
function applyTheme(theme) {
  currentTheme = theme;
  document.documentElement.setAttribute('data-theme', theme);
  $('themeIcon').innerHTML = theme === 'dark' ? MOON_SVG : SUN_SVG;
  const hljsLink = $('hljs-theme');
  if (hljsLink) hljsLink.href = theme === 'dark'
    ? '/lib/hljs-dark.min.css'
    : '/lib/hljs-light.min.css';
  mermaid.initialize({ startOnLoad: false, theme: theme === 'dark' ? 'dark' : 'default', securityLevel: 'strict' });
}

themeBtn.addEventListener('click', () => {
  applyTheme(currentTheme === 'dark' ? 'light' : 'dark');
  contentEl.querySelectorAll('.mermaid-block.mermaid-rendered').forEach(b => {
    b.classList.remove('mermaid-rendered');
    renderMermaidBlock(b);
  });
});

// ── Mermaid ───────────────────────────────────────────────────
async function renderMermaidBlock(block) {
  const source = decodeURIComponent(block.dataset.mermaidSource || '');
  if (!source.trim()) return;
  const wrap = block.querySelector('.mermaid-svg-wrap');
  if (!wrap) return;
  try {
    mermaidCounter++;
    const { svg } = await mermaid.render('mmd-' + mermaidCounter, source);
    wrap.innerHTML = svg;
    block.classList.add('mermaid-rendered');
    block.classList.remove('mermaid-errored');
    lastGoodSvg[block.id] = svg;
    block.querySelectorAll('.error,.error-icon,.error-text,.errorText').forEach(n => n.remove());
  } catch(e) {
    block.classList.add('mermaid-errored');
    if (lastGoodSvg[block.id]) wrap.innerHTML = lastGoodSvg[block.id];
    let errEl = block.querySelector('.mermaid-error');
    if (!errEl) { errEl = document.createElement('div'); errEl.className = 'mermaid-error'; block.appendChild(errEl); }
    errEl.textContent = String(e?.message || e || 'Invalid diagram')
      .split('\n').find(Boolean)?.replace(/\s*mermaid version[\d.\s]+$/i,'').trim() || 'Invalid diagram';
  }
}

async function renderAllMermaid() {
  for (const b of contentEl.querySelectorAll('.mermaid-block[data-mermaid-source]:not(.mermaid-rendered)')) {
    await renderMermaidBlock(b);
  }
}

// ── Code: syntax highlight + lang badge + copy button ────────
function attachCodeFeatures(root) {
  root.querySelectorAll('.code-wrap').forEach(wrap => {
    const pre  = wrap.querySelector('pre');
    const code = pre?.querySelector('code');
    if (!pre || !code) return;

    // highlight.js でシンタックスハイライト（未処理のブロックのみ）
    if (window.hljs && !code.dataset.highlighted) {
      window.hljs.highlightElement(code);
    }

    // コピーボタン（既に付いていたらスキップ）
    if (wrap.querySelector('.copy-btn')) return;
    const btn = document.createElement('button');
    btn.className = 'copy-btn'; btn.title = 'Copy'; btn.innerHTML = COPY_ICON;
    btn.addEventListener('click', async e => {
      e.stopPropagation();
      // highlight.js 適用後もテキストは textContent で取れる
      const text = code.textContent || '';
      try {
        await navigator.clipboard.writeText(text);
        btn.innerHTML = CHECK_ICON; btn.classList.add('copied');
        setTimeout(() => { btn.innerHTML = COPY_ICON; btn.classList.remove('copied'); }, 1500);
      } catch(_) {}
    });
    wrap.appendChild(btn);
  });
}

// ── Content update ────────────────────────────────────────────
// 文字列の簡易ハッシュ（ブロックの同一性判定に使う）
function hashStr(str) {
  let h = 5381;
  for (let i = 0; i < str.length; i++) h = ((h << 5) + h + str.charCodeAt(i)) | 0;
  return (h >>> 0).toString(36) + ':' + str.length;
}

// 新しいブロック列を contentEl に差分適用する。
// data-key が同じ既存ノードはそのまま使い回す（Mermaid の描画結果やスクロール位置を保つ）
function reconcile(parent, newChildren) {
  const pool = new Map();
  for (const n of Array.from(parent.children)) {
    const k = n.dataset.key;
    if (!pool.has(k)) pool.set(k, []);
    pool.get(k).push(n);
  }
  const result = newChildren.map(n => {
    const list = pool.get(n.dataset.key);
    return list && list.length ? list.shift() : n;
  });
  result.forEach((node, i) => {
    const cur = parent.children[i];
    if (cur !== node) parent.insertBefore(node, cur || null);
  });
  while (parent.children.length > result.length) parent.lastElementChild.remove();
}

async function renderContent(html) {
  // 新しい内容は切り離した要素の上で全部加工してから、最後に差分だけ DOM に入れる
  const tmp = document.createElement('div');
  tmp.innerHTML = html;

  // pulldown-cmark の ```mermaid → mermaid-block に変換
  tmp.querySelectorAll('code.language-mermaid').forEach(code => {
    const source = encodeURIComponent(code.textContent || '');
    const id = 'mmd-' + hashStr(source);
    const block = document.createElement('div');
    block.className = 'mermaid-block'; block.id = id;
    block.dataset.mermaidSource = source;
    block.innerHTML =
      `<button class="mermaid-expand-btn" data-expand="${id}" title="Full screen">${EXPAND_SVG}</button>` +
      `<div class="mermaid-svg-wrap"><p style="color:var(--fg-muted);font-size:.85rem">Rendering…</p></div>`;
    code.closest('pre').replaceWith(block);
  });

  // コードブロックに code-wrap + lang badge を付ける
  // pulldown-cmark は <pre><code class="language-rust"> の形式で出力する
  tmp.querySelectorAll('pre:not(.mermaid-block pre)').forEach(pre => {
    if (pre.closest('.code-wrap')) return;
    const code = pre.querySelector('code');
    // "language-rust" → "rust" を抽出
    const langClass = code?.className?.match(/(?:^|\s)language-(\S+)/)?.[1] || '';
    const wrap = document.createElement('div');
    wrap.className = 'code-wrap';
    pre.replaceWith(wrap);
    if (langClass) {
      const badge = document.createElement('span');
      badge.className = 'code-lang'; badge.textContent = langClass;
      wrap.appendChild(badge);
    }
    wrap.appendChild(pre);
  });

  // 加工はすべて tmp 上で行う（DOM に入れる前なのでちらつかない）
  rewriteRelativeImages(tmp);
  attachCodeFeatures(tmp);
  renderGithubAlerts(tmp);
  addHeadingAnchors(tmp);
  replaceEmoji(tmp);
  renderMath(tmp);

  // ブロックごとのキー。Mermaid はソースで、それ以外は加工後の HTML で判定する
  for (const el of Array.from(tmp.children)) {
    el.dataset.key = el.classList.contains('mermaid-block')
      ? 'mmd:' + el.dataset.mermaidSource
      : hashStr(el.outerHTML);
  }

  reconcile(contentEl, Array.from(tmp.children));
  skeleton.style.display = 'none';
  contentEl.style.display = '';

  applyDiff();
  buildToc();
  if (!slidesEl.hidden) { buildSlides(); showSlide(slideIdx); }
  // 検索中なら新しい内容で検索し直す
  if (!searchBar.hidden && searchInput.value) runSearch();
  await renderAllMermaid();
  positionDiffMarkers();
  if (pendingScroll) { scrollToLine(pendingScroll.line, pendingScroll.total); pendingScroll = null; }
}

// ── Markdown 以外のファイル: 行番号付きのハイライトしたコード ──
let viewKind = 'markdown';
// Neovim の filetype → highlight.js の言語名（同じ名前のものは不要）
const LANG_ALIAS = {
  sh: 'bash', zsh: 'bash', html: 'xml', svg: 'xml', typescriptreact: 'typescript',
  javascriptreact: 'javascript', tex: 'latex', dosini: 'ini', conf: 'ini', make: 'makefile',
  cs: 'csharp', jsonc: 'json', yml: 'yaml', text: 'plaintext', gitcommit: 'plaintext',
};
function escapeHtml(t) { return t.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;'); }
// ハイライトした HTML を行ごとに分ける。複数行にまたがる <span> は行末で閉じ、次の行で開き直す
function highlightLines(text, lang) {
  const hl = window.hljs;
  const name = hl && [lang, LANG_ALIAS[lang]].find(l => l && hl.getLanguage(l));
  const html = name ? hl.highlight(text, { language: name, ignoreIllegals: true }).value : escapeHtml(text);
  const lines = [], open = [];
  let cur = '';
  for (const tok of html.split(/(<span[^>]*>|<\/span>|\n)/)) {
    if (!tok) continue;
    if (tok === '\n') { lines.push(cur + '</span>'.repeat(open.length)); cur = open.join(''); }
    else if (tok.startsWith('<span')) { open.push(tok); cur += tok; }
    else if (tok === '</span>') { open.pop(); cur += tok; }
    else cur += tok;
  }
  lines.push(cur + '</span>'.repeat(open.length));
  return lines;
}
async function renderCode(text, lang) {
  const lines = highlightLines(text, lang);
  const view = document.createElement('div');
  view.className = 'code-view hljs';
  // 各行に元の行番号の印を入れる（スクロール同期・クリックで移動・差分表示に使う）
  view.innerHTML = lines.map((l, i) =>
    `<div class="cv-line"><span class="src-line" data-line="${i + 1}"></span>` +
    `<span class="cv-ln">${i + 1}</span><code class="cv-code">${l || ' '}</code></div>`).join('');
  contentEl.replaceChildren(view);
  skeleton.style.display = 'none';
  contentEl.style.display = '';
  applyDiff();
  if (!searchBar.hidden && searchInput.value) runSearch();
  positionDiffMarkers();
  if (pendingScroll) { scrollToLine(pendingScroll.line, pendingScroll.total); pendingScroll = null; }
}

// ── Overlay ───────────────────────────────────────────────────
function openOverlay(svgHtml) {
  ovSvgSource = svgHtml;
  ovInner.innerHTML = svgHtml;
  overlay.classList.add('active');
  const svg = ovInner.querySelector('svg');
  if (svg) {
    const vb = svg.getAttribute('viewBox');
    if (vb) {
      const p = vb.trim().split(/\s+/).map(Number);
      if (p.length === 4) { ovBaseW = p[2]; ovBaseH = p[3]; }
    } else {
      ovBaseW = parseFloat(svg.getAttribute('width')) || 800;
      ovBaseH = parseFloat(svg.getAttribute('height')) || 600;
    }
  }
  ovZoom = 1; ovFitMode = 'width';
  updateOverlayFit(); updateOverlayFitBtns();
}

function closeOverlay() { overlay.classList.remove('active'); ovInner.innerHTML = ''; }

function applyOverlayZoom() {
  const svg = ovInner.querySelector('svg');
  if (!svg || !ovBaseW || !ovBaseH) return;
  svg.setAttribute('width',  String(Math.max(1, Math.round(ovBaseW * ovZoom))));
  svg.setAttribute('height', String(Math.max(1, Math.round(ovBaseH * ovZoom))));
  $('ovZoomReset').textContent = Math.round(ovZoom * 100) + '%';
}

function updateOverlayFit() {
  if (!ovBaseW || !ovBaseH) return;
  const cw = ovPanzoom.clientWidth || ovBaseW;
  const ch = ovPanzoom.clientHeight || ovBaseH;
  if (ovFitMode === 'width')  ovZoom = cw / ovBaseW;
  if (ovFitMode === 'height') ovZoom = ch / ovBaseH;
  ovZoom = Math.min(8, Math.max(0.1, ovZoom));
  applyOverlayZoom();
}

function updateOverlayFitBtns() {
  $('ovFitWidth').style.borderColor  = ovFitMode === 'width'  ? 'var(--accent)' : '';
  $('ovFitHeight').style.borderColor = ovFitMode === 'height' ? 'var(--accent)' : '';
}

$('ovZoomIn').addEventListener('click',    () => { ovFitMode='none'; ovZoom*=1.2; applyOverlayZoom(); updateOverlayFitBtns(); });
$('ovZoomOut').addEventListener('click',   () => { ovFitMode='none'; ovZoom/=1.2; applyOverlayZoom(); updateOverlayFitBtns(); });
$('ovZoomReset').addEventListener('click', () => { ovFitMode='none'; ovZoom=1;    applyOverlayZoom(); updateOverlayFitBtns(); });
$('ovFitWidth').addEventListener('click',  () => { ovFitMode = ovFitMode==='width'  ? 'none' : 'width';  updateOverlayFit(); updateOverlayFitBtns(); });
$('ovFitHeight').addEventListener('click', () => { ovFitMode = ovFitMode==='height' ? 'none' : 'height'; updateOverlayFit(); updateOverlayFitBtns(); });
$('ovClose').addEventListener('click', closeOverlay);
document.addEventListener('keydown', e => { if (e.key === 'Escape') closeOverlay(); });

// pan (drag)
ovPanzoom.addEventListener('mousedown', e => {
  ovDragging=true; ovPanzoom.classList.add('dragging');
  ovStartX=e.clientX; ovStartY=e.clientY; ovStartL=ovPanzoom.scrollLeft; ovStartT=ovPanzoom.scrollTop;
});
window.addEventListener('mousemove', e => {
  if (!ovDragging) return;
  ovPanzoom.scrollLeft = ovStartL-(e.clientX-ovStartX);
  ovPanzoom.scrollTop  = ovStartT-(e.clientY-ovStartY);
});
window.addEventListener('mouseup', () => { ovDragging=false; ovPanzoom.classList.remove('dragging'); });

// wheel zoom in overlay
ovPanzoom.addEventListener('wheel', e => {
  if (e.ctrlKey || e.metaKey) {
    e.preventDefault(); ovFitMode='none';
    ovZoom *= e.deltaY > 0 ? 1/1.1 : 1.1;
    applyOverlayZoom(); updateOverlayFitBtns();
  }
}, { passive: false });

// export SVG
$('ovExport').addEventListener('click', () => {
  if (!ovSvgSource) return;
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([ovSvgSource], { type: 'image/svg+xml' }));
  a.download = 'diagram.svg'; a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 2000);
});

// ── Event delegation: expand button ──────────────────────────
contentEl.addEventListener('click', e => {
  const btn = e.target.closest('[data-expand]');
  if (!btn) return;
  const block = document.getElementById(btn.dataset.expand);
  if (!block) return;
  const wrap = block.querySelector('.mermaid-svg-wrap');
  if (!wrap) return;
  // SVG HTML 文字列を openOverlay に渡す（mdkite 方式）
  const svgHtml = wrap.innerHTML;
  if (svgHtml && svgHtml.includes('<svg')) openOverlay(svgHtml);
});

// ── Front matter パネル ───────────────────────────────────────
function updateFrontMatter(fm) {
  let panel = document.getElementById('fm-panel');
  if (!fm) {
    if (panel) panel.style.display = 'none';
    return;
  }
  if (!panel) {
    panel = document.createElement('details');
    panel.id = 'fm-panel';
    panel.innerHTML =
      '<summary>Front matter</summary>' +
      '<pre id="fm-content"></pre>';
    contentEl.parentNode.insertBefore(panel, contentEl);
  }
  const pre = document.getElementById('fm-content');
  if (pre) pre.textContent = fm;
  panel.style.display = '';
}

// ── GitHub Alerts ─────────────────────────────────────────────
const ALERT_ICONS = {
  note:      '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>',
  tip:       '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.595h4.017c.084-.699.337-1.184.621-1.595.203-.292.45-.584.673-.848.29-.342.214-.253.214-.253.56-.679.984-1.32.984-2.304 0-2.06-1.637-3.75-4-3.75ZM6 7.058C5.763 6.66 5.5 6.04 5.5 5.25c0-1.311.989-2.25 2.5-2.25s2.5.939 2.5 2.25c0 .79-.263 1.41-.5 1.808V7.5h-4V7.058Zm2 7.942a1.5 1.5 0 0 1-1.422-1h2.844A1.5 1.5 0 0 1 8 15Zm3.479-4H4.521A5.578 5.578 0 0 0 4.5 11h7a5.578 5.578 0 0 0-.021.058Z"/></svg>',
  warning:   '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>',
  caution:   '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z"/></svg>',
  important: '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/></svg>',
};
const ALERT_LABEL = { note:'Note', tip:'Tip', warning:'Warning', caution:'Caution', important:'Important' };

// Obsidian の callout 名 → 表示に使う種類（GitHub Alerts の 5 種 + 追加分）
const CALLOUT_ALIAS = {
  note:'note', tip:'tip', hint:'tip', important:'important', warning:'warning', attention:'warning',
  caution:'caution', danger:'caution', error:'caution', info:'info', todo:'todo',
  success:'success', check:'success', done:'success', question:'question', help:'question', faq:'question',
  failure:'failure', fail:'failure', missing:'failure', bug:'bug', example:'example',
  quote:'quote', cite:'quote', abstract:'abstract', summary:'abstract', tldr:'abstract',
};
const CALLOUT_ICON_OF = { info:'note', todo:'important', success:'tip', question:'warning',
  failure:'caution', bug:'caution', example:'important', quote:'note', abstract:'important' };

// > [!TYPE] / > [!type]- 任意のタイトル （- で折りたたみ・+ で開いた状態の折りたたみ）
function renderGithubAlerts(root) {
  root.querySelectorAll('blockquote').forEach(bq => {
    const firstP = bq.querySelector('p');
    if (!firstP) return;
    const re = /^\[!([A-Za-z]+)\]([+-])?[ \t]*([^<\n]*)(<br\s*\/?>|\n)?/;
    const match = firstP.innerHTML.match(re);
    if (!match) return;
    const name = match[1].toLowerCase();
    const type = CALLOUT_ALIAS[name];
    if (!type) return;
    const fold = match[2];
    const customTitle = match[3].trim();
    firstP.innerHTML = firstP.innerHTML.replace(re, '');
    if (!firstP.innerHTML.trim()) firstP.remove();

    const label = customTitle || (name.charAt(0).toUpperCase() + name.slice(1));
    const icon = ALERT_ICONS[type] || ALERT_ICONS[CALLOUT_ICON_OF[type]] || '';
    if (fold) {
      // 折りたたみ: blockquote を <details> に置き換える
      const details = document.createElement('details');
      details.className = `markdown-alert markdown-alert-${type}`;
      if (fold === '+') details.open = true;
      const summary = document.createElement('summary');
      summary.className = 'markdown-alert-title';
      summary.innerHTML = icon + ' ' + label;
      details.appendChild(summary);
      while (bq.firstChild) details.appendChild(bq.firstChild);
      bq.replaceWith(details);
    } else {
      bq.classList.add('markdown-alert', `markdown-alert-${type}`);
      const title = document.createElement('div');
      title.className = 'markdown-alert-title';
      title.innerHTML = icon + ' ' + label;
      bq.insertBefore(title, bq.firstChild);
    }
  });
}

// ── 見出しアンカーリンク ──────────────────────────────────────
function addHeadingAnchors(root) {
  root.querySelectorAll('h1,h2,h3,h4,h5,h6').forEach(h => {
    // 既にアンカーがあればスキップ
    if (h.querySelector('.anchor-link')) return;
    const id = h.textContent.trim()
      .toLowerCase()
      .replace(/\s+/g, '-')
      .replace(/[^\w\u3040-\u30FF\u3400-\u4DBF\u4E00-\u9FFF-]/g, '');
    if (!id) return;
    h.id = id;
    const a = document.createElement('a');
    a.className = 'anchor-link';
    a.href = '#' + id;
    a.title = 'Link to this heading';
    a.innerHTML = '<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor"><path d="m7.775 3.275 1.25-1.25a3.5 3.5 0 1 1 4.95 4.95l-2.5 2.5a3.5 3.5 0 0 1-4.95 0 .751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018 1.998 1.998 0 0 0 2.83 0l2.5-2.5a2.002 2.002 0 0 0-2.83-2.83l-1.25 1.25a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042Zm-4.69 9.64a1.998 1.998 0 0 0 2.83 0l1.25-1.25a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042l-1.25 1.25a3.5 3.5 0 1 1-4.95-4.95l2.5-2.5a3.5 3.5 0 0 1 4.95 0 .751.751 0 0 1-.018 1.042.751.751 0 0 1-1.042.018 1.998 1.998 0 0 0-2.83 0l-2.5 2.5a1.998 1.998 0 0 0 0 2.83Z"/></svg>';
    h.appendChild(a);
  });
}

// ── 絵文字 :name: → Unicode ──────────────────────────────────
const EMOJI = {
  '+1':'👍','-1':'👎','100':'💯','tada':'🎉','sparkles':'✨','star':'⭐',
  'heart':'❤️','fire':'🔥','rocket':'🚀','check':'✅','x':'❌','warning':'⚠️',
  'bulb':'💡','eyes':'👀','wave':'👋','clap':'👏','pray':'🙏','muscle':'💪',
  'smile':'😊','grin':'😁','laughing':'😆','wink':'😉','blush':'😊',
  'sweat_smile':'😅','joy':'😂','rofl':'🤣','sob':'😭','cry':'😢',
  'angry':'😠','rage':'😡','confused':'😕','thinking':'🤔','nerd_face':'🤓',
  'sunglasses':'😎','partying_face':'🥳','raised_hands':'🙌','ok_hand':'👌',
  'point_right':'👉','point_left':'👈','point_up':'👆','point_down':'👇',
  'thumbsup':'👍','thumbsdown':'👎','v':'✌️','crossed_fingers':'🤞',
  'bug':'🐛','memo':'📝','pencil':'✏️','book':'📖','books':'📚',
  'computer':'💻','keyboard':'⌨️','phone':'📱','email':'📧','link':'🔗',
  'lock':'🔒','unlock':'🔓','key':'🔑','hammer':'🔨','wrench':'🔧',
  'gear':'⚙️','package':'📦','inbox_tray':'📥','outbox_tray':'📤',
  'white_check_mark':'✅','ballot_box_with_check':'☑️','negative_squared_cross_mark':'❎',
  'information_source':'ℹ️','question':'❓','exclamation':'❗',
  'arrow_up':'⬆️','arrow_down':'⬇️','arrow_left':'⬅️','arrow_right':'➡️',
  'zap':'⚡','sunny':'☀️','cloud':'☁️','snowflake':'❄️','umbrella':'☂️',
  'coffee':'☕','pizza':'🍕','beer':'🍺','cake':'🎂','tada':'🎉',
  'trophy':'🏆','medal_sports':'🏅','dart':'🎯','game_die':'🎲',
  'art':'🎨','musical_note':'🎵','microphone':'🎤','headphones':'🎧',
  'earth_asia':'🌏','earth_americas':'🌎','earth_africa':'🌍','globe_with_meridians':'🌐',
  'japan':'🗾','construction':'🚧','no_entry':'⛔','stop_sign':'🛑',
  'recycle':'♻️','sos':'🆘','new':'🆕','up':'🆙','cool':'🆒','free':'🆓',
  'abc':'🔤','1234':'🔢','symbols':'🔣','capital_abcd':'🔠',
  'white_square_button':'🔳','black_square_button':'🔲',
};

function replaceEmoji(root) {
  // テキストノードのみ走査（code/pre 内は除外）
  const walker = document.createTreeWalker(
    root, NodeFilter.SHOW_TEXT,
    { acceptNode: n => {
      const p = n.parentElement;
      if (!p) return NodeFilter.FILTER_REJECT;
      if (p.closest('code,pre,.katex')) return NodeFilter.FILTER_REJECT;
      return NodeFilter.FILTER_ACCEPT;
    }}
  );
  const nodes = [];
  while (walker.nextNode()) nodes.push(walker.currentNode);
  nodes.forEach(node => {
    if (!node.textContent.includes(':')) return;
    const replaced = node.textContent.replace(/:([a-z0-9_+\-]+):/g, (m, name) => EMOJI[name] || m);
    if (replaced !== node.textContent) node.textContent = replaced;
  });
}

// ── KaTeX 数式レンダリング ────────────────────────────────────
function renderMath(root) {
  if (!window.renderMathInElement) return;
  renderMathInElement(root, {
    // $ / $$ は Rust 側（pulldown-cmark の数式対応）で \( \) / \[ \] に変換済み。
    // ここで $ を区切りにすると "$5 and $10" のような金額まで数式になるので使わない
    delimiters: [
      { left: '\\[', right: '\\]', display: true },
      { left: '\\(', right: '\\)', display: false },
    ],
    throwOnError: false,
    // コードブロック内はスキップ
    ignoredTags: ['script','noscript','style','textarea','pre','code'],
  });
}

// ── 相対パス画像を /assets?p= に書き換え ────────────────────
function rewriteRelativeImages(root) {
  root.querySelectorAll('img[src]').forEach(img => {
    const src = img.getAttribute('src');
    // 絶対URLや data URI はスキップ
    if (!src || /^(?:[a-zA-Z][a-zA-Z0-9+.-]*:|\/\/|\/|#|data:)/.test(src)) return;
    img.setAttribute('src', `/assets?p=${encodeURIComponent(src)}`);
  });
}

// ── スクロール同期（手動スクロール中は一時停止） ─────────────
let scrollSyncPaused = false;
let scrollSyncTimer  = null;
window.addEventListener('wheel',     () => { scrollSyncPaused = true; clearTimeout(scrollSyncTimer); scrollSyncTimer = setTimeout(() => { scrollSyncPaused = false; }, 3000); }, { passive: true });
window.addEventListener('touchmove', () => { scrollSyncPaused = true; clearTimeout(scrollSyncTimer); scrollSyncTimer = setTimeout(() => { scrollSyncPaused = false; }, 3000); }, { passive: true });

// 内容更新の直後に届いたスクロールは、描画が終わってから反映する
let pendingScroll = null;
let lastScroll = { line: 1, total: 1 };

// 行マーカー（<span class="src-line" data-line>）の一覧を行番号順に返す
function lineMarkers() {
  return Array.from(contentEl.querySelectorAll('.src-line'))
    .map(el => ({ line: +el.dataset.line, top: el.getBoundingClientRect().top + window.scrollY }))
    .filter(m => Number.isFinite(m.line))
    .sort((a, b) => a.line - b.line);
}

// カーソル行 line に対応するページ上の Y 座標。
// 行番号がぴったり一致するマーカーが無ければ、前後のマーカーの間を行数で按分する
function yForLine(line, total) {
  const markers = lineMarkers();
  if (!markers.length) {
    const ratio = Math.max(0, Math.min(1, (line - 1) / Math.max(1, total - 1)));
    return (document.documentElement.scrollHeight - window.innerHeight) * ratio;
  }
  let before = null, after = null;
  for (const m of markers) {
    if (m.line <= line) before = m; else { after = m; break; }
  }
  if (!before) return 0;
  const endTop = contentEl.getBoundingClientRect().bottom + window.scrollY;
  const nextLine = after ? after.line : total + 1;
  const nextTop  = after ? after.top  : endTop;
  const span = Math.max(1, nextLine - before.line);
  return before.top + (nextTop - before.top) * Math.min(1, (line - before.line) / span);
}

function scrollToLine(line, total) {
  lastScroll = { line, total };
  if (scrollSyncPaused) return;
  // カーソル行が画面の上から 30% の位置に来るようにする
  const y = yForLine(line, total) - window.innerHeight * 0.3;
  window.scrollTo({ top: Math.max(0, y), behavior: 'instant' });
}

// ── クリックでエディタの該当行へジャンプ ──────────────────────
contentEl.classList.add('click-sync');
contentEl.addEventListener('click', e => {
  // リンク・ボタン・図の操作、テキスト選択中はジャンプしない
  if (e.target.closest('a,button,input,details,summary,.mermaid-block,.copy-btn')) return;
  if (String(window.getSelection?.() || '').length) return;
  const y = e.clientY + window.scrollY;
  let best = null;
  for (const el of contentEl.querySelectorAll('.src-line')) {
    const top = el.getBoundingClientRect().top + window.scrollY;
    if (top <= y + 1 && (!best || top >= best.top)) best = { top, line: +el.dataset.line };
  }
  if (best && wsConn && wsConn.readyState === WebSocket.OPEN) {
    wsConn.send(JSON.stringify({ type: 'goto', line: best.line }));
  }
});

// ── WebSocket ─────────────────────────────────────────────────
let wsConn = null;
let rendering = false;
function connect() {
  const ws = new WebSocket(`ws://${location.host}/ws`);
  wsConn = ws;
  ws.onopen  = () => { statusDot.className = 'status-dot connected'; statusDot.title = 'Connected'; };
  ws.onmessage = async e => {
    const data = JSON.parse(e.data);

    // スクロール同期メッセージ
    if (data.type === 'scroll') {
      if (!slidesEl.hidden) { const i = slideForLine(data.line); if (i !== slideIdx) showSlide(i); return; }
      if (rendering) pendingScroll = { line: data.line, total: data.total };
      else scrollToLine(data.line, data.total);
      return;
    }
    if (data.type === 'diff') {
      diffState = data;
      applyDiffColors();
      if (!rendering) applyDiff();
      return;
    }

    // コンテンツ更新（Markdown は描画、それ以外はハイライトしたコード）
    document.title = data.title;
    titleEl.textContent = data.title;
    viewKind = data.kind === 'code' ? 'code' : 'markdown';
    document.body.dataset.kind = viewKind;
    rendering = true;
    try {
      if (viewKind === 'code') {
        updateFrontMatter(null);
        document.body.classList.remove('toc-open');
        await renderCode(data.text || '', data.lang || '');
      } else {
        updateFrontMatter(data.front_matter || null);
        await renderContent(data.html);
      }
    } finally { rendering = false; }
  };
  ws.onclose = () => { statusDot.className = 'status-dot error'; statusDot.title = 'Disconnected'; setTimeout(connect, 3000); };
  ws.onerror = () => ws.close();
}

// テーマ: ユーザーが切り替えたもの（localStorage）> 設定（/settings）> OS の設定
const prefersDark = window.matchMedia('(prefers-color-scheme: dark)');
function storedTheme() { try { return localStorage.getItem('mdp-theme'); } catch (_) { return null; } }
let configuredTheme = 'auto';
function resolveTheme() {
  const t = storedTheme() || configuredTheme;
  return t === 'auto' ? (prefersDark.matches ? 'dark' : 'light') : t;
}
// PDF 用の印刷ページは常にライトテーマ
applyTheme(PRINT ? 'light' : resolveTheme());
// キーボードショートカット（t / / / Ctrl+F / p）。browser_keys = false なら使わない。
// Vimium などブラウザの Vim 系拡張とキーがぶつかるため、無効にできるようにしている
let browserKeys = true;
fetch('/settings').then(r => r.json()).then(cfg => {
  if (PRINT) return;
  configuredTheme = cfg.theme || 'auto';
  if (resolveTheme() !== currentTheme) themeBtn.click();
  browserKeys = cfg.browser_keys !== false;
  // export.formats に無い形式はメニューから外し、1 つも無ければ Export ボタンごと隠す
  if (Array.isArray(cfg.export_formats)) {
    const allowed = new Set(cfg.export_formats), menu = $('exportMenu');
    menu.querySelectorAll('[data-format]').forEach(b => { b.hidden = !allowed.has(b.dataset.format); });
    menu.querySelector('.export-sep').hidden = !allowed.has('markdown') || allowed.size === 1;
    $('exportBtn').parentElement.hidden = allowed.size === 0;
  }
  if (!browserKeys) {
    // ボタンの説明からキーの案内を外す
    $('tocBtn').title = 'Table of contents';
    $('searchBtn').title = 'Search';
    $('slidesBtn').title = 'Presentation mode';
  }
}).catch(() => {});
prefersDark.addEventListener('change', () => {
  if (!storedTheme() && configuredTheme === 'auto' && resolveTheme() !== currentTheme) themeBtn.click();
});
themeBtn.addEventListener('click', e => {
  // 実際のクリック（isTrusted）のときだけ選択を保存する
  if (e.isTrusted) { try { localStorage.setItem('mdp-theme', currentTheme); } catch (_) {} }
});

// ── 目次パネル ────────────────────────────────────────────────
const tocEl = $('toc');
function storedToc() { try { return localStorage.getItem('mdp-toc') === '1'; } catch (_) { return false; } }
if (storedToc()) document.body.classList.add('toc-open');
$('tocBtn').addEventListener('click', () => {
  const open = document.body.classList.toggle('toc-open');
  try { localStorage.setItem('mdp-toc', open ? '1' : '0'); } catch (_) {}
});
function buildToc() {
  const hs = Array.from(contentEl.querySelectorAll('h1,h2,h3,h4'));
  if (!hs.length) { tocEl.innerHTML = `<div class="toc-empty">No headings</div>`; return; }
  const min = Math.min(...hs.map(h => +h.tagName[1]));
  tocEl.innerHTML = '';
  for (const h of hs) {
    const a = document.createElement('a');
    a.href = '#' + h.id;
    // 差分表示の削除語（del）は目次に含めない
    const clone = h.cloneNode(true);
    clone.querySelectorAll('del.diff-w,.anchor-link').forEach(n => n.remove());
    a.textContent = clone.textContent.trim();
    a.style.paddingLeft = (0.5 + (+h.tagName[1] - min) * 0.9) + 'rem';
    a.dataset.target = h.id;
    tocEl.appendChild(a);
  }
  highlightToc();
}
function highlightToc() {
  const links = tocEl.querySelectorAll('a');
  let current = null;
  for (const a of links) {
    const h = document.getElementById(a.dataset.target);
    if (h && h.getBoundingClientRect().top <= window.innerHeight * 0.35) current = a;
  }
  links.forEach(a => a.classList.toggle('active', a === current));
  if (current && document.body.classList.contains('toc-open')) {
    const r = current.getBoundingClientRect(), tr = tocEl.getBoundingClientRect();
    if (r.top < tr.top || r.bottom > tr.bottom) current.scrollIntoView({ block: 'center' });
  }
}
window.addEventListener('scroll', () => requestAnimationFrame(highlightToc), { passive: true });

// ── 検索 ──────────────────────────────────────────────────────
const searchBar = $('searchBar'), searchInput = $('searchInput'), searchCount = $('searchCount');
let searchHits = [], searchIdx = -1;
function clearSearch() {
  contentEl.querySelectorAll('mark.search-hit').forEach(m => m.replaceWith(document.createTextNode(m.textContent)));
  contentEl.normalize();
  searchHits = []; searchIdx = -1; searchCount.textContent = '';
}
function runSearch() {
  clearSearch();
  const q = searchInput.value;
  if (!q) return;
  const lower = q.toLowerCase();
  const walker = document.createTreeWalker(contentEl, NodeFilter.SHOW_TEXT, {
    acceptNode: n => n.parentElement?.closest('script,style,.katex,svg') ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT,
  });
  const nodes = [];
  while (walker.nextNode()) nodes.push(walker.currentNode);
  for (const node of nodes) {
    const text = node.textContent;
    let i = text.toLowerCase().indexOf(lower);
    if (i < 0) continue;
    const frag = document.createDocumentFragment();
    let last = 0;
    while (i >= 0) {
      frag.appendChild(document.createTextNode(text.slice(last, i)));
      const m = document.createElement('mark');
      m.className = 'search-hit';
      m.textContent = text.slice(i, i + q.length);
      frag.appendChild(m); searchHits.push(m);
      last = i + q.length;
      i = text.toLowerCase().indexOf(lower, last);
    }
    frag.appendChild(document.createTextNode(text.slice(last)));
    node.replaceWith(frag);
  }
  gotoHit(searchHits.length ? 0 : -1);
}
function gotoHit(i) {
  if (!searchHits.length) { searchCount.textContent = searchInput.value ? 'No results' : ''; return; }
  searchIdx = (i + searchHits.length) % searchHits.length;
  searchHits.forEach((m, k) => m.classList.toggle('current', k === searchIdx));
  searchHits[searchIdx].scrollIntoView({ block: 'center' });
  searchCount.textContent = `${searchIdx + 1} / ${searchHits.length}`;
  // 検索で移動したらエディタのスクロール同期で戻されないようにする
  scrollSyncPaused = true; clearTimeout(scrollSyncTimer);
  scrollSyncTimer = setTimeout(() => { scrollSyncPaused = false; }, 3000);
}
function openSearch() { searchBar.hidden = false; searchInput.focus(); searchInput.select(); }
function closeSearch() { searchBar.hidden = true; searchInput.blur(); clearSearch(); }
$('searchBtn').addEventListener('click', openSearch);
$('searchClose').addEventListener('click', closeSearch);
$('searchNext').addEventListener('click', () => gotoHit(searchIdx + 1));
$('searchPrev').addEventListener('click', () => gotoHit(searchIdx - 1));
let searchTimer = null;
searchInput.addEventListener('input', () => { clearTimeout(searchTimer); searchTimer = setTimeout(runSearch, 150); });
searchInput.addEventListener('keydown', e => {
  if (e.key === 'Enter') { e.preventDefault(); gotoHit(searchIdx + (e.shiftKey ? -1 : 1)); }
  if (e.key === 'Escape') closeSearch();
});
document.addEventListener('keydown', e => {
  if (!browserKeys) return;
  const typing = e.target.closest?.('input,textarea');
  if ((e.key === '/' && !typing) || ((e.ctrlKey || e.metaKey) && e.key === 'f')) { e.preventDefault(); openSearch(); }
  else if (e.key === 't' && !typing && !e.ctrlKey && !e.metaKey) $('tocBtn').click();
});

// ── 脚注のポップアップ ────────────────────────────────────────
let fnPop = null;
contentEl.addEventListener('mouseover', e => {
  const ref = e.target.closest('sup.footnote-reference a');
  if (!ref) return;
  const id = decodeURIComponent((ref.getAttribute('href') || '').slice(1));
  const def = id && document.getElementById(id);
  if (!def) return;
  fnPop?.remove();
  fnPop = document.createElement('div');
  fnPop.id = 'fn-pop';
  fnPop.innerHTML = def.innerHTML;
  fnPop.querySelector('.footnote-definition-label')?.remove();
  fnPop.querySelectorAll('.src-line').forEach(n => n.remove());
  document.body.appendChild(fnPop);
  const r = ref.getBoundingClientRect();
  fnPop.style.left = Math.min(window.scrollX + r.left, window.scrollX + window.innerWidth - fnPop.offsetWidth - 16) + 'px';
  fnPop.style.top = (window.scrollY + r.bottom + 6) + 'px';
});
contentEl.addEventListener('mouseout', e => {
  if (e.target.closest('sup.footnote-reference a')) { fnPop?.remove(); fnPop = null; }
});

// ── ローカルの Markdown リンク・Wiki リンクは Neovim で開く ──
contentEl.addEventListener('click', e => {
  const a = e.target.closest('a[href]');
  if (!a) return;
  const href = a.getAttribute('href');
  if (/^(?:[a-z][a-z0-9+.-]*:|\/\/|#)/i.test(href)) return;
  const path = decodeURIComponent(href.split('#')[0]);
  if (!/\.(md|markdown)$/i.test(path)) return;
  e.preventDefault();
  e.stopPropagation();
  if (wsConn && wsConn.readyState === WebSocket.OPEN) wsConn.send(JSON.stringify({ type: 'open', path }));
}, true);

// ── Git 差分の表示 ────────────────────────────────────────────
// エディタの差分の色を使う。エディタとプレビューのテーマが違うと読みにくい色になることがあるので、
// 背景とのコントラストが足りなければ既定の色に戻す
function luminance(hex) {
  const [r, g, b] = [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map(c => c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}
function contrast(a, b) {
  const [x, y] = [luminance(a), luminance(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}
function applyDiffColors() {
  const bg = currentTheme === 'dark' ? '#0d1117' : '#ffffff';
  for (const k of ['add', 'mod', 'del']) {
    const c = diffState.colors?.[k];
    if (c && contrast(c, bg) >= 2.2) document.documentElement.style.setProperty('--diff-' + k, c);
    else document.documentElement.style.removeProperty('--diff-' + k);
  }
}
themeBtn.addEventListener('click', () => applyDiffColors());

// diffState: { enabled, available, base, colors, hunks: [{ s, e, kind, old_html, ctx: [s, e] }] }
let diffState = { enabled: false, hunks: [] };
let diffTargets = [], diffIdx = -1;

// 各行マーカーが受け持つブロック要素と行範囲を返す（入れ子の <li> なども含む）
function markerBlocks() {
  const ms = Array.from(contentEl.querySelectorAll('.src-line'));
  return ms.map((m, i) => {
    const start = +m.dataset.line;
    const next = ms.slice(i + 1).find(n => +n.dataset.line > start);
    const end = next ? +next.dataset.line - 1 : Infinity;
    // <li>/<blockquote> の中のマーカーは親が、それ以外は直後の要素がブロック
    const parent = m.parentElement;
    const el = parent !== contentEl && m === parent.firstElementChild ? parent : m.nextElementSibling;
    return { el, start, end };
  }).filter(b => b.el && b.el !== contentEl && !b.el.classList.contains('src-line'));
}

// contentEl 直下のブロックと行範囲
function topBlocks() {
  const tops = [];
  let pending = null;
  for (const el of contentEl.children) {
    if (el.classList.contains('src-line')) { pending = +el.dataset.line; continue; }
    if (el.classList.contains('diff-ghost')) continue;
    let line = pending; pending = null;
    if (line == null) { const m = el.querySelector('.src-line'); line = m ? +m.dataset.line : null; }
    if (line != null) tops.push({ el, start: line });
  }
  tops.forEach((t, i) => {
    const next = tops.slice(i + 1).find(n => n.start > t.start);
    t.end = next ? next.start - 1 : Infinity;
  });
  return tops;
}

// 前回の表示を消す（再利用された DOM ノードにも残っているので毎回消す）
function clearDiff() {
  contentEl.querySelectorAll('.diff-add,.diff-mod').forEach(e => e.classList.remove('diff-add', 'diff-mod'));
  contentEl.querySelectorAll('ins.diff-w').forEach(e => e.replaceWith(document.createTextNode(e.textContent)));
  contentEl.querySelectorAll('del.diff-w,.diff-ghost').forEach(e => e.remove());
  contentEl.querySelectorAll('.diff-flash').forEach(e => e.classList.remove('diff-flash'));
  contentEl.normalize();
  $('diffMarkers').innerHTML = '';
  diffTargets = []; diffIdx = -1;
}

// 単語単位の差分に使うテキストノード（数式・図・アイコンなどは除く）
const DIFF_SKIP = '.src-line,.katex,.math-inline,.math-display,.anchor-link,.copy-btn,.code-lang,svg,script,style,del.diff-w';
function textNodesOf(els) {
  const nodes = [];
  for (const el of els) {
    const w = document.createTreeWalker(el, NodeFilter.SHOW_TEXT, {
      acceptNode: n => n.parentElement?.closest(DIFF_SKIP) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT,
    });
    while (w.nextNode()) nodes.push(w.currentNode);
  }
  return nodes;
}
// 英数字は単語ごと、日本語などは 1 文字ずつに分ける（日本語には空白が無いため）
function tokenize(text) { return text.match(/[A-Za-z0-9_À-ɏ]+|\s+|[^]/gu) || []; }
// 最長共通部分列で差分を取る。大きすぎる場合は null（変更前の内容を表示する方に切り替える）
function diffTokens(a, b) {
  const n = a.length, m = b.length;
  if (n * m > 3e6) return null;
  const w = m + 1, dp = new Uint32Array((n + 1) * w);
  for (let i = n - 1; i >= 0; i--)
    for (let j = m - 1; j >= 0; j--)
      dp[i * w + j] = a[i] === b[j] ? dp[(i + 1) * w + j + 1] + 1 : Math.max(dp[(i + 1) * w + j], dp[i * w + j + 1]);
  const ops = [];
  const push = (type, text) => { const l = ops[ops.length - 1]; if (l && l.type === type) l.text += text; else ops.push({ type, text }); };
  let i = 0, j = 0;
  while (i < n && j < m) {
    if (a[i] === b[j]) { push('eq', a[i]); i++; j++; }
    else if (dp[(i + 1) * w + j] >= dp[i * w + j + 1]) push('del', a[i++]);
    else push('ins', b[j++]);
  }
  while (i < n) push('del', a[i++]);
  while (j < m) push('ins', b[j++]);
  return tidyOps(ops);
}
// 変更の間にある空白だけの一致は変更に含め、削除 → 追加の順にまとめる（細切れの表示を防ぐ）
function tidyOps(ops) {
  const out = ops.flatMap((o, k) =>
    o.type === 'eq' && /^\s+$/.test(o.text) && k > 0 && k < ops.length - 1 && ops[k - 1].type !== 'eq' && ops[k + 1].type !== 'eq'
      ? [{ type: 'del', text: o.text }, { type: 'ins', text: o.text }] : [o]);
  const res = [];
  let dels = '', inss = '';
  const flush = () => { if (dels) res.push({ type: 'del', text: dels }); if (inss) res.push({ type: 'ins', text: inss }); dels = inss = ''; };
  for (const o of out) {
    if (o.type === 'eq') { flush(); res.push(o); }
    else if (o.type === 'del') dels += o.text; else inss += o.text;
  }
  flush();
  return res;
}
function makeWord(tag, text) { const e = document.createElement(tag); e.className = 'diff-w'; e.textContent = text; return e; }

// 変更前の HTML をプレビューと同じ加工にかけた要素にする
function oldFragment(html) {
  const div = document.createElement('div');
  div.innerHTML = html;
  div.querySelectorAll('.src-line').forEach(n => n.remove());
  renderGithubAlerts(div);
  replaceEmoji(div);
  return div;
}

// el（現在のブロック）に変更前のブロック oldEl との単語単位の差分を書き込む。できなければ false
function applyWordDiff(el, oldEl) {
  const newNodes = textNodesOf([el]);
  const newText = newNodes.map(n => n.data).join('');
  const oldText = textNodesOf([oldEl]).map(n => n.data).join('');
  const ops = diffTokens(tokenize(oldText), tokenize(newText));
  if (!ops) return false;
  const changed = ops.filter(o => o.type !== 'eq').reduce((sum, o) => sum + o.text.trim().length, 0);
  const total = oldText.trim().length + newText.trim().length;
  // 比較できる違いが無い（数式の中だけが変わった等）か、ほとんど書き換わっている場合は、
  // 変更前の内容をまとめて見せた方が分かりやすい
  if (!changed || changed / Math.max(1, total) > 0.7) return false;

  let k = 0, off = 0;
  newNodes.forEach((node, idx) => {
    const text = node.data, frag = document.createDocumentFragment();
    let pos = 0, touched = false;
    while (pos < text.length || (idx === newNodes.length - 1 && k < ops.length && ops[k].type === 'del')) {
      // 空白・改行だけの変更は表示しない（HTML の整形の違いで生じるため）
      while (k < ops.length && ops[k].type === 'del') {
        if (ops[k].text.trim()) { frag.append(makeWord('del', ops[k].text)); touched = true; }
        k++;
      }
      if (pos >= text.length) break;
      if (k >= ops.length) { frag.append(text.slice(pos)); break; }
      const op = ops[k], take = Math.min(op.text.length - off, text.length - pos), piece = text.slice(pos, pos + take);
      if (op.type === 'ins' && piece.trim()) { frag.append(makeWord('ins', piece)); touched = true; } else frag.append(piece);
      pos += take; off += take;
      if (off >= op.text.length) { k++; off = 0; }
    }
    if (touched) node.replaceWith(frag);
  });
  return true;
}

// 変更前後のブロック列を対応づける（本文が同じものを一致とみなす）。
// 戻り値: [{ type: 'eq'|'del'|'ins', o, n }]（o: 変更前の番号, n: 変更後の番号）
function alignBlocks(oldTexts, newTexts) {
  const n = oldTexts.length, m = newTexts.length, w = m + 1, dp = new Uint32Array((n + 1) * w);
  for (let i = n - 1; i >= 0; i--)
    for (let j = m - 1; j >= 0; j--)
      dp[i * w + j] = oldTexts[i] === newTexts[j] ? dp[(i + 1) * w + j + 1] + 1 : Math.max(dp[(i + 1) * w + j], dp[i * w + j + 1]);
  const ops = [];
  let i = 0, j = 0;
  while (i < n && j < m) {
    if (oldTexts[i] === newTexts[j]) ops.push({ type: 'eq', o: i++, n: j++ });
    else if (dp[(i + 1) * w + j] >= dp[i * w + j + 1]) ops.push({ type: 'del', o: i++ });
    else ops.push({ type: 'ins', n: j++ });
  }
  while (i < n) ops.push({ type: 'del', o: i++ });
  while (j < m) ops.push({ type: 'ins', n: j++ });
  return ops;
}
const RICH = '.code-wrap,.mermaid-block,table,.math-display,pre';
const isRich = el => el.matches(RICH) || !!el.querySelector(RICH);
const blockText = el => el.textContent.replace(/\s+/g, ' ').trim();

// 削除された内容 / 変更前の内容を折りたたみで表示する要素
function makeGhost(kind, html) {
  const d = document.createElement('details');
  d.className = 'diff-ghost diff-ghost-' + kind;
  const body = oldFragment(html);
  body.className = 'diff-ghost-body';
  renderMath(body);
  // 表などで単語がつながらないよう、テキストを空白区切りで集める
  const pieces = [];
  // KaTeX の MathML（\sigma などの元の式）は除き、表示されている文字だけを使う
  const walker = document.createTreeWalker(body, NodeFilter.SHOW_TEXT, {
    acceptNode: n => n.parentElement?.closest('.katex-mathml') ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT,
  });
  while (walker.nextNode()) { const t = walker.currentNode.data.trim(); if (t) pieces.push(t); }
  const preview = pieces.join(' ').replace(/\s+/g, ' ').slice(0, 80);
  const summary = document.createElement('summary');
  summary.textContent = kind === 'del' ? '− Removed' : '~ Before';
  const pv = document.createElement('span');
  pv.className = 'diff-ghost-preview';
  pv.textContent = preview;
  summary.append(pv);
  d.append(summary, body);
  return d;
}

function applyDiff() {
  clearDiff();
  const nav = $('diffNav');
  // 差分の表示がオフ、または Git の管理外のファイルなら何も出さない
  if (!diffState.enabled || diffState.in_repo === false) { nav.hidden = true; return; }
  nav.hidden = false;
  const base = diffState.base || 'HEAD';
  const hunks = diffState.hunks || [];
  if (diffState.available === false) { $('diffSummary').textContent = `${base}: Git diff unavailable`; return; }

  const blocks = markerBlocks();
  const counts = { add: 0, mod: 0, del: 0 };
  // コード表示: 行単位で印を付け、削除された行・変更前の行は折りたたみで見せる
  if (viewKind === 'code') {
    const rows = new Map(blocks.map(b => [b.start, b.el]));
    for (const h of hunks) {
      if (h.kind === 'del') {
        const ghost = makeGhost('del', h.old_html);
        const after = rows.get(h.s);
        if (after) after.after(ghost); else contentEl.querySelector('.code-view')?.prepend(ghost);
        diffTargets.push({ el: ghost, kind: 'del' });
        continue;
      }
      const changed = [];
      for (let l = h.s; l <= h.e; l++) { const r = rows.get(l); if (r) { r.classList.add('diff-' + h.kind); changed.push(r); } }
      if (!changed.length) continue;
      if (h.kind === 'mod' && h.old_html) changed[changed.length - 1].after(makeGhost('mod', h.old_html));
      diffTargets.push({ el: changed[0], kind: h.kind });
    }
  } else
  for (const h of hunks) {
    counts[h.kind] = (counts[h.kind] || 0) + 1;
    const tops = topBlocks();
    let target = null;

    if (h.kind === 'del') {
      // 削除位置（直前の行 s）を含むブロックの後ろに、削除された内容を入れる
      const ghost = makeGhost('del', h.old_html);
      const after = h.s > 0 ? tops.filter(t => t.start <= h.s).pop() : null;
      if (after) after.el.after(ghost); else contentEl.prepend(ghost);
      diffTargets.push({ el: ghost, kind: 'del' });
      continue;
    }

    // 変わったブロックに印（入れ子は一番内側だけ）
    const hit = blocks.filter(b => h.s <= b.end && h.e >= b.start).map(b => b.el);
    const inner = hit.filter(el => !hit.some(o => o !== el && el.contains(o)));
    inner.forEach(el => el.classList.add('diff-' + h.kind));
    target = inner[0] || null;

    if (h.kind === 'mod') {
      // 変更前後の段落を対応づけ、組になったものは単語単位、片方にしかないものは追加 / 削除として見せる
      const group = tops.filter(t => t.start <= h.ctx[1] && t.end >= h.ctx[0]).map(t => t.el);
      const oldEls = Array.from(oldFragment(h.old_html).children);
      const ops = alignBlocks(oldEls.map(blockText), group.map(blockText));
      let cursor = group[0]?.previousElementSibling;
      while (cursor && cursor.classList.contains('src-line')) cursor = cursor.previousElementSibling;
      const insertGhost = ghost => {
        if (cursor) cursor.after(ghost); else if (group[0]) group[0].before(ghost); else contentEl.append(ghost);
        cursor = ghost;
      };
      for (let k = 0; k < ops.length; k++) {
        const op = ops[k];
        if (op.type === 'eq') { cursor = group[op.n]; continue; }
        // 連続する削除と追加を 1 組ずつ対にする
        const dels = [], inss = [];
        while (k < ops.length && ops[k].type !== 'eq') { (ops[k].type === 'del' ? dels : inss).push(ops[k]); k++; }
        k--;
        for (let p = 0; p < Math.max(dels.length, inss.length); p++) {
          const oldEl = p < dels.length ? oldEls[dels[p].o] : null;
          const newEl = p < inss.length ? group[inss[p].n] : null;
          if (oldEl && newEl) {
            if (!newEl.matches('.diff-mod,.diff-add') && !newEl.querySelector('.diff-mod,.diff-add')) newEl.classList.add('diff-mod');
            // コード・図・表・数式や、ほぼ書き換えたものは、変更前の内容をまとめて見せる
            if (isRich(newEl) || isRich(oldEl) || !applyWordDiff(newEl, oldEl)) {
              cursor = newEl;
              insertGhost(makeGhost('mod', oldEl.outerHTML));
            } else cursor = newEl;
            diffTargets.push({ el: newEl.querySelector('ins.diff-w,del.diff-w') || newEl, kind: 'mod' });
          } else if (newEl) {
            newEl.querySelectorAll('.diff-mod').forEach(e => e.classList.remove('diff-mod'));
            newEl.classList.remove('diff-mod');
            newEl.classList.add('diff-add');
            cursor = newEl;
            diffTargets.push({ el: newEl, kind: 'add' });
          } else {
            const ghost = makeGhost('del', oldEl.outerHTML);
            insertGhost(ghost);
            diffTargets.push({ el: ghost, kind: 'del' });
          }
        }
      }
      continue;
    }
    if (target) diffTargets.push({ el: target, kind: h.kind });
  }

  diffTargets.sort((a, b) => a.el.getBoundingClientRect().top - b.el.getBoundingClientRect().top);
  // 件数はブロック単位の結果で数える（Git の変更単位だと、変更と削除が 1 つにまとまることがある）
  counts.add = counts.mod = counts.del = 0;
  diffTargets.forEach(t => counts[t.kind]++);
  const total = counts.add + counts.mod + counts.del;
  $('diffSummary').textContent = total
    ? `${total} change${total === 1 ? '' : 's'} vs ${base} (+${counts.add} ~${counts.mod} −${counts.del})`
    : `No changes vs ${base}`;
  $('diffPrev').disabled = $('diffNext').disabled = !diffTargets.length;
  positionDiffMarkers();
}

// スクロールバーの横に、文書全体のどこが変わったかを示す
function positionDiffMarkers() {
  const box = $('diffMarkers');
  box.innerHTML = '';
  if (!diffTargets.length) return;
  const header = document.querySelector('header');
  const top = header ? header.getBoundingClientRect().height + 4 : 0;
  box.style.top = top + 'px';
  const docH = Math.max(1, document.documentElement.scrollHeight);
  const trackH = window.innerHeight - top - 4;
  diffTargets.forEach((t, i) => {
    const r = t.el.getBoundingClientRect();
    const m = document.createElement('div');
    m.className = 'm-' + t.kind;
    m.style.top = ((r.top + window.scrollY) / docH * trackH) + 'px';
    m.style.height = Math.max(3, r.height / docH * trackH) + 'px';
    m.title = { add: 'Added', mod: 'Changed', del: 'Removed' }[t.kind];
    m.addEventListener('click', () => gotoDiff(i));
    box.appendChild(m);
  });
}
let diffResizeTimer = null;
window.addEventListener('resize', () => { clearTimeout(diffResizeTimer); diffResizeTimer = setTimeout(positionDiffMarkers, 150); });

function gotoDiff(i) {
  if (!diffTargets.length) return;
  diffIdx = (i + diffTargets.length) % diffTargets.length;
  const el = diffTargets[diffIdx].el;
  if (el.tagName === 'DETAILS') el.open = true;
  el.scrollIntoView({ block: 'center' });
  contentEl.querySelectorAll('.diff-flash').forEach(e => e.classList.remove('diff-flash'));
  void el.offsetWidth; el.classList.add('diff-flash');
  // 移動した位置がエディタのスクロール同期で戻されないようにする
  scrollSyncPaused = true; clearTimeout(scrollSyncTimer);
  scrollSyncTimer = setTimeout(() => { scrollSyncPaused = false; }, 3000);
}
$('diffNext').addEventListener('click', () => gotoDiff(diffIdx + 1));
$('diffPrev').addEventListener('click', () => gotoDiff(diffIdx < 0 ? diffTargets.length - 1 : diffIdx - 1));

// ── プレゼンモード ────────────────────────────────────────────
// --- （<hr>）でスライドを区切る。<hr> が無ければ h1/h2 ごとに区切る
const slidesEl = $('slides'), slideBody = $('slideBody');
let slides = [], slideIdx = 0;
function buildSlides() {
  const kids = Array.from(contentEl.children).filter(k => !k.classList.contains('diff-ghost'));
  const useHr = kids.some(k => k.tagName === 'HR');
  slides = [[]];
  for (const k of kids) {
    if (useHr && k.tagName === 'HR') { slides.push([]); continue; }
    if (!useHr && /^H[12]$/.test(k.tagName) && slides[slides.length - 1].some(n => !n.classList.contains('src-line'))) slides.push([]);
    slides[slides.length - 1].push(k);
  }
  slides = slides.filter(sl => sl.some(n => !n.classList.contains('src-line')));
  if (!slides.length) slides = [[]];
  slideIdx = Math.min(slideIdx, slides.length - 1);
}
function showSlide(i) {
  slideIdx = Math.max(0, Math.min(slides.length - 1, i));
  slideBody.innerHTML = '';
  for (const n of slides[slideIdx]) slideBody.appendChild(n.cloneNode(true));
  slideBody.scrollTop = 0;
  $('slideNum').textContent = `${slideIdx + 1} / ${slides.length}`;
  $('slideProgress').firstElementChild.style.width = ((slideIdx + 1) / slides.length * 100) + '%';
}
function openSlides() { buildSlides(); slidesEl.hidden = false; showSlide(slideIdx); }
function closeSlides() {
  slidesEl.hidden = true;
  if (document.fullscreenElement) document.exitFullscreen().catch(() => {});
}
// カーソル行を含むスライドに移動する（エディタとの同期）
function slideForLine(line) {
  let best = 0;
  slides.forEach((sl, i) => {
    const first = sl.find(n => n.classList.contains('src-line')) || sl[0]?.querySelector?.('.src-line');
    if (first && +first.dataset.line <= line) best = i;
  });
  return best;
}
$('slidesBtn').addEventListener('click', openSlides);
slidesEl.addEventListener('click', e => { if (!e.target.closest('a,button')) showSlide(slideIdx + 1); });
document.addEventListener('keydown', e => {
  if (slidesEl.hidden) return;
  const k = e.key;
  if (['ArrowRight', 'ArrowDown', 'PageDown', ' ', 'Enter'].includes(k)) showSlide(slideIdx + 1);
  else if (['ArrowLeft', 'ArrowUp', 'PageUp', 'Backspace'].includes(k)) showSlide(slideIdx - 1);
  else if (k === 'Home') showSlide(0);
  else if (k === 'End') showSlide(slides.length - 1);
  else if (k === 'Escape') closeSlides();
  else if (k === 'f') { slidesEl.requestFullscreen?.().catch(() => {}); }
  else return;
  e.preventDefault(); e.stopImmediatePropagation();
}, true);

// ── 印刷プレビュー ────────────────────────────────────────────
$('printBtn').addEventListener('click', () => {
  // 1 回目: 紙面表示に切り替え / 2 回目: 印刷ダイアログ
  if (document.body.classList.contains('print-preview')) window.print();
  else document.body.classList.add('print-preview');
});
document.addEventListener('keydown', e => {
  if (e.key === 'Escape' && document.body.classList.contains('print-preview') && slidesEl.hidden) {
    document.body.classList.remove('print-preview');
  }
  const typing = e.target.closest?.('input,textarea');
  if (browserKeys && e.key === 'p' && !typing && !e.ctrlKey && !e.metaKey && slidesEl.hidden) { e.preventDefault(); openSlides(); }
});
// 印刷時は折りたたみを開き、印刷後に戻す
let closedDetails = [];
window.addEventListener('beforeprint', () => {
  closedDetails = Array.from(contentEl.querySelectorAll('details:not([open])'));
  closedDetails.forEach(d => d.open = true);
});
window.addEventListener('afterprint', () => { closedDetails.forEach(d => d.open = false); closedDetails = []; });

// ── PDF 用の印刷ページ（/print）──────────────────────────────
// 用紙・余白・フォントは @page と CSS に、目次と見出し番号は印刷用の要素と CSS カウンターにする
async function startPrint(p) {
  const o = p.options || {};
  const css = [
    `@page { size: ${o.paper || 'A4'}; margin: ${o.margin || '2cm'}; }`,
    `#content { font-size: ${o.font_size || 11}pt; }`,
  ];
  if (o.font) css.push(`#content { font-family: ${JSON.stringify(o.font)}, -apple-system, "Segoe UI", sans-serif; }`);
  if (o.numbering) css.push(
    '#content { counter-reset: h2; }',
    '#content h2 { counter-reset: h3; counter-increment: h2; }',
    '#content h2::before { content: counter(h2) ". "; }',
    '#content h3 { counter-increment: h3; }',
    '#content h3::before { content: counter(h2) "." counter(h3) " "; }',
  );
  const style = document.createElement('style');
  style.textContent = css.join('\n');
  document.head.appendChild(style);

  const c = p.content;
  if (c) {
    document.title = c.title;
    titleEl.textContent = c.title;
    updateFrontMatter(c.front_matter || null);
    await renderContent(c.html);
  }
  if (o.toc) {
    const hs = Array.from(contentEl.querySelectorAll('h1,h2,h3'));
    if (hs.length) {
      const nav = document.createElement('nav');
      nav.className = 'print-toc';
      const title = document.createElement('div');
      title.className = 'print-toc-title';
      title.textContent = 'Contents';
      nav.appendChild(title);
      for (const h of hs) {
        const a = document.createElement('a');
        a.href = '#' + h.id;
        a.textContent = h.textContent.trim();
        a.style.paddingLeft = ((+h.tagName[1] - 1) * 1.2) + 'em';
        nav.appendChild(a);
      }
      contentEl.prepend(nav);
    }
  }
}

if (PRINT) startPrint(PRINT); else connect();

// ── Export ────────────────────────────────────────────────────
const exportBtn  = document.getElementById('exportBtn');
const exportMenu = document.getElementById('exportMenu');

exportBtn.addEventListener('click', e => {
  e.stopPropagation();
  exportMenu.style.display = exportMenu.style.display === 'none' ? 'block' : 'none';
});
document.addEventListener('click', () => { exportMenu.style.display = 'none'; });

// type="module" のスクリプトは onclick 属性から関数を参照できないので、リスナーで登録する
exportMenu.querySelectorAll('[data-format]').forEach(item => {
  item.addEventListener('click', () => exportAs(item.dataset.format));
});

// fetch で取得してからダウンロードする（失敗時にプレビュー画面がエラー JSON に遷移しないように）
async function exportAs(format) {
  exportMenu.style.display = 'none';
  const label = exportBtn.lastChild.textContent;
  exportBtn.disabled = true;
  exportBtn.lastChild.textContent = ' Converting…';
  try {
    const res = await fetch(`/export?format=${encodeURIComponent(format)}`);
    if (!res.ok) {
      let msg = `HTTP ${res.status}`;
      try { msg = (await res.json()).error || msg; } catch (_) {}
      throw new Error(msg);
    }
    const disposition = res.headers.get('Content-Disposition') || '';
    // 日本語名を保つため filename*（RFC 5987）を優先し、無ければ filename を使う
    const star  = disposition.match(/filename\*=UTF-8''([^;]+)/);
    const plain = disposition.match(/filename="([^"]+)"/);
    const filename = star ? decodeURIComponent(star[1]) : plain ? plain[1] : `export.${format}`;
    const url = URL.createObjectURL(await res.blob());
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
  } catch (e) {
    alert(`Export failed (${format})\n\n${e.message}`);
  } finally {
    exportBtn.disabled = false;
    exportBtn.lastChild.textContent = label;
  }
}
