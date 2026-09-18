/* AXIOM 前端 —— 纯 JS,无框架 */

const Plotly = window.Plotly;

// ============================================================
// 工具
// ============================================================
const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);
async function api(method, url, body) {
    const opts = { method, headers: { 'Content-Type': 'application/json' } };
    if (body) opts.body = JSON.stringify(body);
    let lastErr = null;
    for (let attempt = 0; attempt < 3; attempt++) {
        try {
            const r = await fetch(url, opts);
            if (!r.ok) {
                const t = await r.text();
                throw new Error(`${r.status}: ${t.slice(0, 200)}`);
            }
            return r.json();
        } catch (e) {
            lastErr = e;
            if (attempt < 2) await new Promise(r => setTimeout(r, 300 * (attempt + 1)));
        }
    }
    throw lastErr || new Error('Unknown');
}
function fmtPct(v) { return (v * 100).toFixed(2) + '%'; }
function fmtNum(v, d = 2) { return Number(v).toFixed(d); }
function fmtMoney(v) { return '$' + Number(v).toLocaleString(undefined, { maximumFractionDigits: 2 }); }

// ============================================================
// Plotly 主题
// ============================================================
const PLOT_LAYOUT = {
    paper_bgcolor: '#1c2128', plot_bgcolor: '#1c2128',
    font: { color: '#e6edf3', family: 'system-ui, sans-serif', size: 11 },
    margin: { t: 30, b: 40, l: 50, r: 20 },
    xaxis: { gridcolor: '#21262d', zerolinecolor: '#21262d' },
    yaxis: { gridcolor: '#21262d', zerolinecolor: '#21262d' },
};
const PLOT_CONFIG = { responsive: true, displayModeBar: false };

// ============================================================
// 自定义下拉框组件 (替代原生 <select>,永远不溢出)
// 用法: <div class="dropdown" data-name="..." data-options='[{"v":"x","l":"X"}]'></div>
// JS:   const d = createDropdown(el, options, defaultValue, onChange)
//       d.value / d.setOptions(...) / d.destroy()
// ============================================================
function createDropdown(rootEl, options, defaultValue, onChange) {
    const triggerText = (val) => {
        const opt = options.find(o => o.v === val);
        return opt ? opt.l : '选择...';
    };

    rootEl.classList.add('dropdown');
    rootEl.innerHTML = `
        <button class="dropdown-trigger" type="button"></button>
        <div class="dropdown-menu">
            ${options.length > 5 ? '<input class="dropdown-search" placeholder="搜索...">' : ''}
            <div class="dropdown-options"></div>
        </div>
    `;
    const trigger = rootEl.querySelector('.dropdown-trigger');
    const menu = rootEl.querySelector('.dropdown-menu');
    const search = rootEl.querySelector('.dropdown-search');
    const optsEl = rootEl.querySelector('.dropdown-options');

    let currentValue = defaultValue;
    let isOpen = false;

    function render(filterText = '') {
        const q = filterText.trim().toLowerCase();
        const filtered = q
            ? options.filter(o => (o.l || '').toLowerCase().includes(q) || (o.v || '').toLowerCase().includes(q))
            : options;
        if (filtered.length === 0) {
            optsEl.innerHTML = '<div class="dropdown-empty">无匹配项</div>';
            return;
        }
        optsEl.innerHTML = filtered.map(o =>
            `<div class="dropdown-option${o.v === currentValue ? ' selected' : ''}" data-value="${o.v}">${o.l}</div>`
        ).join('');
        optsEl.querySelectorAll('.dropdown-option').forEach(el => {
            el.addEventListener('click', (e) => {
                e.stopPropagation();
                setValue(el.dataset.value);
                close();
            });
        });
    }

    function setValue(v) {
        if (v === currentValue) return;
        currentValue = v;
        trigger.textContent = triggerText(v);
        optsEl.querySelectorAll('.dropdown-option').forEach(el => {
            el.classList.toggle('selected', el.dataset.value === v);
        });
        if (onChange) onChange(v);
    }

    function open() {
        if (isOpen) return;
        isOpen = true;
        rootEl.classList.add('open');
        menu.style.display = 'flex';
        if (search) { search.value = ''; render(); search.focus(); }
    }
    function close() {
        if (!isOpen) return;
        isOpen = false;
        rootEl.classList.remove('open');
        menu.style.display = 'none';
    }
    function toggle() { isOpen ? close() : open(); }

    trigger.addEventListener('click', (e) => { e.stopPropagation(); toggle(); });
    if (search) search.addEventListener('input', () => render(search.value));
    document.addEventListener('click', (e) => {
        if (!rootEl.contains(e.target)) close();
    });

    // 初始化
    trigger.textContent = triggerText(currentValue);
    render();
    menu.style.display = 'none';

    const api = {
        get value() { return currentValue; },
        set value(v) { setValue(v); },
        setOptions(newOpts, newDefault) {
            options = newOpts;
            if (newDefault !== undefined) currentValue = newDefault;
            trigger.textContent = triggerText(currentValue);
            render(search ? search.value : '');
        },
        refresh() { render(search ? search.value : ''); },
        open, close,
        destroy() { rootEl.innerHTML = ''; rootEl.classList.remove('dropdown'); },
    };
    rootEl._axiom = api;     // 存到 DOM 上,方便外部通过 el._axiom.value 访问
    return api;
}

