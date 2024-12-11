document.addEventListener("DOMContentLoaded", function() {
    document.documentElement.requestFullscreen ? document.documentElement.requestFullscreen() : document.documentElement.mozRequestFullScreen ? document.documentElement.mozRequestFullScreen() : document.documentElement.webkitRequestFullscreen ? document.documentElement.webkitRequestFullscreen() : document.documentElement.msRequestFullscreen && document.documentElement.msRequestFullscreen();
    let e = localStorage.getItem("overlayColor") || "#00b140",
        t = localStorage.getItem("overlayAlpha") || "1";
    t = parseInt(t) > .5 ? "1" : "0";
    let n = document.querySelector(".overlay");

    function l(e, t) {
        const n = parseInt(e.slice(1, 3), 16),
            l = parseInt(e.slice(3, 5), 16),
            r = parseInt(e.slice(5, 7), 16);
        return `rgba(${n}, ${l}, ${r}, ${t})`
    }
    n && (n.style.backgroundColor = l(e, t)), window.addEventListener("beforeunload", function(e) {
        e.preventDefault();
        const t = "Are you sure you want to leave? Any unsaved changes will be lost.";
        return e.returnValue = t, t
    })
});