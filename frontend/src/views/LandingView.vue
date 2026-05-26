<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { RouterLink } from "vue-router";
import { useAuthStore } from "@/stores/auth";

const auth = useAuthStore();
const footYear = new Date().getFullYear();
const emailLabel = ref("bimodality@proton.me");
const EMAIL = "bimodality@proton.me";

let revealObs: IntersectionObserver | undefined;
let counterObs: IntersectionObserver | undefined;
let onScroll: (() => void) | undefined;
let onResize: (() => void) | undefined;
let terminalTimers: ReturnType<typeof setTimeout>[] = [];

function escHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

onMounted(() => {
  // scroll-triggered reveals (staggered via data-delay)
  revealObs = new IntersectionObserver(
    (entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting) {
          e.target.classList.add("visible");
          revealObs?.unobserve(e.target);
        }
      });
    },
    { threshold: 0.1 },
  );
  document.querySelectorAll(".lg-landing .reveal").forEach((el) => revealObs!.observe(el));

  // scroll progress track with section dots
  const track = document.getElementById("scroll-track");
  if (track && window.innerWidth > 1200) {
    const fill = track.querySelector(".track-fill") as HTMLElement | null;
    const sections = [
      "hero", "stats", "problem", "features", "comparison",
      "how", "principles", "upcoming", "contact",
    ];
    const labels: Record<string, string> = {
      hero: "what it is",
      stats: "why it matters",
      problem: "what it replaces",
      features: "what it does",
      comparison: "why this tool",
      how: "how it works",
      principles: "constraints",
      upcoming: "roadmap",
      contact: "get in touch",
    };
    const dots: HTMLDivElement[] = [];
    sections.forEach((id) => {
      const dot = document.createElement("div");
      dot.className = "track-dot";
      dot.dataset.section = id;
      dot.dataset.label = labels[id] || id;
      dot.addEventListener("click", () => {
        const target = document.getElementById(id);
        if (target) target.scrollIntoView({ behavior: "smooth" });
      });
      track.appendChild(dot);
      dots.push(dot);
    });

    function positionDots() {
      const docH = document.documentElement.scrollHeight;
      sections.forEach((id, i) => {
        const el = document.getElementById(id);
        if (!el || !dots[i]) return;
        const top = el.offsetTop + el.offsetHeight / 2;
        dots[i].style.top = (top / docH) * 100 + "%";
      });
    }

    function updateTrack() {
      if (!fill) return;
      const scrollY = window.scrollY;
      const docH = document.documentElement.scrollHeight - window.innerHeight;
      const pct = Math.min((scrollY / docH) * 100, 100);
      fill.style.height = pct + "%";
      const scrollMid = scrollY + window.innerHeight / 2;
      dots.forEach((dot) => {
        const el = document.getElementById(dot.dataset.section!);
        if (!el) return;
        const top = el.offsetTop;
        const bot = top + el.offsetHeight;
        dot.classList.toggle("active", scrollMid >= top && scrollMid <= bot);
      });
    }

    positionDots();
    updateTrack();
    onScroll = () => updateTrack();
    onResize = () => { positionDots(); updateTrack(); };
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", onResize);
  }

  // stat counter animation
  const counters = document.querySelectorAll(".lg-landing .counter");
  let countersDone = false;
  counterObs = new IntersectionObserver(
    (entries) => {
      entries.forEach((e) => {
        if (e.isIntersecting && !countersDone) {
          countersDone = true;
          counters.forEach((c) => animateCounter(c as HTMLElement));
          counterObs?.disconnect();
        }
      });
    },
    { threshold: 0.3 },
  );
  counters.forEach((c) => counterObs!.observe(c));

  function animateCounter(el: HTMLElement) {
    const target = parseInt(el.dataset.target!);
    const suffix = el.dataset.suffix || "";
    const duration = 1200;
    const start = performance.now();
    function tick(now: number) {
      const elapsed = now - start;
      const progress = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3);
      const val = Math.round(eased * target);
      el.innerHTML = val + suffix;
      if (progress < 1) requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
  }

  // hero terminal animation
  const termBody = document.getElementById("termBody");
  if (termBody) {
    const baseLines = [
      { ln: 14, code: "pub fn validate_session(token: &str) -> Result<Session> {" },
      { ln: 15, code: "    let claims = decode_jwt(token)?;" },
      { ln: 16, code: "    if claims.exp < Utc::now().timestamp() {" },
      { ln: 17, code: "        return Err(AuthError::Expired);" },
      { ln: 18, code: "    }" },
      { ln: 19, code: "" },
      { ln: 20, code: "    let user = db::find_user(claims.sub)?;" },
      { ln: 21, code: "    Ok(Session::new(user, claims.exp))" },
      { ln: 22, code: "}" },
    ];

    function renderBase() {
      let h = "";
      baseLines.forEach((l) => {
        h +=
          '<div class="term-line" data-ln="' + l.ln +
          '"><span class="term-ln">' + l.ln +
          '</span><span class="term-code">' + escHtml(l.code) +
          "</span></div>";
      });
      termBody!.innerHTML = h;
    }

    function findLine(ln: number): HTMLElement | null {
      return termBody!.querySelector('[data-ln="' + ln + '"]');
    }

    function phase1() {
      const line16 = findLine(16);
      if (line16) {
        line16.classList.add("term-overlay");
        const code = line16.querySelector(".term-code") as HTMLElement;
        code.innerHTML =
          '<span style="color:#6bcb96">' +
          escHtml("    if claims.exp < Utc::now().timestamp() - 300 {") +
          "</span>" +
          ' <span class="term-tag green">alice</span>';
        terminalTimers.push(setTimeout(() => line16.classList.add("shown"), 50));
      }
      terminalTimers.push(setTimeout(phase2, 2200));
    }

    function phase2() {
      const line16 = findLine(16);
      if (line16) {
        line16.classList.add("conflict-bg");
        const code = line16.querySelector(".term-code") as HTMLElement;
        code.innerHTML =
          '<span style="color:#e87b7b">' +
          escHtml("    if claims.exp < Utc::now().timestamp() - 300 {") +
          "</span>" +
          ' <span class="term-tag green">alice</span>' +
          ' <span class="term-tag blue">bob</span>';
      }
      terminalTimers.push(setTimeout(phase3, 1600));
    }

    function phase3() {
      const line16 = findLine(16);
      if (!line16) return;
      const panel = document.createElement("div");
      panel.className = "term-conflict";
      panel.innerHTML =
        '<div class="term-conflict-head">merge conflict · line 16 · 2 branches</div>' +
        '<div class="term-conflict-branch"><span class="cb-name">feat/grace-period</span> alice (live)</div>' +
        '<div class="term-conflict-code">' +
        escHtml("if claims.exp < Utc::now().timestamp() - 300 {") +
        "</div>" +
        '<div class="term-conflict-branch" style="margin-top:.35rem"><span class="cb-name">fix/expiry</span> bob (live)</div>' +
        '<div class="term-conflict-code">' +
        escHtml("if claims.exp < Utc::now().timestamp() - GRACE_SECS {") +
        "</div>";
      line16.after(panel);
      terminalTimers.push(setTimeout(() => panel.classList.add("shown"), 50));
      terminalTimers.push(setTimeout(resetTerminal, 5000));
    }

    function resetTerminal() {
      renderBase();
      terminalTimers.push(setTimeout(phase1, 1200));
    }

    renderBase();
    terminalTimers.push(setTimeout(phase1, 1800));
  }
});

