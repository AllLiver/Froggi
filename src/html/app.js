// * Bring in the hamburger menu and side navbar from offscreen
function toggleMenu() {
    const menuButton = document.querySelector('.hamburger-menu');
    const sideNav = document.querySelector('.sidenavbar');
    menuButton.classList.toggle('active');
    sideNav.classList.toggle('active');
}