function parseOptionsFromEl(el) {
    const raw = el.dataset.options;
    if (!raw) return [];
    try { return JSON.parse(raw); }
    catch { return []; }
}

function initDropdowns() {
    document.querySelectorAll('[data-name]').forEach(el => {
        const opts = parseOptionsFromEl(el);
        if (opts.length === 0) return;
        createDropdown(el, opts, opts[0].v);
    });
}

// ============================================================
// 顶部 tab 切换 + 学习中心子导航
// ============================================================
$$('.tab').forEach(btn => {
    btn.addEventListener('click', () => {
        $$('.tab').forEach(b => b.classList.remove('active'));
        $$('.tab-content').forEach(c => c.classList.remove('active'));
        btn.classList.add('active');
        document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
    });
});
$$('.learn-sub').forEach(btn => {
    btn.addEventListener('click', () => {
        $$('.learn-sub').forEach(b => b.classList.remove('active'));
        $$('.learn-pane').forEach(p => p.classList.remove('active'));
        btn.classList.add('active');
        const pane = document.querySelector(`.learn-pane[data-pane="${btn.dataset.sub}"]`);
        if (pane) pane.classList.add('active');
    });
});

// ============================================================
// 数据探索
// ============================================================
async function loadData() {
    const customSymbol = $('#data-symbol-custom').value.trim();
    const selSym = document.getElementById('data-symbol')._axiom?.value || '';
    const symbol = (customSymbol || selSym || 'BTCUSDT').toUpperCase();
    const limit = parseInt($('#data-limit').value) || 200;
    const source = document.getElementById('data-source')._axiom?.value || 'real';
    const indEl = document.getElementById('data-indicators');
    const indicatorSet = indEl._axiom?.value || 'sma_20,rsi_14';
    const chartType = document.getElementById('data-chart-type')?._axiom?.value || 'candle';

    $('#data-summary').textContent = '加载中...';
    const url = `/api/indicators?symbol=${encodeURIComponent(symbol)}&limit=${limit}&source=${source}&indicators=${encodeURIComponent(indicatorSet)}`;
    try {
        const data = await api('GET', url);
        // 如果选了 Heikin Ashi, 替换 bars 为 HA
        if (chartType === 'heikin_ashi') {
            try {
                const ha = await api('GET', `/api/heikin_ashi?symbol=${encodeURIComponent(symbol)}&limit=${limit}&source=${source}`);
                if (ha && ha.bars) data.bars = ha.bars;
            } catch (e) { console.warn('HA 加载失败, 用原图:', e); }
        }
        renderDataChart(data);
        renderDataSummary(data);
        // 加载 K线形态识别
        try {
            const pat = await api('GET', `/api/patterns?symbol=${encodeURIComponent(symbol)}&limit=${Math.min(limit, 100)}&source=${source}`);
            if (pat && pat.patterns) renderPatterns(pat.patterns);
        } catch (e) { console.warn('形态识别失败:', e); }
    } catch (e) {
        console.error('loadData indicators failed:', e);
        const fallbackUrl = `/api/data?symbol=${encodeURIComponent(symbol)}&limit=${limit}&source=${source}`;
        try {
            const data = await api('GET', fallbackUrl);
            renderDataChart(data);
            renderDataSummary(data);
        } catch (e2) {
            $('#data-summary').innerHTML =
                `<div style="color:var(--red);padding:0.8rem">
                  <b>加载失败</b><br>
                  <pre style="white-space:pre-wrap;font-size:0.8rem">${e2.message}</pre>
                  <button onclick="loadData()" style="margin-top:0.5rem">重试</button>
                </div>`;
        }
    }
}

// 渲染 K线形态识别结果
function renderPatterns(patterns) {
    const target = $('#data-patterns');
    if (!target.length) return;
    const recent = patterns.filter(p => p.pattern && p.pattern !== '无特殊形态').slice(0, 5);
    target.html(recent.length === 0
        ? '<span style="color:var(--text-dim); font-size:0.78rem">最近无明显形态</span>'
        : recent.map(p => `<span style="display:inline-block; margin-right:0.4rem; padding:0.15rem 0.45rem; background:var(--bg-elev); border:1px solid var(--border); border-radius:3px; font-size:0.75rem;"><b>${p.pattern}</b> <span style="color:var(--text-dim)">@ ${new Date(p.timestamp).toLocaleDateString()}</span></span>`).join(''));
}