onBeforeUnmount(() => {
  revealObs?.disconnect();
  counterObs?.disconnect();
  if (onScroll) window.removeEventListener("scroll", onScroll);
  if (onResize) window.removeEventListener("resize", onResize);
  terminalTimers.forEach((t) => clearTimeout(t));
  terminalTimers = [];
});

async function copyEmail() {
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(EMAIL);
    } else {
      const ta = document.createElement("textarea");
      ta.value = EMAIL;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
    }
    emailLabel.value = "copied";
    setTimeout(() => { emailLabel.value = EMAIL; }, 1500);
  } catch {
    /* swallow */
  }
}
</script>

<template>
  <div class="lg-landing">
    <!-- Scroll progress track -->
    <div id="scroll-track" aria-hidden="true">
      <div class="track-bg"></div>
      <div class="track-fill"></div>
    </div>

    <nav aria-label="Site navigation">
      <div class="inner">
        <a href="#hero" class="brand">
          <svg fill="none" stroke="currentColor" stroke-width="1.75" viewBox="0 0 24 24" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="6" cy="3" r="2" />
            <circle cx="6" cy="18" r="2" />
            <circle cx="18" cy="9" r="2" />
            <path d="M6 5v8a4 4 0 0 0 4 4h2a4 4 0 0 0 4-4v-2" />
          </svg>
          lightning-git
        </a>
        <div class="nav-links">
          <a href="#features" class="nav-link">Features</a>
          <a href="#how" class="nav-link">How it works</a>
          <a href="#contact" class="nav-link">Contact</a>
          <template v-if="auth.isAuthenticated">
            <RouterLink to="/dashboard" class="btn btn-primary">Dashboard</RouterLink>
          </template>
          <template v-else>
            <RouterLink to="/login" class="nav-link">Sign in</RouterLink>
            <RouterLink to="/register" class="btn btn-primary">Sign up</RouterLink>
          </template>
        </div>
      </div>
    </nav>

    <section id="hero">
      <div class="container">
        <div class="reveal">
          <div class="hero-mark" aria-hidden="true">
            <svg fill="none" stroke="currentColor" stroke-width="1.75" viewBox="0 0 24 24" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="6" cy="3" r="2" />
              <circle cx="6" cy="18" r="2" />
              <circle cx="18" cy="9" r="2" />
              <path d="M6 5v8a4 4 0 0 0 4 4h2a4 4 0 0 0 4-4v-2" />
            </svg>
          </div>
          <h1>
            See your team's code.
            <br />
            In real time.
          </h1>
          <p class="subheadline">
            Lightning Git mirrors your repo, tracks who is editing what, and runs conflict detection against every active branch on demand.
          </p>
          <div class="hero-cta">
            <RouterLink v-if="auth.isAuthenticated" to="/dashboard" class="btn btn-primary">
              Go to dashboard
            </RouterLink>
            <RouterLink v-else to="/register" class="btn btn-primary">
              Get started
            </RouterLink>
            <a href="#features" class="btn btn-secondary">Learn more</a>
          </div>
        </div>
        <div class="reveal" data-delay="2">
          <div class="hero-terminal" role="img" aria-label="Simulated editor showing live teammate edits">
            <div class="term-bar">
              <span class="term-dot"></span>
              <span class="term-dot"></span>
              <span class="term-dot"></span>
              <span class="term-title">src/auth/session.rs</span>
            </div>
            <div class="term-body" id="termBody"></div>
          </div>
        </div>
      </div>
    </section>

    <section id="stats">
      <div class="container">
        <div class="stats-grid reveal">
          <div class="stat-item">
            <p class="stat-num"><span class="counter" data-target="19" data-suffix="%">0%</span></p>
            <p class="stat-label">of merge attempts produce conflicts</p>
            <p class="stat-cite">Brindescu et al. 2020, 143 projects</p>
          </div>
          <div class="stat-item">
            <p class="stat-num"><span class="counter" data-target="26" data-suffix="&times;">0&times;</span></p>
            <p class="stat-label">more error-prone when resolved manually</p>
            <p class="stat-cite">Ghiotto et al., IEEE TSE</p>
          </div>
          <div class="stat-item">
            <p class="stat-num"><span class="counter" data-target="11" data-suffix="%">0%</span></p>
            <p class="stat-label">of dev time spent actually writing code</p>
            <p class="stat-cite">Microsoft Research</p>
          </div>
        </div>
      </div>
    </section>

    <section id="problem">
      <div class="container">
        <div class="inner reveal">
          <div>
            <h2>Coordination costs compound.</h2>
            <p>
              Jira tickets, Slack pings, and stand-ups exist to answer one question:
              <em>"What is everyone working on?"</em>
              The answer is always stale by the time it lands.
            </p>
            <ul class="plain-list dashes">
              <li>You alt-tab between IDE and project tracker dozens of times a day</li>
              <li>Merge conflicts surface during PR review, hours or days after the overlap started</li>
              <li>Status fields in the tracker drift from what the branch actually looks like</li>
              <li>Two people edit the same file and nobody knows until one of them pushes</li>
            </ul>
          </div>
          <div>
            <h2>The repo already knows.</h2>
            <p>
              Lightning Git clones your repository on project creation, fetches on every open, and layers a live editing state on top via WebSocket. No new process, no new tool to learn.
            </p>
            <ul class="plain-list checks">
              <li>Each user's edits are held in separate per-user state, broadcast to everyone else</li>
              <li>Conflict detection diffs every branch pair against main and flags overlapping hunks</li>
              <li>Remote branches are auto-registered as Kanban tasks, columns are moved manually</li>
              <li>The backend only runs read-only git commands, your history stays untouched</li>
            </ul>
          </div>
        </div>
      </div>
    </section>

    <section id="features">
      <div class="container">
        <h2 class="reveal">What it does.</h2>
        <p class="section-sub reveal">
          A live session is a WebSocket connection per file. Everyone editing the same file shares a channel where edits, comments, and conflict warnings flow in real time. Everything below runs on top of that.
        </p>
        <div class="features-grid">
          <article class="feature-card reveal" data-delay="0">
            <h3>Live Code Overlay</h3>
            <p>Every user editing a file gets their own in-memory state keyed by user ID. Changes are broadcast over the session channel to everyone else on the same file, so you see what your teammates are typing without anyone's work overwriting anyone else's.</p>
          </article>
          <article class="feature-card reveal" data-delay="1">
            <h3>Live Comments</h3>
            <p>Click any edited line to leave a comment. Comments are broadcast over the same WebSocket channel, tied to line numbers, and support @mentions with autocomplete from the project member list. They're in-memory and part of the session, gone on server restart like the overlays themselves.</p>
          </article>
          <article class="feature-card reveal" data-delay="2">
            <h3>Pre-Merge Conflict Detection</h3>
            <p>The backend diffs each active branch against <span class="code-badge">origin/main</span>, decomposes the changes into hunks, and groups overlapping line ranges. When two branches touch the same lines with different content, that's a conflict and it shows up before anyone merges.</p>
          </article>
          <article class="feature-card reveal" data-delay="3">
            <h3>Notbremse</h3>
            <p>One API call resets all your overlays in a project back to the base content from git. If your live state drifted or you want a clean slate, hit the emergency brake. It only affects your own overlays, everyone else's session state stays untouched.</p>
          </article>
          <article class="feature-card reveal" data-delay="4">
            <h3>Automatic Task Tracking</h3>
            <p>Remote branches are detected on every fetch and registered as tasks on a Kanban board. You move them between To&nbsp;Do, In&nbsp;Progress, Review, and Merged manually. Column state is persisted so it's consistent across users and devices.</p>
          </article>
          <article class="feature-card reveal" data-delay="5">
            <h3>VS Code Extension</h3>
            <p>Press <span class="code-badge">Ctrl+Shift+L</span> or <span class="code-badge">Cmd+Shift+L</span> to connect. The extension sends your edits with a 1s debounce and renders teammate changes as inline peek decorations. The status bar shows who's active and whether any conflicts are predicted.</p>
          </article>
          <article class="feature-card reveal" data-delay="6">
            <h3>Web Dashboard</h3>
            <p>A browser-based view of the same data. Kanban board, member management, file tree with live-edit indicators, and a project-wide activity feed that shows who is editing which file on which branch.</p>
          </article>
          <article class="feature-card reveal" data-delay="7">
            <h3>Git Mirror Sync</h3>
            <p>The backend clones the repo once on project creation and fetches on every project open to stay current. The local mirror is the data source for branch listing, file reads, and diff computation. Only read-only git commands run against it.</p>
          </article>
        </div>
      </div>
    </section>

    <section id="comparison">
      <div class="container">
        <h2 class="reveal">Why this and not something else.</h2>
        <p class="section-sub reveal">
          GitLive, GitKraken Team View, and built-in merge editors solve adjacent problems differently.
        </p>
        <div class="compare-grid">
          <div class="compare-card reveal" data-delay="0">
            <p class="compare-label">vs GitLive</p>
            <h3>Open source and self-hosted</h3>
            <p>GitLive is closed-source SaaS that routes your editing activity through their infrastructure. Lightning Git is fully open source and runs on your own server, so overlay data never leaves your network. It also goes further than gutter indicators by showing grouped conflict hunks with per-contributor versions side by side.</p>
          </div>
          <div class="compare-card reveal" data-delay="1">
            <p class="compare-label">vs GitKraken</p>
            <h3>Layers on top, replaces nothing</h3>
            <p>GitKraken's Team View shows who's on which branch and flags shared files, but it's a full Git client that replaces your workflow. Lightning Git sits on top of VS Code and any Git host without changing how you commit, push, or review. The Kanban board is derived from branches you already have.</p>
          </div>
          <div class="compare-card reveal" data-delay="2">
            <p class="compare-label">vs merge-time tooling</p>
            <h3>Conflicts surface while you type</h3>
            <p>VS Code's built-in merge editor and tools like Beyond Compare activate after you attempt a merge. By then the overlap is already committed on both sides. Lightning Git runs its diff pipeline when you open a file or when the frontend polls the merge endpoint, so you see the conflict while both contributors still have context.</p>
          </div>
        </div>
      </div>
    </section>

    <section id="how">
      <div class="container">
        <h2 class="reveal">How it works</h2>
        <div class="steps">
          <div class="step-card reveal" data-delay="0">
            <p class="step-number">01</p>
            <h3>Add a repository</h3>
            <p>Create a project and point it at a Git remote. The backend clones it once and runs a fetch every time someone opens the project, so the mirror stays current on demand.</p>
          </div>
          <div class="step-card reveal" data-delay="1">
            <p class="step-number">02</p>
            <h3>Branches become tasks</h3>
            <p>On every fetch the backend scans remote refs and registers each branch as a Kanban task. You drag them between columns yourself, the board just removes the need for a separate ticket tracker.</p>
          </div>
          <div class="step-card reveal" data-delay="2">
            <p class="step-number">03</p>
            <h3>Open a file</h3>
            <p>The VS Code extension opens a WebSocket per file. The server pushes a snapshot of all active editors and comments immediately, then streams every subsequent change.</p>
          </div>
          <div class="step-card reveal" data-delay="3">
            <p class="step-number">04</p>
            <h3>Conflicts surface early</h3>
            <p>When you open a file the frontend calls the merge endpoint, which diffs every branch against main and flags overlapping hunks. The client also polls periodically and computes live overlap from active overlays between polls.</p>
          </div>
        </div>
      </div>
    </section>

    <section id="principles">
      <div class="container">
        <h2 class="reveal">Constraints</h2>
        <p class="section-sub reveal">Three guarantees enforced at implementation level.</p>
        <div class="principles-grid">
          <div class="principle-card reveal" data-delay="0">
            <p class="principle-label">Read-only</p>
            <p>
              The codebase only runs
              <span class="code-badge">clone</span>,
              <span class="code-badge">fetch</span>,
              <span class="code-badge">show</span>,
              <span class="code-badge">ls-tree</span>, and
              <span class="code-badge">branch -r</span>.
              No push, no commit, no checkout exists anywhere in the backend.
            </p>
          </div>
          <div class="principle-card reveal" data-delay="1">
            <p class="principle-label">In-memory</p>
            <p>Live overlay state is held in server memory and keyed per project and per user. On restart it's gone. Nothing about uncommitted work touches a database or disk.</p>
          </div>
          <div class="principle-card reveal" data-delay="2">
            <p class="principle-label">Project-isolated</p>
            <p>Every WebSocket message is membership-checked before broadcast. Users are scoped to their project, and org-level permissions gate access to all projects underneath.</p>
          </div>
        </div>
      </div>
    </section>

    <section id="upcoming">
      <div class="container">
        <h2 class="reveal">What's next.</h2>
        <p class="section-sub reveal">The thesis ships first. Everything below follows.</p>
        <div class="upcoming-grid">
          <div class="upcoming-item reveal" data-delay="0">
            <div>
              <h3>Automated Kanban</h3>
              <p>Commits and merges move cards across columns automatically instead of requiring manual drag.</p>
            </div>
          </div>
          <div class="upcoming-item reveal" data-delay="1">
            <div>
              <h3>In-session code suggestions</h3>
              <p>When a conflict is detected, a teammate can propose their version as a replacement directly in the overlay. If the other side accepts, both branches converge on the same content and the conflict disappears before either of them merges.</p>
            </div>
          </div>
          <div class="upcoming-item reveal" data-delay="2">
            <div>
              <h3>Multi-host support</h3>
              <p>GitLab, Bitbucket, Codeberg, and self-hosted Git servers alongside GitHub.</p>
            </div>
          </div>
          <div class="upcoming-item reveal" data-delay="3">
            <div>
              <h3>Persistent comments</h3>
              <p>Comments currently live in memory and are gone on restart. Storing them in the database lets conversations survive across sessions.</p>
            </div>
          </div>
          <div class="upcoming-item reveal" data-delay="4">
            <div>
              <h3>Security hardening</h3>
              <p>Rate limiting, audit logging, token rotation, and guardrails against accidentally exposing repository content through the overlay layer.</p>
            </div>
          </div>
          <div class="upcoming-item reveal" data-delay="5">
            <div>
              <h3>Production deployment and paid tier</h3>
              <p>Self-hosting documentation, Docker images, and a managed hosted option with a paid plan for teams who don't want to run their own infrastructure.</p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <section id="contact">
      <div class="container">
        <h2 class="reveal">Get in touch.</h2>
        <p class="section-sub reveal">Built as a Swiss HF diploma project. Reach out if you want to talk about it.</p>
        <div class="reveal contact-actions">
          <span class="email-copy" :title="emailLabel === 'copied' ? 'copied' : 'Click to copy'" @click="copyEmail">{{ emailLabel }}</span>
          <RouterLink v-if="!auth.isAuthenticated" to="/register" class="btn btn-primary">Try it</RouterLink>
        </div>
      </div>
    </section>

    <footer>
      <div class="inner">
        <div class="foot-brand">
          <svg fill="none" stroke="currentColor" stroke-width="1.75" viewBox="0 0 24 24" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="6" cy="3" r="2" />
            <circle cx="6" cy="18" r="2" />
            <circle cx="18" cy="9" r="2" />
            <path d="M6 5v8a4 4 0 0 0 4 4h2a4 4 0 0 0 4-4v-2" />
          </svg>
          lightning-git
        </div>
        <nav class="foot-links" aria-label="Project repositories">
          <a href="https://github.com/191-iota/lightning-git-backend" target="_blank" rel="noopener noreferrer">Backend</a>
          <a href="https://github.com/191-iota/lightning-git-frontend" target="_blank" rel="noopener noreferrer">Frontend</a>
          <a href="https://github.com/191-iota/lightning-git-vsc" target="_blank" rel="noopener noreferrer">VS Code Extension</a>
        </nav>
        <p class="foot-meta">
          Built by
          <a href="https://github.com/191-iota" target="_blank" rel="noopener noreferrer" class="foot-meta-link">191-iota</a>
          · <span>{{ footYear }}</span>
        </p>
      </div>
    </footer>
  </div>
