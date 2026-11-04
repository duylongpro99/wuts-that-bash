// Shared drawing helpers for the Product Hunt assets: pencil SVG filters, paper texture,
// sketched outlines behind boxes, and a terminal renderer that mirrors wib's real list.

const PP_DEFS = `
<svg width="0" height="0" style="position:absolute" aria-hidden="true">
  <defs>
    <filter id="pp-ink" x="-5%" y="-5%" width="110%" height="110%">
      <feTurbulence type="fractalNoise" baseFrequency="0.011" numOctaves="2" seed="7" result="slow"/>
      <feDisplacementMap in="SourceGraphic" in2="slow" scale="4" xChannelSelector="R" yChannelSelector="G"/>
    </filter>
    <filter id="pp-ink-b" x="-5%" y="-5%" width="110%" height="110%">
      <feTurbulence type="fractalNoise" baseFrequency="0.014" numOctaves="2" seed="23" result="slow"/>
      <feDisplacementMap in="SourceGraphic" in2="slow" scale="5" xChannelSelector="R" yChannelSelector="G"/>
    </filter>
    <!-- Same ink passes in user space, for straight lines whose bounding box has no height. -->
    <filter id="pp-ink-u" filterUnits="userSpaceOnUse" x="0" y="0" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.011" numOctaves="2" seed="7" result="slow"/>
      <feDisplacementMap in="SourceGraphic" in2="slow" scale="4" xChannelSelector="R" yChannelSelector="G"/>
    </filter>
    <filter id="pp-ink-bu" filterUnits="userSpaceOnUse" x="0" y="0" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.014" numOctaves="2" seed="23" result="slow"/>
      <feDisplacementMap in="SourceGraphic" in2="slow" scale="5" xChannelSelector="R" yChannelSelector="G"/>
    </filter>
    <filter id="pp-pencil" x="-5%" y="-5%" width="110%" height="110%">
      <feTurbulence type="fractalNoise" baseFrequency="0.45 1.9" numOctaves="2" seed="11" result="grain"/>
      <feColorMatrix in="grain" type="matrix" values="0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 -3.2 2.3" result="holes"/>
      <feComposite in="SourceGraphic" in2="holes" operator="in" result="grained"/>
      <feTurbulence type="fractalNoise" baseFrequency="0.02" numOctaves="2" seed="5" result="slow"/>
      <feDisplacementMap in="grained" in2="slow" scale="6" xChannelSelector="R" yChannelSelector="G"/>
    </filter>
    <filter id="pp-paper" x="0" y="0" width="100%" height="100%">
      <feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="3" seed="2" result="n"/>
      <feColorMatrix in="n" type="matrix" values="0 0 0 0 0.36  0 0 0 0 0.30  0 0 0 0 0.22  0 0 0 0.10 0"/>
      <feComposite in2="SourceGraphic" operator="in"/>
      <feBlend in2="SourceGraphic" mode="multiply"/>
    </filter>
    <pattern id="pp-hatch" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(35)">
      <line x1="0" y1="0" x2="0" y2="6" stroke="#5a5258" stroke-width="0.9" stroke-linecap="round"/>
    </pattern>
    <marker id="pp-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="9" markerHeight="9" orient="auto-start-reverse" markerUnits="userSpaceOnUse">
      <path d="M1 1.2 L8.6 5 L1 8.8" fill="none" stroke="#1d1a1e" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
    </marker>
    <marker id="pp-arrow-red" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="9" markerHeight="9" orient="auto-start-reverse" markerUnits="userSpaceOnUse">
      <path d="M1 1.2 L8.6 5 L1 8.8" fill="none" stroke="#a8443b" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/>
    </marker>
  </defs>
</svg>`;

const INK = "#1d1a1e";

function paperBackground(stage) {
  const w = stage.offsetWidth, h = stage.offsetHeight;
  stage.insertAdjacentHTML("afterbegin",
    `<svg class="paper-bg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
       <rect width="${w}" height="${h}" fill="#f6f1e6" filter="url(#pp-paper)"/>
     </svg>`);
}

