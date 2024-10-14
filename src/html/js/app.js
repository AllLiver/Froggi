const PING_TIME = 1000;
const DEFAULT_THEME = 'theme-dark';
const LOCK_INTERFACE_BTN_ID = 'lockInterfaceBtn';
const POPUP_DURATION = 7500;
const VERSION = '1.0.0';
const PING_URL = '/';
const PREVIEW_URL = '/overlay';

const pingValue = document.getElementById('ping-value');
const lockInterfaceBtn = document.getElementById(LOCK_INTERFACE_BTN_ID);
const body = document.body;

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => document.querySelectorAll(selector);

function pingServer() {
    const startTime = performance.now();
    fetch(PING_URL, { method: 'HEAD', cache: 'no-store' })
        .then(response => {
            if (response.ok) {
                const time = Math.round(performance.now() - startTime);
                pingValue.textContent = `${time} ms`;
            } else {
                throw new Error(`Ping failed with status ${response.status}`);
            }
        })
        .catch(error => {
            pingValue.textContent = 'Configure IP E003.5';
            handleError('003', 'Ping request failed', error);
        });
}

function toggleMenu() {
    try {
        $('.hamburger-menu')?.classList.toggle('active');
        $('.sidenavbar')?.classList.toggle('active');
    } catch (error) {
        handleError('005', 'Error toggling menu', error);
    }
}

function toggleInterfaceLock() {
    body.classList.toggle('interface-locked');
    const isLocked = body.classList.contains('interface-locked');
    lockInterfaceBtn.innerHTML = `<strong>${isLocked ? 'Unlock' : 'Lock'} Interface</strong>`;
    localStorage.setItem('lockState', isLocked ? 'locked' : 'unlocked');
}

function deleteCookie(name, path = '/', domain) {
    let cookieString = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=${path};`;
    if (domain) {
        cookieString += ` domain=${domain};`;
    }
    document.cookie = cookieString;
}

document.getElementById('log-out').addEventListener('click', function(event) {
	event.preventDefault();

	deleteCookie('AuthToken');
	deleteCookie('SessionToken');

	console.log('Current cookies after deletion attempt:', document.cookie);

	window.location.href = '/login';
});

document.getElementById('log-out').addEventListener('click', function (event) {
    event.preventDefault(); 
    deleteCookie('AuthToken'); 
    deleteCookie('SessionToken'); 
    window.location.href = '/login'; 
});

function applyTheme(theme) {
    const themeClasses = ['theme-light', 'theme-dark', 'theme-colorblind'];

    themeClasses.forEach(cls => body.classList.remove(cls));

    body.classList.add(theme);

    localStorage.setItem('theme', theme);
    $$('.theme-button').forEach(btn => {
        btn.classList.toggle('theme-active', btn.id === `${theme.replace('theme-', '')}-theme`);
    });
}
function initializeButtonIndicators(startButtonId, stopButtonId, storageKey) {
    const startButton = document.getElementById(startButtonId);
    const stopButton = document.getElementById(stopButtonId);

    if (!startButton || !stopButton) {
        handleError('008', `Button elements not found for: ${startButtonId}, ${stopButtonId}`);
        return;
    }

    function updateButtonState(isStart) {
        startButton.classList.toggle('clock-active', isStart);
        stopButton.classList.toggle('clock-active', !isStart);
        localStorage.setItem(storageKey, isStart ? 'start' : 'stop');
    }

    startButton.addEventListener('click', () => updateButtonState(true));
    stopButton.addEventListener('click', () => updateButtonState(false));

    const savedState = localStorage.getItem(storageKey);
    if (savedState === 'start' || savedState === 'stop') {
        updateButtonState(savedState === 'start');
    }
}

function sanitizeInput(input) {
    return input.replace(/\s+/g, '');
}

function escapeHTML(str) {
    return str.replace(/[&<>"']/g, function(match) {
        switch (match) {
            case '&':
                return '&amp;';
            case '<':
                return '&lt;';
            case '>':
                return '&gt;';
            case '"':
                return '&quot;';
            case "'":
                return '&#39;';
            default:
                return match;
        }
    });
}

function handleInput(event) {
    let input = event.target.value;
    input = sanitizeInput(input);
    event.target.value = input;
}

function processAndDisplay(input) {
    const safeInput = escapeHTML(input);
    document.getElementById('output').innerHTML = safeInput; 
}

document.addEventListener('DOMContentLoaded', () => {

    applyTheme(localStorage.getItem('theme') || DEFAULT_THEME);

    const lockState = localStorage.getItem('lockState');
    if (lockState === 'locked') {
        body.classList.add('interface-locked');
        lockInterfaceBtn.innerHTML = '<strong>Unlock Interface</strong>';
    }

    const iframe = $('#previewIframe');
    if (iframe) iframe.src = PREVIEW_URL;

    const hostIPInput = $('#host-ip');
    if (hostIPInput) {
        hostIPInput.value = localStorage.getItem('hostIP') || '';
        hostIPInput.addEventListener('input', () => {
            const sanitizedValue = hostIPInput.value.replace(/[&<>"'\\/?*]/g, '');
            hostIPInput.value = sanitizedValue;
            localStorage.setItem('hostIP', sanitizedValue);
        });
    }

    ['light', 'dark', 'colorblind'].forEach(themeName => {
        $(`#${themeName}-theme`)?.addEventListener('click', () => applyTheme(`theme-${themeName}`));
    });

    lockInterfaceBtn?.addEventListener('click', toggleInterfaceLock);

    $('.hamburger-menu')?.addEventListener('click', toggleMenu);

    initializeButtonIndicators('countdownStartButton', 'countdownStopButton', 'countdownState');
    initializeButtonIndicators('clockStartButton', 'clockStopButton', 'clockState');

    setInterval(pingServer, PING_TIME);

    initializePopup();
});

