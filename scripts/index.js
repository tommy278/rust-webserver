const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

function toggleTheme() {
  const localPrefersDark = localStorage.getItem("theme");

  if (!localPrefersDark) {
    localStorage.setItem("theme", prefersDark ? "white" : "dark");
    document.documentElement.setAttribute('data-theme', prefersDark ? "white" : "dark")
  } else {
    localStorage.setItem("theme", localPrefersDark ===  "dark" ? "white" : "dark");
    document.documentElement.setAttribute('data-theme', localPrefersDark === "dark" ? "white" : "dark");
  }
}

document.getElementById("theme").addEventListener("click", toggleTheme);

const links = [
    { name: "Home", path: "/" },
    { name: "About", path: "/about" },
    { name: "404 Test", path: "/non-existent" }
];

const nav = document.getElementById('navbar');

links.forEach(link => {
    const a = document.createElement('a');
    
    a.href = link.path;
    a.textContent = link.name;

    nav.appendChild(a);
});

