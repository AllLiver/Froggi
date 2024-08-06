const ping_time = '1000'; // Change to update ping at a different interval, default is 1000ms.
const default_theme = 'theme-dark'; // Default theme, change to 'theme-light' or 'theme-colorblind' if you want to change the default theme (clear local storage), default is theme-dark.
const lock_interface_btn = document.getElementById('lockInterfaceBtn'); // Change to the id of the button that will lock the interface, if set to null, the button will be disabled, default is 'lockInterfaceBtn'.
const popup_duration = "7500"; // Change to update the duration of the popup, default is 7500ms.

// Default/backup URLs, if accessing from a 2nd device, set this to your local ip for fallback.
let ping_url = 'http://localhost:3000'; // The url that will be pinged to check the connection, default is 'http://localhost:3000'.
let preview_url = 'http://localhost:3000/overlay'; // The url that will be used for the preview iframe, default is 'http://localhost:3000/overlay'.

function update_version() {
    const version_element = document.getElementById('version-value');
    if (version_element) {
        version_element.textContent = version;
    } else {
        console.error('E001: Version value not found');
    }
}

function toggle_menu() {
    const menu_button = document.querySelector('.hamburger-menu');
    const side_nav = document.querySelector('.sidenavbar');
    menu_button.classList.toggle('active');
    side_nav.classList.toggle('active');
}

function measure_ping(url, callback) {
    const start_time = performance.now();

    fetch(url, { method: 'HEAD', cache: 'no-store' })
        .then(response => {
            if (response.ok) {
                const end_time = performance.now();
                const time = Math.round(end_time - start_time);
                callback(time);
            } else {
                callback('Ping Error: ' + response.status);
                console.error('E002: Ping failed with status', response.status);
            }
        })
        .catch(() => {
            callback('Ping Error');
            console.error('E003: Ping request failed');
        });
}

function saveHostIP() {
    const hostIPInput = document.getElementById('host-ip');
    if (hostIPInput) {
        const hostIP = hostIPInput.value.trim();
        const isValid = /^([0-9]{1,3}\.){3}[0-9]{1,3}:\d+$/.test(hostIP); 
        if (isValid) {
            localStorage.setItem('hostIP', hostIP);
            ping_url = `http://${hostIP}`; 
            preview_url = `http://${hostIP}/overlay`; 
            console.log('Host IP saved:', hostIP);
        } else {
            console.error('E004: Invalid Host IP format. Please use the format: X.X.X.X:PORT');
        }
    } else {
        console.error('E005: Host IP input element not found');
    }
}

function pingServer() {
    const hostIP = localStorage.getItem('hostIP');
    if (!hostIP) {
        console.error('E006: Host IP is not set.');
        return;
    }

    measure_ping(ping_url, function(pingTime) {
        document.getElementById('ping-value').textContent = pingTime + ' ms';
    });
}

lock_interface_btn.addEventListener('click', function(event) {
    if (event.target !== lock_interface_btn) {
        document.body.classList.toggle('interface-locked');
        
        if (document.body.classList.contains('interface-locked')) {
            lock_interface_btn.innerHTML = '<strong>Unlock Interface</strong>';
            localStorage.setItem('lockState', 'locked');
        } else {
            lock_interface_btn.innerHTML = '<strong>Lock Interface</strong>';
            localStorage.setItem('lockState', 'unlocked');
        }
    }
});

window.addEventListener('load', function() {
    const lock_state = localStorage.getItem('lockState');
    
    if (lock_state === 'locked') {
        document.body.classList.add('interface-locked');
        lock_interface_btn.innerHTML = '<strong>Unlock Interface</strong>';
    } else {
        document.body.classList.remove('interface-locked');
        lock_interface_btn.innerHTML = '<strong>Lock Interface</strong>';
    }

    const savedHostIP = localStorage.getItem('hostIP');
    if (savedHostIP) {
        document.getElementById('host-ip').value = savedHostIP;
        ping_url = `http://${savedHostIP}`;
        preview_url = `http://${savedHostIP}/overlay`;
    }
});

