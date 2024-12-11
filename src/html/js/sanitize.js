function handleSecureInput(event) {
    const inputElement = event.target;
    const sanitized = sanitizeInput(inputElement.value);
    inputElement.value = sanitized; 
}

function sanitizeInput(input) {
    return input.replace(/[^a-zA-Z0-9\s]/g, ''); 
}

