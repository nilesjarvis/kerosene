const WebSocket = require('ws');
const ws = new WebSocket('wss://api.hyperliquid.xyz/ws');
ws.on('open', () => {
  ws.send(JSON.stringify({"method": "subscribe", "subscription": {"type": "webData2", "user": "0x0000000000000000000000000000000000000000"}}));
});
ws.on('message', (data) => {
  const msg = JSON.parse(data);
  if (msg.channel === 'webData2') {
    console.log(JSON.stringify(msg.data.assetCtxs[0], null, 2));
    process.exit(0);
  }
});
