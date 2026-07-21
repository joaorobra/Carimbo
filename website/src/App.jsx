// Carimbo landing page. Mockups are built in HTML/CSS (no screenshots) so they
// follow the light/dark tokens automatically. Light-first: light is the
// default theme; dark is a nav toggle (persisted, ?theme=dark for testing).
// Icons are Lucide — the same set the app uses.

import { useCallback, useEffect, useRef, useState } from "react";
import {
  motion,
  MotionConfig,
  AnimatePresence,
  useReducedMotion,
} from "framer-motion";
import {
  ArchiveRestore,
  CaseUpper,
  Check,
  ClipboardList,
  Contrast,
  Download as DownloadIcon,
  FileText,
  Globe,
  Keyboard,
  Moon,
  Pin,
  Pipette,
  Power,
  Search,
  ShieldCheck,
  Stamp,
  Star,
  Sun,
  Usb,
} from "lucide-react";

// TODO: point at the real installer once releases are published.
const DOWNLOAD_URL = "https://github.com/carimbo-app/carimbo/releases/latest";

const EASE = [0.32, 0.72, 0, 1];

// ?static freezes all motion at its settled state — used by headless
// screenshot checks, where requestAnimationFrame never runs to completion.
const STATIC =
  typeof window !== "undefined" &&
  new URLSearchParams(window.location.search).has("static");

/* ---------- Icon wrapper — app stroke weight, decorative by default ---------- */

function Ic({ icon: C, size = 20, ...rest }) {
  return <C size={size} strokeWidth={1.8} aria-hidden="true" {...rest} />;
}

/* ---------- Theme (light-first) ---------- */

function useTheme() {
  const [theme, setThemeState] = useState(() => {
    let initial = "light";
    try {
      const q = new URLSearchParams(window.location.search).get("theme");
      initial = q || localStorage.getItem("theme") || "light";
    } catch {
      /* fall through to light */
    }
    // Stamp the attribute before anything renders — children (the shader)
    // read CSS variables in effects that run before this hook's effect.
    document.documentElement.dataset.theme = initial;
    return initial;
  });
  // Stamp the DOM synchronously inside the setter so the new theme's CSS
  // tokens are live before ANY effect runs this render. Child effects (the
  // shader) fire before the parent's effect would, so deferring the stamp to
  // an effect here made the shader read the previous theme's colors — the
  // "needs two clicks / shader out of sync" bug.
  const setTheme = useCallback((next) => {
    setThemeState((prev) => {
      const value = typeof next === "function" ? next(prev) : next;
      document.documentElement.dataset.theme = value;
      try {
        localStorage.setItem("theme", value);
      } catch {
        /* private mode etc. */
      }
      return value;
    });
  }, []);
  return [theme, setTheme];
}

/* ---------- Animated background ---------- */

function Blobs({ blobs }) {
  return (
    <div className="bg-blobs" aria-hidden="true">
      {blobs.map((b, i) => (
        <motion.span
          key={i}
          className="blob"
          style={{ width: b.size, height: b.size, left: b.left, top: b.top }}
          animate={
            STATIC
              ? undefined
              : {
                  x: [0, b.dx, 0],
                  y: [0, b.dy, 0],
                  scale: [1, b.scale ?? 1.12, 1],
                }
          }
          transition={{
            duration: b.dur,
            repeat: Infinity,
            ease: "easeInOut",
            delay: b.delay ?? 0,
          }}
        />
      ))}
    </div>
  );
}

const DOWNLOAD_BLOBS = [
  { size: 380, left: "-8%", top: "-40%", dx: 60, dy: 35, dur: 24 },
  { size: 320, left: "78%", top: "30%", dx: -50, dy: -30, dur: 28, delay: 3 },
];

/* ---------- Shader aurora — Raycast-style animated hero background.
   Raw WebGL (no lib): a fullscreen quad running domain-warped fbm noise in
   the brand's ink hue. Colors come from the live CSS tokens, so it retheme
   with light/dark. Falls back to the plain page background when WebGL is
   unavailable. ---------- */

const SHADER_VERT = `
attribute vec2 a_pos;
void main() {
  gl_Position = vec4(a_pos, 0.0, 1.0);
}`;

const SHADER_FRAG = `
precision highp float;
uniform vec2 u_res;
uniform float u_time;
uniform vec3 u_bg;
uniform vec3 u_c1;
uniform vec3 u_c2;
uniform vec3 u_c3;
uniform float u_int;

float hash(vec2 p) {
  return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453123);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(
    mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
    mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x),
    u.y
  );
}

float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  for (int i = 0; i < 4; i++) {
    v += a * noise(p);
    p = p * 2.03 + vec2(1.7, 3.1);
    a *= 0.5;
  }
  return v;
}

void main() {
  vec2 uv = gl_FragCoord.xy / u_res;
  vec2 p = vec2(uv.x * u_res.x / u_res.y, uv.y) * 1.6;
  float t = u_time * 0.06;

  vec2 q = vec2(
    fbm(p + vec2(0.0, t)),
    fbm(p - vec2(t, 0.0) + vec2(5.2, 1.3))
  );
  vec2 r = vec2(
    fbm(p + 3.0 * q + vec2(1.7, 9.2) + 0.35 * t),
    fbm(p + 3.0 * q + vec2(8.3, 2.8) - 0.30 * t)
  );
  float f = fbm(p + 3.0 * r);

  vec3 aurora = mix(u_c1, u_c2, smoothstep(0.15, 0.85, f));
  aurora = mix(aurora, u_c3, clamp(length(q) * 0.7, 0.0, 1.0));
  float glow = smoothstep(0.30, 0.95, f * f + length(r) * 0.25);
  vec3 col = mix(u_bg, aurora, glow * u_int);
  gl_FragColor = vec4(col, 1.0);
}`;

