document.addEventListener("DOMContentLoaded", function() {
    let e = document.querySelectorAll('input[name="tabset-2"]'),
        t = document.getElementById("tab5"),
        o = document.querySelectorAll(".counter-input"),
        c = document.querySelectorAll(".counter-value, #clock-counter-minutes, #clock-counter-seconds, #countdown-counter-minutes, #countdown-counter-seconds");

    function n(e) {
        o.forEach((t, o) => {
            let n = c[o];
            t.style.display = e ? "inline-block" : "none", n.style.display = e ? "none" : "inline-block", e && (t.value = n.textContent)
        })
    }

    function r(e) {
        let t = "tab5" === e.target.id;
        n(t), t || o.forEach((e, t) => {
            c[t].textContent = e.value.padStart(2, "0")
        })
    }

    function a(e) {
        let t = e.value,
            o = e.dataset.uri;
        if (o) {
            htmx.ajax("POST", o + t, {
                target: e,
                swap: "none"
            });
            let c = e.closest(".counter").querySelector(".counter-value, #clock-counter-minutes, #clock-counter-seconds, #countdown-counter-minutes, #countdown-counter-seconds");
            c && (c.textContent = t.padStart(2, "0"))
        } else console.error("No data-uri found for this input")
    }
    e.forEach(e => {
        e.addEventListener("change", r)
    }), o.forEach(e => {
        e.addEventListener("change", function() {
            a(this)
        }), e.addEventListener("keydown", function(e) {
            "Enter" === e.key && (a(this), this.blur())
        })
    }), n(t.checked)
}), document.addEventListener("keyup", function(e) {
    if ("input" === e.target.tagName.toLowerCase() || "textarea" === e.target.tagName.toLowerCase()) return;
    let t = document.querySelectorAll(".counter-input");
    for (let o of t)
        if ("none" !== o.style.display) return;
    switch (e.key) {
        case "f":
            document.querySelector(".button-increment-home").click();
            break;
        case "j":
            document.querySelector(".button-increment-away").click();
            break;
        case "1":
            document.querySelector("#button-preset-down-1").click();
            break;
        case "2":
            document.querySelector("#button-preset-down-2").click();
            break;
        case "3":
            document.querySelector("#button-preset-down-3").click();
            break;
        case "4":
            document.querySelector("#button-preset-down-4").click();
            break;
        case "z":
            document.querySelector("#togo-button-1").click();
            break;
        case "x":
            document.querySelector("#togo-button-2").click();
            break;
        case "c":
            document.querySelector("#togo-button-3").click();
            break;
        case "v":
            document.querySelector("#togo-button-4").click();
            break;
        case "b":
            document.querySelector("#togo-button-5").click();
            break;
        case "n":
            document.querySelector("#togo-button-6").click();
            break;
        case "m":
            document.querySelector("#togo-button-goal").click();
            break;
        case " ":
            e.preventDefault(), document.querySelector(".keybind-toggle-clock").dispatchEvent(new Event("change", {
                bubbles: !0
            }))
    }
});

document.addEventListener('DOMContentLoaded', () => {
    function handleCountdownInput(event) {
      console.log("Input event triggered!");
      const inputElement = event.target;
      inputElement.value = sanitizeHtml(inputElement.value, {
        allowedTags: [],
        allowedAttributes: {}
      });
    }
 
    document.getElementById('countdown-form').addEventListener('submit', function (event) {
      const inputElement = document.getElementById('set-countdown-clock-text');
      inputElement.value = sanitizeHtml(inputElement.value, {
        allowedTags: [],
        allowedAttributes: {}
      });
    });
  });