document.addEventListener('DOMContentLoaded', () => {
    const popup = document.getElementById('log-popup');
    const popupContent = document.getElementById('popup-content');
    const logLink = document.getElementById('open-logs'); 

    function togglePopup() {
        popup.style.display = popup.style.display === 'none' || popup.style.display === '' ? 'block' : 'none';
        if (popup.style.display === 'block') scrollToBottom();
    }

    function scrollToBottom() {
        if (popupContent) popupContent.scrollTop = popupContent.scrollHeight;
    }

    popup.style.display = 'none';

    document.addEventListener('keydown', (event) => {
        if (event.shiftKey && (event.key === '`' || event.key === '~')) {
            togglePopup();
        }
    });

    logLink?.addEventListener('click', (event) => {
        event.preventDefault();
        togglePopup();
    });

    document.querySelector('.popup-close')?.addEventListener('click', () => {
        popup.style.display = 'none';
    });

    document.body.addEventListener('htmx:afterOnLoad', (event) => {
        if (event.detail.elt.id === 'log-content') {
            scrollToBottom();
        }
    });

    popupContent?.addEventListener('DOMSubtreeModified', scrollToBottom);
});

window.onload = function () {
    const savedColor = localStorage.getItem('overlayColor');
    const savedAlpha = localStorage.getItem('overlayAlpha');

    if (savedColor) {
        document.getElementById('overlay-color').value = savedColor;
        document.getElementById('color-value').textContent = savedColor;
    }

    if (savedAlpha) {
        document.getElementById('overlay-alpha').value = savedAlpha;
        document.getElementById('alpha-value').textContent = savedAlpha;
    }
};

document.getElementById('overlay-color').addEventListener('input', function () {
    const color = this.value;
    document.getElementById('color-value').textContent = color;
    localStorage.setItem('overlayColor', color);
});

document.getElementById('overlay-alpha').addEventListener('input', function () {
    const alpha = this.value;
    document.getElementById('alpha-value').textContent = alpha;
    localStorage.setItem('overlayAlpha', alpha);
});

document.getElementById('reset-color').addEventListener('click', function () {
    const defaultColor = '#00b140';
    const defaultAlpha = '100';
    const defaultOpacity = '50';

    document.getElementById('overlay-color').value = defaultColor;
    document.getElementById('color-value').textContent = defaultColor;
    localStorage.setItem('overlayColor', defaultColor);

    document.getElementById('overlay-alpha').value = defaultAlpha;
    document.getElementById('alpha-value').textContent = defaultAlpha;
    localStorage.setItem('overlayAlpha', defaultAlpha);

});

