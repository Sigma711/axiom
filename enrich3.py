"""
直接用字符串替换升级关键 entry 的 summary/example/related 内容。
绕过正则问题。
"""
import os

KNOWLEDGE = r'C:\Users\朱泓睿\quant-demo-rs\src\knowledge.rs'

# 升级的条目: id -> (new_summary, new_example, new_related)
# 文本里 "\n" 写成 \\n
ENTRIES = {
    'macd': {
        's': 'DIF 是 12 周期 EMA 减 26 周期 EMA,DEA 是 DIF 的 9 周期 EMA,柱体是 DIF-DEA。趋势和动量合二为一。',
        'e': 'BTC 1 小时图,参数默认 (12,26,9)。DIF 在零轴上方且柱体由负转正 = 多头确立;零轴下方柱体翻负 = 空头。\\n\\n2020 年 3 月大跌:DIF 连续远离零轴 = 强趋势;价格反弹时 DIF 回到零轴附近 = 趋势暂缓但未反转。\\n\\n实际用法:很多 trader 用 MACD 零轴交叉作为多空分水岭,而不是简单的金叉死叉。',
        'r': ['ema', 'sma', 'ppo', 'tsi'],
    },
    'rsi': {
        's': 'N 日涨/跌力度比,值域 0-100。>70 超买,<30 超卖。最经典的动量震荡指标。',
        'e': '14 周期 RSI 在 BTC 4 小时图的应用:2021 年 4 月顶部,RSI 创下 92 的极端值(常规 >70 已超买),价格随后暴跌 50% 以上。\\n\\n但 RSI 超买 ≠ 立即卖出:强趋势中 RSI 可长期 >70。如果在 2021 年 1 月 RSI=80 时卖出,会错过最大的上涨波段。\\n\\n正确用法:超买 + 价格出现反转形态(头肩顶、双顶等)才卖;或者等 RSI 跌出超买区(从 80 跌到 70 以下)再说。',
        'r': ['stochastic', 'kdj', 'williams_r', 'mfi'],
    },
    'bollinger_bands': {
        's': '中轨是 N 周期 SMA,上下轨是中轨 ± k 倍标准差。99.7% 的价格会落在 3σ 内,所以触及外轨是统计极端。',
        'e': '20 周期 SMA + 2σ 的经典组合,在 BTC 日线上:2021 年 5 月 19 日价格 43,000 美元,精确触及上轨 = 顶部信号;但价格并未立即下跌,而是横盘 2 周后跌穿中轨才确认。\\n\\nBBands Squeeze(挤压):上下轨收窄到极致时,常预示即将大幅波动。2017 年 12 月 BTC 启动前,BBands 经历了 4 个月的极度挤压。\\n\\n不要把触及外轨当反转信号,把它当"波动率异常"看。',
        'r': ['atr', 'keltner', 'squeeze'],
    },
    'ichimoku': {
        's': '5 条线 + 1 个云带。转换线 (Tenkan) 是 9 日中点,基准线 (Kijun) 是 26 日中点,先行带 A/B 围成云。',
        'e': '默认参数 (9,26,52)。交易信号三层:\\n\\n1. 云在价格之上 = 强空头(整片云都成阻力);云在价格之下 = 强多头(成支撑)\\n2. 价格在云边缘穿越 = 趋势减弱,可能在反转\\n3. 迟行线(Chikou Span)在价格之上/之下,确认趋势\\n\\n实战案例:2020 年 3 月 BTC 暴跌后,价格在 26 日后回升突破云下方,转换线上穿基准线,云由绿翻红 = 标准的多头反转信号。\\n\\n最强大的用法是"三役一目":(1) 价格突破云,(2) 转换线上穿基准线,(3) 迟行线在价格之上。三个条件同时满足,信号最强。',
        'r': ['sma', 'dmi', 'parabolic_sar', 'aroon'],
    },
    'kdj': {
        's': '中国市场的经典。RSV=(C-LN)/(HN-LN)×100,K 是 RSV 的 Wilder 平滑,D 是 K 的平滑,J=3K-2D。',
        'e': '默认参数 (9,3,3)。A 股和港股特别流行。\\n\\n关键阈值:J>100 严重超买(短线见顶概率大),J<0 严重超卖(短线见底概率大)。\\n\\n最有效的形态:K 和 D 在 20 以下金叉,且 J 从 -10 以下拐头向上 = 强烈的买入信号。\\n\\n和 RSI 不同的是 KDJ 多了 J 这个放大器(3K-2D 经常超过 100 或低于 0),所以 KDJ 对短期反转更敏感,但也更容易在强趋势中假信号。',
        'r': ['rsi', 'stochastic'],
    },
    'sma': {
        's': '前 N 根 K 线收盘价的算术平均。最朴素也最有用的趋势指标。滞后 = 半周期。',
        'e': 'BTC 200 周 SMA 是加密圈最有名的指标。历史上:2020 年 3 月 BTC 价格短暂跌破 200 周 SMA,后来证明是世纪抄底机会;任何 200 周 SMA 之上的周收盘价都意味着长期牛市。\\n\\n交易用法:20 日 SMA 是月线级别的趋势(被对冲基金广泛使用),50 日 SMA 是季线级别,200 日 SMA 是年级别。\\n\\n局限:滞后 10 天(N=20),意味着趋势反转要等价格偏离 N/2 个周期才确认 = 总是迟到。',
        'r': ['ema', 'hma', 'dema', 'tema', 'vwma'],
    },
    'ema': {
        's': '指数加权均线,近期权重 α = 2/(N+1)。比 SMA 反应快 ~ 2 倍,但仍有滞后。',
        'e': '12 和 26 周期 EMA 的差是 MACD 的核心,26 周期 EMA 是 Ichimoku 基准线,20 周期 EMA 是 DMI 的一部分。EMA 是几乎所有趋势型指标的基石。\\n\\n为什么选 12/26:据说 Gerald Appel 当年用周线算(一年 52 周 / 4 周 ≈ 12;中间值 26),这两个数字就这么沿用下来了。\\n\\n坑:EMA 永远比 SMA 滞后少,但当价格突然反转时 EMA 也会"假突破"再回归 = 这就是为什么用 EMA 交叉做策略的胜率很难超过 50%。',
        'r': ['sma', 'dema', 'tema', 'hma', 'macd'],
    },
    'vwap': {
        's': '成交量加权的平均价。机构算法单用它作为执行基准,价格偏离 VWAP 越远,越说明有人在买入/卖出。',
        'e': '机构日内算法:把订单拆成小单,在 VWAP 附近成交以减少市场冲击。专业 trader 看价格与 VWAP 的关系判断多空:\\n\\n- 价格 > VWAP + 价格在 VWAP 上方运行 = 多头占优\\n- 价格 < VWAP + 价格在 VWAP 下方 = 空头占优\\n\\n与 SMA 的区别:VWAP 永远跟着成交量加权,反应真实资金成本,不会因为前期"沉默"的价格而被拖偏。\\n\\n锚定 VWAP (Anchored VWAP) 从特定事件(财报、突破)开始算,比标准 VWAP 更有用。',
        'r': ['mfi', 'cmf', 'pvt', 'chaikin_osc'],
    },
    'rsi_strategy': {
        's': '从超卖区反弹买入,从超买区回落卖出。A 股 / 美股 trader 最常用的反转策略。',
        'e': '最常见的 14 周期 RSI 策略:RSI<30 时观察是否反弹(站回 30)→ 买入;RSI>70 时观察是否回落(跌穿 70)→ 卖出。\\n\\n但单独用 RSI 容易挨打:2021 年 BTC 牛市,RSI 在 70-90 之间横盘半年。\\n\\n增强版:RSI + 200 日 SMA 趋势过滤 — 只在 SMA 之上做 RSI<30 的买入,只在 SMA 之下做 RSI>70 的卖出。这能把胜率从 50% 提到 60%+。',
        'r': ['rsi', 'stochastic', 'kdj'],
    },
    'sma_cross_strategy': {
        's': '快线上穿慢线 → 多头;下穿 → 空头。最古老也最被滥用的策略。',
        'e': '经典 (50, 200) "黄金交叉" 长期被认为是牛市信号。BTC 在 2015/2016、2019/2020 都有此信号。\\n\\n但"金叉"的本质滞后:50 日均线在价格上涨时需要 ~ 25 天才能确认方向改变,200 日均线需要 100 天。这意味着金叉时,你可能已经在趋势中段了。\\n\\n致命缺陷:震荡市(金叉死叉频繁)反复打脸。研究显示,简单的 50/200 金叉策略长期跑不赢买入持有(因为它在震荡市被反复止损)。',
        'r': ['sma', 'ema', 'macd'],
    },
    'macd_strategy': {
        's': 'DIF 上穿/下穿 DEA。最经典的动量趋势策略。',
        'e': '零轴交叉更稳定:每次 MACD 从零轴下方上穿 → 买入,跌回零轴下方 → 卖出。2008-2016 年回测:S&P 500 年化 ~ 10%,但最大回撤只有 16%(买入持有 55%)。\\n\\n但 MACD 的金叉死叉(DEA 上穿/下穿)比零轴交叉更频繁,在震荡市假信号多。\\n\\n更稳健的玩法:零轴交叉 + ADX > 25(确认有趋势)+ ATR 做止损。',
        'r': ['macd', 'ema', 'ppo'],
    },
    'sharpe': {
        's': '(收益 - 无风险) / 波动率,年化。>1 合格,>2 优秀,>3 顶级。',
        'e': '巴菲特 50 年年化约 20%,夏普约 0.8。文艺复兴科技的大奖章基金年化 66%,夏普长期 2-3。\\n\\n关键陷阱:夏普可以被"对冲"和"杠杆"放大 — 用 1% 资金 + 99% 现金的策略可能夏普 5,但绝对收益 0.1%。\\n\\n更可靠的:用 IR(信息比率)= (策略 - 基准) / Tracking Error 评估主动管理能力。',
        'r': ['sortino', 'calmar', 'max_drawdown', 'var'],
    },
    'sortino': {
        's': '夏普的改进版,只算下行波动率分母。比夏普更贴近实际感受。',
        'e': '同样年化 20% 收益:一个策略年波动率 30%,下行波动率 18%(跌得少涨得多);另一个 30% 全是下行的。两者夏普一样,但 Sortino 差距明显。\\n\\n业界共识:Sortino > 1.5 是好策略,> 2 是优秀。\\n\\nA 股里很多"长牛"策略(比如银行股红利)夏普低但 Sortino 高 — 因为它们跌得少。',
        'r': ['sharpe', 'calmar', 'max_drawdown'],
    },
    'max_drawdown': {
        's': '从历史峰值到谷底的最大跌幅。心理上能不能承受这个数字,决定了你能不能坚持到底。',
        'e': '1987 年股灾:道指一天跌 22.6%,但最大回撤是那 1 天。LTCM 长期资本管理公司 1998 年回撤 50% 就破产了 — 数学上还能救,心理上已经崩了。\\n\\n经验法则:你能承受的最大回撤 = 你的年龄/2 - 5。比如 30 岁,最大能忍 10%;50 岁,最大能忍 20%。超过就睡不好觉。\\n\\n策略对比:策略 A 收益 50% 但最大回撤 60%;策略 B 收益 40% 但最大回撤 15%。A 长期可能跑赢,但 90% 的人在 A 上面坚持不下来。',
        'r': ['calmar', 'sharpe', 'sortino'],
    },
    'calmar': {
        's': '年化收益 / 最大回撤。越大越好,>1 合格,>3 顶级。',
        'e': '海龟交易法 1980s 年化 80%,最大回撤 30%,Calmar 约 2.7。\\n\\n2020-2024 年测试 BTC 上的 20 日突破策略:年化 50%,最大回撤 35%,Calmar 1.4。\\n\\n相比 Sharpe 的优势:Sharpe 关心的是"波动",Calmar 关心的是"最痛苦的时候" — 后者更贴近实际投资体验。',
        'r': ['sharpe', 'sortino', 'max_drawdown'],
    },
    'var': {
        's': 'VaR(95%) = "1 天内 95% 概率不会亏超过 X"。X 就是 VaR。风险管理最常用的指标。',
        'e': 'BTC 24 小时 95% VaR = 4.2%,意味着每天有 5% 的概率会亏超过 4.2%。\\n\\n但 VaR 不描述尾部 — 那 5% 的"灾难日"可能亏 15% 以上(肥尾)。2008 年雷曼倒闭,VaR 模型没预测到 30% 单日跌幅。\\n\\n更稳的做法:用 CVaR(95%) = 超过 VaR 那 5% 天的平均损失,反映了"灾难有多大"。',
        'r': ['cvar', 'max_drawdown', 'sharpe'],
    },
    'atr': {
        's': 'N 日真实波幅的 Wilder 平滑。波动率的"温度计"。',
        'e': 'BTC 日线 14 周期 ATR 通常在 500-1500 美元之间(随波动率变)。用它设置止损最自然:止损 = 入场价 - 2×ATR,既给策略足够呼吸空间,又把单笔风险控制在合理范围。\\n\\n2008 年 10 月,A 股 14 日 ATR 是平时的 3 倍 — 当时所有基于历史波动率的模型都失效了,才有了"波动率聚集"研究。',
        'r': ['bollinger_bands', 'supertrend', 'keltner'],
    },
    'donchian': {
        's': 'N 日最高价和最低价形成的通道。海龟交易法的基础。',
        'e': 'Richard Donchian 在 1970s 提出,后来被海龟交易法发扬光大。\\n\\n20 日 Donchian 通道:上轨=前 20 日最高,下轨=前 20 日最低。\\n\\n2021 年 BTC 顶部:价格从 64K 跌到 28K,过程中 Donchian 下轨从 28K 一路下移到 32K,成为强阻力。Donchian 通道在趋势中非常好用,但震荡市会来回打脸。',
        'r': ['keltner', 'bollinger_bands'],
    },
    'supertrend': {
        's': '基于 ATR 的趋势线。价格 > 线 = 多头,价格 < 线 = 空头。简单粗暴,信号清晰。',
        'e': '参数 (10, 3.0) 在 BTC 4 小时图:每根 K 线明确告诉你"多/空",新手也容易跟。\\n\\n关键观察:震荡市会反复翻多翻空 — 加 ADX 过滤(只 ADX>20 时交易)能减少 60% 假信号。\\n\\n2017-2024 年 BTC 7 年回测:年化 60%,但最大回撤高达 80%(2018 年熊市)。这是趋势策略的通病 — 抓住大趋势,死于震荡。',
        'r': ['atr', 'dmi', 'parabolic_sar'],
    },
    'ichimoku_indicator': {
        's': '完整的一目均衡表含 5 条线 + 云带。最适合长周期趋势交易(4 小时 +)。',
        'e': '默认参数 (9,26,52,26) 在 BTC 4 小时图:\\n- 转换线 (Tenkan-sen) = (9日最高 + 9日最低) / 2:短期趋势,灵敏\\n- 基准线 (Kijun-sen) = (26日最高 + 26日最低) / 2:中期趋势,稳定\\n- 先行带 A (Senkou A) = (Tenkan + Kijun) / 2,前移 26 根:未来支撑/阻力\\n- 先行带 B (Senkou B) = (52日最高 + 52日最低) / 2,前移 26 根:同上\\n- 迟行线 (Chikou) = 今日收盘前移 26 根:确认趋势\\n云 = Senkou A 和 B 之间的区域:绿(多头云, Senkou A > B)/红(空头云)。',
        'r': ['sma', 'dmi', 'parabolic_sar'],
    },
    'pitfall_rsi': {
        's': 'RSI 超买 ≠ 立刻卖出。强趋势中 RSI 长期 > 70,你会被反复止损。',
        'e': 'BTC 2020-2021 牛市,14 周期 RSI 在 70-90 之间横盘 6 个月。如果按"超买卖出"操作,你会错过 400% 涨幅。\\n\\n正确做法:RSI 超买 + 价格形态反转(头肩顶、双顶、跌破趋势线) + 成交量放大 = 卖。单独 RSI 超买什么都不算。\\n\\nGeorge Lane(RSI 发明者)本人多次强调:RSI 是动量指标,不是反转指标。',
    },
    'pitfall_golden_cross': {
        's': '金叉是滞后信号,不是预测。等它发生时趋势已经走了一段。',
        'e': '50/200 金叉在 2020 年 BTC 上出现时,价格已经从 4K 涨到 9K(已经涨了 125%)。这时"预测"的价值已经不大。\\n\\n更要命的是:50/200 金叉之后还可能出现"假金叉"然后"死叉"(比如 2021 年 5 月那波假突破)。\\n\\n金叉最适合作为"趋势确认",而不是"入场信号" — 确认后再追,而不是等金叉抄底。',
    },
    'pitfall_multi_osc': {
        's': '用 RSI、KDJ、Williams %R、Stochastic 一起"多重确认" — 没用,它们本质都是动量指标。',
        'e': 'RSI 和 KDJ 的相关系数通常 > 0.85,它们几乎说同一件事。当 RSI 超买时,KDJ 的 J 值几乎一定 > 100 — 你没有"多重确认",只是看了同一件事三遍。\\n\\n真正多样化的确认:1 个动量指标(RSI) + 1 个趋势指标(ADX 或 MA) + 1 个成交量指标(OBV 或 CMF)。这三类指标真的提供独立信息。\\n\\nPDF 第二十八节专门讲了这条误区:同类指标叠加 = 增加假信号,不是增加确定性。',
    },
    'simulatedbroker': {
        's': '回测和模拟盘用的"假券商"。和实盘接口一致,所以策略代码不用改。',
        'e': '所有回测都用 SimulatedBroker 跑 — 你写的策略只产生 BUY/SELL 信号,具体怎么执行交给 Broker。\\n\\n换 Broker 就行,不用改策略:\\n- SimulatedBroker (回测)\\n- SimulatedBroker (模拟盘,接实时行情)\\n- CcxtBroker (实盘,接 OKX/Binance 等)',
    },
    'backtestengine': {
        's': '按"K线一根一根"地驱动整个系统的核心。每根 K 线做一次:指标 → 信号 → 风控 → 下单 → 记录净值。',
        'e': '主循环:\\n1. 拉新 K 线\\n2. 更新市场价\\n3. 策略生成信号\\n4. 风控检查(止损/仓位)\\n5. 组合转成 Order\\n6. 模拟券商成交\\n7. 记录 Fill / Trade\\n8. 记录净值快照',
    },
    'riskmanager': {
        's': '兜底安全。不管策略怎么说,风控说了算。',
        'e': '三种主要风控:\\n1. 止损:亏损达到 X% 强制平仓\\n2. 止盈:盈利达到 X% 强制平仓\\n3. 仓位上限:单标的最多占净值 Y%\\n\\nBTC 2022 年熊市,一个没止损的策略回撤 80%,加了 20% 止损后回撤降到 45%(虽然触发时心很痛)。',
    },
}