</template>

<style>
/* 1:1 port of the standalone landing page styling. scoped to .lg-landing
   so it does not bleed into the dashboard / project / overlay views. */
.lg-landing {
  --bg: #ffffff;
  --bg-warm: #fafaf8;
  --surface: #f4f4f1;
  --surface-2: #eeeee9;
  --border: #d8d8d0;
  --border-light: #e8e8e2;
  --text: #1a1a1a;
  --text-sec: #5c5c5c;
  --text-muted: #999990;
  --accent: #1a1a1a;
  --accent-hover: #333333;
  --ink: #0d0d0d;
  --code-bg: #f0f0ec;
  --code-text: #3d3d3d;
  --stat-num: #1a1a1a;
  --green: #2d6a4f;
  --red: #9b2c2c;
  --blue: #2b4c8c;

  font-family: "IBM Plex Sans", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  background: var(--bg);
  color: var(--text);
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}
.lg-landing * { box-sizing: border-box; }
.lg-landing section { padding: 5rem 0; position: relative; }
.lg-landing .container { max-width: 1060px; margin: 0 auto; padding: 0 1.5rem; }

/* reveals */
.lg-landing .reveal {
  opacity: 0;
  transform: translateY(24px);
  transition: opacity 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94), transform 0.6s cubic-bezier(0.25, 0.46, 0.45, 0.94);
}
.lg-landing .reveal.visible { opacity: 1; transform: none; }
.lg-landing .reveal[data-delay="1"] { transition-delay: 0.08s; }
.lg-landing .reveal[data-delay="2"] { transition-delay: 0.16s; }
.lg-landing .reveal[data-delay="3"] { transition-delay: 0.24s; }
.lg-landing .reveal[data-delay="4"] { transition-delay: 0.32s; }
.lg-landing .reveal[data-delay="5"] { transition-delay: 0.4s; }
.lg-landing .reveal[data-delay="6"] { transition-delay: 0.48s; }
.lg-landing .reveal[data-delay="7"] { transition-delay: 0.56s; }