function renderDataChart(data) {
    // 强制 purge,避免多次渲染后图重叠加长
    const chartEl = document.getElementById('data-chart');
    if (chartEl) Plotly.purge(chartEl);

    const candles = data.bars.map(b => ({
        x: new Date(b.timestamp),
        open: b.open, high: b.high, low: b.low, close: b.close,
    }));
    const traces = [{
        x: candles.map(c => c.x),
        open: candles.map(c => c.open), high: candles.map(c => c.high),
        low: candles.map(c => c.low), close: candles.map(c => c.close),
        type: 'candlestick', name: data.symbol,
        increasing: { line: { color: '#3fb950' } },
        decreasing: { line: { color: '#f85149' } },
    }];
    if (data.indicators) {
        const colors = ['#d29922', '#58a6ff', '#a371f7', '#ff7b72', '#56d4dd', '#7ee787'];
        let ci = 0;

        // 一目均衡表特殊处理:画云带
        if (data.indicators.ichimoku_tenkan) {
            const ts = data.bars.map(b => new Date(b.timestamp));
            const sa = data.indicators.ichimoku_senkou_a || [];
            const sb = data.indicators.ichimoku_senkou_b || [];
            const t = data.indicators.ichimoku_tenkan || [];
            const k = data.indicators.ichimoku_kijun || [];
            // 云带:Senkou A 和 B 之间的填充
            // 多头云(A>B) 绿色,空头云(A<B) 红色
            traces.push({
                x: ts, y: sa.map(v => v ? v.y : null),
                type: 'scatter', mode: 'lines', name: 'Senkou A',
                line: { color: 'rgba(63, 185, 80, 0.7)', width: 1 },
                fill: 'tonexty',
                fillcolor: 'rgba(63, 185, 80, 0.12)',
            }, 0);  // 插入到 K 线之后
            traces.push({
                x: ts, y: sb.map(v => v ? v.y : null),
                type: 'scatter', mode: 'lines', name: 'Senkou B (云带)',
                line: { color: 'rgba(248, 81, 73, 0.7)', width: 1 },
                fill: 'tonexty',
                fillcolor: 'rgba(248, 81, 73, 0.12)',
            }, 1);
            traces.push({
                x: ts, y: t.map(v => v ? v.y : null),
                type: 'scatter', mode: 'lines', name: 'Tenkan (转换线)',
                line: { color: '#d29922', width: 1.3 },
            });
            traces.push({
                x: ts, y: k.map(v => v ? v.y : null),
                type: 'scatter', mode: 'lines', name: 'Kijun (基准线)',
                line: { color: '#f85149', width: 1.3 },
            });
            // 跳过 ichimoku_* 这些指标避免重复画
            for (const k of ['ichimoku_tenkan','ichimoku_kijun','ichimoku_senkou_a','ichimoku_senkou_b','ichimoku_chikou']) {
                delete data.indicators[k];
            }
        }

        // 其它指标正常画
        for (const [name, vals] of Object.entries(data.indicators)) {
            traces.push({
                x: vals.map(v => v ? new Date(v.x) : null),
                y: vals.map(v => v ? v.y : null),
                type: 'scatter', mode: 'lines', name,
                line: { color: colors[ci++ % colors.length], width: 1.5 },
                connectgaps: false,
                opacity: 0.85,
            });
        }
    }
    Plotly.newPlot('data-chart', traces, {
        ...PLOT_LAYOUT,
        title: `${data.symbol} · ${data.source === 'real' ? 'Binance 实时' : '合成数据'}`,
        xaxis: { ...PLOT_LAYOUT.xaxis, rangeslider: { visible: false } },
        legend: { ...PLOT_LAYOUT.legend, orientation: 'h', y: -0.15 },
    }, PLOT_CONFIG);
}

function renderDataSummary(data) {
    const closes = data.bars.map(b => b.close);
    if (closes.length === 0) { $('#data-summary').textContent = '没有数据'; return; }
    const first = closes[0], last = closes[closes.length - 1];
    const high = Math.max(...data.bars.map(b => b.high));
    const low = Math.min(...data.bars.map(b => b.low));
    const ret = (last / first - 1) * 100;
    const avgVol = data.bars.reduce((s, b) => s + b.volume, 0) / data.bars.length;
    $('#data-summary').innerHTML = `
        <table>
            <tr><th>K 线数</th><td>${data.count}</td><th>数据源</th><td>${data.source}</td></tr>
            <tr><th>首价</th><td>${fmtNum(first)}</td><th>末价</th><td>${fmtNum(last)}</td></tr>
            <tr><th>期间最高</th><td>${fmtNum(high)}</td><th>期间最低</th><td>${fmtNum(low)}</td></tr>
            <tr><th>累计涨跌</th><td style="color:${ret >= 0 ? 'var(--green)' : 'var(--red)'}">${ret.toFixed(2)}%</td>
                <th>平均成交量</th><td>${fmtNum(avgVol)}</td></tr>
        </table>`;
}
$('#btn-load-data').addEventListener('click', loadData);

// ============================================================
// 动态数据加载 (symbols / strategies)
// ============================================================
let strategiesMeta = [];
let symbolDropdown = null;