function cssRgb(name) {
  const v = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  const m = v.match(/^#([0-9a-f]{6})$/i);
  if (!m) return [0.5, 0.5, 0.5];
  const n = parseInt(m[1], 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

function mixRgb(a, b, k) {
  return a.map((v, i) => v + (b[i] - v) * k);
}

function ShaderBackground({ theme }) {
  const canvasRef = useRef(null);
  const reduce = useReducedMotion();

  useEffect(() => {
    const canvas = canvasRef.current;
    const gl = canvas.getContext("webgl", { antialias: false, depth: false });
    if (!gl) return undefined;

    const compile = (type, src) => {
      const sh = gl.createShader(type);
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      return sh;
    };
    const prog = gl.createProgram();
    gl.attachShader(prog, compile(gl.VERTEX_SHADER, SHADER_VERT));
    gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, SHADER_FRAG));
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) return undefined;
    gl.useProgram(prog);

    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 3, -1, -1, 3]),
      gl.STATIC_DRAW,
    );
    const loc = gl.getAttribLocation(prog, "a_pos");
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);

    const u = (name) => gl.getUniformLocation(prog, name);
    const bg = cssRgb("--bg");
    const accent = cssRgb("--accent");
    const weak = cssRgb("--accent-weak");
    gl.uniform3fv(u("u_bg"), bg);
    gl.uniform3fv(u("u_c1"), weak);
    gl.uniform3fv(u("u_c2"), accent);
    gl.uniform3fv(u("u_c3"), mixRgb(accent, bg, 0.5));
    gl.uniform1f(u("u_int"), theme === "dark" ? 0.55 : 0.7);
    const uRes = u("u_res");
    const uTime = u("u_time");

    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
      const w = Math.round(canvas.clientWidth * dpr);
      const h = Math.round(canvas.clientHeight * dpr);
      if (canvas.width !== w || canvas.height !== h) {
        canvas.width = w;
        canvas.height = h;
        gl.viewport(0, 0, w, h);
      }
      gl.uniform2f(uRes, canvas.width, canvas.height);
    };

    const drawAt = (t) => {
      resize();
      gl.uniform1f(uTime, t);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    };

    // Static contexts get one settled, already-interesting frame.
    if (STATIC || reduce) {
      drawAt(8.0);
      const ro = new ResizeObserver(() => drawAt(8.0));
      ro.observe(canvas);
      return () => ro.disconnect();
    }

    let raf;
    const start = performance.now();
    const tick = (now) => {
      drawAt(8.0 + (now - start) / 1000);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    const onVisibility = () => {
      cancelAnimationFrame(raf);
      if (!document.hidden) raf = requestAnimationFrame(tick);
    };
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      cancelAnimationFrame(raf);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [theme, reduce]);

  return (
    <div className="shader-bg" aria-hidden="true">
      <canvas ref={canvasRef} />
    </div>
  );
}

/* ---------- Expander chips — the hero background demos the product:
   each chip "fires" like a real trigger and morphs into its expansion,
   with a stamp-press pulse (Carimbo = rubber stamp). ---------- */

const EXPANDER_CHIPS = [
  { trigger: ";cpf", expansion: "123.456.789-01", pos: { left: "2%", top: "7%" }, offset: 0 },
  { trigger: "{date+7d}", expansion: "07/26/2026", pos: { left: "40%", top: "3%" }, offset: 2 },
  { trigger: ";sig", expansion: "Best regards, Maria", pos: { right: "3%", top: "6%" }, offset: 4 },
  { trigger: "{uuid}", expansion: "8a4e-41f7-b2c9", pos: { left: "4%", top: "84%" }, offset: 6 },
  { trigger: "[[name:Client]]", expansion: "John Doe", pos: { right: "6%", top: "88%" }, offset: 8 },
];

const CHIP_CYCLE_MS = 10000;
const CHIP_EXPANDED_MS = 3200;

const TYPE_SPEED_MS = 45;

function ExpanderChip({ chip }) {
  const [autoFired, setAutoFired] = useState(false);
  const [typed, setTyped] = useState(null); // null = not hovering
  const hoverRef = useRef(false);
  const typeTimer = useRef(null);
  const reduce = useReducedMotion();
  const off = STATIC || reduce;

  useEffect(() => {
    if (off) return;
    let collapseT, interval;
    const fire = () => {
      if (hoverRef.current) return;
      setAutoFired(true);
      collapseT = setTimeout(() => setAutoFired(false), CHIP_EXPANDED_MS);
    };
    const startT = setTimeout(() => {
      fire();
      interval = setInterval(fire, CHIP_CYCLE_MS);
    }, 1400 + chip.offset * 1000);
    return () => {
      clearTimeout(startT);
      clearTimeout(collapseT);
      clearInterval(interval);
    };
  }, [off, chip.offset]);

  // Hover: the trigger "types out" its value letter by letter, like a real
  // expansion happening in front of you.
  const startTyping = () => {
    hoverRef.current = true;
    setAutoFired(false);
    clearInterval(typeTimer.current);
    let i = 0;
    setTyped("");
    typeTimer.current = setInterval(() => {
      i += 1;
      setTyped(chip.expansion.slice(0, i));
      if (i >= chip.expansion.length) clearInterval(typeTimer.current);
    }, TYPE_SPEED_MS);
  };
  const stopTyping = () => {
    hoverRef.current = false;
    clearInterval(typeTimer.current);
    setTyped(null);
  };
  useEffect(() => () => clearInterval(typeTimer.current), []);

  const showing = typed !== null ? "typing" : autoFired ? "expansion" : "trigger";
  const text = typed !== null ? typed : autoFired ? chip.expansion : chip.trigger;
  const expanded = showing !== "trigger";

  return (
    <motion.span
      className="chip-spot"
      style={chip.pos}
      animate={off ? undefined : { y: [0, -8, 0] }}
      transition={{
        duration: 6 + chip.offset * 0.4,
        repeat: Infinity,
        ease: "easeInOut",
      }}
      onPointerEnter={off ? undefined : startTyping}
      onPointerLeave={off ? undefined : stopTyping}
    >
      <motion.span
        layout
        className={"float-chip" + (expanded ? " expanded" : "")}
        animate={off ? undefined : { scale: showing === "expansion" ? [1, 0.9, 1] : 1 }}
        whileHover={off ? undefined : { scale: 1.05 }}
        transition={{ duration: 0.35, ease: EASE }}
      >
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.span
            key={showing}
            className="chip-text"
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
            transition={{ duration: 0.22, ease: EASE }}
          >
            {text}
            {showing === "typing" && <span className="caret" />}
          </motion.span>
        </AnimatePresence>
      </motion.span>
    </motion.span>
  );
}

function ExpanderChips() {
  return (
    <div className="bg-blobs" aria-hidden="true">
      {EXPANDER_CHIPS.map((chip) => (
        <ExpanderChip key={chip.trigger} chip={chip} />
      ))}
    </div>
  );
}

/* ---------- Scroll reveal ---------- */

