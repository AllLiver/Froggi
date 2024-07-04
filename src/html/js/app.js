const version = '2.0.0';
const pingTime = '1000'; // Change to update ping at a different interval, default is 1000ms
const pingUrl = 'http://localhost:3000'; // Change when to the local ip when acsessing from a different device, default is localhost:3000/
const previewUrl = 'http://localhost:3000/overlay'; // Change when to the local ip when acsessing from a different device, default is localhost:3000/overlay
const lockInterfaceBtn = document.getElementById('lockInterfaceBtn');

function updateVersion() {
    const versionElement = document.getElementById('version-value');
    versionElement.textContent = version;
}

function toggleMenu() {
    const menuButton = document.querySelector('.hamburger-menu');
    const sideNav = document.querySelector('.sidenavbar');
    menuButton.classList.toggle('active');
    sideNav.classList.toggle('active');
}

function measurePing(url, callback) {
    const startTime = performance.now();

    fetch(url, { method: 'HEAD', cache: 'no-store' })
        .then(response => {
            if (response.ok) {
                const endTime = performance.now();
                const time = Math.round(endTime - startTime);
                callback(time);
            } else {
                callback('Ping Error: ' + response.status);
            }
        })
        .catch(() => {
            callback('Ping Error');
        });
}

lockInterfaceBtn.addEventListener('click', function(event) {
    if (event.target !== lockInterfaceBtn) {
        document.body.classList.toggle('interface-locked');
        
        if (document.body.classList.contains('interface-locked')) {
            lockInterfaceBtn.innerHTML = '<strong>Unlock Interface</strong>';
            localStorage.setItem('lockState', 'locked');
        } else {
            lockInterfaceBtn.innerHTML = '<strong>Lock Interface</strong>';
            localStorage.setItem('lockState', 'unlocked');
        }
    }
});

window.addEventListener('load', function() {
    const lockState = localStorage.getItem('lockState');
    
    if (lockState === 'locked') {
        document.body.classList.add('interface-locked');
        lockInterfaceBtn.innerHTML = '<strong>Unlock Interface</strong>';
    } else {
        document.body.classList.remove('interface-locked');
        lockInterfaceBtn.innerHTML = '<strong>Lock Interface</strong>';
    }
});

function testLatency() {
    var startTime = Date.now();
    fetch(window.location.href) 
        .then(function(response) {
            var endTime = Date.now();
            var latency = endTime - startTime;
            document.getElementById('ping-value').textContent = latency + ' ms';
        })
        .catch(function(err) {
            console.error('Error fetching data:', err);
        });
}

document.addEventListener("DOMContentLoaded", function() {
    var iframe = document.getElementById("previewIframe");
    iframe.src = previewUrl;
});

function sanitize(inputElement) {
    var sanitizedValue = inputElement.value.replace(/[&<>"'/]/g, '');
    inputElement.value = sanitizedValue;
}

updateVersion();
testLatency();
setInterval(testLatency, pingTime);