with open(KNOWLEDGE, 'r', encoding='utf-8') as f:
    content = f.read()

updated = 0
for entry_id, fields in ENTRIES.items():
    # 找对应的 entry
    # pattern: id: "xxx".into(),\s*summary: "...",\s*example: "...",\s*related: vec![...]
    # 我们需要一次性替换 summary, example, related 三行
    # 先找开始位置
    search_start = f'id: "{entry_id}".into(),'
    pos = content.find(search_start)
    if pos < 0:
        print(f'  ❌ {entry_id}: id 找不到')
        continue

    # 找 summary: "..." 的结束位置 (".into(),)
    # 找下一个 ", summary 或 ", example
    # 用简单方法: 找 ",  summary: ", ", example: ", ", related: "
    seg_start = content.find('summary:', pos)
    if seg_start < 0: continue

    # 找 ",  example:
    example_start = content.find('example:', seg_start)
    if example_start < 0: continue

    related_start = content.find('related:', example_start)
    if related_start < 0: continue

    # 找 vec![...] 的结束
    related_end = content.find(']', related_start)
    if related_end < 0: continue
    # 找 .into(),  在 ] 之后
    related_close = content.find('.into(),', related_end, related_end + 30)
    if related_close < 0:
        related_close = related_end + 1
    else:
        related_close += len('.into(),')

    # 提取原 summary
    sum_start = content.find('"', seg_start) + 1
    sum_end = content.find('".into(),', sum_start)
    old_summary = content[sum_start:sum_end]

    # 提取原 example
    ex_start = content.find('"', example_start) + 1
    ex_end = content.find('".into(),', ex_start)
    old_example = content[ex_start:ex_end]

    # 提取原 related
    rel_start = content.find('[', related_start) + 1
    rel_end = related_end
    old_related = content[rel_start:rel_end]

    # 构造新字符串
    new_summary = fields.get('s', old_summary)
    new_example = fields.get('e', old_example)
    new_related_list = fields.get('r', None)
    if new_related_list:
        new_related = ', '.join('"' + r + '"' for r in new_related_list)
    else:
        new_related = old_related

    # 替换从 seg_start 到 related_close 的整段
    new_segment = (
        f'summary: "{new_summary}".into(),\n            '
        f'example: "{new_example}".into(),\n            '
        f'related: vec![{new_related}],'
    )

    content = content[:seg_start] + new_segment + content[related_close:]
    updated += 1
    print(f'  ✓ {entry_id}')

print(f'\n更新了 {updated} 个 entry')

with open(KNOWLEDGE, 'w', encoding='utf-8') as f:
    f.write(content)