async function loadSymbols() {
    try {
        const d = await api('GET', '/api/symbols');
        // 把所有 493 个交易对塞入 dropdown
        const opts = d.symbols.map(s => ({ v: s, l: s }));
        ['data-symbol', 'bt-symbol', 'cmp-symbol'].forEach(id => {
            const el = document.getElementById(id);
            if (!el) return;
            symbolDropdown = createDropdown(el, opts, 'BTCUSDT', (v) => {
                // 这里可以加联动逻辑
            });
        });
    } catch (e) {
        console.error('加载交易对失败:', e);
    }
}

async function loadStrategiesMeta() {
    try {
        const d = await api('GET', '/api/strategies');
        strategiesMeta = d.strategies;
        ['bt-strategy', 'paper-strategy'].forEach(id => {
            const el = document.getElementById(id);
            if (!el) return;
            createDropdown(el, strategiesMeta.map(s => ({ v: s.name, l: s.display_name })),
                strategiesMeta[0]?.name, (v) => {
                    if (id === 'bt-strategy') updateBtParams();
                });
        });
        updateBtParams();
        renderStrategyPool();
    } catch (e) {
        console.error('加载策略失败:', e);
    }
}

function updateBtParams() {
    const sel = document.getElementById('bt-strategy')._axiom;
    const strategy = sel ? sel.value : 'sma_cross';
    const meta = strategiesMeta.find(s => s.name === strategy);
    if (!meta || !meta.params || meta.params.length === 0) {
        $('#bt-params').innerHTML = '';
        return;
    }
    let html = '';
    for (const p of meta.params) {
        const step = (p.key.includes('pct') || p.key === 'num_std' || p.key === 'multiplier') ? 0.1 : 1;
        html += `<label>${p.label}
                    <input type="number" id="bt-${p.key}" value="${p.default}" min="${p.min||0}" max="${p.max||9999}" step="${step}">
                  </label>`;
    }
    $('#bt-params').innerHTML = html;
}

// ============================================================
// 策略对比:可勾选 + 自定义策略
// ============================================================
let customStrategies = [];

function renderStrategyPool() {
    const container = $('#strategy-checks');
    let html = '';
    strategiesMeta.forEach(s => {
        html += `<label class="strategy-check" data-name="${s.name}">
                    <input type="checkbox" value="${s.name}" checked>
                    <span>${s.display_name}</span>
                  </label>`;
    });
    customStrategies.forEach((cs, idx) => {
        html += `<label class="strategy-check custom checked" data-cust="${idx}">
                    <input type="checkbox" value="custom_${idx}" checked>
                    <span>自定义: ${cs.name}</span>
                  </label>`;
    });
    container.innerHTML = html;
    container.querySelectorAll('.strategy-check').forEach(el => {
        el.addEventListener('click', (e) => {
            if (e.target.tagName === 'INPUT') {
                el.classList.toggle('checked', e.target.checked);
            }
        });
    });
}

function getSelectedStrategies() {
    const checks = $('#strategy-checks').querySelectorAll('.strategy-check input:checked');
    return Array.from(checks).map(c => c.value);
}

window.selectAllStrategies = function(checked) {
    $('#strategy-checks').querySelectorAll('.strategy-check input').forEach(i => {
        i.checked = checked;
        i.parentElement.classList.toggle('checked', checked);
    });
};
window.selectDefaultStrategies = function() {
    $('#strategy-checks').querySelectorAll('.strategy-check input').forEach(i => {
        const isDefault = !['random'].includes(i.value);
        i.checked = isDefault;
        i.parentElement.classList.toggle('checked', isDefault);
    });
};

window.showCustomStrategyForm = function() {
    const container = $('#custom-strategy-container');
    if (container.querySelector('.custom-strategy-form')) {
        container.innerHTML = '';
        return;
    }
    const stratOpts = strategiesMeta.map(s => ({ v: s.name, l: s.display_name }));
    container.innerHTML = `
        <div class="custom-strategy-form">
            <div class="form-row">
                <label>策略名称
                    <input type="text" id="cust-name" placeholder="我的 SMA 策略">
                </label>
                <label>基础策略
                    <div id="cust-base" data-name="cust-base"></div>
                </label>
            </div>
            <div class="form-row">
                <label>参数 (JSON,可选)
                    <input type="text" id="cust-params" placeholder='{"fast": 8, "slow": 30}'>
                </label>
                <label>止损 %
                    <input type="number" id="cust-sl" value="0" step="0.5" min="0" max="50">
                </label>
            </div>
            <div class="form-actions">
                <button id="cust-save">保存并加入对比</button>
                <button onclick="document.getElementById('custom-strategy-container').innerHTML=''" style="background:transparent;color:var(--text-dim);border:1px solid var(--border)">取消</button>
            </div>
        </div>`;
    // 初始化 base 下拉
    const baseEl = document.getElementById('cust-base');
    createDropdown(baseEl, stratOpts, 'sma_cross');
    document.getElementById('cust-save').addEventListener('click', saveCustomStrategy);
};

