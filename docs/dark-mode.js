// Dark mode toggle with localStorage persistence and OS preference detection
(function () {
  var STORAGE_KEY = "jarl-dark-mode";

  function isDark() {
    var stored = localStorage.getItem(STORAGE_KEY);
    if (stored !== null) return stored === "true";
    return window.matchMedia("(prefers-color-scheme: dark)").matches;
  }

  function apply(dark) {
    document.documentElement.classList.toggle("dark-mode", dark);
    var btn = document.querySelector(".dark-mode-toggle");
    if (btn) btn.innerHTML = dark ? '<i class="bi bi-sun"></i>' : '<i class="bi bi-moon-fill"></i>';
  }

  // Apply immediately to avoid flash
  apply(isDark());

  document.addEventListener("DOMContentLoaded", function () {
    // Insert toggle button in the navbar
    var navRight = document.querySelector(".navbar-nav.navbar-nav-scroll.ms-auto");
    if (!navRight) navRight = document.querySelector(".navbar-nav.ms-auto");
    if (!navRight) return;

    var li = document.createElement("li");
    li.className = "nav-item-compact";

    var btn = document.createElement("button");
    btn.className = "dark-mode-toggle nav-link";
    btn.type = "button";
    btn.setAttribute("aria-label", "Toggle dark mode");
    btn.innerHTML = isDark() ? '<i class="bi bi-sun"></i>' : '<i class="bi bi-moon-fill"></i>';

    btn.addEventListener("click", function () {
      var dark = !document.documentElement.classList.contains("dark-mode");
      localStorage.setItem(STORAGE_KEY, dark);
      apply(dark);
    });

    li.appendChild(btn);
    navRight.insertBefore(li, navRight.firstChild);
  });

  // Follow OS changes unless the user has set a manual preference
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", function (e) {
    if (localStorage.getItem(STORAGE_KEY) === null) {
      apply(e.matches);
    }
  });
})();
