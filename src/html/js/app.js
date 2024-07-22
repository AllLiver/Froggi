const version = '2.0.0'; // Deprecated, update cargo.toml. Current version of the app, change when updating the app, using semantic versions.
const ping_time = '1000'; // Change to update ping at a different interval, default is 1000ms.
const ping_url = 'http://localhost:3000'; // Change when to the local ip when accessing from a different device, default is localhost:3000/
const preview_url = 'http://localhost:3000/overlay'; // Change when to the local ip when accessing from a different device, default is localhost:3000/overlay
const default_theme = 'theme-dark'; // Default theme, change to 'theme-light' or 'theme-colorblind' if you want to change the default theme (clear local storage), default is theme-dark.
const lock_interface_btn = document.getElementById('lockInterfaceBtn'); // Change to the id of the button that will lock the interface, if null, the button will be disabled, default is 'lockInterfaceBtn'.
const popup_duration = "7500"; // Change to update the duration of the popup, default is 7500ms.


function update_version() {
    const version_element = document.getElementById('version-value');
    if (version_element) {
        version_element.textContent = version;
    } else {
        console.error('Version value not found');
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
            }
        })
        .catch(() => {
            callback('Ping Error');
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
            console.error('Error fetching data:', err);
        });
}

document.addEventListener("DOMContentLoaded", function() {
    const iframe = document.getElementById("previewIframe");
    iframe.src = preview_url;
});

function sanitize(input_element) {
    const sanitized_value = input_element.value.replace(/[&<>"'\\/?*]/g, '');
    input_element.value = sanitized_value;
}

function toggle_theme(theme) {
    document.body.classList.remove('theme-light', 'theme-dark', 'theme-colorblind');
    document.body.classList.add(theme);
    localStorage.setItem('currentTheme', theme); 
}

function load_theme() {
    const saved_theme = localStorage.getItem('currentTheme');
    console.log('Saved Theme:', saved_theme);

    if (saved_theme) {
        toggle_theme(saved_theme);
    } else {
        console.log('No theme set, applying default:', default_theme);
        toggle_theme(default_theme); 
    }
}

document.addEventListener('DOMContentLoaded', function() {
    document.getElementById('light-theme').addEventListener('click', function() {
        toggle_theme('theme-light');
    });

    document.getElementById('dark-theme').addEventListener('click', function() {
        toggle_theme('theme-dark');
    });

    document.getElementById('colorblind-theme').addEventListener('click', function() {
        toggle_theme('theme-colorblind');
    });

    load_theme();
});

document.addEventListener("DOMContentLoaded", function () {
    function initialize_buttons(start_button_id, stop_button_id, storage_key) {
        const start_button = document.getElementById(start_button_id);
        const stop_button = document.getElementById(stop_button_id);

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
test_latency();
setInterval(test_latency, ping_time);