function saveCustomStrategy() {
    const name = $('#cust-name').value.trim() || '未命名';
    const base = document.getElementById('cust-base')._axiom.value;
    let params = {};
    const paramsStr = $('#cust-params').value.trim();
    if (paramsStr) {
        try { params = JSON.parse(paramsStr); }
        catch (e) { alert('参数 JSON 格式错误: ' + e.message); return; }
    }
    const sl = parseFloat($('#cust-sl').value) / 100;
    customStrategies.push({ name, base, params, sl });
    $('#custom-strategy-container').innerHTML = '';
    renderStrategyPool();
}

// ============================================================
// 回测
// ============================================================
async function runBacktest() {
    const stratDd = document.getElementById('bt-strategy')._axiom;
    const strategy = stratDd ? stratDd.value : 'sma_cross';
    const meta = strategiesMeta.find(s => s.name === strategy);
    const params = {};
    if (meta && meta.params) {
        for (const p of meta.params) {
            const el = $('#bt-' + p.key);
            if (el) params[p.key] = parseFloat(el.value);
        }
    }
    const symDd = document.getElementById('bt-symbol')._axiom;
    const symbol = (symDd ? symDd.value : 'BTCUSDT') || 'BTCUSDT';
    const srcDd = document.getElementById('bt-source')._axiom;
    const source = srcDd ? srcDd.value : 'real';

    const req = {
        strategy, params, symbol, source,
        limit: 1000,
        initial_capital: parseFloat($('#bt-capital').value),
        stop_loss_pct: parseFloat($('#bt-sl').value) / 100,
        take_profit_pct: parseFloat($('#bt-tp').value) / 100,
        max_position_pct: parseFloat($('#bt-mp').value) / 100,
    };

    $('#btn-run-bt').disabled = true;
    $('#bt-metrics').innerHTML = '<div class="metric"><div class="label">运行中</div><div class="value">...</div></div>';
    try {
        const result = await api('POST', '/api/backtest', req);
        renderBacktestResult(result);
    } catch (e) {
        $('#bt-metrics').innerHTML = `<div class="metric"><div class="label">错误</div><div class="value" style="color:var(--red)">${e.message}</div></div>`;
    } finally {
        $('#btn-run-bt').disabled = false;
    }
}

function renderBacktestResult(r) {
    const m = r.metrics;
    const fields = [
        { label: '总收益', key: '总收益率', sign: true, pct: true },
        { label: '年化收益', key: '年化收益率', sign: true, pct: true },
        { label: '最大回撤', key: '最大回撤_pct', sign: false, pct: true },
        { label: '夏普', key: '夏普比率', sign: true },
        { label: '索提诺', key: '索提诺比率', sign: true },
        { label: 'Calmar', key: 'Calmar比率', sign: true },
        { label: '波动率', key: '年化波动率', pct: true },
        { label: 'VaR 95%', key: 'VaR_95', pct: true },
        { label: 'CVaR 95%', key: 'CVaR_95', pct: true },
        { label: '偏度', key: '偏度' },
        { label: '交易笔数', key: '交易笔数' },
        { label: '胜率', key: '胜率', pct: true },
        { label: '最终净值', key: '最终净值', money: true },
    ];
    $('#bt-metrics').innerHTML = fields.map(f => {
        let v = m[f.key];
        if (v === undefined) return '';
        let display;
        if (f.money) display = fmtMoney(v);
        else if (f.pct) display = fmtPct(v);
        else display = typeof v === 'number' ? fmtNum(v, 2) : v;
        let cls = '';
        if (f.sign && typeof v === 'number') {
            cls = v > 0 ? 'positive' : (v < 0 ? 'negative' : '');
        }
        return `<div class="metric"><div class="label">${f.label}</div>
                <div class="value ${cls}">${display}</div></div>`;
    }).join('');

    // 净值曲线
    // 强制 purge 避免重复渲染问题
    Plotly.purge('bt-equity-chart');
    Plotly.purge('bt-trades-chart');

    Plotly.newPlot('bt-equity-chart', [{
        x: r.equity_curve.map(p => new Date(p.timestamp)),
        y: r.equity_curve.map(p => p.equity),
        type: 'scatter', mode: 'lines', name: '净值',
        line: { color: '#d29922', width: 1.8 },
    }], { ...PLOT_LAYOUT, title: `净值曲线 (${r.config.strategy})`,
        yaxis: { ...PLOT_LAYOUT.yaxis, title: 'Equity' } }, PLOT_CONFIG);

    // 交易点位
    const buys = r.trades.filter(t => t.side === 'BUY');
    const sells = r.trades.filter(t => t.side === 'SELL');
    const priceTrace = {
        x: r.equity_curve.map(p => new Date(p.timestamp)),
        y: r.equity_curve.map(p => p.equity - p.cash + p.position_value),
        type: 'scatter', mode: 'lines', name: '价格',
        line: { color: '#484f58', width: 1 }, opacity: 0.5,
    };
    const buyTrace = buys.length ? {
        x: buys.map(t => new Date(t.entry_time)), y: buys.map(t => t.entry_price),
        type: 'scatter', mode: 'markers', name: '买入',
        marker: { symbol: 'triangle-up', size: 11, color: '#3fb950' },
    } : null;
    const sellTrace = sells.length ? {
        x: sells.map(t => new Date(t.entry_time)), y: sells.map(t => t.entry_price),
        type: 'scatter', mode: 'markers', name: '卖出',
        marker: { symbol: 'triangle-down', size: 11, color: '#f85149' },
    } : null;
    Plotly.newPlot('bt-trades-chart',
        [priceTrace, buyTrace, sellTrace].filter(Boolean),
        { ...PLOT_LAYOUT, title: '交易点位', yaxis: { ...PLOT_LAYOUT.yaxis, title: 'Price' } }, PLOT_CONFIG);

    // 交易列表
    $('#bt-trades-count').textContent = r.trades.length;
    if (r.trades.length === 0) {
        $('#bt-trades').innerHTML = '<p style="color:var(--text-dim);padding:0.6rem">没有交易</p>';
    } else {
        $('#bt-trades').innerHTML = `
            <table>
                <thead><tr><th>开仓</th><th>方向</th><th>开仓价</th><th>数量</th>
                    <th>平仓</th><th>平仓价</th><th>盈亏</th><th>收益率</th></tr></thead>
                <tbody>${r.trades.map(t => `<tr>
                    <td>${new Date(t.entry_time).toLocaleString()}</td>
                    <td>${t.side}</td>
                    <td>${fmtNum(t.entry_price)}</td>
                    <td>${fmtNum(t.size, 4)}</td>
                    <td>${t.exit_time ? new Date(t.exit_time).toLocaleString() : '--'}</td>
                    <td>${t.exit_price ? fmtNum(t.exit_price) : '--'}</td>
                    <td class="${t.pnl >= 0 ? 'pos' : 'neg'}">${fmtMoney(t.pnl)}</td>
                    <td class="${t.pnl_pct >= 0 ? 'pos' : 'neg'}">${fmtNum(t.pnl_pct)}%</td>
                </tr>`).join('')}</tbody>
            </table>`;
    }
}
$('#btn-run-bt').addEventListener('click', runBacktest);

