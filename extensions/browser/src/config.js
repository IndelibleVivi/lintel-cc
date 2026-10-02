export const CONFIG = {
  browser: 'chromium',
  fixture: false,
  sites: [
    {origin:'https://claude.ai', domain:'claude.ai', label:'Claude', default:true},
    {origin:'https://console.anthropic.com', domain:'anthropic.com', label:'Anthropic Console', default:false}
  ]
};
