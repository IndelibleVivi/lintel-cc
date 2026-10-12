import assert from 'node:assert/strict';
import test from 'node:test';
import {GREETING_DATE_KEY, homeGreeting} from '../apps/desktop/src/homeGreetings.ts';
import {zhCN} from '../apps/desktop/src/i18n.catalog.zh-CN.ts';
import {en} from '../apps/desktop/src/i18n.catalog.en.ts';

const date = (month, day, hour = 9, minute = 0, year = 2026, seconds = 0) => new Date(year, month - 1, day, hour, minute, seconds);
function store() { const values = new Map(); return {getItem: key => values.get(key) ?? null, setItem: (key, value) => values.set(key, value), values}; }
function random(...values) { return () => values.shift() ?? 0.5; }
// The chooser now returns a {code,key,text} identity; assert the Chinese voice
// and, for every pick, that the identity resolves in BOTH catalogs so a
// language switch can render the same greeting.
const zh = g => { assert.ok(g.key in zhCN, 'identity resolves in zh-CN'); assert.ok(g.key in en, 'identity resolves in en'); assert.equal(zhCN[g.key], g.text, 'chooser text matches the canonical zh catalog'); return g.text; };

test('date wins once locally, later openings use lower priorities and next year can celebrate again', () => {
  const storage = store();
  assert.equal(zh(homeGreeting(date(10, 13, 0), () => 0, storage)), '一周年。门还开着。');
  assert.deepEqual([...storage.values], [[GREETING_DATE_KEY, '2026-10-13']]);
  assert.equal(zh(homeGreeting(date(10, 13, 0), () => 0, storage)), '新的一天，从凌晨开始也算。');
  assert.notEqual(zh(homeGreeting(date(10, 13, 9), () => 0.5, storage)), '一周年。门还开着。');
  assert.equal(zh(homeGreeting(date(10, 13, 9, 0, 2027), () => 0, storage)), '一周年。门还开着。');
  for (const [month, day, text] of [[1,1,'New year. Same rules.'],[3,14,'3.14159… 今天不用算完。'],[4,1,'今天说的话，一句都别信。除了这句。'],[12,31,'今年最后一次收拾。干干净净地走。']]) {
    assert.equal(zh(homeGreeting(date(month,day), () => 0, store())), text);
  }
});

test('time wins throughout its minute, then rare is a one-percent opening draw', () => {
  for (const [hour, minute, text] of [[0,0,'新的一天，从凌晨开始也算。'],[3,14,"π o'clock."],[4,4,'404: 睡眠 not found.']]) {
    assert.equal(zh(homeGreeting(date(10,6,hour,minute,2026,59), () => 0)), text);
    assert.notEqual(zh(homeGreeting(date(10,6,hour,minute+1), () => 0.5)), text);
  }
  assert.equal(zh(homeGreeting(date(10,6), random(0.0099,0))), 'You found a rare greeting. 别声张。');
  assert.equal(zh(homeGreeting(date(10,6), random(0.01,0))), '早上好，慢慢开始就好。');
});

test('both sets share each time-of-day pool, with Clawd naming and exact boundaries', () => {
  for (const [hour, text] of [[0,'Go to bed. 草案会等你。'],[5,'Go to bed. 草案会等你。'],[6,'Clawd 醒了，你呢？'],[11,'Clawd 醒了，你呢？'],[12,'Clawd 在看书，随时待命。'],[17,'Clawd 在看书，随时待命。'],[18,'先预览，再动手。'],[23,'先预览，再动手。']]) {
    assert.equal(zh(homeGreeting(date(10,6,hour,1), random(0.5,0.9999))), text);
  }
  for (const hour of [5,9,15,21]) for (let index=0; index<14; index++) {
    assert.ok(!zh(homeGreeting(date(10,6,hour,1), random(0.5,index/14))).includes('小克'));
  }
});

test('unavailable persistence skips the date egg and ordinary opens write no history', () => {
  const broken = {getItem() { throw new Error('unavailable'); }, setItem() { throw new Error('unavailable'); }};
  const unwritable = {getItem() { return null; }, setItem() { throw new Error('unavailable'); }};
  for (const storage of [undefined,broken,unwritable]) assert.equal(zh(homeGreeting(date(10,13), random(0.5,0), storage)), '早上好，慢慢开始就好。');
  const storage = store(); homeGreeting(date(10,6), () => 0.5, storage);
  assert.equal(storage.values.size, 0);
});

test('the same chosen identity translates across a language switch without a second draw', () => {
  const storage = store();
  const chosen = homeGreeting(date(10, 13, 0), () => 0, storage);
  // One date-egg consumption; the identity renders in either catalog.
  assert.equal(chosen.key, 'greeting.date.10-13');
  assert.equal(en[chosen.key], 'One year. The door’s still open.');
  assert.equal(zhCN[chosen.key], '一周年。门还开着。');
  assert.deepEqual([...storage.values], [[GREETING_DATE_KEY, '2026-10-13']]);
});