// ============================================================
// 模拟盘
// ============================================================
let paperWs = null;

$('#btn-paper-start').addEventListener('click', async () => {
    const stratDd = document.getElementById('paper-strategy')._axiom;
    const strategy = stratDd ? stratDd.value : 'sma_cross';
    try {
        await api('POST', '/api/paper/strategy', { strategy });
        await api('POST', '/api/paper/start', {});
        $('#paper-status').textContent = '运行中';
        $('#paper-status').style.color = 'var(--green)';
        startPaperWs();
    } catch (e) { alert('启动失败: ' + e.message); }
});

$('#btn-paper-stop').addEventListener('click', async () => {
    try {
        await api('POST', '/api/paper/stop', {});
        $('#paper-status').textContent = '已停止';
        $('#paper-status').style.color = 'var(--text-dim)';
        if (paperWs) paperWs.close();
    } catch (e) { alert('停止失败: ' + e.message); }
});

function startPaperWs() {
    if (paperWs) paperWs.close();
    const proto = location.protocol === 'https:' ? 'wss' : 'ws';
    paperWs = new WebSocket(`${proto}://${location.host}/api/paper/ws`);
    paperWs.onmessage = (evt) => {
        try { updatePaperUi(JSON.parse(evt.data)); }
        catch (e) { console.error(e); }
    };
    paperWs.onclose = () => {
        $('#paper-status').textContent = '已断开';
        $('#paper-status').style.color = 'var(--text-dim)';
    };
}

function updatePaperUi(snap) {
    $('#paper-cash').textContent = fmtMoney(snap.cash);
    $('#paper-pos-value').textContent = fmtMoney(snap.position_value);
    $('#paper-equity').textContent = fmtMoney(snap.equity);
    $('#paper-pos-size').textContent = fmtNum(snap.position_size, 4);
    $('#paper-fills').textContent = snap.fills_count || 0;
    $('#paper-price').textContent = snap.current_bar ? fmtMoney(snap.current_bar.close) : '--';

    if (snap.equity_curve && snap.equity_curve.length > 0) {
        Plotly.react('paper-equity-chart', [{
            x: snap.equity_curve.map(p => new Date(p.timestamp)),
            y: snap.equity_curve.map(p => p.equity),
            type: 'scatter', mode: 'lines',
            line: { color: '#d29922', width: 1.8 },
        }], { ...PLOT_LAYOUT, title: '模拟盘净值曲线',
            yaxis: { ...PLOT_LAYOUT.yaxis, title: 'Equity' } }, PLOT_CONFIG);
    }

    if (snap.log && snap.log.length > 0) {
        const html = snap.log.slice().reverse().map(e => `
            <div class="entry">
                <span class="ts">${new Date(e.timestamp).toLocaleTimeString()}</span>
                <span class="lvl ${e.level}">${e.level}</span>
                <span class="msg">${e.message}</span>
            </div>`).join('');
        $('#paper-log').innerHTML = html;
    }
}

(async () => {
    try {
        const snap = await api('GET', '/api/paper/snapshot');
        updatePaperUi(snap);
    } catch (e) {}
})();

