// Constants
const PING_TIME = 1000;
const DEFAULT_THEME = 'theme-dark';
const LOCK_INTERFACE_BTN_ID = 'lockInterfaceBtn';
const POPUP_DURATION = 7500;
const VERSION = '2.0.1';
const PING_URL = '/';
const PREVIEW_URL = '/overlay';

// Cache DOM elements
const pingValue = document.getElementById('ping-value');
const lockInterfaceBtn = document.getElementById(LOCK_INTERFACE_BTN_ID);
const body = document.body;

// Utility functions
const $ = (selector) => document.querySelector(selector);
const $$ = (selector) => document.querySelectorAll(selector);

// Error handling
function handleError(code, message, error) {
    console.error(`E${code}: ${message}`, error);
    // Implement logging or error reporting here
}

// Ping functionality
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

// Menu toggle
function toggleMenu() {
    try {
        $('.hamburger-menu')?.classList.toggle('active');
        $('.sidenavbar')?.classList.toggle('active');
    } catch (error) {
        handleError('005', 'Error toggling menu', error);
    }
}

// Interface lock functionality
function toggleInterfaceLock() {
    body.classList.toggle('interface-locked');
    const isLocked = body.classList.contains('interface-locked');
    lockInterfaceBtn.innerHTML = `<strong>${isLocked ? 'Unlock' : 'Lock'} Interface</strong>`;
    localStorage.setItem('lockState', isLocked ? 'locked' : 'unlocked');
}

// Theme application
function applyTheme(theme) {
    body.className = theme;
    localStorage.setItem('theme', theme);
    $$('.theme-button').forEach(btn => btn.classList.toggle('theme-active', btn.id === `${theme.replace('theme-', '')}-theme`));
}

// Button indicator functionality
function initializeButtonIndicators(startButtonId, stopButtonId, storageKey) {
    const startButton = document.getElementById(startButtonId);
    const stopButton = document.getElementById(stopButtonId);

    if (!startButton || !stopButton) {
        handleError('008', `Button elements not found for: ${startButtonId}, ${stopButtonId}`);
        return;
    }

    function updateButtonState(isStart) {
        startButton.classList.toggle('selector-active', isStart);
        stopButton.classList.toggle('selector-active', !isStart);
        localStorage.setItem(storageKey, isStart ? 'start' : 'stop');
    }

    startButton.addEventListener('click', () => updateButtonState(true));
    stopButton.addEventListener('click', () => updateButtonState(false));

    // Initialize state from localStorage
    const savedState = localStorage.getItem(storageKey);
    if (savedState === 'start' || savedState === 'stop') {
        updateButtonState(savedState === 'start');
    }
}

