const socket = new WebSocket('ws://localhost:8080');

socket.addEventListener('message', function (event) {
    const data = JSON.parse(event.data);
    if (data.type === 'count') {
        document.getElementById('clientCount').textContent = `${data.count} devices are currently viewing this page.`;
    }
});


// Compression
app.use(compression()); 
app.use(express.static('public')); 

app.listen(8080, function () {
  console.log('Compression');
});

