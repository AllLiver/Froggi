const PING_TIME = 1000;
const DEFAULT_THEME = 'theme-dark';
const LOCK_INTERFACE_BTN_ID = 'lockInterfaceBtn';
const POPUP_DURATION = 7500;
const PING_URL = '/';
const PREVIEW_URL = '/overlay';

const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => document.querySelectorAll(selector);

function pingServer() {
    const pingValue = document.getElementById('ping-value');
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
            console.error('003', 'Ping request failed', error);
        });
}

function deleteCookie(name, path = '/', domain) {
    let cookieString = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 UTC; path=${path};`;
    if (domain) {
        cookieString += ` domain=${domain};`;
    }
    document.cookie = cookieString;
}

function sanitizeInput(input) {
    return input.replace(/\s+/g, '');
}

function escapeHTML(str) {
    return str.replace(/[&<>"']/g, function(match) {
        switch (match) {
            case '&': return '&amp;';
            case '<': return '&lt;';
            case '>': return '&gt;';
            case '"': return '&quot;';
            case "'": return '&#39;';
            default: return match;
        }
    });
}

function applyTheme(theme) {
    const body = document.body;
    const themeClasses = ['theme-light', 'theme-dark', 'theme-colorblind'];

    themeClasses.forEach(cls => body.classList.remove(cls));

    body.classList.add(theme);

    localStorage.setItem('theme', theme);
    $$('.theme-button').forEach(btn => {
        btn.classList.toggle('theme-active', btn.id === `${theme.replace('theme-', '')}-theme`);
    });
}

function toggleInterfaceLock() {
    const body = document.body;
    const lockInterfaceBtn = document.getElementById(LOCK_INTERFACE_BTN_ID);

    body.classList.toggle('interface-locked');
    const isLocked = body.classList.contains('interface-locked');
    lockInterfaceBtn.innerHTML = `<strong>${isLocked ? 'Unlock' : 'Lock'} Interface</strong>`;
    localStorage.setItem('lockState', isLocked ? 'locked' : 'unlocked');
}

function initializeButtonIndicators(startButtonId, stopButtonId, storageKey) {
    const startButton = document.getElementById(startButtonId);
    const stopButton = document.getElementById(stopButtonId);

    if (!startButton || !stopButton) {
        console.error('008', `Button elements not found for: ${startButtonId}, ${stopButtonId}`);
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

function applyCooldown(button, duration = 7500) {
    button.disabled = true;
    button.classList.add('popup-cooldown');
    setTimeout(function () {
        button.disabled = false;
        button.classList.remove('popup-cooldown');
    }, duration);
}

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

document.addEventListener('DOMContentLoaded', () => {

    applyTheme(localStorage.getItem('theme') || DEFAULT_THEME);

    const body = document.body;
    const lockInterfaceBtn = document.getElementById(LOCK_INTERFACE_BTN_ID);
    const lockState = localStorage.getItem('lockState');
    if (lockState === 'locked') {
        body.classList.add('interface-locked');
        lockInterfaceBtn.innerHTML = '<strong>Unlock Interface</strong>';
    }

    ['light', 'dark', 'colorblind'].forEach(themeName => {
        $(`#${themeName}-theme`)?.addEventListener('click', () => applyTheme(`theme-${themeName}`));
    });

    lockInterfaceBtn?.addEventListener('click', toggleInterfaceLock);

    setInterval(pingServer, PING_TIME);
});

window.IndexUtils = {
    pingServer,
    deleteCookie,
    sanitizeInput,
    escapeHTML,
    applyTheme,
    toggleInterfaceLock,
    initializeButtonIndicators,
    applyCooldown,
    apiCopy
};

function initializeButtonIndicators(startButtonId, stopButtonId, storageKey) {
    const startButton = document.getElementById(startButtonId);
    const stopButton = document.getElementById(stopButtonId);

    if (!startButton || !stopButton) {
        console.error('008', `Button elements not found for: ${startButtonId}, ${stopButtonId}`);
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

initializeButtonIndicators('countdownStartButton', 'countdownStopButton', 'countdownState');
initializeButtonIndicators('clockStartButton', 'clockStopButton', 'clockState');
function initializeButtonIndicators(startButtonId, stopButtonId, storageKey) {
    const startButton = document.getElementById(startButtonId);
    const stopButton = document.getElementById(stopButtonId);

    if (!startButton || !stopButton) {
        console.error('008', `Button elements not found for: ${startButtonId}, ${stopButtonId}`);
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