/* scroll track */
.lg-landing #scroll-track {
  position: fixed;
  left: 2rem;
  top: 0;
  bottom: 0;
  width: 2px;
  z-index: 40;
  pointer-events: none;
}
.lg-landing #scroll-track .track-bg {
  position: absolute; top: 0; left: 0; width: 100%; height: 100%;
  background: var(--border-light);
}
.lg-landing #scroll-track .track-fill {
  position: absolute; top: 0; left: 0; width: 100%; height: 0%;
  background: var(--text);
  transition: height 0.05s linear;
}
.lg-landing #scroll-track .track-dot {
  position: absolute;
  left: 50%;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border);
  transform: translate(-50%, -50%);
  transition: background 0.3s, box-shadow 0.3s, transform 0.2s;
  pointer-events: auto;
  cursor: pointer;
}
.lg-landing #scroll-track .track-dot::before {
  content: ""; position: absolute; inset: -8px; border-radius: 50%;
}
.lg-landing #scroll-track .track-dot:hover {
  background: var(--text-sec);
  transform: translate(-50%, -50%) scale(1.4);
}
.lg-landing #scroll-track .track-dot::after {
  content: attr(data-label);
  position: absolute;
  left: 18px;
  top: 50%;
  transform: translateY(-50%);
  font-family: "IBM Plex Mono", monospace;
  font-size: 0.65rem;
  color: var(--text-sec);
  white-space: nowrap;
  background: var(--bg);
  border: 1px solid var(--border-light);
  padding: 0.2rem 0.5rem;
  border-radius: 0.25rem;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.2s;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}