function test_latency() {
    const start_time = Date.now();
    fetch(window.location.href) 
        .then(function(response) {
            const end_time = Date.now();
            const latency = end_time - start_time;
            document.getElementById('ping-value').textContent = latency + ' ms';
        })
        .catch(function(err) {
            console.error('E007: Error fetching data:', err);
        });
}

document.addEventListener("DOMContentLoaded", function() {
    const iframe = document.getElementById("previewIframe");
    iframe.src = preview_url;

    const hostIPInput = document.getElementById('host-ip');
    if (hostIPInput) {
        hostIPInput.addEventListener('input', saveHostIP);
    }
});

function sanitize(input_element) {
    const sanitized_value = input_element.value.replace(/[&<>"'\\/?*]/g, '');
    input_element.value = sanitized_value;
}

function applyTheme(theme) {
    document.body.classList.remove('theme-dark', 'theme-light', 'theme-colorblind');

    document.body.classList.add(theme);

    localStorage.setItem('theme', theme);
}

window.onload = function() {
    const savedTheme = localStorage.getItem('theme') || 'theme-dark'; 
    applyTheme(savedTheme);
};

if (document.getElementById('light-theme')) {
    document.getElementById('light-theme').onclick = function() {
        applyTheme('theme-light');
    };
}

if (document.getElementById('dark-theme')) {
    document.getElementById('dark-theme').onclick = function() {
        applyTheme('theme-dark');
    };
}

if (document.getElementById('colorblind-theme')) {
    document.getElementById('colorblind-theme').onclick = function() {
        applyTheme('theme-colorblind');
    };
}

document.addEventListener("DOMContentLoaded", function () {
    function initialize_buttons(start_button_id, stop_button_id, storage_key) {
        const start_button = document.getElementById(start_button_id);
        const stop_button = document.getElementById(stop_button_id);

        if (!start_button || !stop_button) {
            console.error('E008: Button elements not found for:', start_button_id, stop_button_id);
            return;
        }

        start_button.addEventListener("click", function () {
            start_button.classList.add("selector-active");
            stop_button.classList.remove("selector-active");
            localStorage.setItem(storage_key, "start");
        });

        stop_button.addEventListener("click", function () {
            stop_button.classList.add("selector-active");
            start_button.classList.remove("selector-active");
            localStorage.setItem(storage_key, "stop");
        });

        const saved_state = localStorage.getItem(storage_key);
        if (saved_state === "start") {
            start_button.classList.add("selector-active");
            stop_button.classList.remove("selector-active");
        } else if (saved_state === "stop") {
            stop_button.classList.add("selector-active");
            start_button.classList.remove("selector-active");
        }
    }

    initialize_buttons("countdownStartButton", "countdownStopButton", "countdownState");
    initialize_buttons("clockStartButton", "clockStopButton", "clockState");
    initialize_buttons("showAnimationStartButton", "showAnimationStopButton", "showAnimationState");
    initialize_buttons("showCountdownStartButton", "showCountdownStopButton", "showCountdownState");
    initialize_buttons("showDownsStartButton", "showDownsStopButton", "showDownsState");
    initialize_buttons("showScoreboardStartButton", "showScoreboardStopButton", "showScoreboardState");
    initialize_buttons("showPopupStartButton", "showPopupStopButton", "showPopupState");
    initialize_buttons("showSponsorsStartButton", "showSponsorsStopButton", "showSponsorsState");
});

document.querySelectorAll('.cooldown').forEach(button => {
    button.addEventListener('click', function() {
        this.disabled = true;
        this.classList.add('popup-cooldown');
        
        setTimeout(() => {
            this.disabled = false;
            this.classList.remove('popup-cooldown');
        }, popup_duration);
    });
});

update_version();
pingServer();
setInterval(pingServer, ping_time);