// ============================================================
// 策略对比
// ============================================================
async function runCompare() {
    const symDd = document.getElementById('cmp-symbol')._axiom;
    const symbol = (symDd ? symDd.value : 'BTCUSDT') || 'BTCUSDT';
    const srcDd = document.getElementById('cmp-source')._axiom;
    const source = srcDd ? srcDd.value : 'real';
    const capital = parseFloat($('#cmp-capital').value);

    const selected = getSelectedStrategies();
    if (selected.length < 2) {
        $('#cmp-table').innerHTML = '<p style="color:var(--accent);padding:0.6rem">请至少选 2 个策略</p>';
        return;
    }
    $('#cmp-table').innerHTML = '<p style="color:var(--text-dim);padding:0.6rem">运行中 (' + selected.length + ' 个策略)...</p>';

    const results = [];
    for (const item of selected) {
        let req;
        if (item.startsWith('custom_')) {
            const idx = parseInt(item.replace('custom_', ''));
            const cs = customStrategies[idx];
            req = {
                strategy: cs.base,
                params: cs.params || {},
                symbol, source, limit: 500, initial_capital: capital,
                stop_loss_pct: cs.sl || 0,
            };
        } else {
            const meta = strategiesMeta.find(s => s.name === item);
            const params = {};
            if (meta && meta.params) {
                for (const p of meta.params) params[p.key] = p.default;
            }
            req = {
                strategy: item, params,
                symbol, source, limit: 500, initial_capital: capital,
            };
        }
        try {
            const r = await api('POST', '/api/backtest', req);
            results.push({ name: item, isCustom: item.startsWith('custom_'), result: r });
        } catch (e) {
            console.error('策略失败:', item, e);
        }
    }

    if (results.length === 0) {
        $('#cmp-table').innerHTML = '<p style="color:var(--red);padding:0.6rem">所有策略都跑失败</p>';
        return;
    }

    // 表格
    const metrics = [
        { key: '总收益率', label: '总收益', sign: true, pct: true },
        { key: '年化收益率', label: '年化', sign: true, pct: true },
        { key: '最大回撤_pct', label: '回撤', pct: true },
        { key: '夏普比率', label: '夏普', sign: true },
        { key: '索提诺比率', label: '索提诺', sign: true },
        { key: 'Calmar比率', label: 'Calmar', sign: true },
        { key: '交易笔数', label: '笔数' },
        { key: '胜率', label: '胜率', pct: true },
        { key: '盈亏比', label: '盈亏比' },
        { key: '最终净值', label: '净值', money: true },
    ];
    const friendlyNames = {};
    strategiesMeta.forEach(s => friendlyNames[s.name] = s.display_name);
    customStrategies.forEach((cs, idx) => {
        friendlyNames[`custom_${idx}`] = '✦ ' + cs.name;
    });

    let html = '<table><thead><tr><th>策略</th>';
    metrics.forEach(m => html += `<th>${m.label}</th>`);
    html += '</tr></thead><tbody>';
    for (const { name, result } of results) {
        const m = result.metrics;
        html += `<tr><td><b>${friendlyNames[name] || name}</b></td>`;
        for (const f of metrics) {
            let v = m[f.key];
            let display = '—';
            let cls = '';
            if (v !== undefined && v !== null) {
                if (f.money) display = fmtMoney(v);
                else if (f.pct) display = fmtPct(v);
                else if (f.key === '盈亏比') display = v === Infinity ? '∞' : fmtNum(v, 2);
                else display = typeof v === 'number' ? fmtNum(v, 2) : v;
                if (f.sign && typeof v === 'number') {
                    cls = v > 0 ? 'pos' : (v < 0 ? 'neg' : '');
                }
            }
            html += `<td class="${cls}">${display}</td>`;
        }
        html += '</tr>';
    }
    html += '</tbody></table>';
    $('#cmp-table').innerHTML = html;

    // 净值图
    const colors = ['#d29922', '#58a6ff', '#a371f7', '#ff7b72', '#56d4dd', '#7ee787', '#ffa657'];
    const traces = results.map(({ name, result }, i) => ({
        x: result.equity_curve.map(p => new Date(p.timestamp)),
        y: result.equity_curve.map(p => p.equity),
        type: 'scatter', mode: 'lines',
        name: friendlyNames[name] || name,
        line: { color: colors[i % colors.length], width: 1.8 },
    }));
    Plotly.purge('cmp-chart');
    Plotly.newPlot('cmp-chart', traces, {
        ...PLOT_LAYOUT, title: '策略净值对比',
        yaxis: { ...PLOT_LAYOUT.yaxis, title: 'Equity' },
        legend: { ...PLOT_LAYOUT.legend, orientation: 'h', y: -0.15 },
    }, PLOT_CONFIG);
}
$('#btn-run-cmp').addEventListener('click', runCompare);

// ============================================================
// 知识库
// ============================================================
let kbData = null;