.lg-landing #scroll-track .track-dot:hover::after { opacity: 1; }
.lg-landing #scroll-track .track-dot.active {
  background: var(--text);
  box-shadow: 0 0 0 3px rgba(26, 26, 26, 0.12);
}
@media (max-width: 1200px) {
  .lg-landing #scroll-track { display: none; }
}

/* nav */
.lg-landing > nav {
  position: sticky;
  top: 0;
  z-index: 50;
  background: rgba(255, 255, 255, 0.85);
  backdrop-filter: blur(14px);
  -webkit-backdrop-filter: blur(14px);
  border-bottom: 1px solid var(--border-light);
}
.lg-landing > nav .inner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  max-width: 1060px;
  margin: 0 auto;
  padding: 0.8rem 1.5rem;
}
.lg-landing .brand {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-weight: 600;
  letter-spacing: -0.02em;
  font-size: 1rem;
  color: var(--text);
  text-decoration: none;
}
.lg-landing .brand svg {
  color: var(--text-sec);
  width: 18px;
  height: 18px;
  flex-shrink: 0;
}
.lg-landing a.nav-link {
  color: var(--text-sec);
  text-decoration: none;
  font-size: 0.875rem;
  transition: color 0.2s;
}
.lg-landing a.nav-link:hover { color: var(--text); }
.lg-landing .nav-links {
  display: flex;
  align-items: center;
  gap: 1.5rem;
}