function Reveal({ children, delay = 0, className }) {
  if (STATIC) {
    return <div className={className}>{children}</div>;
  }
  return (
    <motion.div
      className={className}
      initial={{ opacity: 0, y: 28 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "-60px" }}
      transition={{ duration: 0.6, ease: EASE, delay }}
    >
      {children}
    </motion.div>
  );
}

/* ---------- Mock: search palette ---------- */

function PaletteMock() {
  return (
    <div className="mock" role="img" aria-label="Carimbo search palette showing snippet results for the query cpf">
      <div className="mock-titlebar">
        <Ic icon={Stamp} size={14} />
        Carimbo · Quick search
        <div className="mock-dots">
          <span />
          <span />
        </div>
      </div>
      <div className="mock-search">
        <Ic icon={Search} size={16} />
        <span>
          cpf<span className="caret" />
        </span>
      </div>
      <div className="mock-group">Favorites</div>
      <div className="mock-row active">
        <Ic icon={Star} size={14} className="star" fill="currentColor" />
        <span className="trigger-chip">;cpf</span>
        <span className="row-name">CPF, formatted</span>
        <span className="row-preview">123.456.789-01</span>
      </div>
      <div className="mock-row">
        <Ic icon={Star} size={14} className="star" fill="currentColor" />
        <span className="trigger-chip">;sig</span>
        <span className="row-name">Email signature</span>
        <span className="row-preview">Best regards, Maria…</span>
      </div>
      <div className="mock-group">Others</div>
      <div className="mock-row">
        <span className="trigger-chip">;addr</span>
        <span className="row-name">Office address</span>
        <span className="row-preview">742 Evergreen Terrace…</span>
      </div>
      <div className="mock-footer">
        <span>
          <kbd>↑↓</kbd>Navigate
        </span>
        <span>
          <kbd>↵</kbd>Insert
        </span>
        <span>
          <kbd>Tab</kbd>Clipboard
        </span>
        <span>
          <kbd>Esc</kbd>Close
        </span>
      </div>
    </div>
  );
}

/* ---------- Mock: fill-in form — a live, playable demo.
   Autopilot types a client name, presses Insert, and the template resolves
   ({date+7d} → a real date, {uuid} → a fresh id) in a loop. The moment the
   visitor focuses or types, autopilot stops and it's their demo: type any
   name, Insert resolves it, Cancel hands control back to autopilot. ---------- */

const FORM_AUTO_NAME = "John Doe";

