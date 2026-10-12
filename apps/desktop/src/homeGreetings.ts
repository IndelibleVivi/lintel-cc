// A greeting is chosen once before React mounts. Only the last special local
// calendar date is remembered; greetings never inspect accounts or config.
//
// The Chinese greeting text lives ONLY in the zh-CN catalog; this module derives
// its lookup tables from that single source, so the catalog and the chooser can
// never drift. Each looked-up entry is a stable identity: a language switch
// renders the *same* chosen greeting from the other catalog without drawing
// again or consuming the once-per-date special-date egg twice.
import { zhCN } from './i18n.catalog.zh-CN.ts';

export type GreetingPeriod = 'morning' | 'afternoon' | 'evening' | 'late';

function lineIndexes(period: GreetingPeriod): number[] {
  const prefix = `greeting.line.${period}.`;
  return Object.keys(zhCN)
    .filter(key => key.startsWith(prefix))
    .map(key => Number(key.slice(prefix.length)))
    .filter(index => Number.isInteger(index))
    .sort((a, b) => a - b);
}

export const greetingLines: Record<GreetingPeriod, string[]> = {
  morning: lineIndexes('morning').map(index => zhCN[`greeting.line.morning.${index}`]),
  afternoon: lineIndexes('afternoon').map(index => zhCN[`greeting.line.afternoon.${index}`]),
  evening: lineIndexes('evening').map(index => zhCN[`greeting.line.evening.${index}`]),
  late: lineIndexes('late').map(index => zhCN[`greeting.line.late.${index}`]),
};

export const dateGreetings: Record<string, string> = Object.fromEntries(
  Object.keys(zhCN).filter(key => key.startsWith('greeting.date.'))
    .map(key => [key.slice('greeting.date.'.length), zhCN[key]]),
);
export const timeGreetings: Record<string, string> = Object.fromEntries(
  Object.keys(zhCN).filter(key => key.startsWith('greeting.time.'))
    .map(key => [key.slice('greeting.time.'.length), zhCN[key]]),
);
export const rareGreetings: string[] = Object.keys(zhCN).filter(key => key.startsWith('greeting.rare.'))
  .sort()
  .map(key => zhCN[key]);

export const GREETING_DATE_KEY = 'lintel.greeting.special-date';
type GreetingStorage = Pick<Storage, 'getItem' | 'setItem'>;

// Identity of one chosen greeting: a stable code plus the i18n key used to
// render it in the active catalog.
export type Greeting = { code: string; key: string; text: string };

export function homeGreeting(now: Date, random = Math.random, storage?: GreetingStorage): Greeting {
  const pad = (value: number) => String(value).padStart(2, '0');
  const date = `${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
  const fullDate = `${now.getFullYear()}-${date}`;
  // If persistence is unavailable, skip the date egg rather than repeating it
  // on every opening. The ordinary greeting still works.
  if (dateGreetings[date] && storage) {
    try {
      if (storage.getItem(GREETING_DATE_KEY) !== fullDate) {
        storage.setItem(GREETING_DATE_KEY, fullDate);
        return { code: `date:${date}`, key: `greeting.date.${date}`, text: dateGreetings[date] };
      }
    } catch { /* Storage is optional for the companion. */ }
  }
  const time = `${pad(now.getHours())}:${pad(now.getMinutes())}`;
  const timeKey = time.replace(':', '');
  if (timeGreetings[timeKey]) return { code: `time:${time}`, key: `greeting.time.${timeKey}`, text: timeGreetings[timeKey] };
  const hour = now.getHours();
  const period: GreetingPeriod = hour < 6 ? 'late' : hour < 12 ? 'morning' : hour < 18 ? 'afternoon' : 'evening';
  if (random() < 0.01) {
    const index = Math.floor(random() * rareGreetings.length);
    return { code: `rare:${index}`, key: `greeting.rare.${index}`, text: rareGreetings[index] };
  }
  const lines = greetingLines[period];
  const index = Math.floor(random() * lines.length);
  return { code: `period:${period}:${index}`, key: `greeting.line.${period}.${index}`, text: lines[index] };
}