function apiCopy(button, text) {
    if (navigator.clipboard && window.isSecureContext) {
        navigator.clipboard.writeText(text).then(() => {
            console.log('Text copied to clipboard');
        }).catch(err => {
            console.error('Failed to copy text: ', err);
        });
    } else {
        const tempInput = document.createElement('input');
        tempInput.value = text;
        document.body.appendChild(tempInput);
        tempInput.select();
        try {
            document.execCommand('copy');
            console.log('Text copied to clipboard');
        } catch (err) {
            console.error('Failed to copy text: ', err);
        }
        document.body.removeChild(tempInput);
    }

    button.textContent = 'Copied!';

    setTimeout(() => {
        button.textContent = 'Copy';
    }, 2000);
}

function applyCooldown(button) {
    button.disabled = true;
    button.classList.add('popup-cooldown');
    setTimeout(function () {
        button.disabled = false;
        button.classList.remove('popup-cooldown');
    }, 7500);
}

let modes = ["High School", "Professional", "Jason Mode", "Custom"];
let currentModeIndex = 0;

function toggleButtonGroup() {
    currentModeIndex = (currentModeIndex + 1) % modes.length;
    let currentMode = modes[currentModeIndex];
    try {
        localStorage.setItem('currentMode', currentMode);
    } catch (error) {
        console.error('Error saving mode to localStorage:', error);
    }
    document.getElementById('current-mode').textContent = `Mode: ${currentMode}`;
    loadDefaultDistances(currentMode);

    document.getElementById('custom-togo-inputs').style.display =
        currentMode === "Custom" ? "block" : "none";

    applyCooldown(document.getElementById('toggle-mode'));
}

function loadDefaultDistances(mode) {
    let defaultDistances = {
        "High School": [0, 10, 20, 30, 40, 50],
        "Professional": [0, 15, 25, 35, 45, 55],
        "Jason Mode": [0, 3, 6, 9, 13, 17],
        "Custom": [0, 0, 0, 0, 0, 0]
    };

    let distances;
    if (mode === "Custom") {
        try {
            distances = JSON.parse(localStorage.getItem('customDistances')) || defaultDistances["Custom"];
        } catch (error) {
            console.error('Error parsing custom distances:', error);
            distances = defaultDistances["Custom"];
        }
    } else {
        distances = defaultDistances[mode] || defaultDistances["High School"];
    }

    updateToGoButtons(distances);
}

function updateToGoButtons(distances) {
    for (let i = 0; i < 6; i++) {
        let button = document.getElementById(`togo-button-${i + 1}`);
        if (button) {
            button.textContent = distances[i];
            button.setAttribute('hx-post', `/downs/togo/set/${distances[i]}`);
        }
    }
    if (modes[currentModeIndex] === "Custom") {
        for (let i = 0; i < 6; i++) {
            let input = document.getElementById(`input${i + 1}`);
            if (input) {
                input.value = distances[i];
            }
        }
    }
}

let saveTimeout;
function saveDistances() {
    clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
        let distances = [];
        for (let i = 1; i <= 6; i++) {
            let inputValue = parseInt(document.getElementById(`input${i}`).value);
            distances.push(isNaN(inputValue) ? 0 : inputValue);
        }
        try {
            localStorage.setItem('customDistances', JSON.stringify(distances));
            updateToGoButtons(distances);
            alert('Distances saved!');
        } catch (error) {
            console.error('Error saving custom distances:', error);
            alert('Error saving distances. Please try again.');
        }
    }, 500);
}

window.addEventListener('load', function () {
    let savedMode;
    try {
        savedMode = localStorage.getItem('currentMode') || "High School";
    } catch (error) {
        console.error('Error reading mode from localStorage:', error);
        savedMode = "High School";
    }
    currentModeIndex = modes.indexOf(savedMode);
    document.getElementById('current-mode').textContent = `Mode: ${savedMode}`;
    loadDefaultDistances(savedMode);

    document.getElementById('custom-togo-inputs').style.display =
        savedMode === "Custom" ? "block" : "none";
});

document.addEventListener('DOMContentLoaded', (event) => {
    const toggle = document.getElementById('professional-mode-toggle');

    const professionalMode = localStorage.getItem('professionalMode') === 'true';
    toggle.checked = professionalMode;

    function updateProfessionalMode(isEnabled) {
        localStorage.setItem('professionalMode', isEnabled);
        console.log(`Professional Mode set to ${isEnabled}`);
    }

    toggle.addEventListener('change', (event) => {
        updateProfessionalMode(event.target.checked);
    });
});

pingServer();
checkForUpdate();
loadPresets()
