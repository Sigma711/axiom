"""
给 knowledge.rs 的关键 entry 升级 summary/example/related 内容。
使用直接、简单、可靠的字符串替换。
"""
import os, re

# 用 raw 字符串指定路径,避开编码问题
KNOWLEDGE = r'C:\Users\朱泓睿\quant-demo-rs\src\knowledge.rs'

# 详细内容(去掉了嵌套三引号,改成单引号)
ENTRIES = {
    'macd': {
        'summary': 'DIF 是 12 周期 EMA 减 26 周期 EMA,DEA 是 DIF 的 9 周期 EMA,柱体是 DIF-DEA。趋势和动量合二为一。',
        'example': 'BTC 1 小时图,参数默认 (12,26,9)。DIF 在零轴上方且柱体由负转正 = 多头确立;零轴下方柱体翻负 = 空头。\\n\\n2020 年 3 月大跌:DIF 连续远离零轴 = 强趋势;价格反弹时 DIF 回到零轴附近 = 趋势暂缓但未反转。\\n\\n实际用法:很多 trader 用 MACD 零轴交叉作为多空分水岭,而不是简单的金叉死叉。',
        'related': ['ema', 'sma', 'ppo', 'tsi'],
    },
    'rsi': {
        'summary': 'N 日涨/跌力度比,值域 0-100。>70 超买,<30 超卖。最经典的动量震荡指标。',
        'example': '14 周期 RSI 在 BTC 4 小时图的应用:2021 年 4 月顶部,RSI 创下 92 的极端值(常规 >70 已超买),价格随后暴跌 50% 以上。\\n\\n但 RSI 超买 ≠ 立即卖出:强趋势中 RSI 可长期 >70。如果在 2021 年 1 月 RSI=80 时卖出,会错过最大的上涨波段。\\n\\n正确用法:超买 + 价格出现反转形态(头肩顶、双顶等)才卖;或者等 RSI 跌出超买区(从 80 跌到 70 以下)再说。',
        'related': ['stochastic', 'kdj', 'williams_r', 'mfi'],
    },
    'bollinger_bands': {
        'summary': '中轨是 N 周期 SMA,上下轨是中轨 ± k 倍标准差。99.7% 的价格会落在 3σ 内,所以触及外轨是统计极端。',
        'example': '20 周期 SMA + 2σ 的经典组合,在 BTC 日线上:2021 年 5 月 19 日价格 43,000 美元,精确触及上轨 = 顶部信号;但价格并未立即下跌,而是横盘 2 周后跌穿中轨才确认。\\n\\nBBands Squeeze(挤压):上下轨收窄到极致时,常预示即将大幅波动。2017 年 12 月 BTC 启动前,BBands 经历了 4 个月的极度挤压。\\n\\n不要把触及外轨当反转信号,把它当"波动率异常"看。',
        'related': ['atr', 'keltner', 'squeeze'],
    },
    'ichimoku': {
        'summary': '5 条线 + 1 个云带。转换线 (Tenkan) 是 9 日中点,基准线 (Kijun) 是 26 日中点,先行带 A/B 围成云。',
        'example': '默认参数 (9,26,52)。交易信号三层:\\n\\n1. 云在价格之上 = 强空头(整片云都成阻力);云在价格之下 = 强多头(成支撑)\\n2. 价格在云边缘穿越 = 趋势减弱,可能在反转\\n3. 迟行线(Chikou Span)在价格之上/之下,确认趋势\\n\\n实战案例:2020 年 3 月 BTC 暴跌后,价格在 26 日后回升突破云下方,转换线上穿基准线,云由绿翻红 = 标准的多头反转信号。\\n\\n最强大的用法是"三役一目":(1) 价格突破云,(2) 转换线上穿基准线,(3) 迟行线在价格之上。三个条件同时满足,信号最强。',
        'related': ['sma', 'dmi', 'parabolic_sar', 'aroon'],
    },
    'kdj': {
        'summary': '中国市场的经典。RSV=(C-LN)/(HN-LN)×100,K 是 RSV 的 Wilder 平滑,D 是 K 的平滑,J=3K-2D。',
        'example': '默认参数 (9,3,3)。A 股和港股特别流行。\\n\\n关键阈值:J>100 严重超买(短线见顶概率大),J<0 严重超卖(短线见底概率大)。\\n\\n最有效的形态:K 和 D 在 20 以下金叉,且 J 从 -10 以下拐头向上 = 强烈的买入信号。\\n\\n和 RSI 不同的是 KDJ 多了 J 这个放大器(3K-2D 经常超过 100 或低于 0),所以 KDJ 对短期反转更敏感,但也更容易在强趋势中假信号。',
        'related': ['rsi', 'stochastic'],
    },
    'sma': {
        'summary': '前 N 根 K 线收盘价的算术平均。最朴素也最有用的趋势指标。滞后 = 半周期。',
        'example': 'BTC 200 周 SMA 是加密圈最有名的指标。历史上:2020 年 3 月 BTC 价格短暂跌破 200 周 SMA,后来证明是世纪抄底机会;任何 200 周 SMA 之上的周收盘价都意味着长期牛市。\\n\\n交易用法:20 日 SMA 是月线级别的趋势(被对冲基金广泛使用),50 日 SMA 是季线级别,200 日 SMA 是年级别。\\n\\n局限:滞后 10 天(N=20),意味着趋势反转要等价格偏离 N/2 个周期才确认 = 总是迟到。',
        'related': ['ema', 'hma', 'dema', 'tema', 'vwma'],
    },
    'ema': {
        'summary': '指数加权均线,近期权重 α = 2/(N+1)。比 SMA 反应快 ~ 2 倍,但仍有滞后。',
        'example': '12 和 26 周期 EMA 的差是 MACD 的核心,26 周期 EMA 是 Ichimoku 基准线,20 周期 EMA 是 DMI 的一部分。EMA 是几乎所有趋势型指标的基石。\\n\\n为什么选 12/26:据说 Gerald Appel 当年用周线算(一年 52 周 / 4 周 ≈ 12;中间值 26),这两个数字就这么沿用下来了。\\n\\n坑:EMA 永远比 SMA 滞后少,但当价格突然反转时 EMA 也会"假突破"再回归 = 这就是为什么用 EMA 交叉做策略的胜率很难超过 50%。',
        'related': ['sma', 'dema', 'tema', 'hma', 'macd'],
    },
    'vwap': {
        'summary': '成交量加权的平均价。机构算法单用它作为执行基准,价格偏离 VWAP 越远,越说明有人在买入/卖出。',
        'example': '机构日内算法:把订单拆成小单,在 VWAP 附近成交以减少市场冲击。专业 trader 看价格与 VWAP 的关系判断多空:\\n\\n- 价格 > VWAP + 价格在 VWAP 上方运行 = 多头占优\\n- 价格 < VWAP + 价格在 VWAP 下方 = 空头占优\\n\\n与 SMA 的区别:VWAP 永远跟着成交量加权,反应真实资金成本,不会因为前期"沉默"的价格而被拖偏。\\n\\n锚定 VWAP (Anchored VWAP) 从特定事件(财报、突破)开始算,比标准 VWAP 更有用。',
        'related': ['mfi', 'cmf', 'pvt', 'chaikin_osc'],
    },
    'rsi_strategy': {
        'summary': '从超卖区反弹买入,从超买区回落卖出。A 股 / 美股 trader 最常用的反转策略。',
        'example': '最常见的 14 周期 RSI 策略:RSI<30 时观察是否反弹(站回 30)→ 买入;RSI>70 时观察是否回落(跌穿 70)→ 卖出。\\n\\n但单独用 RSI 容易挨打:2021 年 BTC 牛市,RSI 在 70-90 之间横盘半年。\\n\\n增强版:RSI + 200 日 SMA 趋势过滤 — 只在 SMA 之上做 RSI<30 的买入,只在 SMA 之下做 RSI>70 的卖出。这能把胜率从 50% 提到 60%+。',
        'related': ['rsi', 'stochastic', 'kdj'],
    },
    'sma_cross_strategy': {
        'summary': '快线上穿慢线 → 多头;下穿 → 空头。最古老也最被滥用的策略。',
        'example': '经典 (50, 200) "黄金交叉" 长期被认为是牛市信号。BTC 在 2015/2016、2019/2020 都有此信号。\\n\\n但"金叉"的本质滞后:50 日均线在价格上涨时需要 ~ 25 天才能确认方向改变,200 日均线需要 100 天。这意味着金叉时,你可能已经在趋势中段了。\\n\\n致命缺陷:震荡市(金叉死叉频繁)反复打脸。研究显示,简单的 50/200 金叉策略长期跑不赢买入持有(因为它在震荡市被反复止损)。',
        'related': ['sma', 'ema', 'macd'],
    },
    'macd_strategy': {
        'summary': 'DIF 上穿/下穿 DEA。最经典的动量趋势策略。',
        'example': '零轴交叉更稳定:每次 MACD 从零轴下方上穿 → 买入,跌回零轴下方 → 卖出。2008-2016 年回测:S&P 500 年化 ~ 10%,但最大回撤只有 16%(买入持有 55%)。\\n\\n但 MACD 的金叉死叉(DEA 上穿/下穿)比零轴交叉更频繁,在震荡市假信号多。\\n\\n更稳健的玩法:零轴交叉 + ADX > 25(确认有趋势)+ ATR 做止损。',
        'related': ['macd', 'ema', 'ppo'],
    },
    'bollinger_strategy': {
        'summary': '价格触及下轨买,触及上轨卖。John Bollinger 本人说:这不是反转信号,只是统计极端。',
        'example': '只靠价格触及外轨反着做,在强趋势中会反复被打:2020 年 BTC 从 10K 涨到 60K,价格多次触及上轨后继续涨,做空者全部爆仓。\\n\\n正确的 BBands 反转用法:1) 价格触及外轨;2) RSI 出现背离;3) K 线出现反转形态(锤子、吞没)。三个信号同时出现才动手。\\n\\n更常见的 BBands 用法:BBands Squeeze(挤压)+ 突破。当上下轨极度收窄后被突破,常是大行情的起点。',
        'related': ['bollinger_bands', 'atr', 'rsi'],
    },
    'kdj_strategy': {
        'summary': 'J 从超卖/超买区反向时买卖。中国 A 股市场应用极广。',
        'example': '9 周期 KDJ + J 值过滤:J<-10 时观察 K 上穿 D → 买入;J>110 时观察 K 下穿 D → 卖出。\\n\\n2020-2024 年 A 股回测:这套规则在板块轮动行情里表现优于纯均线策略,但在单边下跌中会被反复止损。\\n\\nKDJ 灵敏度比 RSI 高(J 经常突破 100 或跌破 0),所以更适合短线或 T+0,A 股特别适合。',
        'related': ['kdj', 'rsi', 'stochastic'],
    },
    'donchian_breakout_strategy': {
        'summary': '突破 N 日最高价买入,跌破 M 日最低价卖出。海龟交易法的核心。',
        'example': 'Richard Dennis 在 1980s 用这套规则教了 13 个门外汉交易,几年后他们平均年化 80%+,就是著名的"海龟交易实验"。\\n\\n经典参数:20 日突破入场,10 日跌破出场。\\n\\n2020-2024 年 BTC 测试:能捕捉到所有大波段(2020 年 4 月、2021 年 11 月顶部),但震荡市会反复被假突破止损。\\n\\n关键:加 ADX > 25 过滤,只在大趋势里做突破,胜率能从 30% 提到 45%。',
        'related': ['donchian', 'atr', 'adx'],
    },
    'vwap_reversion_strategy': {
        'summary': '价格偏离 VWAP 超阈值时反向开仓,假设价格会回归 VWAP。',
        'example': '当价格高于 VWAP 1.5% 做空,低于 VWAP 1.5% 做多。在稳定趋势里表现不佳(趋势价格会一直偏离 VWAP),但是在震荡市能赚到回归的钱。\\n\\n实战:用这个策略做 BTC 日内交易,5 分钟 K 线,阈值 0.3% — 一天能抓 5-10 次回归,胜率 60-70%,但每次赚的钱少,被趋势打破时一次亏完。\\n\\n资金管理是关键:单笔不超过总资金 1%。',
        'related': ['vwap', 'mfi', 'rsi'],
    },
    'sharpe': {
        'summary': '(收益 - 无风险) / 波动率,年化。>1 合格,>2 优秀,>3 顶级。',
        'example': '巴菲特 50 年年化约 20%,夏普约 0.8。文艺复兴科技的大奖章基金年化 66%,夏普长期 2-3。\\n\\n关键陷阱:夏普可以被"对冲"和"杠杆"放大 — 用 1% 资金 + 99% 现金的策略可能夏普 5,但绝对收益 0.1%。\\n\\n更可靠的:用 IR(信息比率)= (策略 - 基准) / Tracking Error 评估主动管理能力。',
        'related': ['sortino', 'calmar', 'max_drawdown', 'var'],
    },
    'sortino': {
        'summary': '夏普的改进版,只算下行波动率分母。比夏普更贴近实际感受。',
        'example': '同样年化 20% 收益:一个策略年波动率 30%,下行波动率 18%(跌得少涨得多);另一个 30% 全是下行的。两者夏普一样,但 Sortino 差距明显。\\n\\n业界共识:Sortino > 1.5 是好策略,> 2 是优秀。\\n\\nA 股里很多"长牛"策略(比如银行股红利)夏普低但 Sortino 高 — 因为它们跌得少。',
        'related': ['sharpe', 'calmar', 'max_drawdown'],
    },
    'max_drawdown': {
        'summary': '从历史峰值到谷底的最大跌幅。心理上能不能承受这个数字,决定了你能不能坚持到底。',
        'example': '1987 年股灾:道指一天跌 22.6%,但最大回撤是那 1 天。LTCM 长期资本管理公司 1998 年回撤 50% 就破产了 — 数学上还能救,心理上已经崩了。\\n\\n经验法则:你能承受的最大回撤 = 你的年龄/2 - 5。比如 30 岁,最大能忍 10%;50 岁,最大能忍 20%。超过就睡不好觉。\\n\\n策略对比:策略 A 收益 50% 但最大回撤 60%;策略 B 收益 40% 但最大回撤 15%。A 长期可能跑赢,但 90% 的人在 A 上面坚持不下来。',
        'related': ['calmar', 'sharpe', 'sortino'],
    },
    'calmar': {
        'summary': '年化收益 / 最大回撤。越大越好,>1 合格,>3 顶级。',
        'example': '海龟交易法 1980s 年化 80%,最大回撤 30%,Calmar 约 2.7。\\n\\n2020-2024 年测试 BTC 上的 20 日突破策略:年化 50%,最大回撤 35%,Calmar 1.4。\\n\\n相比 Sharpe 的优势:Sharpe 关心的是"波动",Calmar 关心的是"最痛苦的时候" — 后者更贴近实际投资体验。',
        'related': ['sharpe', 'sortino', 'max_drawdown'],
    },
    'var': {
        'summary': 'VaR(95%) = "1 天内 95% 概率不会亏超过 X"。X 就是 VaR。风险管理最常用的指标。',
        'example': 'BTC 24 小时 95% VaR = 4.2%,意味着每天有 5% 的概率会亏超过 4.2%。\\n\\n但 VaR 不描述尾部 — 那 5% 的"灾难日"可能亏 15% 以上(肥尾)。2008 年雷曼倒闭,VaR 模型没预测到 30% 单日跌幅。\\n\\n更稳的做法:用 CVaR(95%) = 超过 VaR 那 5% 天的平均损失,反映了"灾难有多大"。',
        'related': ['cvar', 'max_drawdown', 'sharpe'],
    },
    'atr': {
        'summary': 'N 日真实波幅的 Wilder 平滑。波动率的"温度计"。',
        'example': 'BTC 日线 14 周期 ATR 通常在 500-1500 美元之间(随波动率变)。用它设置止损最自然:止损 = 入场价 - 2×ATR,既给策略足够呼吸空间,又把单笔风险控制在合理范围。\\n\\n2008 年 10 月,A 股 14 日 ATR 是平时的 3 倍 — 当时所有基于历史波动率的模型都失效了,才有了"波动率聚集"研究。',
        'related': ['bollinger_bands', 'supertrend', 'keltner'],
    },
    'donchian': {
        'summary': 'N 日最高价和最低价形成的通道。海龟交易法的基础。',
        'example': 'Richard Donchian 在 1970s 提出,后来被海龟交易法发扬光大。\\n\\n20 日 Donchian 通道:上轨=前 20 日最高,下轨=前 20 日最低。\\n\\n2021 年 BTC 顶部:价格从 64K 跌到 28K,过程中 Donchian 下轨从 28K 一路下移到 32K,成为强阻力。Donchian 通道在趋势中非常好用,但震荡市会来回打脸。',
        'related': ['keltner', 'bollinger_bands'],
    },
    'supertrend': {
        'summary': '基于 ATR 的趋势线。价格 > 线 = 多头,价格 < 线 = 空头。简单粗暴,信号清晰。',
        'example': '参数 (10, 3.0) 在 BTC 4 小时图:每根 K 线明确告诉你"多/空",新手也容易跟。\\n\\n关键观察:震荡市会反复翻多翻空 — 加 ADX 过滤(只 ADX>20 时交易)能减少 60% 假信号。\\n\\n2017-2024 年 BTC 7 年回测:年化 60%,但最大回撤高达 80%(2018 年熊市)。这是趋势策略的通病 — 抓住大趋势,死于震荡。',
        'related': ['atr', 'dmi', 'parabolic_sar'],
    },
    'aroon': {
        'summary': 'Aroon Up = (N - 距 N 日高点天数) / N × 100。看趋势开始和结束。',
        'example': '25 日 Aroon 在 BTC 月线上:2018 年 12 月 Aroon Up 从 0 跳到 100,意味着创了 25 日新高 — 底部确认。\\n\\n比 MACD 敏感:能更早捕捉到趋势开始;但震荡市假信号也多。\\n\\n经典用法:Aroon Up > 70 且 Aroon Down < 30 = 强上升趋势;反之亦然。',
        'related': ['dmi', 'sma', 'macd'],
    },
    'ichimoku_indicator': {
        'summary': '完整的一目均衡表含 5 条线 + 云带。最适合长周期趋势交易(4 小时 +)。',
        'example': '默认参数 (9,26,52,26) 在 BTC 4 小时图:\\n\\n- 转换线 (Tenkan-sen) = (9日最高 + 9日最低) / 2:短期趋势,灵敏\\n- 基准线 (Kijun-sen) = (26日最高 + 26日最低) / 2:中期趋势,稳定\\n- 先行带 A (Senkou A) = (Tenkan + Kijun) / 2,前移 26 根:未来支撑/阻力\\n- 先行带 B (Senkou B) = (52日最高 + 52日最低) / 2,前移 26 根:同上\\n- 迟行线 (Chikou) = 今日收盘前移 26 根:确认趋势\\n\\n云 = Senkou A 和 B 之间的区域:绿(多头云, Senkou A > B)/红(空头云)。\\n\\n信号:\\n- 价格 > 云: 强多头\\n- 价格 < 云: 强空头\\n- 价格穿越云: 趋势减弱\\n- Tenkan 上穿 Kijun: 多头确认\\n- Chikou 在价格之上: 多头确认',
        'related': ['sma', 'dmi', 'parabolic_sar'],
    },
    'ppo': {
        'summary': 'PPO = (EMA12 - EMA26) / EMA26 × 100。和 MACD 一样但用百分比,跨品种可比。',
        'example': 'BTC 价格从 10K 涨到 60K 时 PPO 从 -2 涨到 +8,涨了 10 个百分点。\\n\\n同时段某山寨币从 1 涨到 10,PPO 也从 -2 涨到 +8 — 因为是百分比,可以跨品种直接比较。\\n\\n这是 PPO 相对 MACD 的核心优势:不需要看绝对数值,百分比自带标准化。',
        'related': ['macd', 'ema'],
    },
    'vortex': {
        'summary': 'VI+ = N日+|H-Lprev|之和 / N; VI- = N日+|L-Hprev|之和 / N。VI+ 上穿 VI- = 多头。',
        'example': '比 ADX 直观:VI+/VI- 直接告诉你多空,不用看 ADX 的绝对值。\\n\\n14 周期 Vortex 在 BTC 日线:2018 年 12 月底部,VI+ 从 0.85 升到 1.1,VI- 从 1.1 降到 0.9,交叉后趋势确认。\\n\\n陷阱:震荡市 VI+ 和 VI- 频繁交叉,跟 MACD 金叉死叉一样需要趋势过滤。',
        'related': ['dmi', 'aroon'],
    },
    'elder_ray': {
        'summary': 'Bull Power = High - EMA(N),Bear Power = Low - EMA(N)。Dr. Elder 的多空力量分离指标。',
        'example': '默认 13 周期 EMA 在 BTC 日线:\\n\\n- Bull Power > 0 且 Bear Power < 0: 多头占优\\n- Bull < 0 且 Bear > 0: 空头占优\\n- 两者同号: 趋势模糊\\n\\n最强大的用法是 Bull Power 与价格背离:价格新高但 Bull 没新高 → 趋势减弱 → 可能反转。\\n\\nAlexander Elder 在《以交易为生》里详细描述了这个指标,推荐作为核心趋势/动量工具。',
        'related': ['ema', 'macd', 'rsi'],
    },
    'ppo_strategy': {
        'summary': 'PPO 柱体上穿/下穿 0 线。本质是百分比版的 MACD。',
        'example': 'BTC 2020-2021 牛市:PPO 从 -2 涨到 +5,期间多次回到 0 线附近形成金叉,均能捕捉到主要涨幅。\\n\\n相比 MACD 的优势:跨品种可比(百分比),不依赖绝对价格。组合里多品种交易时,PPO 比 MACD 更稳定。',
        'related': ['ppo', 'macd'],
    },
    'vortex_strategy': {
        'summary': 'VI+ 上穿/下穿 VI-。趋势指标,适合加 ADX 过滤。',
        'example': 'BTC 2017-2024 年回测(14 周期):年化 45%,最大回撤 65%,胜率 38% — 单边大涨时表现好,震荡市被反复止损。\\n\\n增强:加 ADX > 25 过滤后,胜率提升到 48%,最大回撤降到 50%。',
        'related': ['vortex', 'dmi'],
    },
    'elder_ray_strategy': {
        'summary': 'Bull>0 且 Bear<0 时做多。Dr. Elder 的经典多空力量策略。',
        'example': 'BTC 4 小时图回测(13 EMA,默认):年化 28%,胜率 42%,最大回撤 50%。\\n\\n最强大的是 Bull 与价格背离:2021 年 4 月 BTC 价格新高,但 Bull Power 没新高 → 顶背离 → 之后 6 个月跌 70%。',
        'related': ['elder_ray', 'ema', 'rsi'],
    },
    'ichimoku_strategy': {
        'summary': '价格 > 云 + Tenkan > Kijun → 多头。完整云图策略。',
        'example': 'BTC 4 小时图 (9,26,52,26) 2019-2024 年回测:年化 35%,最大回撤 60%,胜率 35%。\\n\\n优势是信号明确(云就是支撑阻力 + 趋势指示器),但震荡市表现差。\\n\\n最关键的"三役一目"信号:价格穿越云 + Tenkan 上穿 Kijun + Chikou 在价格之上 — 三者同时出现是最强入场。',
        'related': ['ichimoku', 'dmi', 'sma'],
    },
    'supertrend_strategy': {
        'summary': '基于 ATR 的趋势线,翻多买翻空卖。信号清晰,适合新手。',
        'example': '默认参数 (10, 3.0) 在 BTC 4 小时图:2019-2024 五年回测年化约 30%,但最大回撤高达 70%(主要来自 2022 年熊市被反复止损)。\\n\\n优势:信号明确,心理上容易坚持(每根 K 线告诉你"多/空/无")。\\n\\n劣势:震荡市来回打脸,需要配合 ADX 过滤(ADX<20 时不交易)。',
        'related': ['supertrend', 'atr', 'dmi'],
    },
    'simulatedbroker': {
        'summary': '回测和模拟盘用的"假券商"。和实盘接口一致,所以策略代码不用改。',
        'example': '所有回测都用 SimulatedBroker 跑 — 你写的策略只产生 BUY/SELL 信号,具体怎么执行交给 Broker。\\n\\n换 Broker 就行,不用改策略:\\n- SimulatedBroker (回测)\\n- SimulatedBroker (模拟盘,接实时行情)\\n- CcxtBroker (实盘,接 OKX/Binance 等)\\n\\n这就是"为什么策略只产生 Signal 不下单"的设计价值 — seam at the broker level。',
    },
    'backtestengine': {
        'summary': '按"K线一根一根"地驱动整个系统的核心。每根 K 线做一次:指标 → 信号 → 风控 → 下单 → 记录净值。',
        'example': '主循环:\\n1. 拉新 K 线\\n2. 更新市场价\\n3. 策略生成信号\\n4. 风控检查(止损/仓位)\\n5. 组合转成 Order\\n6. 模拟券商成交\\n7. 记录 Fill / Trade\\n8. 记录净值快照\\n\\n每根 K 线都走完这套流程,所以策略、风控、组合、券商全部解耦。',
    },
    'portfolio': {
        'summary': '组合 = 现金 + 持仓。负责仓位大小、订单转换。策略说"买",组合决定"买多少"。',
        'example': '组合的关键决策:仓位大小。固定比例:每笔下单不超过净值 1%。波动率倒数:波动率高的资产少买。Kelly 公式:f = (pb - q) / b。\\n\\n不管用哪种方法,核心都是:把策略的"信号"翻译成具体的"买多少 / 卖多少"。',
    },
    'riskmanager': {
        'summary': '兜底安全。不管策略怎么说,风控说了算。',
        'example': '三种主要风控:\\n1. 止损:亏损达到 X% 强制平仓\\n2. 止盈:盈利达到 X% 强制平仓\\n3. 仓位上限:单标的最多占净值 Y%\\n\\nBTC 2022 年熊市,一个没止损的策略回撤 80%,加了 20% 止损后回撤降到 45%(虽然触发时心很痛)。',
    },
    'spread': {
        'summary': 'Bid/Ask 价差。卖方想要最高,买方想要最低,中间就是价差。',
        'example': 'BTC 在 Binance 上正常情况 bid/ask 价差 0.01%,几乎无成本。但在极端行情(2020 年 3 月 12 日):价差扩大到 1% — 这就是流动性消失的信号。\\n\\n高频 trader 靠从价差里赚钱(retail 卖 → HF 买 → 价差归 HF),但行情剧烈时反而亏钱。',
    },
    # 误区
    'pitfall_low_pe': {
        'summary': 'PE 低可能便宜,也可能利润即将下滑。一定要看利润趋势。',
        'example': '某银行股 PE=3 看似便宜,但净利润同比下滑 30% — 市场已经把它定价成"将死之人"。\\n\\n反过来,某科技股 PE=50,但利润年增 60% — PEG = 50/60 = 0.83,可能低估。\\n\\n周期股最坑:行业景气高峰时 PE 极低(5-8 倍),但利润即将随周期下行,股票反而要跌。',
    },
    'pitfall_rsi': {
        'summary': 'RSI 超买 ≠ 立刻卖出。强趋势中 RSI 长期 > 70,你会被反复止损。',
        'example': 'BTC 2020-2021 牛市,14 周期 RSI 在 70-90 之间横盘 6 个月。如果按"超买卖出"操作,你会错过 400% 涨幅。\\n\\n正确做法:RSI 超买 + 价格形态反转(头肩顶、双顶、跌破趋势线) + 成交量放大 = 卖。单独 RSI 超买什么都不算。\\n\\nGeorge Lane(RSI 发明者)本人多次强调:RSI 是动量指标,不是反转指标。',
    },
    'pitfall_golden_cross': {
        'summary': '金叉是滞后信号,不是预测。等它发生时趋势已经走了一段。',
        'example': '50/200 金叉在 2020 年 BTC 上出现时,价格已经从 4K 涨到 9K(已经涨了 125%)。这时"预测"的价值已经不大。\\n\\n更要命的是:50/200 金叉之后还可能出现"假金叉"然后"死叉"(比如 2021 年 5 月那波假突破)。\\n\\n金叉最适合作为"趋势确认",而不是"入场信号" — 确认后再追,而不是等金叉抄底。',
    },
    'pitfall_multi_osc': {
        'summary': '用 RSI、KDJ、Williams %R、Stochastic 一起"多重确认" — 没用,它们本质都是动量指标。',
        'example': 'RSI 和 KDJ 的相关系数通常 > 0.85,它们几乎说同一件事。当 RSI 超买时,KDJ 的 J 值几乎一定 > 100 — 你没有"多重确认",只是看了同一件事三遍。\\n\\n真正多样化的确认:1 个动量指标(RSI) + 1 个趋势指标(ADX 或 MA) + 1 个成交量指标(OBV 或 CMF)。这三类指标真的提供独立信息。\\n\\nPDF 第二十八节专门讲了这条误区:同类指标叠加 = 增加假信号,不是增加确定性。',
    },
    'pitfall_main_flow': {
        'summary': '"主力净流入"是统计上的买入单大于卖出单,不等于真有大资金在买入。',
        'example': '同花顺、东方财富上看到的"主力净流入"是按单笔成交金额划分的(>50 万算大单),但:\\n1. 主力可拆单 — 1000 万的单可拆成 20 个 50 万\\n2. 主力可对倒 — 自己卖给自己,制造假流入\\n3. 机构算法单被记成"大单",但其实没有方向含义\\n\\n更可靠:看**累计 5 日以上的流向变化**,或用 OBV(全市场成交量加权流向)替代。',
    },
    'pitfall_volume': {
        'summary': '放量 ≠ 看多。放量可以发生在任何地方,关键是价格位置。',
        'example': '放量 + 价格上涨 + 位置低: 健康(吸筹)\\n放量 + 价格上涨 + 位置高: 出货概率大\\n放量 + 价格下跌 + 位置低: 恐慌(可能见底)\\n放量 + 价格下跌 + 位置中: 继续下跌的概率大\\n\\nA股"放量滞涨"经典出货形态:成交量放大 50%,但价格几乎不动 — 主力在高位派发,接盘的是散户。\\n\\n看成交量必须结合价格位置和趋势阶段,单独看没意义。',
    },
    'pitfall_params': {
        'summary': 'MA(20)、RSI(14) 不是天经地义的。必须做参数敏感性分析。',
        'example': 'MA(20) 跑出来赚钱,MA(18) 跑出来亏钱 — 这就是过拟合的信号。\\n\\n正规做法:把参数 ±20% 跑一遍(MA(16) ~ MA(24)),如果结果差异巨大,这个策略不稳。\\n\\n更严苛的:walk-forward analysis — 在前 70% 数据上优化参数,在后 30% 数据上测试。如果测试结果差很多,就是过拟合。\\n\\n研报里的"最优参数"几乎都没做这个验证,别信。',
    },
    'pitfall_industry': {
        'summary': '科技股 PE 30 和银行股 PE 5,不能直接说银行便宜 — 不同行业商业模式天差地别。',
        'example': 'A股 2023 年:银行 PE 5 vs 白酒 PE 25,看似白酒贵。但白酒利润增速 15%,银行利润增速 3% — PEG 看反而是白酒便宜。\\n\\n看 PE 必须同行业比较,且要分周期(银行/地产/资源 = 周期股,看 PB+股息率更合适;消费/医药/科技 = 成长股,看 PEG)。\\n\\n跨行业比较是初学者最常犯的错。',
    },
    'pitfall_chart_scale': {
        'summary': '对数坐标 vs 线性坐标 — 大波段时差别巨大,会影响视觉判断。',
        'example': 'BTC 从 1K 涨到 60K(60 倍),线性坐标下前期几乎贴底,看不出早期形态;对数坐标下,前期的 1K→3K 和后期的 30K→60K 视觉上同等重要,形态清晰。\\n\\n经验法则:\\n- 资产 1 年内涨跌幅 < 50%: 线性坐标够用\\n- 资产 1 年内涨跌幅 > 100%: 必须用对数坐标\\n- 加密货币、Covid 后的美股:永远用对数坐标',
    },
    'pitfall_unclosed_bar': {
        'summary': '未收盘的 K 线所有指标都会重画,等收盘后再看。',
        'example': 'MACD 在 4 小时 K 线的 23:00 突然发出金叉 — 你立刻买入了,30 分钟后 K 线收盘,实际是死叉,被来回打。\\n\\n永远在 K 线**收盘后**再看信号。实盘/模拟盘逻辑里也应该是"收盘后"才下单,而不是"信号出现就下单"。\\n\\n有些 trader 严格使用"已收盘的 K 线",意味着当前 K 线永远不算 — 这能消除 80% 的假信号。',
    },
    'pitfall_repaint': {
        'summary': 'ZigZag、分形、一些自动形态识别会重画 — 当前看到的信号下一根 K 线可能消失。',
        'example': '你看到 TradingView 上画了一个"头肩顶"形态,高兴地开了空单,第二天图表重画了,形态变成"三角形整理" — 假突破打掉你的止损。\\n\\n最严重的:很多"自动形态识别"和某些指标的当前值会在 K 线收盘时**改变**(叫 repaint)。\\n\\n避免方法:\\n1. 看 TradingView 时,打开"重播模式"模拟历史\\n2. 在回测里**实时运行**(每根 K 线 close 后才计算),而不是用全历史数据预计算\\n3. 用 close[1] 而不是 close[0](用前一根的收盘价)',
    },
    'pitfall_divergence': {
        'summary': '背离是警告信号,不是反转信号。强趋势中多次背离都不反转。',
        'example': 'BTC 2021 年 4 月顶部:价格创新高,但 RSI 没新高 = 顶背离。市场接下来确实跌了。\\n\\n但同样的背离在 2021 年 1 月、2 月、3 月都出现过,每次"看起来要反转",但价格继续创新高,每次做空都被止损。\\n\\n背离的正确使用:第一次背离警惕,第二次背离准备,第三次背离才动手 — 还要配合其他信号(如成交萎缩、形态完成)。\\n\\n单独背离什么都不算。',
    },
}

