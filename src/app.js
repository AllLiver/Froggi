const WebSocket = require('ws');

const wss = new WebSocket.Server({ port: 8080 });

let count = 0;

wss.on('connection', ws => {
  count++;
  wss.clients.forEach(client => {
    if (client.readyState === WebSocket.OPEN) {
      client.send(JSON.stringify({ type: 'count', count: count }));
    }
  });

  ws.on('close', () => {
    count--;
    wss.clients.forEach(client => {
      if (client.readyState === WebSocket.OPEN) {
        client.send(JSON.stringify({ type: 'count', count: count }));
      }
    });
  });
});

// Compression
app.use(compression()); 
app.use(express.static('public')); 

app.listen(3000, function () {
  console.log('Example app listening on port 3000!');
});