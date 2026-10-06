// A greeting is chosen once before React mounts. Only the last special local
// calendar date is remembered; greetings never inspect accounts or config.
const greetings = {
  morning: [
    '早上好，慢慢开始就好。',
    '新的一天，Clawd 先来陪你。',
    '晨光到了，给今天留一点空白。',
    '早呀，先伸个懒腰。',
    '今天的第一步，可以很小。',
    '窗边亮起来了，你也来啦。',
    '给自己一点时间，再出发。',
    '早上好，愿今天有一点好玩的事。',
  ],
  afternoon: [
    '午后好，先在这里坐坐。',
    '回来啦，今天想做什么？',
    'Clawd 在这儿，慢慢来。',
    '把忙碌放一放，留一点呼吸。',
    '有灵感就接着做，没灵感也没关系。',
    '给你留了一小块安静。',
    '事情一件件来，Clawd 陪着。',
    '今天也可以从一个小念头开始。',
  ],
  evening: [
    '晚上好，Clawd 给你留了位置。',
    '忙了一天，先在这里歇一会儿。',
    '灯亮起来了，欢迎回来。',
    '今天做了多少，都可以慢慢收好。',
    '把值得留下的，轻轻放在这里。',
    '天色暗了，这里还是暖的。',
    '晚风路过，Clawd 还在。',
    '今晚想继续，还是先摸摸 Clawd？',
  ],
  late: [
    '夜深啦，给自己留一点安静。',
    '灯还亮着，Clawd 也在。',
    '小声一点，灵感可能正睡着。',
    '这一会儿，不用急着赶路。',
    '夜里来的小念头，也值得留着。',
    '先喝口水，再看看眼前的事。',
    '困了就歇一歇，草案会好好留着。',
    '星星不催你，Clawd 也不催。',
  ],
};

const saelGreetings = {
  morning: ['Morning. 一切照旧。', '早。昨晚的环境都在。', 'Rise and rebuild.', '早上好，没人动过你的东西。', '新的一天，干净地开始。', 'Clawd 醒了，你呢？'],
  afternoon: ['Afternoon. 动哪一个？', '回来啦，草案都还在。', 'Clean slate, whenever.', '你的机器，你说了算。', '要留的留，要关的关。', 'Clawd 在看书，随时待命。'],
  evening: ['Evening. 收拾一下？', '晚上好，门开着。', 'Nothing leaves without asking you.', '外发？今晚不发。', 'Tidy is a mood.', '先预览，再动手。'],
  late: ['Night shift. Clawd 值班。', '还没睡？那就轻一点。', '3 a.m. config hours.', '凌晨了，先存档。', '夜里动的手，都有记录。', 'Go to bed. 草案会等你。'],
};

const dates: Record<string, string> = {
  '01-01': 'New year. Same rules.',
  '03-14': '3.14159… 今天不用算完。',
  '04-01': '今天说的话，一句都别信。除了这句。',
  '10-13': '一周年。门还开着。',
  '12-31': '今年最后一次收拾。干干净净地走。',
};
const times: Record<string, string> = {
  '00:00': '新的一天，从凌晨开始也算。',
  '03:14': "π o'clock.",
  '04:04': '404: 睡眠 not found.',
};
const rare = [
  'You found a rare greeting. 别声张。',
  'Clawd 刚才偷偷看了一眼你的配置。没说什么。',
  '这句只有一百个人里的一个能看到。是你。',
  'Telemetry 想出门。被我拦住了。',
  'sudo make me a sandwich.',
  '门楣不说话，但门一直没塌。',
];
export const GREETING_DATE_KEY = 'lintel.greeting.special-date';
type GreetingStorage = Pick<Storage, 'getItem' | 'setItem'>;

export function homeGreeting(now: Date, random = Math.random, storage?: GreetingStorage): string {
  const pad = (value: number) => String(value).padStart(2, '0');
  const date = `${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
  const fullDate = `${now.getFullYear()}-${date}`;
  // If persistence is unavailable, skip the date egg rather than repeating it
  // on every opening. The ordinary greeting still works.
  if (dates[date] && storage) {
    try {
      if (storage.getItem(GREETING_DATE_KEY) !== fullDate) {
        storage.setItem(GREETING_DATE_KEY, fullDate);
        return dates[date];
      }
    } catch { /* Storage is optional for the companion. */ }
  }
  const time = `${pad(now.getHours())}:${pad(now.getMinutes())}`;
  if (times[time]) return times[time];
  if (random() < 0.01) return rare[Math.floor(random() * rare.length)];
  const hour = now.getHours();
  const period = hour < 6 ? 'late' : hour < 12 ? 'morning' : hour < 18 ? 'afternoon' : 'evening';
  const lines = [...greetings[period], ...saelGreetings[period]];
  return lines[Math.floor(random() * lines.length)];
}
