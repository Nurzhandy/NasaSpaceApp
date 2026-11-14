// Звёздный фон
const canvas = document.getElementById("stars");
const ctx = canvas.getContext("2d");
let stars = [];

function resize() {
  canvas.width = innerWidth;
  canvas.height = innerHeight;
  stars = Array.from({ length: 200 }, () => ({
    x: Math.random() * canvas.width,
    y: Math.random() * canvas.height,
    r: Math.random() * 1.5,
    s: Math.random() * 0.3 + 0.05,
  }));
}

function draw() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = "#fff";
  for (const st of stars) {
    st.y += st.s;
    if (st.y > canvas.height) st.y = 0;
    ctx.beginPath();
    ctx.arc(st.x, st.y, st.r, 0, Math.PI * 2);
    ctx.fill();
  }
  requestAnimationFrame(draw);
}

addEventListener("resize", resize);
resize();
draw();

// Планеты Солнечной системы
const planets = [
  { name: "Меркурий", color: "#b1b1b1", info: "0.39 а.е. от Солнца" },
  { name: "Венера", color: "#e6c27a", info: "0.72 а.е. от Солнца" },
  { name: "Земля", color: "#4f8cff", info: "1 а.е. от Солнца" },
  { name: "Марс", color: "#d9643a", info: "1.52 а.е. от Солнца" },
  { name: "Юпитер", color: "#d8a878", info: "5.2 а.е. от Солнца" },
  { name: "Сатурн", color: "#e8d49a", info: "9.5 а.е. от Солнца" },
  { name: "Уран", color: "#8fe0e6", info: "19.2 а.е. от Солнца" },
  { name: "Нептун", color: "#4166f5", info: "30.1 а.е. от Солнца" },
];

document.getElementById("planets").innerHTML = planets
  .map(
    (p) => `
    <div class="planet">
      <div class="dot" style="background:${p.color}"></div>
      <h2>${p.name}</h2>
      <span>${p.info}</span>
    </div>`
  )
  .join("");