async function loadKnowledge() {
    try {
        console.log('loadKnowledge: 拉取知识库');
        const d = await api('GET', '/api/knowledge');
        console.log('loadKnowledge: 拿到', d.total, '条');
        kbData = d;
        const totalEl = $('#kb-total');
        if (totalEl) totalEl.textContent = d.total;
        renderKbCategories();
        renderKbEntries();
    } catch (e) {
        console.error('loadKnowledge failed:', e);
        const el = $('#kb-content');
        if (el) el.innerHTML = '<p style="color:var(--red);padding:0.8rem">加载失败: ' + e.message + '</p>';
    }
}

function renderKbCategories() {
    if (!kbData) return;
    const cats = Object.keys(kbData.categories);
    let html = '<div style="display:flex;flex-wrap:wrap;gap:0.4rem;margin-bottom:0.8rem">';
    html += '<button class="kb-cat active" data-cat="">全部</button>';
    cats.forEach(c => {
        html += `<button class="kb-cat" data-cat="${c}">${c} (${kbData.categories[c].length})</button>`;
    });
    html += '</div>';
    $('#kb-categories').innerHTML = html;
    $$('.kb-cat').forEach(btn => {
        btn.addEventListener('click', () => {
            $$('.kb-cat').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            renderKbEntries();
        });
    });
}

function renderKbEntries() {
    if (!kbData) {
        $('#kb-content').innerHTML = '<p style="color:var(--text-dim);padding:1rem">等待数据...</p>';
        return;
    }
    const activeCat = document.querySelector('.kb-cat.active')?.dataset.cat || '';
    const search = $('#kb-search').value;
    const cats = activeCat ? [activeCat] : Object.keys(kbData.categories);
    let html = '';
    cats.forEach(c => {
        let items = kbData.categories[c];
        const q = search.trim().toLowerCase();
        if (q) {
            items = items.filter(e =>
                (e.name || '').toLowerCase().includes(q) ||
                (e.id || '').toLowerCase().includes(q) ||
                (e.formula || '').toLowerCase().includes(q) ||
                (e.summary || '').toLowerCase().includes(q) ||
                (e.meaning || '').toLowerCase().includes(q) ||
                (e.signals || '').toLowerCase().includes(q) ||
                (e.example || '').toLowerCase().includes(q)
            );
        }
        if (items.length === 0) return;
        html += `<h3 style="margin-top:1.25rem">${c}<span style="color:var(--text-dim);font-weight:normal;font-size:0.85em;margin-left:0.5rem">(${items.length})</span></h3>`;
        items.forEach(e => {
            const ghBadge = e.code_url
                ? `<a href="${e.code_url}" target="_blank" rel="noopener" style="display:inline-block; margin-left:0.5rem; padding:0.1rem 0.4rem; background:var(--bg-elev); color:var(--accent); border:1px solid var(--accent); border-radius:3px; font-size:0.7rem; text-decoration:none; font-family:var(--mono);" title="查看 GitHub 源码">↗ GitHub</a>`
                : '';
            html += `
                <details class="kb-entry">
                    <summary>
                        <strong>${e.name}</strong>${ghBadge}
                        <span style="color:var(--text-dim);font-weight:normal;font-size:0.78em;margin-left:0.5rem">${e.id}</span>
                        <div style="color:var(--text-dim);font-weight:normal;font-size:0.82em;margin-top:0.25rem;font-style:italic">${e.summary || ''}</div>
                    </summary>
                    <div class="kb-body">
                        <div class="kb-section">
                            <div class="kb-label">公式</div>
                            <div class="kb-formula"><code>${e.formula || 'N/A'}</code></div>
                        </div>
                        <div class="kb-section">
                            <div class="kb-label">含义</div>
                            <div>${formatParagraphs(e.meaning || '')}</div>
                        </div>
                        ${e.example ? `<div class="kb-section">
                            <div class="kb-label">例子</div>
                            <div style="color:var(--accent-2)">${formatParagraphs(e.example)}</div>
                        </div>` : ''}
                        <div class="kb-section">
                            <div class="kb-label">信号</div>
                            <div>${formatParagraphs(e.signals || '')}</div>
                        </div>
                        <div class="kb-section">
                            <div class="kb-label">误区</div>
                            <div style="color:var(--red)">${formatParagraphs(e.pitfalls || '')}</div>
                        </div>
                        ${e.code_url ? `<div class="kb-section">
                            <div class="kb-label">代码实现</div>
                            <a href="${e.code_url}" target="_blank" rel="noopener" class="kb-code-link">${e.implementation || e.code_url}</a>
                        </div>` : ''}
                    </div>
                </details>`;
        });
    });
    if (!html) html = '<p style="color:var(--text-dim);padding:0.8rem">未找到匹配的概念</p>';
    $('#kb-content').innerHTML = html;
}

// 把多段文本(以空行分隔)变成 <p> 标签
function formatParagraphs(text) {
    if (!text) return '';
    return text.split(/\n\s*\n/).map(p => `<p style="margin:0.3rem 0">${p.trim()}</p>`).join('');
}
$('#kb-search').addEventListener('input', renderKbEntries);

// ============================================================
// 启动
// ============================================================
initDropdowns();
loadData();
loadKnowledge();
loadStrategiesMeta();
loadSymbols();