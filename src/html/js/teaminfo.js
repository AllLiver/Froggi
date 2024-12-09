document.addEventListener('DOMContentLoaded', () => {
    function handleTeamInput(event) {
        console.log("Input event triggered!");
        const inputElement = event.target;
        let sanitizedValue = DOMPurify.sanitize(inputElement.value, {
            ALLOWED_TAGS: [],
            ALLOWED_ATTR: []
        });
        sanitizedValue = sanitizedValue.replace(/[<>"']/g, '');
        inputElement.value = sanitizedValue;
    }
 
    const awayNameInput = document.getElementById('away_name');
    const homeNameInput = document.getElementById('home_name');
 
    if (awayNameInput) {
        awayNameInput.addEventListener('input', handleTeamInput);
    }
 
    if (homeNameInput) {
        homeNameInput.addEventListener('input', handleTeamInput);
    }
 
    document.getElementById('countdown-form').addEventListener('submit', function (event) {
        const inputElement = document.getElementById('set-countdown-clock-text');
        let sanitizedValue = DOMPurify.sanitize(inputElement.value, {
            ALLOWED_TAGS: [],
            ALLOWED_ATTR: []
        });
        sanitizedValue = sanitizedValue.replace(/[<>"']/g, '');
        inputElement.value = sanitizedValue;
    });
});

function toggleMenu() {
    try {
        $('.hamburger-menu')?.classList.toggle('active');
        $('.sidenavbar')?.classList.toggle('active');
    } catch (error) {
        console.error('005', 'Error toggling menu', error);
    }
}