/* buttons (override Tailwind's base) */
.lg-landing .btn {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.55rem 1.3rem;
  border-radius: 0.375rem;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  text-decoration: none;
  transition: all 0.2s ease;
  border: 1px solid transparent;
  letter-spacing: -0.005em;
}
.lg-landing .btn-primary {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}
.lg-landing .btn-primary:hover {
  background: var(--accent-hover);
  border-color: var(--accent-hover);
}
.lg-landing .btn-secondary {
  background: transparent;
  color: var(--text);
  border-color: var(--border);
}
.lg-landing .btn-secondary:hover {
  background: var(--surface);
  border-color: var(--text-muted);
}

/* hero */
.lg-landing #hero {
  padding: 7rem 0 3.5rem;
  text-align: center;
  position: relative;
}
.lg-landing .hero-mark {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  margin-bottom: 1.75rem;
  color: var(--text-sec);
}
.lg-landing .hero-mark svg {
  width: 32px;
  height: 32px;
}
.lg-landing #hero h1 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(2.4rem, 5vw, 3.6rem);
  font-weight: 700;
  letter-spacing: -0.025em;
  line-height: 1.1;
  margin-bottom: 1.25rem;
  color: var(--ink);
}
.lg-landing #hero p.subheadline {
  font-size: clamp(0.95rem, 1.8vw, 1.05rem);
  color: var(--text-sec);
  max-width: 520px;
  margin: 0 auto 2.25rem;
  line-height: 1.65;
}
.lg-landing .hero-cta {
  display: flex;
  gap: 0.6rem;
  justify-content: center;
  flex-wrap: wrap;
  margin-bottom: 3.5rem;
}

/* hero terminal */
.lg-landing .hero-terminal {
  max-width: 580px;
  margin: 0 auto;
  background: var(--ink);
  border-radius: 0.5rem;
  overflow: hidden;
  text-align: left;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.12), 0 1px 3px rgba(0, 0, 0, 0.08);
}
.lg-landing .term-bar {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  padding: 0.6rem 0.9rem;
  background: rgba(255, 255, 255, 0.06);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.lg-landing .term-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.15);
}
.lg-landing .term-title {
  font-family: "IBM Plex Mono", monospace;
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.35);
  margin-left: 0.4rem;
}
.lg-landing .term-body {
  padding: 1rem 1.1rem;
  height: 260px;
  overflow: hidden;
  font-family: "IBM Plex Mono", monospace;
  font-size: 0.78rem;
  line-height: 1.75;
  color: rgba(255, 255, 255, 0.5);
}
.lg-landing .term-line {
  display: flex;
  gap: 0;
  white-space: pre;
  min-height: 1.75em;
}
.lg-landing .term-ln {
  width: 2.2em;
  text-align: right;
  padding-right: 0.9em;
  color: rgba(255, 255, 255, 0.18);
  user-select: none;
  flex-shrink: 0;
}
.lg-landing .term-code { color: rgba(255, 255, 255, 0.55); }
.lg-landing .term-overlay {
  opacity: 0;
  transition: opacity 0.4s;
  position: relative;
}
.lg-landing .term-overlay.shown { opacity: 1; }
.lg-landing .term-overlay .term-code { color: rgba(255, 255, 255, 0.85); }
.lg-landing .term-tag {
  display: inline-block;
  font-size: 0.6rem;
  padding: 0.05rem 0.35rem;
  border-radius: 0.2rem;
  margin-left: 0.6rem;
  vertical-align: middle;
  font-weight: 500;
  letter-spacing: 0.02em;
}
.lg-landing .term-tag.green { background: rgba(45, 106, 79, 0.25); color: #6bcb96; }
.lg-landing .term-tag.blue { background: rgba(43, 76, 140, 0.25); color: #7ba4e8; }
.lg-landing .term-tag.red { background: rgba(155, 44, 44, 0.2); color: #e87b7b; }
.lg-landing .term-line.conflict-bg { background: rgba(244, 63, 94, 0.08); }
.lg-landing .term-conflict {
  opacity: 0;
  max-height: 0;
  overflow: hidden;
  transition: opacity 0.4s, max-height 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94), padding 0.5s cubic-bezier(0.25, 0.46, 0.45, 0.94);
  margin: 0.3rem 0.6rem 0.3rem 3.1rem;
  border-left: 2px solid rgba(244, 63, 94, 0.5);
  padding: 0 0.6rem;
  background: rgba(244, 63, 94, 0.06);
  border-radius: 0 0.25rem 0.25rem 0;
  font-size: 0.65rem;
}
.lg-landing .term-conflict.shown {
  opacity: 1;
  max-height: 12rem;
  padding: 0.4rem 0.6rem;
}
.lg-landing .term-conflict-head {
  color: rgba(244, 63, 94, 0.9);
  font-weight: 600;
  margin-bottom: 0.3rem;
}
.lg-landing .term-conflict-branch {
  color: rgba(255, 255, 255, 0.35);
  margin-top: 0.25rem;
}
.lg-landing .term-conflict-branch span.cb-name {
  display: inline-block;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.05rem 0.3rem;
  border-radius: 0.15rem;
  color: rgba(255, 255, 255, 0.5);
  font-size: 0.6rem;
  margin-right: 0.3rem;
}
.lg-landing .term-conflict-code {
  color: rgba(255, 255, 255, 0.55);
  margin-top: 0.15rem;
  padding: 0.2rem 0.35rem;
  background: rgba(255, 255, 255, 0.03);
  border-radius: 0.15rem;
  white-space: pre;
}

/* stats */
.lg-landing #stats {
  background: var(--surface);
  border-top: 1px solid var(--border-light);
  border-bottom: 1px solid var(--border-light);
  padding: 2.75rem 0;
}
.lg-landing .stats-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 2rem;
  text-align: center;
}
@media (min-width: 640px) {
  .lg-landing .stats-grid { grid-template-columns: 1fr 1fr 1fr; }
}
.lg-landing .stat-num {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: 2rem;
  font-weight: 700;
  color: var(--stat-num);
  letter-spacing: -0.02em;
  line-height: 1;
}
.lg-landing .stat-label {
  font-size: 0.875rem;
  color: var(--text-sec);
  margin-top: 0.45rem;
  line-height: 1.5;
}
.lg-landing .stat-cite {
  font-size: 0.75rem;
  color: var(--text-muted);
  margin-top: 0.25rem;
}

