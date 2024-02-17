htmx.on('startButton', 'htmx:afterRequest', function () {
    document.getElementById('startButton').classList.add('selected');
    document.getElementById('stopButton').classList.remove('selected');
});

htmx.on('stopButton', 'htmx:afterRequest', function () {
    document.getElementById('stopButton').classList.add('selected');
    document.getElementById('startButton').classList.remove('selected');
});
var quarters = document.querySelectorAll('.quarter-container button');

function removeActiveClass() {
    quarters.forEach(function (quarter) {
        quarter.classList.remove('active');
    });
}
quarters.forEach(function (quarter) {
    quarter.addEventListener('click', function () {
        removeActiveClass();
        this.classList.add('active');
        document.getElementById('show_quarter').classList.remove('active');
    });
});
document.addEventListener('DOMContentLoaded', function () {
    const startButton = document.getElementById('startButton');
    const stopButton = document.getElementById('stopButton');

    startButton.addEventListener('click', function () {
        startButton.classList.add('selected');
        stopButton.classList.remove('selected');
    });

    stopButton.addEventListener('click', function () {
        stopButton.classList.add('selected');
        startButton.classList.remove('selected');
    });
});

// Teaminfo

document.addEventListener('DOMContentLoaded', function () {
    window.previewImage = function (input, previewId) {
        if (input.files && input.files[0]) {
            var reader = new FileReader();
            reader.onload = function (e) {
                document.getElementById(previewId).src = e.target.result;
                document.getElementById(previewId).style.display = 'block';
            }
            reader.readAsDataURL(input.files[0]);
        }
    }

    document.querySelector('#logo-form').addEventListener('submit', function (event) {
        event.preventDefault();
        var home = document.getElementById('home_img').value;
        var away = document.getElementById('away_img').value;
        if (home && away) {
            var xhr = new XMLHttpRequest();
            xhr.open("POST", "/logo_upload", true);
            xhr.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
            xhr.onreadystatechange = function () {
                if (this.readyState === XMLHttpRequest.DONE && this.status === 200) {
                    console.log("Logo upload request finished and response is ready.");
                }
            };
            xhr.send("home=" + home + "&away=" + away);
        }
    });

    document.querySelector('#name-form').addEventListener('submit', function (event) {
        event.preventDefault();
        var homeName = document.getElementById('homeName').value;
        var awayName = document.getElementById('awayName').value;
        if (homeName && awayName) {
            var xhr = new XMLHttpRequest();
            xhr.open("POST", "/name_submit", true);
            xhr.setRequestHeader("Content-Type", "application/x-www-form-urlencoded");
            xhr.onreadystatechange = function () {
                if (this.readyState === XMLHttpRequest.DONE && this.status === 200) {
                    console.log("Name submit request finished and response is ready.");

                    document.getElementById('homeNameDisplay').textContent = homeName;
                    document.getElementById('awayNameDisplay').textContent = awayName;
                }
            };
            xhr.send("home=" + homeName + "&away=" + awayName);
        }
    });
});

document.querySelector('#logo-form').addEventListener('htmx:afterOnLoad', function () {
    var submitButton = document.getElementById('file-submit');
    submitButton.style.backgroundColor = 'green';
    submitButton.value = 'Success';

    setTimeout(function () {
        submitButton.style.backgroundColor = '';
        submitButton.value = 'Submit';
    }, 10000);

    document.querySelector('#name-form').addEventListener('htmx:afterOnLoad', function () {
        var submitButton = document.getElementById('name-submit');
        submitButton.style.backgroundColor = 'green';
        submitButton.value = 'Success';


        setTimeout(function () {
            submitButton.style.backgroundColor = '';
            submitButton.value = 'Submit';
        }, 10000);
    });
});


fetch('/teamload/ID', {
    method: 'POST',
})


// Countdown

document.getElementById('show-countdown').addEventListener('click', function () {
    this.classList.toggle('on');
});
document.getElementById('show-sponsor_roll').addEventListener('click', function () {
    this.classList.toggle('on');
});



// Background & Login

const canvas = document.getElementById('background');
const ctx = canvas.getContext('2d');

canvas.width = window.innerWidth;
canvas.height = window.innerHeight;

const circles = [];
for (let i = 0; i < 50; i++) {
    circles.push({
        x: Math.random() * canvas.width,
        y: Math.random() * canvas.height,
        radius: Math.random() * 20 + 10,
        color: `rgba(${Math.random() * 255},${Math.random() * 255},${Math.random() * 255},0.3)`, // Reduced opacity
        vx: Math.random() * 4 - 2,
        vy: Math.random() * 4 - 2
    });
}

function draw() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    circles.forEach(circle => {
        ctx.beginPath();
        ctx.arc(circle.x, circle.y, circle.radius, 0, Math.PI * 2);
        ctx.fillStyle = circle.color;
        ctx.fill();
        ctx.closePath();

        circle.x += circle.vx;
        circle.y += circle.vy;

        if (circle.x - circle.radius < 0 || circle.x + circle.radius > canvas.width) {
            circle.vx *= -1;
        }
        if (circle.y - circle.radius < 0 || circle.y + circle.radius > canvas.height) {
            circle.vy *= -1;
        }
    });
    requestAnimationFrame(draw);
}

draw();

canvas.addEventListener('mousemove', function(event) {
    circles.forEach(circle => {
        const dx = event.clientX - circle.x;
        const dy = event.clientY - circle.y;
        const distance = Math.sqrt(dx * dx + dy * dy);
        if (distance < 100) {
            circle.vx += dx * 0.001;
            circle.vy += dy * 0.001;
        }
    });
});

function validateForm() {
    const password = document.getElementById("password").value;
    const confirm_password = document.getElementById("confirm_password").value;
    const password_error = document.getElementById("password_error");

    if (password !== confirm_password) {
        password_error.textContent = "The passwords do not match";
        return false;
    } else {
        password_error.textContent = "";
        return true;
    }
}