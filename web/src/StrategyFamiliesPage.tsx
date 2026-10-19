import './StrategyFamiliesPage.css';

type Family = {
  id: string;
  name: string;
  english: string;
  question: string;
  idea: string;
  example: string;
  data: string;
  rule: string;
  failure: string;
  check: string;
  flow: readonly [string, string, string];
};

export const STRATEGY_FAMILIES: readonly Family[] = [
  {
    id: 'trend', name: '趋势跟踪', english: 'Trend Following', question: '一只资产已经走出方向，还会走一段吗？',
    idea: '只比较它与自己的历史。突破或均线转强后跟随，跌破退出；赚的是少数持续行情，横盘时可能反复小亏。趋势规则会晚进晚出，不会抓住最低点。',
    example: '教学例子：收盘价 105 元，过去 20 根已完成日线的最高收盘价为 100 元；收盘确认突破后，最早在下一可交易时点尝试买入。',
    data: '同口径的历史价格、交易日历和成交量；跨资产还要处理合约换月、复权与波动率尺度。',
    rule: '先固定突破窗口、退出窗口与仓位，再逐根生成信号；把下一时点成交价、滑点和停牌限制交给执行模块。',
    failure: '震荡市容易来回止损；跳空会使实际退出价远离阈值，趋势反转时可能回吐大量浮盈。',
    check: '在回测里分别运行双均线与 Donchian 突破，比较趋势段、横盘段的交易次数、回撤和成本。',
    flow: ['已完成价格突破', '下一时点尝试跟随', '跌破退出线并核算成本'],
  },
  {
    id: 'momentum', name: '动量策略', english: 'Momentum', question: '谁最近更强；自己的方向又是否够强？',
    idea: '“时间序列动量”用同一资产过去的涨跌判断方向，与趋势跟踪重叠；“横截面动量”把同一时点的多只资产排名，持有相对强者。两者不能混为一个回测。',
    example: '教学例子：A、B、C 在同一回看期分别涨 12%、4%、−3%，横截面规则选 A；若只研究 A 的涨幅是否为正，那是时间序列问题。',
    data: '可对齐的多标的历史价格、当时真实存在的标的池、拆股分红和交易成本；排名时只能用调仓前已知的收益。',
    rule: '预先写好回看期、是否跳过最近一个月、排名数量和调仓频率；下一交易时点按目标权重下单。',
    failure: '强势资产可能集体急跌；频繁换仓、拥挤交易和样本中缺失退市标的都会夸大结果。',
    check: '先在因子研究看单标的价格变化特征，再手工对齐三只股票同一天的收益排名；不要把单标的诊断当作横截面组合收益。',
    flow: ['同日对齐历史收益', '按预定规则排名', '下次调仓并计入换手'],
  },
  {
    id: 'reversion', name: '均值回归', english: 'Mean Reversion', question: '偏离什么“正常水平”，又凭什么回来？',
    idea: '先定义一个有意义的参照物，例如一段稳定关系的价差或成交量加权均价；偏离后押注收敛。价格跌了很多并不自动意味着便宜。',
    example: '教学例子：过去 30 天价差均值为 0、标准差为 2；今天价差为 5，即偏离 2.5 个标准差。但若企业基本面变了，旧均值可能不再成立。',
    data: '已完成行情、固定的估计窗口、可交易报价；若是两腿价差，还需同步行情、融资融券与两笔成交。',
    rule: '写清参照水平、入场偏离、回归退出和“关系失效”止损；只用信号形成前的窗口估计均值。',
    failure: '趋势行情会让价格持续远离均值；看似连续小赢，可能被一次结构性变化吞没。',
    check: '用 RSI、布林带或 VWAP 回归示例比较不同市场阶段；振荡指标超卖只是观察，不是回归必然发生的证据。',
    flow: ['定义可解释参照', '确认偏离后入场', '回归或失效时退出'],
  },
  {
    id: 'factor', name: '因子投资', english: 'Factor Investing', question: '哪些可量化特征，可能与之后的收益有关？',
    idea: '价值、质量、规模、动量等特征先被量化为因子，再在同一时点对股票排序，形成可执行组合。一个因子值与后续收益相关，不等于把它单独买入就能赚钱。',
    example: '教学例子：在同一天给 100 只股票计算“盈利/市值”，按行业内分组比较下一期收益；随后检验费用、行业暴露与调仓后的净收益。',
    data: '历史标的池、带公布时间的财报、可交易价格、公司行动与行业分类；财报所属期间不等于市场知道它的日期。',
    rule: '先定因子定义、缺失值处理、极值处理、排序和调仓周期，再定组合约束与样本外区间。',
    failure: '因子可能只是行业或小盘暴露；拥挤、数据修订、口径漂移和反复筛选也会制造虚假显著性。',
    check: '逐项阅读 Alpha101/Alpha158，并在真实日线研究台核算可运行因子；另列出从单标的诊断走到横截面投资还缺哪些数据。',
    flow: ['按可得时间计算特征', '同日横截面排序', '建组合并检验净收益'],
  },
  {
    id: 'stat-arb', name: '统计套利', english: 'Statistical Arbitrage', question: '两项价格之间的关系，是否暂时偏离了历史规律？',
    idea: '先建立可解释的相对价值关系，再交易残差或价差。例如买入相对便宜的一腿、卖出相对昂贵的一腿。它依赖模型和关系维持，绝非无风险套利。',
    example: '教学例子：按事先估计的对冲比例构造 A−1.2×B；价差偏离训练期均值 2 个标准差后观察是否收敛。不能用全样本未来价格重估比例。',
    data: '同步的多资产报价、历史成员、对冲比例、借券可得性和资金成本；相关系数高并不能单独证明价差会回归。',
    rule: '分开训练和检验区间，固定配对与参数；同时处理双腿数量、成交时间和未成交一腿的暴露。',
    failure: '公司合并、业务分化或制度变化会使旧关系断裂；做空召回、杠杆与单腿成交也会放大损失。',
    check: '在数据探索对齐两条真实价格序列，观察价差、样本外稳定性及缺口；不要用单标的 K 线回测宣称双腿收益。',
    flow: ['建立历史相对关系', '观察样本外残差', '双腿执行并控制敞口'],
  },
  {
    id: 'event', name: '事件驱动', english: 'Event-Driven', question: '新信息在何时公开，价格又在何时能交易？',
    idea: '研究财报、并购、分拆、回购等事件带来的价格变化。事件名称不是信号；必须知道公告具体时间、市场预期和首次可交易价格。',
    example: '教学例子：公司 16:30 发布超预期财报，15:00 收盘价不能当作读到公告后的买入价；下一交易日开盘若已跳涨，利润空间可能消失。',
    data: '原始公告、精确发布时间、修订记录、公司行动、交易日历、停牌及逐时或逐笔价格。',
    rule: '事先定义事件筛选、公开时点、最早下单时点、退出期限和对照组；同一公司多次公告需去重。',
    failure: '市场可能早已预期；重大事件常伴跳空、停牌和成交不足，事后挑“好消息”会产生选择偏差。',
    check: '先在指标大全的财务披露案例核对原文、期间与发布时间，再画事件前后的可交易价格；不要把固定案例当成已运行的事件策略。',
    flow: ['核实公告公开时间', '确定首个可交易报价', '对照事件后净收益'],
  },
  {
    id: 'arbitrage', name: '套利交易', english: 'Arbitrage', question: '看见价差后，扣完全部约束还剩什么？',
    idea: '同一或可转换资产在不同市场的可执行价格不一致，才可能存在套利空间。只看到屏幕上的两行报价，不等于能同时买到、卖出并完成交割。',
    example: '教学例子：A 市场卖价 100.0，B 市场买价 100.7，表面价差 0.7；手续费 0.2、汇兑与转移 0.3、滑点 0.3 后已是 −0.1。',
    data: '两边同时刻可成交的买卖盘、深度、费用、汇率、融资、提币或交割时间及账户余额。',
    rule: '用买价卖价而非两边最新成交价；预留双边库存，规定未成交一腿的撤单或对冲办法。',
    failure: '价差可能因转移慢、信用或交割风险而存在；交易所限制、延迟和盘口消失会使账面利润无法兑现。',
    check: '在数据探索查看买卖价差和盘口深度，列一张从毛价差到净价差的成本表；再说明为什么日线收盘价不能验证此策略。',
    flow: ['同刻核对可成交报价', '扣除费用与转移成本', '双边成交并完成交割'],
  },
  {
    id: 'market-making', name: '做市策略', english: 'Market Making', question: '报出买卖两边的价格，谁会来成交？',
    idea: '在买卖盘挂限价单提供流动性，目标是赚取部分价差；成交只发生在别人愿意打到你的报价时。报价要随库存和市场波动调整。',
    example: '教学例子：买一 99.9、卖一 100.1，看似有 0.2 价差；只成交买单后价格跌到 99.4，持仓损失已经超过价差。',
    data: '逐笔委托与成交、盘口队列、撤单、延迟、费率和库存；仅有 OHLC K 线无法知道自己的订单是否排到成交。',
    rule: '限制单边库存，规定最大报价宽度、撤单时机和风险停机；先模拟队列与逆向选择，再谈利润。',
    failure: '知情交易者可能只在价格将对你不利时成交；库存失衡、延迟和极端行情会迅速吞掉价差。',
    check: '在数据探索看订单簿与主动买卖成交，分清挂单和成交；不要拿 K 线高低价假装两个报价都成交。',
    flow: ['两侧报价并排队', '观察成交与库存', '重报价格或停止报价'],
  },
  {
    id: 'volatility', name: '波动率策略', english: 'Volatility Trading', question: '市场为未来波动付出的价格，和实际波动有何差别？',
    idea: '用期权等工具交易隐含波动率与未来实际波动率的差异，不必单纯押涨跌。但“方向中性”也不等于风险中性；跳空和对冲成本很关键。',
    example: '教学例子：期权价格隐含年化波动 30%，研究者预估未来实际波动 20%；即使估计正确，也要检验权利金、时间损耗、买卖价差和对冲结果。',
    data: '期权合约链、执行价、到期日、可成交报价、标的价格、利率和股息；还要理解 Delta、Gamma、Vega 等风险。',
    rule: '先写清买卖哪组期权、何时及如何对冲 Delta、最大亏损和保证金；以组合现金流验证，而非只比较两个百分比。',
    failure: '隐含与实际波动口径不同、波动率曲面变化和极端跳空都可能令“正确预测”仍然亏钱。',
    check: '在数据探索用 ATR 和历史收益测量已发生的波动，再解释它为什么不是期权的隐含波动率。',
    flow: ['观察期权隐含定价', '估计未来实际波动', '计入对冲与尾部损失'],
  },
  {
    id: 'carry', name: '套息／持有收益', english: 'Carry', question: '持有期间收到的收益，能否覆盖价格和融资风险？',
    idea: '收益可能来自利差、票息、期货期限结构或资金费率。它不是白拿的利息；资产价格、汇率和融资条件变化都可能抵消累积收益。',
    example: '教学例子：一年预期收到 4% 票息，融资成本 2%，但资产价格下跌 5%；忽略费用前，总回报约为 −3%。',
    data: '历史票息与分红的除权时点、融资利率、期货曲线或资金费率、汇率和担保品要求。',
    rule: '分别记持有收入、价格变化、融资和展期现金流；每次续借或换月都用当时能观察到的条款。',
    failure: '高收益率往往补偿违约、汇率、流动性或尾部风险；融资利率上升时净 carry 会突然消失。',
    check: '把一个真实市场的价格收益与可核验的持有现金流分开列账；当前数据缺项时只演算教学数字，不声称历史策略收益。',
    flow: ['记录持有现金流', '扣融资与展期成本', '合并价格变化核算总回报'],
  },
  {
    id: 'allocation', name: '资产配置', english: 'Asset Allocation', question: '钱该分给哪些风险，而不只是哪些资产？',
    idea: '决定股票、债券、商品或策略之间的目标权重，并按规则再平衡。它可以装入前面多种策略；“分散到不同名称”不等于风险真的分散。',
    example: '教学例子：目标股票 60%、债券 40%；股票上涨后权重变成 67%，下一调仓日才按预定阈值卖出部分股票、补回债券。',
    data: '同期、同币种的总收益序列、历史可交易标的池、相关性、现金流、费用和约束。',
    rule: '先选静态权重、风险预算或其他明确方法，再规定调仓频率、容忍区间和资产上限；估计协方差只用过去窗口。',
    failure: '危机时相关性可能同时上升；短样本估计会让优化器把权重压到少数资产，换仓也会产生税费。',
    check: '用两个真实资产的日期对齐总收益，手算一次漂移与再平衡；不要把页面里的单交易对策略对比当成多资产组合回测。',
    flow: ['确定目标风险与权重', '观察实际权重漂移', '按阈值再平衡'],
  },
];

