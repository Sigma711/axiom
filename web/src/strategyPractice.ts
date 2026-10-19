export type PracticeField = { key: string; label: string; initial: number; min: number; max: number; step?: number; unit: string };
export type PracticeMetric = { label: string; value: number; unit: string; meaning: string };
export type PracticeResult = { metrics: PracticeMetric[]; reading: string };
export type StrategyExercise = {
  id: string;
  title: string;
  task: string;
  fields: readonly PracticeField[];
  compute: (values: Record<string, number>) => PracticeResult;
  realLink?: { label: string; path: string; explanation: string };
};

const field = (key: string, label: string, initial: number, unit: string, min = -1e6, max = 1e6, step = 0.01): PracticeField => ({ key, label, initial, unit, min, max, step });
const metric = (label: string, value: number, unit: string, meaning: string): PracticeMetric => ({ label, value, unit, meaning });
const pct = (ratio: number) => ratio * 100;

export const STRATEGY_EXERCISES: readonly StrategyExercise[] = [
  {
    id: 'trend', title: '趋势跟踪：突破后赚到了吗？', task: '先用已完成日线确认突破，再区分信号价、下一次可交易开盘价与退出价。',
    fields: [field('high', '此前窗口最高收盘价', 100, '元', .01), field('close', '信号日收盘价', 105, '元', .01), field('entry', '下一时点开盘价', 106, '元', .01), field('exit', '退出成交价', 112, '元', .01), field('cost', '双向总成本', .2, '%', 0, 100)],
    compute: v => ({ metrics: [metric('突破是否确认', v.close > v.high ? 1 : 0, '1=是，0=否', '只比较信号日收盘价与此前窗口，不能提前读到下一根。'), metric('入场至退出净变化', pct(v.exit / v.entry - 1) - v.cost, '%', '用下一时点开盘价而不是信号日收盘价作为入场基准。')], reading: '若没有突破，后一个收益数字只是价格区间演算，不代表这条规则会开仓。' }),
    realLink: { label: '用真实行情看 Donchian 突破', path: '/data?concept=donchian&source=binance', explanation: '指标实践展示可观察的突破水平；策略收益仍需在回测核对成交与成本。' },
  },
  {
    id: 'momentum', title: '动量：选出强者后还强吗？', task: '给同一时点的两只资产排序，再看下一期结果；不要把事后赢家放进事前排名。',
    fields: [field('pastA', 'A 的历史收益', 12, '%', -100, 1000), field('pastB', 'B 的历史收益', 4, '%', -100, 1000), field('futureA', 'A 的下一期收益', -3, '%', -100, 1000), field('futureB', 'B 的下一期收益', 2, '%', -100, 1000)],
    compute: v => ({ metrics: [metric('事前领先者', v.pastA >= v.pastB ? 1 : 2, '1=A，2=B', '排名仅使用已经发生的历史收益。'), metric('领先者相对下一期收益', v.pastA >= v.pastB ? v.futureA - v.futureB : v.futureB - v.futureA, '百分点', '正数才表示这一次领先者继续领先；一个样本不证明长期有效。')], reading: '改动下一期收益不应该改变事前选中的资产；这正是检查未来数据泄漏的办法。' }),
    realLink: { label: '用真实行情计算价格动量', path: '/data?concept=roc&source=binance', explanation: '该入口看单资产价格变化；横截面排名需要同日对齐多个标的。' },
  },
  {
    id: 'reversion', title: '均值回归：偏离与回归是两回事', task: '先量化偏离，再观察后来是否缩小；不要把超卖当成一定会涨。',
    fields: [field('mean', '历史参考均值', 100, '元', .01), field('std', '历史标准差', 5, '元', .01), field('now', '当前价格', 90, '元', .01), field('later', '之后价格', 95, '元', .01)],
    compute: v => ({ metrics: [metric('当前偏离', (v.now - v.mean) / v.std, '个标准差', '均值和标准差必须只由此前数据估计。'), metric('之后价格变化', pct(v.later / v.now - 1), '%', '价格回升不代表已扣除成交费用，也不能证明均值稳定。')], reading: '试着把“之后价格”改得更低：偏离可以继续扩大，这就是均值回归策略的核心风险。' }),
    realLink: { label: '在真实 K 线上观察布林带', path: '/data?concept=bbands&source=binance', explanation: '布林带展示滚动参照与偏离；不是自动买卖信号。' },
  },
  {
    id: 'factor', title: '因子投资：排序正确一次够吗？', task: '按事前因子值挑一只，再比较随后收益，理解“因子诊断”与组合证据的距离。',
    fields: [field('scoreA', 'A 的事前因子值', 1.5, '分'), field('scoreB', 'B 的事前因子值', .5, '分'), field('returnA', 'A 的下一期收益', 3, '%', -100, 1000), field('returnB', 'B 的下一期收益', 1, '%', -100, 1000)],
    compute: v => ({ metrics: [metric('因子领先者', v.scoreA >= v.scoreB ? 1 : 2, '1=A，2=B', '只能用调仓前已公布、已可得的因子值。'), metric('领先者相对下一期收益', v.scoreA >= v.scoreB ? v.returnA - v.returnB : v.returnB - v.returnA, '百分点', '还要在多个日期、完整标的池和费用下重复检验。')], reading: '把后续收益对调，事前排序不应改变；一对股票只是演示，不是可交易因子检验。' }),
    realLink: { label: '逐项核算真实行情因子', path: '/learn/alphas', explanation: '研究台展示可核验的单标的因子序列和后续收益。' },
  },
  {
    id: 'stat-arb', title: '统计套利：价差真的收敛了吗？', task: '先固定对冲比例，再比较前后价差；只演算关系，不假装两腿已同步成交。',
    fields: [field('a0', '起点 A 价格', 100, '元', .01), field('b0', '起点 B 价格', 80, '元', .01), field('a1', '之后 A 价格', 98, '元', .01), field('b1', '之后 B 价格', 75, '元', .01), field('hedge', '事前对冲比例', 1, 'B/每 A', 0, 100)],
    compute: v => { const before = v.a0 - v.hedge * v.b0; const after = v.a1 - v.hedge * v.b1; return { metrics: [metric('起点价差', before, '元', 'A 价格减去固定比例的 B 价格。'), metric('价差变化', after - before, '元', '若起点选择做空价差，正变化会不利；这里未计算借券与两腿成交。')], reading: '把“事前对冲比例”换成事后最优数值，会把未来信息带回起点。' }; },
    realLink: { label: '核对真实双标的价差', path: '/data?concept=book_pair_spread&source=binance', explanation: '两条已完成行情按同一时点对齐；图中价差不是双腿实盘收益。' },
  },
  {
    id: 'event', title: '事件驱动：公告之后还能以什么价成交？', task: '把公告前收盘、公告后首个可交易开盘和之后退出分开。',
    fields: [field('before', '公告前收盘价', 100, '元', .01), field('open', '公告后首个开盘价', 110, '元', .01), field('exit', '之后退出价', 112, '元', .01), field('cost', '双向总成本', .2, '%', 0, 100)],
    compute: v => ({ metrics: [metric('公告后的开盘跳空', pct(v.open / v.before - 1), '%', '这部分变化不能假定在公告前收盘价成交。'), metric('首个开盘至退出的净变化', pct(v.exit / v.open - 1) - v.cost, '%', '以读到公告之后的可交易价格为起点，并扣除成本。')], reading: '若公告是收盘后发布，把公告前收盘价当买入价会高估可交易收益。' }),
    realLink: { label: '核对历史公司行动与来源', path: '/data?concept=book_adjustment&source=us_stock', explanation: '固定的 Apple 历史拆股案例用于核对事件和行情口径，不代表自动事件策略。' },
  },
  {
    id: 'arbitrage', title: '套利：表面价差剩下多少？', task: '买便宜市场的卖盘、卖昂贵市场的买盘，再扣完整交易和转移成本。',
    fields: [field('ask', '便宜市场卖一价（买入）', 100, '元', .01), field('bid', '昂贵市场买一价（卖出）', 100.7, '元', .01), field('cost', '每单位全部成本', .8, '元', 0), field('size', '计划数量', 10, '单位', .01)],
    compute: v => ({ metrics: [metric('每单位净价差', v.bid - v.ask - v.cost, '元', '需使用可成交的买一/卖一报价，而非最新成交价。'), metric('计划数量的净额', (v.bid - v.ask - v.cost) * v.size, '元', '仍需两边同时成交、库存和转移约束；负数表示这组条件不划算。')], reading: '把费用从 0.8 改为 0.2，看看结论如何改变；可见价差不是免费利润。' }),
    realLink: { label: '观察真实盘口价差', path: '/data?concept=bid_ask_spread&source=binance', explanation: '盘口快照能核对一处市场的买卖价差；跨市场成交由教学算例分解。' },
  },
  {
    id: 'market-making', title: '做市：只成交一边会怎样？', task: '分别算两边都成交和只买成后的库存盯市；先别假设报价一定能成交。',
    fields: [field('bid', '挂出的买价', 99.9, '元', .01), field('ask', '挂出的卖价', 100.1, '元', .01), field('later', '之后的中间价', 99.4, '元', .01), field('fee', '每次成交每单位费用', .02, '元', 0), field('size', '每边数量', 10, '单位', .01)],
    compute: v => ({ metrics: [metric('假设双边均成交', (v.ask - v.bid - 2 * v.fee) * v.size, '元', '只有两个独立挂单确实成交后才能赚到这个价差。'), metric('只买成后的盯市变化', (v.later - v.bid - v.fee) * v.size, '元', '库存按之后中间价估值；中间价不是保证可卖出的成交价。')], reading: '下跌行情可能只打到你的买单，另一边卖单没有成交；盘口快照不能证明排队中的订单已成交。' }),
    realLink: { label: '观察真实现货盘口', path: '/data?concept=book_order_imbalance&source=binance', explanation: '盘口快照展示挂单分布；练习中的成交情景明确是假设。' },
  },
  {
    id: 'volatility', title: '波动率：跨式期权到期赚亏', task: '把期权权利金与到期支付拆开。这里是到期现金流教学，不预测期权市场价格。',
    fields: [field('strike', '共同执行价', 100, '元', .01), field('call', '看涨权利金', 5, '元/股', 0), field('put', '看跌权利金', 4, '元/股', 0), field('settlement', '到期标的价格', 120, '元', 0), field('multiplier', '合约乘数', 100, '股/组', 1, 100000, 1)],
    compute: v => ({ metrics: [metric('到期支付', (Math.max(v.settlement - v.strike, 0) + Math.max(v.strike - v.settlement, 0)) * v.multiplier, '元/组', '看涨与看跌分别按到期内在价值结算。'), metric('扣权利金后结果', (Math.abs(v.settlement - v.strike) - v.call - v.put) * v.multiplier, '元/组', '还未计买卖价差、手续费与提前平仓价格。')], reading: '把到期价设为执行价，看看最大权利金损失；“波动大”与“足以覆盖权利金”是不同问题。' }),
    realLink: { label: '用真实 K 线测量已实现波动', path: '/data?concept=hv&source=binance', explanation: '历史波动是标的价格的已发生变化，不等同期权隐含波动率。' },
  },
  {
    id: 'carry', title: '套息：利息能盖住跌价吗？', task: '把持有现金流、融资成本和资产价格变化记在同一张账上。',
    fields: [field('income', '期间持有收入', 4, '%', -100, 1000), field('financing', '期间融资成本', 2, '%', 0, 1000), field('price', '期间价格变化', -5, '%', -100, 1000)],
    compute: v => ({ metrics: [metric('净持有收益', v.income - v.financing, '%', '收到的现金流先减借钱的成本。'), metric('价格合并后的总变化', v.income - v.financing + v.price, '%', '简单相加的同基数教学口径；真实账本还要考虑现金流时点、汇率与复利。')], reading: '提高融资成本或加大价格跌幅，看看高票息为何不等于稳赚。' }),
  },
  {
    id: 'allocation', title: '资产配置：上涨后权重漂到哪里？', task: '两类资产从目标权重出发，经过一段涨跌后算新权重与再平衡量。',
    fields: [field('weight', 'A 的初始权重', 60, '%', 0, 100), field('returnA', 'A 的期间收益', 20, '%', -99.99, 1000), field('returnB', 'B 的期间收益', 0, '%', -99.99, 1000)],
    compute: v => { const a = v.weight / 100 * (1 + v.returnA / 100); const b = (1 - v.weight / 100) * (1 + v.returnB / 100); const drift = pct(a / (a + b)); return { metrics: [metric('组合期间收益', pct(a + b - 1), '%', '两类资产按起点权重计算，尚未扣调仓成本。'), metric('A 的期末漂移权重', drift, '%', '权重会随价格变化，即使期间没有主动调仓。'), metric('若回到目标需调整 A', v.weight - drift, '期末资产的百分点', '负数表示卖出 A 的部分期末市值，正数表示买入。')], reading: '把 A 与 B 的收益交换，观察权重如何反向漂移；真实配置还需可交易价格与现金流。' }; },
  },
];

export function validatePracticeValues(exercise: StrategyExercise, values: Record<string, number>): string | null {
  for (const item of exercise.fields) {
    const value = values[item.key];
    if (!Number.isFinite(value) || value < item.min || value > item.max) return `${item.label}必须在 ${item.min} 至 ${item.max} 之间。`;
  }
  return null;
}

export function calculatePractice(exercise: StrategyExercise, values: Record<string, number>): PracticeResult {
  const error = validatePracticeValues(exercise, values);
  if (error) throw new Error(error);
  return exercise.compute(values);
}