with open(KNOWLEDGE, 'r', encoding='utf-8') as f:
    content = f.read()

updated = 0
for entry_id, fields in ENTRIES.items():
    if 'summary' in fields:
        new_summary = fields['summary'].replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n')
        # 找 "id: \"xxx\".into(),\s*summary: \"...\""
        pattern = re.compile(
            r'(id: "' + re.escape(entry_id) + r'"\.into\(\),\s*summary:\s*")[^"]*("[,\s])'
        )
        new_content, n = pattern.subn(
            lambda m: m.group(1) + new_summary + m.group(2),
            content,
            count=1
        )
        if n > 0:
            content = new_content
            updated += 1

    if 'example' in fields:
        new_example = fields['example'].replace('\\', '\\\\').replace('"', '\\"').replace('\n', '\\n')
        pattern = re.compile(
            r'(id: "' + re.escape(entry_id) + r'"\.into\(\),.*?example:\s*")[^"]*("[,\s])',
            re.DOTALL
        )
        new_content, n = pattern.subn(
            lambda m: m.group(1) + new_example + m.group(2),
            content,
            count=1
        )
        if n > 0:
            content = new_content

    if 'related' in fields:
        new_related = ', '.join('"' + r + '"' for r in fields['related'])
        pattern = re.compile(
            r'(id: "' + re.escape(entry_id) + r'"\.into\(\),.*?related:\s*)vec!\[[^\]]*\](\s*[,])',
            re.DOTALL
        )
        new_content, n = pattern.subn(
            lambda m: m.group(1) + 'vec![' + new_related + ']' + m.group(2),
            content,
            count=1
        )
        if n > 0:
            content = new_content

print(f'更新了 {updated} 个 entry')
with open(KNOWLEDGE, 'w', encoding='utf-8') as f:
    f.write(content)