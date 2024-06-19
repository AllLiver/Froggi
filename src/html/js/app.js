const version = '1.0.0';  // * Change this if you want to change the version that is displayed in the net-stats container.
const pingTime = '3000';  // * Change this if you want to change how often the ping is updated (in milliseconds), the default is 3000ms (3 seconds).
const pingUrl = 'http://localhost:3000'; // * Change this to your localhost port, the default port is 3000
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

function updatePing() {
    measurePing(pingUrl, ping => {
        const pingElement = document.getElementById('ping-value');
        if (typeof ping === 'number') {
            pingElement.textContent = `${ping} ms`;
        } else {
            pingElement.textContent = ping;
        }
    });
}

updateVersion();
updatePing();
//* Updates ping every 3 seconds (3000ms)
setInterval(updatePing, pingTime);