// Event listeners
document.addEventListener('DOMContentLoaded', () => {
    // Initialize theme
    applyTheme(localStorage.getItem('theme') || DEFAULT_THEME);

    // Initialize lock state
    const lockState = localStorage.getItem('lockState');
    if (lockState === 'locked') {
        body.classList.add('interface-locked');
        lockInterfaceBtn.innerHTML = '<strong>Unlock Interface</strong>';
    }

    // Set up preview iframe
    const iframe = $('#previewIframe');
    if (iframe) iframe.src = PREVIEW_URL;

    // Set up host IP input
    const hostIPInput = $('#host-ip');
    if (hostIPInput) {
        hostIPInput.value = localStorage.getItem('hostIP') || '';
        hostIPInput.addEventListener('input', () => {
            const sanitizedValue = hostIPInput.value.replace(/[&<>"'\\/?*]/g, '');
            hostIPInput.value = sanitizedValue;
            localStorage.setItem('hostIP', sanitizedValue);
        });
    }

    // Set up theme buttons
    ['light', 'dark', 'colorblind'].forEach(themeName => {
        $(`#${themeName}-theme`)?.addEventListener('click', () => applyTheme(`theme-${themeName}`));
    });

    // Set up lock interface button
    lockInterfaceBtn?.addEventListener('click', toggleInterfaceLock);

    // Set up menu toggle
    $('.hamburger-menu')?.addEventListener('click', toggleMenu);

    // Initialize button indicators
    initializeButtonIndicators('countdownStartButton', 'countdownStopButton', 'countdownState');
    initializeButtonIndicators('clockStartButton', 'clockStopButton', 'clockState');

    // Start ping interval
    setInterval(pingServer, PING_TIME);

    // Initialize popup
    initializePopup();
});

// Popup functionality
function initializePopup() {
    const popup = $('#log-popup');
    const popupContent = $('#popup-content');

    function togglePopup() {
        popup.style.display = popup.style.display === 'none' || popup.style.display === '' ? 'block' : 'none';
        if (popup.style.display === 'block') scrollToBottom();
    }

    function scrollToBottom() {
        if (popupContent) popupContent.scrollTop = popupContent.scrollHeight;
    }

    document.addEventListener('keydown', (event) => {
        if (event.key === '`' || event.key === '~') togglePopup();
    });

    $('.popup-close')?.addEventListener('click', () => popup.style.display = 'none');
    $('#open-logs')?.addEventListener('click', (event) => {
        event.preventDefault();
        togglePopup();
    });

    body.addEventListener('htmx:afterOnLoad', (event) => {
        if (event.detail.elt.id === 'log-content') scrollToBottom();
    });

    popupContent?.addEventListener('DOMSubtreeModified', scrollToBottom);

    // Initially hide the popup
    popup.style.display = 'none';
}
document.addEventListener('DOMContentLoaded', () => {
    checkForUpdate(); // Check for updates on page load
});

function notifyUserOfUpdate(latestRelease) {
    // Check if the banner was previously ignored and if it's still within the week
    const ignoredTimestamp = localStorage.getItem('update-banner-ignored');
    if (ignoredTimestamp) {
        const oneWeek = 7 * 24 * 60 * 60 * 1000;
        const currentTime = Date.now();
        if (currentTime - parseInt(ignoredTimestamp, 10) < oneWeek) {
            return; // Don't show the banner if it was ignored less than a week ago
        }
    }

    // Create the banner element
    const updateBanner = document.createElement('div');
    updateBanner.classList.add('update-banner');

    // Add the HTML content to the banner
    updateBanner.innerHTML = `
        <div class="update-banner-content">
            <a href="${latestRelease.html_url}" target="_blank" class="update-banner-link">
                New update available: ${latestRelease.tag_name}. Click to download
            </a>
            <button class="update-banner-ignore">Ignore</button>
        </div>
    `;

    // Append the banner to the body
    document.body.prepend(updateBanner);

    // Add event listener for the Ignore button
    document.querySelector('.update-banner-ignore').addEventListener('click', () => {
        updateBanner.style.display = 'none';
        // Set the timestamp for when the banner was ignored
        localStorage.setItem('update-banner-ignored', Date.now());
    });
}

async function checkForUpdate() {
    try {
        const response = await fetch('https://api.github.com/repos/AllLiver/Froggi/releases/latest');
        const data = await response.json();
        if (data.tag_name !== VERSION) {
            notifyUserOfUpdate(data);
        }
    } catch (error) {
        handleError('018', 'Error fetching the latest release', error);
    }
}

// Load saved values from local storage on page load
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

// Save values to local storage when inputs change
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

// Reset to default values
document.getElementById('reset-color').addEventListener('click', function () {
    const defaultColor = '#00b140';
    const defaultAlpha = '100';

    document.getElementById('overlay-color').value = defaultColor;
    document.getElementById('color-value').textContent = defaultColor;
    localStorage.setItem('overlayColor', defaultColor);

    document.getElementById('overlay-alpha').value = defaultAlpha;
    document.getElementById('alpha-value').textContent = defaultAlpha;
    localStorage.setItem('overlayAlpha', defaultAlpha);
});

function apiCopy(text) {
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
    localStorage.setItem('currentMode', currentMode);
    document.getElementById('current-mode').textContent = `Mode: ${currentMode}`;
    loadDefaultDistances(currentMode);

    // Toggle custom inputs visibility
    document.getElementById('custom-togo-inputs').style.display =
        currentMode === "Custom" ? "block" : "none";

    applyCooldown(document.getElementById('toggle-mode'));
}

function loadDefaultDistances(mode) {
    let defaultDistances = {
        "High School": [0, 10, 20, 30, 40, 50],
        "Professional": [0, 15, 25, 35, 45, 55],
        "Jason Mode": [0, 3, 6, 9, 13, 17],
        "Custom": JSON.parse(localStorage.getItem('customDistances')) || [0, 0, 0, 0, 0, 0]
    };
    let distances = defaultDistances[mode];
    updateToGoButtons(distances);
}

function updateToGoButtons(distances) {
    for (let i = 0; i < 6; i++) {
        let button = document.getElementById(`togo-button-${i + 1}`);
        button.textContent = distances[i];
        button.setAttribute('hx-post', `/downs/togo/set/${distances[i]}`);
    }
    if (modes[currentModeIndex] === "Custom") {
        for (let i = 0; i < 6; i++) {
            document.getElementById(`input${i + 1}`).value = distances[i];
        }
    }
}

function saveDistances() {
    let distances = [];
    for (let i = 1; i <= 6; i++) {
        distances.push(parseInt(document.getElementById(`input${i}`).value) || 0);
    }
    localStorage.setItem('customDistances', JSON.stringify(distances));
    updateToGoButtons(distances);
    alert('Distances saved!');
}

// On page load
window.onload = function () {
    let savedMode = localStorage.getItem('currentMode') || "High School";
    currentModeIndex = modes.indexOf(savedMode);
    document.getElementById('current-mode').textContent = `Mode: ${savedMode}`;
    loadDefaultDistances(savedMode);

    // Set initial visibility of custom inputs
    document.getElementById('custom-togo-inputs').style.display =
        savedMode === "Custom" ? "block" : "none";
};
// Initial calls
pingServer();
checkForUpdate();