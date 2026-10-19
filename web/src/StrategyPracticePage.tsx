import { useLayoutEffect, useRef, useState } from 'react';
import { appPath } from './api';
import { STRATEGY_EXERCISES, calculatePractice, validatePracticeValues } from './strategyPractice';
import './StrategyPracticePage.css';

export function StrategyPracticePage({ id, onBack }: { id: string; onBack: () => void }) {
  const exercise = STRATEGY_EXERCISES.find(item => item.id === id);
  if (!exercise) return <div className="ax-practice-missing"><h3>没有找到这项实践</h3><button className="ax-btn" type="button" onClick={onBack}>返回策略流派</button></div>;
  return <PracticeWorkspace key={exercise.id} exercise={exercise} onBack={onBack} />;
}

function PracticeWorkspace({ exercise, onBack }: { exercise: typeof STRATEGY_EXERCISES[number]; onBack: () => void }) {
  const titleRef = useRef<HTMLHeadingElement>(null);
  useLayoutEffect(() => { window.scrollTo(0, 0); titleRef.current?.focus({ preventScroll: true }); }, []);
  const initial = Object.fromEntries(exercise.fields.map(item => [item.key, String(item.initial)]));
  const [raw, setRaw] = useState<Record<string, string>>(initial);
  const values = Object.fromEntries(exercise.fields.map(item => {
    const input = raw[item.key]?.trim() ?? '';
    return [item.key, input === '' ? Number.NaN : Number(input)];
  }));
  const error = validatePracticeValues(exercise, values);
  const result = error ? null : calculatePractice(exercise, values);

  return <div className="ax-strategy-practice">
    <button className="ax-practice-back" type="button" onClick={onBack}>← 返回 11 类策略</button>
    <header className="ax-practice-hero"><span className="ax-eyebrow">策略实践 / {exercise.id}</span><h3 ref={titleRef} tabIndex={-1}>{exercise.title}</h3><p>{exercise.task}</p></header>
    <div className="ax-practice-columns">
      <section className="ax-practice-panel" aria-label="教学输入"><div className="ax-practice-panel-head"><span>01</span><div><h4>改动条件</h4><p>教学算例 · 手动输入；数字不是实时行情，也不是历史成交。</p></div></div>
        <div className="ax-practice-fields">{exercise.fields.map(item => <label key={item.key} className="ax-practice-field"><span>{item.label}</span><span className="ax-practice-input"><input type="text" inputMode="decimal" value={raw[item.key] ?? ''} aria-invalid={Boolean(error) && (!Number.isFinite(values[item.key]) || values[item.key] < item.min || values[item.key] > item.max)} onChange={event => setRaw(current => ({ ...current, [item.key]: event.target.value }))} /><small>{item.unit}</small></span></label>)}</div>
        {error && <p className="ax-practice-error" role="alert">{error}</p>}
        <button className="ax-btn" type="button" onClick={() => setRaw(initial)}>恢复示例数值</button>
      </section>
      <section className="ax-practice-panel ax-practice-results" aria-label="演算结果"><div className="ax-practice-panel-head"><span>02</span><div><h4>核对结果</h4><p>每次改值即时重算；先读含义，再看数字。</p></div></div>
        {result && <><div className="ax-practice-metrics">{result.metrics.map(item => <div key={item.label} className="ax-practice-metric"><span>{item.label}</span><strong>{Number.isInteger(item.value) ? item.value : Number(item.value.toFixed(3))}<small> {item.unit}</small></strong><p>{item.meaning}</p></div>)}</div><p className="ax-practice-reading">{result.reading}</p></>}
        {!result && <p className="ax-practice-empty">修正输入后显示计算结果。</p>}
      </section>
    </div>
    {exercise.realLink && <aside className="ax-practice-evidence"><div><span className="ax-eyebrow">下一步 · 真实资料</span><h4>{exercise.realLink.label}</h4><p>{exercise.realLink.explanation}</p></div><a className="ax-btn primary" href={appPath(exercise.realLink.path)}>打开真实数据实践 →</a></aside>}
    {!exercise.realLink && <aside className="ax-practice-evidence"><div><span className="ax-eyebrow">继续核验</span><h4>把每一笔现金流和时间记清楚</h4><p>这页只做可重复的教学演算。将来用真实资料复核时，先记录价格与现金流的来源、发生时点和费用，再判断规则是否可交易。</p></div></aside>}
  </div>;
}