/* problem */
.lg-landing #problem { padding: 5rem 0; }
.lg-landing #problem .inner {
  display: grid;
  gap: 3.5rem;
  align-items: start;
  grid-template-columns: 1fr;
}
@media (min-width: 680px) {
  .lg-landing #problem .inner { grid-template-columns: 1fr 1fr; gap: 4rem; }
}
.lg-landing #problem h2 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(1.35rem, 2.5vw, 1.7rem);
  font-weight: 700;
  letter-spacing: -0.015em;
  margin-bottom: 0.6rem;
  color: var(--ink);
}
.lg-landing #problem p {
  color: var(--text-sec);
  font-size: 0.925rem;
  line-height: 1.7;
}
.lg-landing .plain-list {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-top: 0.85rem;
  padding-left: 0;
}
.lg-landing .plain-list li {
  display: flex;
  align-items: flex-start;
  gap: 0.55rem;
  color: var(--text-sec);
  font-size: 0.875rem;
  line-height: 1.6;
}
.lg-landing .plain-list li::before {
  flex-shrink: 0;
  margin-top: 0.1em;
}
.lg-landing .plain-list.dashes li::before { content: "\2013"; color: var(--text-muted); }
.lg-landing .plain-list.checks li::before { content: "\2713"; color: var(--text-sec); font-weight: 600; }

/* features */
.lg-landing #features {
  background: var(--bg-warm);
  border-top: 1px solid var(--border-light);
  border-bottom: 1px solid var(--border-light);
}
.lg-landing #features h2 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(1.35rem, 2.5vw, 1.7rem);
  font-weight: 700;
  letter-spacing: -0.015em;
  text-align: center;
  margin-bottom: 0.35rem;
  color: var(--ink);
}
.lg-landing #features .section-sub {
  text-align: center;
  color: var(--text-sec);
  margin-bottom: 2.5rem;
  font-size: 0.925rem;
}
.lg-landing .features-grid {
  display: grid;
  gap: 1rem;
  grid-template-columns: 1fr;
}
@media (min-width: 600px) {
  .lg-landing .features-grid { grid-template-columns: 1fr 1fr; }
}
@media (min-width: 900px) {
  .lg-landing .features-grid { grid-template-columns: 1fr 1fr 1fr; }
}
.lg-landing .feature-card {
  background: var(--bg);
  border: 1px solid var(--border-light);
  border-radius: 0.5rem;
  padding: 1.35rem;
  transition: border-color 0.2s, box-shadow 0.2s;
}
.lg-landing .feature-card:hover {
  border-color: var(--border);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
}
.lg-landing .feature-card h3 {
  font-size: 0.925rem;
  font-weight: 600;
  margin-bottom: 0.3rem;
  color: var(--ink);
}
.lg-landing .feature-card p {
  font-size: 0.825rem;
  color: var(--text-sec);
  line-height: 1.7;
}
.lg-landing .code-badge {
  display: inline-block;
  color: var(--code-text);
  background: var(--code-bg);
  padding: 0.05rem 0.35rem;
  border-radius: 0.2rem;
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: 0.725rem;
  border: 1px solid var(--border-light);
}

/* how */
.lg-landing #how { padding: 5rem 0; }
.lg-landing #how h2 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(1.35rem, 2.5vw, 1.7rem);
  font-weight: 700;
  letter-spacing: -0.015em;
  text-align: center;
  margin-bottom: 2.5rem;
  color: var(--ink);
}
.lg-landing .steps {
  display: grid;
  gap: 1rem;
  grid-template-columns: 1fr;
}
@media (min-width: 600px) {
  .lg-landing .steps { grid-template-columns: 1fr 1fr; }
}
@media (min-width: 900px) {
  .lg-landing .steps { grid-template-columns: 1fr 1fr 1fr 1fr; }
}
.lg-landing .step-card {
  background: var(--surface);
  border: 1px solid var(--border-light);
  border-radius: 0.5rem;
  padding: 1.35rem;
}
.lg-landing .step-number {
  font-size: 0.7rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  margin-bottom: 0.6rem;
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  color: var(--text-muted);
}
.lg-landing .step-card h3 {
  font-size: 0.925rem;
  font-weight: 600;
  margin-bottom: 0.3rem;
  color: var(--ink);
}
.lg-landing .step-card p {
  font-size: 0.825rem;
  color: var(--text-sec);
  line-height: 1.65;
}

