const ping_time = '1000'; // Change to update ping at a different interval, default is 1000ms.
const default_theme = 'theme-dark'; // Default theme, change to 'theme-light' or 'theme-colorblind' if you want to change the default theme (clear local storage), default is theme-dark.
const lock_interface_btn = document.getElementById('lockInterfaceBtn'); // Change to the id of the button that will lock the interface, if set to null, the button will be disabled, default is 'lockInterfaceBtn'.
const popup_duration = "7500"; // Change to update the duration of the popup, default is 7500ms.

let ping_url = '/'; // The relative url that will be pinged to check the connection
let preview_url = '/overlay'; // The relative url that will be used for the preview iframe


function pingServer() {
    measure_ping(ping_url, function (time) {
        document.getElementById('ping-value').textContent = time + ' ms';
    });
}

function measure_ping(url, callback) {
    const start_time = performance.now();

    fetch(url, { method: 'HEAD', cache: 'no-store', timeout: 5000 })
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
        .catch((error) => {
            callback('Configure IP E003.5');
            console.error('E003: Ping request failed', error);
        });
}

function toggle_menu() {
    try {
        const menu_button = document.querySelector('.hamburger-menu');
        const side_nav = document.querySelector('.sidenavbar');
        if (!menu_button || !side_nav) {
            throw new Error('E004: Menu elements not found');
        }
        menu_button.classList.toggle('active');
        side_nav.classList.toggle('active');
    } catch (error) {
        console.error('E005: Error toggling menu', error);
    }
}

function test_latency() {
    const start_time = Date.now();
    fetch(window.location.href, { timeout: 5000 })
        .then(function (response) {
            const end_time = Date.now();
            const latency = end_time - start_time;
            try {
                document.getElementById('ping-value').textContent = latency + ' ms';
            } catch (error) {
                console.error('E006: Error updating latency value', error);
            }
        })
        .catch(function (err) {
            console.error('E007: Error fetching data for latency test:', err);
        });
}

lock_interface_btn.addEventListener('click', function (event) {
    if (event.target !== lock_interface_btn) {
        document.body.classList.toggle('interface-locked');

        try {
            if (document.body.classList.contains('interface-locked')) {
                lock_interface_btn.innerHTML = '<strong>Unlock Interface</strong>';
                localStorage.setItem('lockState', 'locked');
            } else {
                lock_interface_btn.innerHTML = '<strong>Lock Interface</strong>';
                localStorage.setItem('lockState', 'unlocked');
            }
        } catch (error) {
            console.error('E009: Error updating lock state', error);
        }
    }
});

window.addEventListener('load', function () {
    try {
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
    } catch (error) {
        console.error('E010: Error loading saved state', error);
    }
});

document.addEventListener("DOMContentLoaded", function () {
    try {
        const iframe = document.getElementById("previewIframe");
        if (!iframe) {
            throw new Error('E011: Preview iframe not found');
        }
        iframe.src = preview_url;

        const hostIPInput = document.getElementById('host-ip');
        if (hostIPInput) {
            hostIPInput.addEventListener('input', saveHostIP);
        } else {
            console.warn('W001: Host IP input not found');
        }
    } catch (error) {
        console.error('E012: Error setting up DOM elements', error);
    }
});

function sanitize(input_element) {
    if (!input_element) {
        console.error('E013: Invalid input element for sanitization');
        return;
    }
    const sanitized_value = input_element.value.replace(/[&<>"'\\/?*]/g, '');
    input_element.value = sanitized_value;
}

function applyTheme(theme) {
    try {
        document.body.classList.remove('theme-dark', 'theme-light', 'theme-colorblind');
        document.body.classList.add(theme);
        localStorage.setItem('theme', theme);

        document.querySelectorAll('.theme-button').forEach(button => {
            button.classList.remove('theme-active');
        });

        let activeButton = document.querySelector(`#${theme.replace('theme-', '')}-theme`);
        if (activeButton) {
            activeButton.classList.add('theme-active');
        } else {
            console.warn('W002: Active theme button not found');
        }
    } catch (error) {
        console.error('E014: Error applying theme', error);
    }
}

window.onload = function () {
    try {
        const savedTheme = localStorage.getItem('theme') || default_theme;
        applyTheme(savedTheme);
    } catch (error) {
        console.error('E015: Error loading saved theme', error);
        applyTheme(default_theme);
    }
};

['light', 'dark', 'colorblind'].forEach(themeName => {
    const themeButton = document.getElementById(`${themeName}-theme`);
    if (themeButton) {
        themeButton.onclick = function () {
            applyTheme(`theme-${themeName}`);
        };
    } else {
        console.warn(`W003: Theme button for ${themeName} not found`);
    }
});

document.addEventListener("DOMContentLoaded", function () {
    function initialize_buttons(start_button_id, stop_button_id, storage_key) {
        const start_button = document.getElementById(start_button_id);
        const stop_button = document.getElementById(stop_button_id);

        if (!start_button || !stop_button) {
            console.error(`E008: Button elements not found for: ${start_button_id}, ${stop_button_id}`);
            return;
        }

        start_button.addEventListener("click", function () {
            start_button.classList.add("selector-active");
            stop_button.classList.remove("selector-active");
            try {
                localStorage.setItem(storage_key, "start");
            } catch (error) {
                console.error('E016: Error saving button state to localStorage', error);
            }
        });

        stop_button.addEventListener("click", function () {
            stop_button.classList.add("selector-active");
            start_button.classList.remove("selector-active");
            try {
                localStorage.setItem(storage_key, "stop");
            } catch (error) {
                console.error('E016: Error saving button state to localStorage', error);
            }
        });

        try {
            const saved_state = localStorage.getItem(storage_key);
            if (saved_state === "start") {
                start_button.classList.add("selector-active");
                stop_button.classList.remove("selector-active");
            } else if (saved_state === "stop") {
                stop_button.classList.add("selector-active");
                start_button.classList.remove("selector-active");
            }
        } catch (error) {
            console.error('E017: Error loading saved button state', error);
        }
    }

    const buttonPairs = [
        ["countdownStartButton", "countdownStopButton", "countdownState"],
        ["clockStartButton", "clockStopButton", "clockState"],
        ["showAnimationStartButton", "showAnimationStopButton", "showAnimationState"],
        ["showCountdownStartButton", "showCountdownStopButton", "showCountdownState"],
        ["showDownsStartButton", "showDownsStopButton", "showDownsState"],
        ["showScoreboardStartButton", "showScoreboardStopButton", "showScoreboardState"],
        ["showPopupStartButton", "showPopupStopButton", "showPopupState"],
        ["showSponsorsStartButton", "showSponsorsStopButton", "showSponsorsState"]
    ];

    buttonPairs.forEach(pair => initialize_buttons(...pair));
});

document.querySelectorAll('.cooldown').forEach(button => {
    button.addEventListener('click', function () {
        this.disabled = true;
        this.classList.add('popup-cooldown');

        setTimeout(() => {
            this.disabled = false;
            this.classList.remove('popup-cooldown');
        }, popup_duration);
    });
});

pingServer();
setInterval(pingServer, ping_time);