export function StrategyFamiliesPage({ onBuild, onExplore, onBacktest, onFactors, onPractice }: {
  onBuild: () => void;
  onExplore: () => void;
  onBacktest: () => void;
  onFactors: () => void;
  onPractice: (id: string) => void;
}) {
  return <div className="ax-families">
    <header className="ax-families-hero"><p className="ax-eyebrow">从想法到可检验规则</p><h3>11 类主流量化策略</h3><p>它们不是互斥的盒子。趋势跟踪与时间序列动量常重叠；统计套利可能借用均值回归；因子投资可以使用动量；资产配置还能组合多种策略。先问“收益从哪里来”，再问“需要什么数据和交易能力”。</p></header>
    <nav className="ax-families-index" aria-label="11 类策略目录">{STRATEGY_FAMILIES.map((family, index) => <a key={family.id} href={`#strategy-${family.id}`}><span>{String(index + 1).padStart(2, '0')}</span>{family.name}</a>)}</nav>
    <div className="ax-families-list">{STRATEGY_FAMILIES.map((family, index) => <article className="ax-family" id={`strategy-${family.id}`} key={family.id}>
      <header><span className="ax-family-number">{String(index + 1).padStart(2, '0')}</span><div><p className="ax-eyebrow">{family.english}</p><h4>{family.name}</h4><p className="ax-family-question">{family.question}</p></div></header>
      <div className="ax-family-flow" role="img" aria-label={`${family.name}机制示意：${family.flow.join('，然后')}`}><span>机制示意 · 非真实行情</span><ol>{family.flow.map(step => <li key={step}>{step}</li>)}</ol></div>
      <div className="ax-family-reading"><section><h5>到底在做什么</h5><p>{family.idea}</p></section><section><h5>用数字想一遍</h5><p>{family.example}</p></section><section><h5>先备齐数据</h5><p>{family.data}</p></section><section><h5>规则怎样落地</h5><p>{family.rule}</p></section><section><h5>什么时候会输</h5><p>{family.failure}</p></section><section><h5>自己怎么核验</h5><p>{family.check}</p></section></div>
      <div className="ax-family-action"><button type="button" className="ax-btn primary" onClick={() => onPractice(family.id)}>进入{family.name}实践 →</button></div>
    </article>)}</div>
    <aside className="ax-families-next"><div><h4>选好问题，再写策略</h4><p>先在数据探索核对输入，在因子研究中检查特征，再把明确的假设写成可以检验的规则。</p></div><div><button type="button" className="ax-btn" onClick={onExplore}>数据探索</button><button type="button" className="ax-btn" onClick={onFactors}>因子研究</button><button type="button" className="ax-btn" onClick={onBacktest}>回测示例</button><button type="button" className="ax-btn primary" onClick={onBuild}>创建策略</button></div></aside>
  </div>;
}