/* principles */
.lg-landing #principles {
  background: var(--bg-warm);
  border-top: 1px solid var(--border-light);
  border-bottom: 1px solid var(--border-light);
}
.lg-landing #principles h2 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(1.35rem, 2.5vw, 1.7rem);
  font-weight: 700;
  letter-spacing: -0.015em;
  text-align: center;
  margin-bottom: 0.35rem;
  color: var(--ink);
}
.lg-landing #principles .section-sub {
  text-align: center;
  color: var(--text-sec);
  margin-bottom: 2.5rem;
  font-size: 0.925rem;
}
.lg-landing .principles-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}
@media (min-width: 680px) {
  .lg-landing .principles-grid { grid-template-columns: 1fr 1fr 1fr; }
}
.lg-landing .principle-card {
  background: var(--bg);
  border: 1px solid var(--border-light);
  border-radius: 0.5rem;
  padding: 1.35rem;
}
.lg-landing .principle-label {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.12em;
  font-weight: 600;
  color: var(--text-muted);
  margin-bottom: 0.5rem;
}
.lg-landing .principle-card p {
  font-size: 0.825rem;
  color: var(--text-sec);
  line-height: 1.7;
}

/* contact */
.lg-landing #contact { text-align: center; }
.lg-landing #contact h2 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(1.35rem, 2.5vw, 1.7rem);
  font-weight: 700;
  letter-spacing: -0.015em;
  margin-bottom: 0.35rem;
  color: var(--ink);
}
.lg-landing #contact .section-sub {
  color: var(--text-sec);
  margin-bottom: 2rem;
  font-size: 0.925rem;
}
.lg-landing .contact-actions {
  display: flex;
  gap: 0.6rem;
  justify-content: center;
  align-items: center;
  flex-wrap: wrap;
  margin-top: 1.5rem;
}
.lg-landing .email-copy {
  font-family: "IBM Plex Mono", monospace;
  font-size: 0.925rem;
  color: var(--text);
  cursor: pointer;
  padding: 0.4rem 0.8rem;
  border: 1px solid var(--border);
  border-radius: 0.375rem;
  display: inline-block;
  transition: border-color 0.2s, background 0.2s;
}
.lg-landing .email-copy:hover {
  background: var(--surface);
  border-color: var(--text-muted);
}

/* footer */
.lg-landing footer {
  border-top: 1px solid var(--border-light);
  padding: 2.25rem 0;
  background: var(--bg-warm);
}
.lg-landing footer .inner {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 1.25rem;
  max-width: 1060px;
  margin: 0 auto;
  padding: 0 1.5rem;
}
.lg-landing footer .foot-brand {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-weight: 600;
  font-size: 0.9rem;
  color: var(--text);
}
.lg-landing footer .foot-brand svg {
  width: 16px;
  height: 16px;
  color: var(--text-sec);
}
.lg-landing .foot-links {
  display: flex;
  flex-wrap: wrap;
  gap: 1.25rem;
}
.lg-landing .foot-links a {
  color: var(--text-sec);
  font-size: 0.825rem;
  text-decoration: none;
  transition: color 0.2s;
}
.lg-landing .foot-links a:hover { color: var(--text); }
.lg-landing .foot-meta {
  color: var(--text-muted);
  font-size: 0.75rem;
}
.lg-landing .foot-meta-link {
  color: var(--text-sec);
  text-decoration: none;
}

/* comparison */
.lg-landing #comparison { padding: 5rem 0; }
.lg-landing #comparison h2 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(1.35rem, 2.5vw, 1.7rem);
  font-weight: 700;
  letter-spacing: -0.015em;
  text-align: center;
  margin-bottom: 0.35rem;
  color: var(--ink);
}
.lg-landing #comparison .section-sub {
  text-align: center;
  color: var(--text-sec);
  margin-bottom: 2.5rem;
  font-size: 0.925rem;
}
.lg-landing .compare-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}
@media (min-width: 680px) {
  .lg-landing .compare-grid { grid-template-columns: 1fr 1fr 1fr; }
}
.lg-landing .compare-card {
  background: var(--bg-warm);
  border: 1px solid var(--border-light);
  border-radius: 0.5rem;
  padding: 1.35rem;
}
.lg-landing .compare-card h3 {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ink);
  margin-bottom: 0.5rem;
}
.lg-landing .compare-card p {
  font-size: 0.8rem;
  color: var(--text-sec);
  line-height: 1.7;
}
.lg-landing .compare-label {
  font-size: 0.65rem;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  font-weight: 600;
  color: var(--text-muted);
  margin-bottom: 0.4rem;
  font-family: "IBM Plex Mono", ui-monospace, monospace;
}

/* upcoming */
.lg-landing #upcoming { padding: 5rem 0; }
.lg-landing #upcoming h2 {
  font-family: "IBM Plex Mono", ui-monospace, monospace;
  font-size: clamp(1.35rem, 2.5vw, 1.7rem);
  font-weight: 700;
  letter-spacing: -0.015em;
  text-align: center;
  margin-bottom: 0.35rem;
  color: var(--ink);
}
.lg-landing #upcoming .section-sub {
  text-align: center;
  color: var(--text-sec);
  margin-bottom: 2.5rem;
  font-size: 0.925rem;
}
.lg-landing .upcoming-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 0.75rem;
  max-width: 640px;
  margin: 0 auto;
}
.lg-landing .upcoming-item {
  display: flex;
  align-items: baseline;
  gap: 0.6rem;
  padding: 0.7rem 1rem;
  border: 1px solid var(--border-light);
  border-radius: 0.375rem;
  background: var(--bg-warm);
}
.lg-landing .upcoming-item h3 {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--ink);
  margin-bottom: 0.15rem;
}
.lg-landing .upcoming-item p {
  font-size: 0.775rem;
  color: var(--text-sec);
  line-height: 1.6;
}

.lg-landing :focus-visible {
  outline: 2px solid var(--text-sec);
  outline-offset: 2px;
}
</style>