// Draw a sketched outline (clean paper, optional pencil tint, two ink passes) behind `el`.
// opts.bar: height of a tinted title bar; opts.tint: whole-box tint; opts.stroke: ink width;
// opts.shadow: hatched offset shadow.
function sketch(el, opts = {}) {
  const w = el.offsetWidth, h = el.offsetHeight;
  const sw = opts.stroke || 2.4;
  const pad = 16;
  const bar = opts.bar ? `
      <rect x="0" y="0" width="${w}" height="${opts.bar}" fill="${opts.barTint || "#c4dbe8"}" filter="url(#pp-pencil)" opacity="0.75"/>
      <line x1="0" y1="${opts.bar}" x2="${w}" y2="${opts.bar}" stroke="${INK}" stroke-width="${sw * 0.7}" filter="url(#pp-ink-u)"/>` : "";
  const tint = opts.tint ? `<rect x="0" y="0" width="${w}" height="${h}" fill="${opts.tint}" filter="url(#pp-pencil)" opacity="${opts.tintOpacity || 0.6}"/>` : "";
  const shadow = opts.shadow ? `<rect x="10" y="10" width="${w}" height="${h}" fill="url(#pp-hatch)" opacity="0.55"/>` : "";
  const svg = `
    <svg class="sk-frame" width="${w + pad * 2}" height="${h + pad * 2}" viewBox="${-pad} ${-pad} ${w + pad * 2} ${h + pad * 2}"
         style="left:${-pad}px;top:${-pad}px">
      ${shadow}
      <rect x="0" y="0" width="${w}" height="${h}" fill="${opts.fill || "#fbf7ee"}"/>
      ${tint}${bar}
      <rect x="0" y="0" width="${w}" height="${h}" fill="none" stroke="${opts.ink || INK}" stroke-width="${sw}" stroke-linejoin="round" filter="url(#pp-ink)"/>
      <rect x="0" y="0" width="${w}" height="${h}" fill="none" stroke="${opts.ink || INK}" stroke-width="${sw * 0.8}" stroke-linejoin="round" filter="url(#pp-ink-b)" opacity="0.45"/>
    </svg>`;
  el.querySelectorAll(":scope > svg.sk-frame").forEach((n) => n.remove());
  el.insertAdjacentHTML("afterbegin", svg.trim());
}

function esc(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// Bold-underline the parts of `cmd` matched by `input`: a plain prefix, or each chunk of the
// input at the start of successive words (wib's word-initial fuzzy match).
function highlight(cmd, input) {
  if (!input) return esc(cmd);
  if (cmd.startsWith(input)) return `<span class="m">${esc(input)}</span>${esc(cmd.slice(input.length))}`;
  // Each word may consume the longest piece of the remaining input it starts with
  // (dcup -> d|c|up over "docker compose up"); a space in the input ends a piece.
  const words = cmd.split(" ");
  let rest = input.replace(/^ +/, "");
  const out = [];
  for (const w of words) {
    let n = 0;
    while (n < rest.length && rest[n] !== " " && w[n] === rest[n]) n++;
    if (n > 0) {
      out.push(`<span class="m">${esc(w.slice(0, n))}</span>${esc(w.slice(n))}`);
      rest = rest.slice(n).replace(/^ +/, "");
    } else out.push(esc(w));
  }
  return out.join(" ");
}

// Highlight keywords anywhere in the command (wib <keywords> mode).
function highlightKeywords(cmd, kws) {
  let html = esc(cmd);
  for (const k of kws) html = html.split(esc(k)).join(`<span class="m">${esc(k)}</span>`);
  return html;
}

// Render a terminal body. state: { lines: [html], prompt, buf, cursor: bool, list: [{cmd, fail}], sel, kw }
function termHTML(s) {
  const rows = [];
  for (const l of s.lines || []) rows.push(`<span class="row out">${l}</span>`);
  const cur = s.cursor === false ? "" : `<span class="cursor">_</span>`;
  rows.push(`<span class="row"><span class="prompt">${esc(s.prompt)}</span> ${esc(s.buf || "")}${cur}</span>`);
  const input = s.kw ? null : (s.buf || "");
  (s.list || []).forEach((it, i) => {
    const cls = ["row", "sug", i === s.sel ? "sel" : "", it.fail ? "fail" : ""].join(" ");
    const body = s.kw ? highlightKeywords(it.cmd, s.kw) : highlight(it.cmd, input);
    const x = it.fail ? `  <span class="x">✗ never worked</span>` : "";
    rows.push(`<span class="${cls}"><span class="mark">›</span> ${body}${x}</span>`);
  });
  return rows.join("");
}

async function ready() {
  document.body.insertAdjacentHTML("afterbegin", PP_DEFS);
  await document.fonts.ready;
  await Promise.all(["Cutive Mono", "Courier Prime", "Nunito Sans"].map((f) =>
    document.fonts.load(`40px "${f}"`).catch(() => null)));
  await document.fonts.load(`italic 40px "Courier Prime"`).catch(() => null);
  await document.fonts.load(`700 40px "Nunito Sans"`).catch(() => null);
}
