// Original Lintel copy. A greeting is chosen once per App visit; no account,
// activity history, network request or recurring animation is involved.
const greetings = {
  morning: [
    '早上好，慢慢开始就好。',
    '新的一天，小克先来陪你。',
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
    '小克在这儿，慢慢来。',
    '把忙碌放一放，留一点呼吸。',
    '有灵感就接着做，没灵感也没关系。',
    '给你留了一小块安静。',
    '事情一件件来，小克陪着。',
    '今天也可以从一个小念头开始。',
  ],
  evening: [
    '晚上好，小克给你留了位置。',
    '忙了一天，先在这里歇一会儿。',
    '灯亮起来了，欢迎回来。',
    '今天做了多少，都可以慢慢收好。',
    '把值得留下的，轻轻放在这里。',
    '天色暗了，这里还是暖的。',
    '晚风路过，小克还在。',
    '今晚想继续，还是先摸摸小克？',
  ],
  late: [
    '夜深啦，给自己留一点安静。',
    '灯还亮着，小克也在。',
    '小声一点，灵感可能正睡着。',
    '这一会儿，不用急着赶路。',
    '夜里来的小念头，也值得留着。',
    '先喝口水，再看看眼前的事。',
    '困了就歇一歇，草案会好好留着。',
    '星星不催你，小克也不催。',
  ],
};

export function homeGreeting(now: Date, random = Math.random): string {
  const hour = now.getHours();
  const lines = hour < 6 ? greetings.late : hour < 12 ? greetings.morning : hour < 18 ? greetings.afternoon : greetings.evening;
  return lines[Math.floor(random() * lines.length)];
}