function formDatePlus7() {
  const d = new Date(Date.now() + 7 * 86400000);
  return d.toLocaleDateString("en-US", {
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

function formUuid() {
  const id =
    typeof crypto !== "undefined" && crypto.randomUUID
      ? crypto.randomUUID()
      : "8f3a1c9e-52b7";
  return `${id.slice(0, 13)}…`;
}

function FormMock() {
  const reduced = useReducedMotion();
  const settled = STATIC || reduced;
  const [value, setValue] = useState(settled ? FORM_AUTO_NAME : "");
  // auto: autopilot runs · user: visitor owns the input · fired: showing the
  // resolved insert before returning to `from`
  const [mode, setMode] = useState(settled ? "user" : "auto");
  const [resolved, setResolved] = useState(null);
  const fromRef = useRef("auto");
  const rootRef = useRef(null);
  const visibleRef = useRef(true);

  const fire = (name, from) => {
    fromRef.current = from;
    setResolved({
      name: name.trim() || "Client name",
      date: formDatePlus7(),
      uuid: formUuid(),
    });
    setMode("fired");
  };

  useEffect(() => {
    if (STATIC) return undefined;
    const io = new IntersectionObserver(([entry]) => {
      visibleRef.current = entry.isIntersecting;
    });
    io.observe(rootRef.current);
    return () => io.disconnect();
  }, []);

  // autopilot typing loop
  useEffect(() => {
    if (settled || mode !== "auto") return undefined;
    let i = 0;
    let timer;
    const tick = () => {
      if (!visibleRef.current) {
        timer = setTimeout(tick, 500);
        return;
      }
      if (i < FORM_AUTO_NAME.length) {
        i += 1;
        setValue(FORM_AUTO_NAME.slice(0, i));
        timer = setTimeout(tick, 75 + (i % 3) * 45);
      } else {
        fire(FORM_AUTO_NAME, "auto");
      }
    };
    timer = setTimeout(tick, 900);
    return () => clearTimeout(timer);
  }, [mode, settled]);

  // linger on the resolved text, then hand control back
  useEffect(() => {
    if (mode !== "fired") return undefined;
    const timer = setTimeout(() => {
      setResolved(null);
      if (fromRef.current === "auto") {
        setValue("");
        setMode("auto");
      } else {
        setMode("user");
      }
    }, 2600);
    return () => clearTimeout(timer);
  }, [mode]);

  const takeOver = () => {
    if (mode === "auto") setMode("user");
  };

  const onChange = (e) => {
    setValue(e.target.value);
    if (mode !== "user") {
      setResolved(null);
      setMode("user");
    }
  };

  const fired = mode === "fired" && resolved;

  return (
    <div className="mock" ref={rootRef}>
      <div className="mock-titlebar">
        <Ic icon={Stamp} size={14} />
        Follow-up email · fill in
        <span className="mock-live">live demo</span>
        <div className="mock-dots">
          <span />
          <span />
        </div>
      </div>
      {fired ? (
        <div className="snippet-body fired" key="fired">
          Hi <span className="tok resolved">{resolved.name}</span>, thanks for
          your time today!{"\n"}
          I'll send the proposal by{" "}
          <span className="tok resolved">{resolved.date}</span>.{"\n"}Ref:{" "}
          <span className="tok resolved">{resolved.uuid}</span>
        </div>
      ) : (
        <div className="snippet-body">
          Hi{" "}
          <span className="tok">
            {value.trim() ? value : "[[name:Client name]]"}
          </span>
          , thanks for your time today!{"\n"}
          I'll send the proposal by <span className="tok">{"{date+7d}"}</span>.
          {"\n"}Ref: <span className="tok">{"{uuid}"}</span>
        </div>
      )}
      <div className="form-fields">
        <div className="form-field">
          <label htmlFor="form-demo-name">Client name</label>
          <input
            id="form-demo-name"
            className={`input${mode === "auto" ? " focused" : ""}`}
            value={value}
            placeholder="Type any name…"
            onChange={onChange}
            onFocus={takeOver}
            autoComplete="off"
            spellCheck="false"
          />
        </div>
      </div>
      <div className="form-actions">
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          onClick={() => {
            setResolved(null);
            setValue("");
            setMode(settled ? "user" : "auto");
          }}
        >
          Cancel
        </button>
        <button
          type="button"
          className="btn btn-primary btn-sm"
          onClick={() => fire(value, "user")}
          disabled={mode === "fired"}
        >
          {fired ? (
            <>
              <Ic icon={Check} size={15} />
              Inserted
            </>
          ) : (
            "Insert"
          )}
        </button>
      </div>
    </div>
  );
}

/* ---------- Mock: color picker — a live, playable demo.
   A tiny "screen" (an abstract wallpaper rendered once to canvas) with a
   magnifier loupe that samples it: drifts on its own when idle, follows the
   cursor on hover, arrow keys nudge it, click/Enter copies the hex. All
   per-frame updates are imperative (refs, one rAF loop) so React renders the
   component exactly once. Sampling recomputes the same color function the
   canvas was painted with, so loupe pixels always match the wallpaper. ---------- */

const PICKER_W = 320;
const PICKER_H = 200;
const PICKER_STEP = 3; // wallpaper px per loupe cell (the "zoom")
const PICKER_DEFAULT = { x: 198, y: 76 }; // center of the ink blob → samples ~#4977AB

// Soft gaussian color blobs — brand ink plus friendly companions.
const PICKER_BLOBS = [
  { x: 0.62, y: 0.38, r: 0.16, c: [73, 119, 171], a: 2.6 }, // ink accent
  { x: 0.14, y: 0.18, r: 0.28, c: [58, 168, 160], a: 1 }, // teal
  { x: 0.86, y: 0.12, r: 0.24, c: [123, 91, 214], a: 1 }, // violet
  { x: 0.22, y: 0.86, r: 0.3, c: [224, 122, 90], a: 1 }, // coral
  { x: 0.9, y: 0.82, r: 0.26, c: [226, 178, 92], a: 1 }, // amber
  { x: 0.5, y: 0.62, r: 0.5, c: [40, 52, 72], a: 0.35 }, // deep wash
];

// Deterministic per-pixel grain so the loupe reads as real screen pixels.
function pickerNoise(ix, iy) {
  let h = (ix * 374761393 + iy * 668265263) | 0;
  h = ((h ^ (h >>> 13)) * 1274126177) | 0;
  h ^= h >>> 16;
  return ((h >>> 0) % 13) - 6;
}

function pickerColorAt(ix, iy) {
  const nx = ix / PICKER_W;
  const ny = iy / PICKER_H;
  let r = 0;
  let g = 0;
  let b = 0;
  let w = 0.02; // neutral backstop so far corners never divide by ~0
  r += w * 92;
  g += w * 106;
  b += w * 132;
  for (const bl of PICKER_BLOBS) {
    const dx = nx - bl.x;
    const dy = ny - bl.y;
    const wi = bl.a * Math.exp(-(dx * dx + dy * dy) / (2 * bl.r * bl.r));
    r += wi * bl.c[0];
    g += wi * bl.c[1];
    b += wi * bl.c[2];
    w += wi;
  }
  const n = pickerNoise(ix, iy);
  const clamp = (v) => Math.max(0, Math.min(255, Math.round(v / w + n)));
  return [clamp(r), clamp(g), clamp(b)];
}

function pickerSample(x, y) {
  const cx = Math.max(0, Math.min(PICKER_W - 1, Math.round(x)));
  const cy = Math.max(0, Math.min(PICKER_H - 1, Math.round(y)));
  const cells = [];
  for (let row = 0; row < 7; row++) {
    for (let col = 0; col < 7; col++) {
      const ix = Math.max(
        0,
        Math.min(PICKER_W - 1, cx + (col - 3) * PICKER_STEP),
      );
      const iy = Math.max(
        0,
        Math.min(PICKER_H - 1, cy + (row - 3) * PICKER_STEP),
      );
      cells.push(`rgb(${pickerColorAt(ix, iy).join(" ")})`);
    }
  }
  const [r, g, b] = pickerColorAt(cx, cy);
  const hex = `#${[r, g, b]
    .map((v) => v.toString(16).padStart(2, "0"))
    .join("")}`.toUpperCase();
  const tints = [-40, -25, -12, 0, 14, 30, 48].map((d) => {
    const t = [r, g, b].map((v) => Math.max(0, Math.min(255, v + d)));
    return `rgb(${t.join(" ")})`;
  });
  return { cells, hex, rgb: [r, g, b], tints, hsl: pickerHsl(r, g, b) };
}

function pickerHsl(r, g, b) {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  const l = (max + min) / 2;
  let h = 0;
  let s = 0;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    h =
      max === rn
        ? (gn - bn) / d + (gn < bn ? 6 : 0)
        : max === gn
          ? (bn - rn) / d + 2
          : (rn - gn) / d + 4;
    h *= 60;
  }
  return `hsl(${Math.round(h)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;
}

function drawPickerWallpaper(canvas) {
  const dpr = Math.min(2, Math.ceil(window.devicePixelRatio || 1));
  canvas.width = PICKER_W * dpr;
  canvas.height = PICKER_H * dpr;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  const img = ctx.createImageData(canvas.width, canvas.height);
  // compute each wallpaper pixel once, fill its dpr×dpr block
  for (let iy = 0; iy < PICKER_H; iy++) {
    for (let ix = 0; ix < PICKER_W; ix++) {
      const [r, g, b] = pickerColorAt(ix, iy);
      for (let sy = 0; sy < dpr; sy++) {
        const rowBase = ((iy * dpr + sy) * canvas.width + ix * dpr) * 4;
        for (let sx = 0; sx < dpr; sx++) {
          const i = rowBase + sx * 4;
          img.data[i] = r;
          img.data[i + 1] = g;
          img.data[i + 2] = b;
          img.data[i + 3] = 255;
        }
      }
    }
  }
  ctx.putImageData(img, 0, 0);
}

function PickerMock() {
  const reduced = useReducedMotion();
  const screenRef = useRef(null);
  const canvasRef = useRef(null);
  const loupeRef = useRef(null);
  const cellRefs = useRef([]);
  const swatchRefs = useRef([]);
  const tintRefs = useRef([]);
  const hexRef = useRef(null);
  const rgbRef = useRef(null);
  const hslRef = useRef(null);
  const copyRef = useRef(null);
  const hintRef = useRef(null);
  const pingRef = useRef(null);
  const st = useRef({
    x: PICKER_DEFAULT.x,
    y: PICKER_DEFAULT.y,
    tx: PICKER_DEFAULT.x,
    ty: PICKER_DEFAULT.y,
    pointer: false,
    visible: true,
    t: 0,
    hex: "",
    lastX: -1,
    lastY: -1,
    copyTimer: 0,
  });

  const init = pickerSample(PICKER_DEFAULT.x, PICKER_DEFAULT.y);
  st.current.hex = init.hex;

  useEffect(() => {
    drawPickerWallpaper(canvasRef.current);
  }, []);

  useEffect(() => {
    if (STATIC) return undefined;
    const io = new IntersectionObserver(([entry]) => {
      st.current.visible = entry.isIntersecting;
    });
    io.observe(screenRef.current);
    return () => io.disconnect();
  }, []);

  useEffect(() => {
    if (STATIC) return undefined;
    let raf;
    let last = performance.now();
    const frame = (now) => {
      raf = requestAnimationFrame(frame);
      const s = st.current;
      const dt = Math.min(50, now - last);
      last = now;
      if (!s.visible) return;
      if (!s.pointer && !reduced) {
        // lazy figure-eight drift while nobody's playing with it
        s.t += dt / 1000;
        s.tx = PICKER_W * (0.5 + 0.34 * Math.sin(s.t * 0.5));
        s.ty = PICKER_H * (0.5 + 0.32 * Math.sin(s.t * 0.8 + 1.9));
      }
      const k = reduced ? 1 : 0.16;
      s.x += (s.tx - s.x) * k;
      s.y += (s.ty - s.y) * k;
      const rx = Math.round(s.x);
      const ry = Math.round(s.y);
      if (rx === s.lastX && ry === s.lastY) return;
      s.lastX = rx;
      s.lastY = ry;
      const sample = pickerSample(s.x, s.y);
      s.hex = sample.hex;
      const loupe = loupeRef.current;
      if (loupe) {
        loupe.style.left = `${(s.x / PICKER_W) * 100}%`;
        loupe.style.top = `${(s.y / PICKER_H) * 100}%`;
      }
      cellRefs.current.forEach((el, i) => {
        if (el) el.style.background = sample.cells[i];
      });
      swatchRefs.current.forEach((el) => {
        if (el) el.style.background = sample.hex;
      });
      tintRefs.current.forEach((el, i) => {
        if (el) el.style.background = sample.tints[i];
      });
      if (hexRef.current) hexRef.current.textContent = sample.hex;
      if (rgbRef.current)
        rgbRef.current.textContent = `rgb(${sample.rgb.join(", ")})`;
      if (hslRef.current) hslRef.current.textContent = sample.hsl;
    };
    raf = requestAnimationFrame(frame);
    return () => cancelAnimationFrame(raf);
  }, [reduced]);

  const pointTo = (e) => {
    const rect = screenRef.current.getBoundingClientRect();
    const s = st.current;
    s.pointer = true;
    s.tx = Math.max(
      0,
      Math.min(PICKER_W, ((e.clientX - rect.left) / rect.width) * PICKER_W),
    );
    s.ty = Math.max(
      0,
      Math.min(PICKER_H, ((e.clientY - rect.top) / rect.height) * PICKER_H),
    );
    hintRef.current?.classList.add("hide");
  };

  const copyHex = (e) => {
    const { hex } = st.current;
    try {
      navigator.clipboard?.writeText(hex);
    } catch {
      /* clipboard unavailable — the flash still shows the value */
    }
    const badge = copyRef.current;
    if (badge) {
      badge.textContent = `${hex} copied!`;
      badge.classList.add("copied");
      clearTimeout(st.current.copyTimer);
      st.current.copyTimer = setTimeout(() => {
        badge.textContent = "click to copy";
        badge.classList.remove("copied");
      }, 1400);
    }
    const ping = pingRef.current;
    if (ping && e && screenRef.current.contains(e.target)) {
      const rect = screenRef.current.getBoundingClientRect();
      ping.style.left = `${e.clientX - rect.left}px`;
      ping.style.top = `${e.clientY - rect.top}px`;
      ping.classList.remove("go");
      void ping.offsetWidth; // restart the animation
      ping.classList.add("go");
    }
  };

  const onKeyDown = (e) => {
    if (STATIC) return;
    const s = st.current;
    const nudge = 8;
    const moves = {
      ArrowLeft: [-nudge, 0],
      ArrowRight: [nudge, 0],
      ArrowUp: [0, -nudge],
      ArrowDown: [0, nudge],
    };
    if (moves[e.key]) {
      e.preventDefault();
      s.pointer = true;
      s.tx = Math.max(0, Math.min(PICKER_W, s.tx + moves[e.key][0]));
      s.ty = Math.max(0, Math.min(PICKER_H, s.ty + moves[e.key][1]));
      hintRef.current?.classList.add("hide");
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      copyHex();
    }
  };

  const release = () => {
    st.current.pointer = false;
  };

  return (
    <div className="mock">
      <div className="mock-titlebar">
        <Ic icon={Stamp} size={14} />
        Color picker
        <span className="mock-live">live demo</span>
        <div className="mock-dots">
          <span />
          <span />
        </div>
      </div>
      <div className="picker-stage">
        <div
          className="picker-screen"
          ref={screenRef}
          tabIndex={0}
          role="application"
          aria-label="Color picking demo. Move the cursor or use arrow keys to sample a color; press Enter or click to copy its hex code."
          onPointerMove={STATIC ? undefined : pointTo}
          onPointerDown={STATIC ? undefined : pointTo}
          onPointerLeave={STATIC ? undefined : release}
          onBlur={STATIC ? undefined : release}
          onKeyDown={onKeyDown}
          onClick={STATIC ? undefined : copyHex}
        >
          <canvas
            ref={canvasRef}
            width={PICKER_W}
            height={PICKER_H}
            aria-hidden="true"
          />
          <div
            className="picker-loupe"
            ref={loupeRef}
            style={{
              left: `${(PICKER_DEFAULT.x / PICKER_W) * 100}%`,
              top: `${(PICKER_DEFAULT.y / PICKER_H) * 100}%`,
            }}
            aria-hidden="true"
          >
            <div className="loupe-cells">
              {init.cells.map((c, i) => (
                <span
                  // eslint-disable-next-line react/no-array-index-key
                  key={i}
                  ref={(el) => {
                    cellRefs.current[i] = el;
                  }}
                  style={{ background: c }}
                />
              ))}
            </div>
            <span className="loupe-center" />
          </div>
          <span className="picker-ping" ref={pingRef} aria-hidden="true" />
          <span className="picker-hint" ref={hintRef} aria-hidden="true">
            move your cursor · click to copy
          </span>
        </div>
        <div className="picker-readout">
          <button
            type="button"
            className="color-chip chip-button"
            onClick={() => copyHex()}
            aria-label="Copy hex color"
          >
            <span
              className="swatch"
              ref={(el) => {
                swatchRefs.current[0] = el;
              }}
              style={{ background: init.hex }}
            />
            <span className="chip-val" ref={hexRef}>
              {init.hex}
            </span>
            <span className="copy-hint" ref={copyRef}>
              click to copy
            </span>
          </button>
          <div className="color-chip">
            <span
              className="swatch"
              ref={(el) => {
                swatchRefs.current[1] = el;
              }}
              style={{ background: init.hex }}
            />
            <span className="chip-val" ref={rgbRef}>
              {`rgb(${init.rgb.join(", ")})`}
            </span>
          </div>
          <div className="color-chip">
            <span
              className="swatch"
              ref={(el) => {
                swatchRefs.current[2] = el;
              }}
              style={{ background: init.hex }}
            />
            <span className="chip-val" ref={hslRef}>
              {init.hsl}
            </span>
          </div>
          <div className="tint-strip">
            {init.tints.map((t, i) => (
              <span
                // eslint-disable-next-line react/no-array-index-key
                key={i}
                ref={(el) => {
                  tintRefs.current[i] = el;
                }}
                style={{ background: t }}
              />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

/* ---------- Mock: clipboard history — a live, playable demo.
   New clips "arrive" at the top on their own (pausing while hovered,
   searching, or offscreen). The search box really filters with match
   highlighting, clicking a row "pastes" it (flash + ✓), and the pin button
   actually pins: the row floats to the top and stays through arrivals. ---------- */

const CLIP_AGES = [
  "now",
  "2m",
  "14m",
  "1h",
  "3h",
  "5h",
  "yesterday",
  "2d",
  "4d",
  "1w",
  "2w",
  "3w",
];

const CLIP_START = [
  { id: 1, type: "url", text: "https://carimbo.app/docs/tokens", pinned: true },
  { id: 2, type: "color", text: "#4977AB", swatch: "#4977ab", pinned: false },
  { id: 3, type: "image", text: "Screenshot · 1440×900", pinned: false },
  {
    id: 4,
    type: "text",
    text: "Quarterly numbers are in: revenue up 12% and churn down…",
    pinned: false,
  },
  {
    id: 5,
    type: "path",
    text: "C:\\Projects\\proposal-final-v3.docx",
    pinned: false,
  },
  {
    id: 6,
    type: "text",
    text: "Thanks for the intro! Looping in our design team.",
    pinned: false,
  },
  { id: 7, type: "url", text: "https://figma.com/file/onboarding-v2", pinned: false },
  { id: 8, type: "color", text: "#E4B35C", swatch: "#e4b35c", pinned: false },
  {
    id: 9,
    type: "path",
    text: "C:\\Users\\maria\\Pictures\\team-offsite.jpg",
    pinned: false,
  },
  { id: 10, type: "image", text: "Screenshot · 2560×1440", pinned: false },
  {
    id: 11,
    type: "text",
    text: "Package tracking: BR-4402-1187-XZ",
    pinned: false,
  },
];

const CLIP_POOL = [
  { type: "text", text: "Meeting moved to Thursday 3pm — can you confirm?" },
  { type: "color", text: "#E07A5A", swatch: "#e07a5a" },
  { type: "url", text: "https://github.com/carimbo-app" },
  { type: "path", text: "C:\\Users\\maria\\Downloads\\invoice-0142.pdf" },
  { type: "image", text: "Screenshot · 1920×1080" },
  { type: "text", text: "ana.lima@acme.com" },
  { type: "color", text: "#3AA8A0", swatch: "#3aa8a0" },
  { type: "text", text: "Can we push the launch to next Friday instead?" },
  { type: "url", text: "https://linear.app/carimbo/issue/CAR-214" },
  { type: "text", text: "Sprint review notes — action items inside" },
  { type: "color", text: "#7B5BD6", swatch: "#7b5bd6" },
  { type: "path", text: "D:\\Design\\logo-final-FINAL-v7.svg" },
  { type: "image", text: "Screenshot · 1366×768" },
];

const CLIP_MONO = new Set(["url", "color", "path"]);

function clipHighlight(text, q) {
  if (!q) return text;
  const i = text.toLowerCase().indexOf(q);
  if (i === -1) return text;
  return (
    <>
      {text.slice(0, i)}
      <mark className="clip-mark">{text.slice(i, i + q.length)}</mark>
      {text.slice(i + q.length)}
    </>
  );
}

function ClipboardMock() {
  const reduced = useReducedMotion();
  const settled = STATIC || reduced;
  const [clips, setClips] = useState(CLIP_START);
  const [query, setQuery] = useState("");
  const [pastedId, setPastedId] = useState(null);
  const rootRef = useRef(null);
  const visibleRef = useRef(true);
  const hoverRef = useRef(false);
  const poolIdx = useRef(0);
  const nextId = useRef(100);
  const pasteTimer = useRef(0);

  useEffect(() => {
    if (STATIC) return undefined;
    const io = new IntersectionObserver(([entry]) => {
      visibleRef.current = entry.isIntersecting;
    });
    io.observe(rootRef.current);
    return () => io.disconnect();
  }, []);

  // autopilot: a fresh clip arrives every few seconds
  useEffect(() => {
    if (settled || query) return undefined;
    const timer = setInterval(() => {
      if (!visibleRef.current || hoverRef.current) return;
      setClips((cur) => {
        const item = CLIP_POOL[poolIdx.current % CLIP_POOL.length];
        poolIdx.current += 1;
        // re-copying something you already have doesn't duplicate it — the
        // existing entry moves back to the top (unless it's pinned in place)
        if (cur.some((c) => c.pinned && c.text === item.text)) return cur;
        const pinned = cur.filter((c) => c.pinned);
        const existing = cur.find((c) => !c.pinned && c.text === item.text);
        if (existing) {
          const rest = cur.filter((c) => !c.pinned && c.id !== existing.id);
          return [...pinned, existing, ...rest];
        }
        nextId.current += 1;
        const arrived = { ...item, id: nextId.current, pinned: false };
        const rest = cur.filter((c) => !c.pinned);
        // history accumulates — that's the point — but keep a sane ceiling
        return [...pinned, arrived, ...rest.slice(0, 23)];
      });
    }, 3400);
    return () => clearInterval(timer);
  }, [settled, query]);

  useEffect(() => () => clearTimeout(pasteTimer.current), []);

  const paste = (id) => {
    setPastedId(id);
    clearTimeout(pasteTimer.current);
    pasteTimer.current = setTimeout(() => setPastedId(null), 1100);
  };

  const togglePin = (id) => {
    setClips((cur) => {
      const updated = cur.map((c) =>
        c.id === id ? { ...c, pinned: !c.pinned } : c,
      );
      return [
        ...updated.filter((c) => c.pinned),
        ...updated.filter((c) => !c.pinned),
      ];
    });
  };

  const q = query.trim().toLowerCase();
  const shown = clips.filter(
    (c) => !q || c.text.toLowerCase().includes(q) || c.type.includes(q),
  );
  let unpinnedIdx = 0;

  return (
    <div
      className="mock"
      ref={rootRef}
      onMouseEnter={() => {
        hoverRef.current = true;
      }}
      onMouseLeave={() => {
        hoverRef.current = false;
      }}
    >
      <div className="mock-titlebar">
        <Ic icon={Stamp} size={14} />
        Clipboard history
        <span className="mock-live">live demo</span>
        <div className="mock-dots">
          <span />
          <span />
        </div>
      </div>
      <div className="mock-search">
        <Ic icon={Search} size={16} />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search clips…"
          aria-label="Search clips"
          autoComplete="off"
          spellCheck="false"
        />
      </div>
      <div className="clip-list">
        <AnimatePresence initial={false} mode="popLayout">
          {shown.map((c) => {
            const age = c.pinned
              ? null
              : CLIP_AGES[Math.min(unpinnedIdx++, CLIP_AGES.length - 1)];
            return (
              <motion.div
                key={c.id}
                layout={!settled}
                initial={settled ? false : { opacity: 0, y: -16, scale: 0.97 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={settled ? undefined : { opacity: 0, scale: 0.94 }}
                transition={{ duration: 0.32, ease: EASE }}
                className={`clip-row${c.pinned ? " pinned" : ""}${
                  pastedId === c.id ? " flash" : ""
                }`}
                role="button"
                tabIndex={0}
                aria-label={`Paste ${c.type} clip: ${c.text}`}
                onClick={() => paste(c.id)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    paste(c.id);
                  }
                }}
              >
                <span
                  className={`type-badge${
                    c.type === "url" || c.type === "color" ? ` ${c.type}` : ""
                  }`}
                >
                  {c.type}
                </span>
                {c.type === "color" && (
                  <span className="clip-swatch" style={{ background: c.swatch }} />
                )}
                {c.type === "image" && <span className="clip-thumb" />}
                <span className={`clip-text${CLIP_MONO.has(c.type) ? " mono" : ""}`}>
                  {clipHighlight(c.text, q)}
                </span>
                {pastedId === c.id ? (
                  <span className="age pasted">
                    <Ic icon={Check} size={13} />
                    pasted
                  </span>
                ) : (
                  <>
                    <button
                      type="button"
                      className={`pin-btn${c.pinned ? " active" : ""}`}
                      aria-label={c.pinned ? "Unpin clip" : "Pin clip"}
                      onClick={(e) => {
                        e.stopPropagation();
                        togglePin(c.id);
                      }}
                    >
                      <Ic icon={Pin} size={14} />
                    </button>
                    {age && <span className="age">{age}</span>}
                  </>
                )}
              </motion.div>
            );
          })}
        </AnimatePresence>
        {shown.length === 0 && (
          <div className="clip-empty">No clips match “{query.trim()}”</div>
        )}
      </div>
      <div className="mock-footer">
        <span>
          <kbd>↵</kbd>Paste
        </span>
        <span>
          <kbd>Ctrl+P</kbd>Pin
        </span>
        <span>Paste as… UPPERCASE · slug · Base64</span>
      </div>
    </div>
  );
}

/* ---------- Page sections ---------- */

function Nav({ theme, onToggleTheme }) {
  return (
    <header className="nav">
      <div className="nav-inner">
        <a className="brand" href="#">
          <Ic icon={Stamp} size={22} />
          Carimbo
        </a>
        <nav className="nav-links" aria-label="Main">
          <a href="#features">Features</a>
          <a href="#more">Everything else</a>
          <button
            type="button"
            className="theme-toggle"
            onClick={onToggleTheme}
            aria-label={theme === "light" ? "Switch to dark theme" : "Switch to light theme"}
            title={theme === "light" ? "Dark theme" : "Light theme"}
          >
            <Ic icon={theme === "light" ? Moon : Sun} size={18} />
          </button>
          <a className="btn btn-primary btn-sm" href="#download">
            Download
          </a>
        </nav>
      </div>
    </header>
  );
}

const heroItem = {
  hidden: { opacity: 0, y: 24 },
  show: { opacity: 1, y: 0, transition: { duration: 0.6, ease: EASE } },
};

function Hero({ theme }) {
  return (
    <section className="hero">
      <ShaderBackground theme={theme} />
      <ExpanderChips />
      <motion.div
        className="container hero-grid"
        initial={STATIC ? "show" : "hidden"}
        animate="show"
        variants={{ show: { transition: { staggerChildren: 0.09 } } }}
      >
        <div>
          <motion.h1 variants={heroItem}>
            Stop retyping. Start <em>thinking</em>.
          </motion.h1>
          <motion.p className="lede" variants={heroItem}>
            You waste minutes every day retyping the same text, digging for
            things you copied, and hunting down details. Carimbo hands them
            back to you in a keystroke, so your energy goes to the work that
            actually needs your judgment.
          </motion.p>
          <motion.div className="hero-ctas" variants={heroItem}>
            <a className="btn btn-primary btn-lg" href={DOWNLOAD_URL}>
              <Ic icon={DownloadIcon} size={18} />
              Download for Windows
            </a>
            <a className="btn btn-secondary btn-lg" href="#features">
              See how it helps
            </a>
          </motion.div>
          <motion.p className="hero-note" variants={heroItem}>
            Free · Windows 10 and 11 · Bring it up anywhere with{" "}
            <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>Space</kbd>
          </motion.p>
        </div>
        <motion.div variants={heroItem}>
          <PaletteMock />
        </motion.div>
      </motion.div>
    </section>
  );
}

function Features() {
  return (
    <section className="features" id="features">
      <div className="container">
        <Reveal className="section-head">
          <h2>Three everyday chores, handled for you</h2>
          <p>
            One quiet app that waits in the background and gives you back the
            minutes you'd otherwise spend on busywork.
          </p>
        </Reveal>

        <div className="feature">
          <Reveal className="feature-copy">
            <span className="feature-kicker">
              <Ic icon={Keyboard} size={15} />
              Ready-made text
            </span>
            <h3>Your go-to phrases, ready in a keystroke</h3>
            <p>
              Save the emails, replies, and details you write again and again.
              Type a short shortcut and the full text appears wherever you're
              working. No copy, no paste, no searching your old messages.
            </p>
            <ul>
              <li>Turn a whole paragraph into a two-letter shortcut</li>
              <li>
                Leave blanks for the parts that change, and Carimbo asks you to
                fill them in as it types
              </li>
              <li>Drop in today's date, next week's date, or a fresh ID for you</li>
              <li>Keeps its formatting in Outlook and Gmail</li>
              <li>Finds your snippets even if you skip the accents</li>
            </ul>
          </Reveal>
          <Reveal delay={0.12}>
            <FormMock />
          </Reveal>
        </div>

        <div className="feature">
          <Reveal className="feature-copy">
            <span className="feature-kicker">
              <Ic icon={Pipette} size={15} />
              Color picker
            </span>
            <h3>Grab any color on your screen</h3>
            <p>
              See a color you like anywhere on screen? Bring up the picker, zoom
              in, and click. The exact color is copied and ready to use, without
              opening a design app or juggling windows.
            </p>
            <ul>
              <li>Zoom in close so you catch the precise spot every time</li>
              <li>Copy the color in whatever format your tools expect</li>
              <li>Get matching lighter and darker shades in one look</li>
              <li>Every color you pick is saved for you to reuse later</li>
            </ul>
          </Reveal>
          <Reveal delay={0.12}>
            <PickerMock />
          </Reveal>
        </div>

        <div className="feature">
          <Reveal className="feature-copy">
            <span className="feature-kicker">
              <Ic icon={ClipboardList} size={15} />
              Clipboard history
            </span>
            <h3>Everything you copied, still here</h3>
            <p>
              Windows forgets what you copied the moment you copy something
              else. Carimbo keeps it all, text and images, so you can copy a
              bunch of things at once and paste them one by one when you need
              them.
            </p>
            <ul>
              <li>Search back through anything you've copied recently</li>
              <li>Skips your passwords, so nothing private is kept</li>
              <li>Pin the things you reach for often so they stay put</li>
              <li>Save any clip as a shortcut you can reuse forever</li>
            </ul>
          </Reveal>
          <Reveal delay={0.12}>
            <ClipboardMock />
          </Reveal>
        </div>
      </div>
    </section>
  );
}

const MORE = [
  {
    icon: ShieldCheck,
    title: "Stays out of the way",
    text: "Tell it to leave certain apps alone, like your password manager, and it won't interfere.",
  },
  {
    icon: CaseUpper,
    title: "Tidy up as you paste",
    text: "Fix the casing, trim extra spaces, or clean up messy text automatically as it goes in.",
  },
  {
    icon: FileText,
    title: "Keeps your formatting",
    text: "Bold, links, and layout come through in Outlook, Word, and Gmail, plain text everywhere else.",
  },
  {
    icon: Power,
    title: "Always there when you need it",
    text: "Opens with Windows and waits quietly in the tray until you call on it.",
  },
  {
    icon: Contrast,
    title: "Easy on your eyes",
    text: "Light, dark, and high-contrast looks, with text-size and spacing you can adjust.",
  },
  {
    icon: Globe,
    title: "English and Português",
    text: "The whole app in English or Brazilian Portuguese, with dates in your local format.",
  },
  {
    icon: ArchiveRestore,
    title: "Never lose your work",
    text: "Back up your snippets, move them to a new computer, or bring in what you had before.",
  },
  {
    icon: Usb,
    title: "Take it with you",
    text: "Run it straight from a USB stick and keep all your snippets right alongside it.",
  },
];

function More() {
  return (
    <section className="more" id="more">
      <div className="container">
        <Reveal className="section-head">
          <h2>All the little things, thought through</h2>
          <p>
            The small touches that make a workday smoother, because they're the
            ones you run into every single day.
          </p>
        </Reveal>
        <div className="more-grid">
          {MORE.map((item, i) => (
            <Reveal className="more-card" key={item.title} delay={(i % 4) * 0.06}>
              <Ic icon={item.icon} />
              <h4>{item.title}</h4>
              <p>{item.text}</p>
            </Reveal>
          ))}
        </div>
      </div>
    </section>
  );
}

function DownloadSection() {
  return (
    <section className="download" id="download">
      <div className="container">
        <Reveal>
          <div className="download-card">
            <Blobs blobs={DOWNLOAD_BLOBS} />
            <h2>Spend your time on what matters.</h2>
            <p>
              Carimbo sets up in seconds, no admin permission and no fuss, then
              waits quietly in your tray. Everything you save stays on your own
              computer.
            </p>
            <div className="download-ctas">
              <a className="btn btn-primary btn-lg" href={DOWNLOAD_URL}>
                <Ic icon={DownloadIcon} size={18} />
                Download for Windows, Free
              </a>
            </div>
            <div className="download-meta">
              <span>
                <Ic icon={Check} size={15} />
                Windows 10 &amp; 11
              </span>
              <span>
                <Ic icon={Check} size={15} />
                No admin permission needed
              </span>
              <span>
                <Ic icon={Check} size={15} />
                Portable mode included
              </span>
              <span>
                <Ic icon={Check} size={15} />
                Works fully offline
              </span>
            </div>
          </div>
        </Reveal>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="footer">
      <div className="container footer-inner">
        <span className="brand">
          <Ic icon={Stamp} size={18} />
          Carimbo
        </span>
        <span>Less busywork, more of the work that matters.</span>
        <div className="footer-links">
          <a href="#features">Features</a>
          <a href="#download">Download</a>
        </div>
      </div>
    </footer>
  );
}

export default function App() {
  const [theme, setTheme] = useTheme();
  return (
    <MotionConfig reducedMotion="user">
      <Nav
        theme={theme}
        onToggleTheme={() => setTheme(theme === "light" ? "dark" : "light")}
      />
      <main>
        <Hero theme={theme} />
        <Features />
        <More />
        <DownloadSection />
      </main>
      <Footer />
    </MotionConfig>
  );
}
