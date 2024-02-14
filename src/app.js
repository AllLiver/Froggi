async function ping() {
    let start = performance.now();
    await fetch('/ping', { method: 'HEAD' });
    let end = performance.now();
    let pingTime = end - start;
    document.getElementById('ping').textContent = `Ping: ${pingTime.toFixed()}ms`;
}
setInterval(ping, 2000);

const socket = new WebSocket('ws://localhost:3000');
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



// Shwoooooooooooooooooooooooosh alpha
