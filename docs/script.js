const translations = {
  en: {
    hero_badge: 'Open Source · GPL-3.0',
    hero_title_1: 'Share files on Linux,',
    hero_title_2: 'the GNOME way.',
    hero_sub: 'Transfer files and text to nearby devices wirelessly. Native GTK4 interface, zero cloud dependency.',
    btn_download: 'Download',
    btn_releases: 'All releases',
    screenshots_title: 'See it in action',
    demo_subtitle: 'Interactive demo — simulates a real file transfer',
    demo_active_transfers: 'Active transfers',
    demo_ready_to: 'Ready to',
    demo_receive: 'receive',
    demo_visible: 'Visible',
    demo_hidden: 'Hidden',
    demo_wants: 'Wants to share 1 file · 430.2 KB',
    demo_accept: 'Accept',
    demo_decline: 'Decline',
    demo_saved_path: 'Saved to ~/Downloads/Quickshare',
    demo_btn_open: 'Open',
    demo_btn_folder: 'Show folder',
    demo_btn_clear: 'Clear',
    demo_send: 'Send',
    demo_receive_btn: 'Receive',
    demo_replay: '↺ Replay',
    demo_label_receive: 'Receiving',
    demo_label_send: 'Sending',
    demo_send_files: 'Send Files',
    demo_add_files: 'Add Files',
    demo_drop_hint: 'Drop files here or use Select',
    demo_nearby: 'Nearby devices',
    demo_selected_files: 'Selected Files',
    demo_sent_ok: 'IMG_20260425.jpg sent',
    features_title: 'Why GnomeQS?',
    feat1_title: 'mDNS Discovery',
    feat1_desc: 'Automatically finds nearby devices on your network. No pairing, no accounts.',
    feat2_title: 'Files & Text',
    feat2_desc: 'Send any file or plain text snippets to nearby devices instantly.',
    feat3_title: 'GNOME Native',
    feat3_desc: 'Built with GTK4 and libadwaita. Feels at home on your GNOME desktop.',
    feat4_title: 'Multiple Packages',
    feat4_desc: 'Available as Flatpak, AUR, Debian, RPM, and AppImage packages.',
    install_title: 'Installation',
    install_releases: 'See all releases on GitHub →',
    stable_url_label: 'Stable URL (always latest):',
    footer_github: 'GitHub',
    footer_issues: 'Issues',
  },
  pt: {
    hero_badge: 'Código Aberto · GPL-3.0',
    hero_title_1: 'Compartilhe arquivos no Linux,',
    hero_title_2: 'do jeito GNOME.',
    hero_sub: 'Transfira arquivos e texto para dispositivos próximos sem fio. Interface GTK4 nativa, sem dependência de nuvem.',
    btn_download: 'Download',
    btn_releases: 'Todos os lançamentos',
    screenshots_title: 'Veja em ação',
    demo_subtitle: 'Demo interativo — simula uma transferência real',
    demo_active_transfers: 'Transferências ativas',
    demo_ready_to: 'Pronto para',
    demo_receive: 'receber',
    demo_visible: 'Visível',
    demo_hidden: 'Oculto',
    demo_wants: 'Quer compartilhar 1 arquivo · 430.2 KB',
    demo_accept: 'Aceitar',
    demo_decline: 'Recusar',
    demo_saved_path: 'Salvo em ~/Downloads/Quickshare',
    demo_btn_open: 'Abrir',
    demo_btn_folder: 'Mostrar pasta',
    demo_btn_clear: 'Limpar',
    demo_send: 'Enviar',
    demo_receive_btn: 'Receber',
    demo_replay: '↺ Repetir',
    demo_label_receive: 'Recebendo',
    demo_label_send: 'Enviando',
    demo_send_files: 'Enviar Arquivos',
    demo_add_files: 'Adicionar Arquivos',
    demo_drop_hint: 'Solte arquivos aqui ou use Selecionar',
    demo_nearby: 'Dispositivos próximos',
    demo_selected_files: 'Arquivos selecionados',
    demo_sent_ok: 'IMG_20260425.jpg enviado',
    features_title: 'Por que GnomeQS?',
    feat1_title: 'Descoberta mDNS',
    feat1_desc: 'Encontra automaticamente dispositivos próximos na sua rede. Sem pareamento, sem contas.',
    feat2_title: 'Arquivos e Texto',
    feat2_desc: 'Envie qualquer arquivo ou texto para dispositivos próximos instantaneamente.',
    feat3_title: 'Nativo para GNOME',
    feat3_desc: 'Construído com GTK4 e libadwaita. Integrado ao seu desktop GNOME.',
    feat4_title: 'Múltiplos Pacotes',
    feat4_desc: 'Disponível como Flatpak, AUR, Debian, RPM e AppImage.',
    install_title: 'Instalação',
    install_releases: 'Ver todos os lançamentos no GitHub →',
    stable_url_label: 'URL estável (sempre a mais recente):',
    footer_github: 'GitHub',
    footer_issues: 'Problemas',
  },
};

let currentLang = 'en';

function detectLang() {
  const saved = localStorage.getItem('lang');
  if (saved && translations[saved]) return saved;
  try {
    const nav = (navigator.language || '').toLowerCase();
    return nav.startsWith('pt') ? 'pt' : 'en';
  } catch (_) {
    return 'en';
  }
}

function setLanguage(lang) {
  if (!translations[lang]) return;
  currentLang = lang;
  localStorage.setItem('lang', lang);
  document.documentElement.lang = lang === 'pt' ? 'pt-BR' : 'en';
  document.getElementById('lang-label').textContent = lang === 'pt' ? 'EN' : 'PT';
  document.querySelectorAll('[data-i18n]').forEach(el => {
    const key = el.dataset.i18n;
    if (translations[lang][key] !== undefined) {
      el.textContent = translations[lang][key];
    }
  });
}

function detectTheme() {
  const saved = localStorage.getItem('theme');
  if (saved) return saved;
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

function setTheme(theme) {
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem('theme', theme);
  const metaColor = document.getElementById('meta-theme-color');
  if (metaColor) metaColor.content = theme === 'dark' ? '#0b0b18' : '#f8f8ff';
}

function toggleTheme() {
  const current = document.documentElement.getAttribute('data-theme');
  setTheme(current === 'dark' ? 'light' : 'dark');
}

const header = document.getElementById('header');
window.addEventListener('scroll', () => {
  header.classList.toggle('scrolled', window.scrollY > 20);
}, { passive: true });

const observer = new IntersectionObserver(entries => {
  entries.forEach(e => {
    if (e.isIntersecting) {
      e.target.classList.add('visible');
      observer.unobserve(e.target);
    }
  });
}, { threshold: 0.12, rootMargin: '0px 0px -40px 0px' });

document.querySelectorAll('.animate').forEach(el => observer.observe(el));

document.querySelectorAll('.tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    const tab = btn.dataset.tab;
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-panel').forEach(p => p.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById('tab-' + tab).classList.add('active');
  });
});

document.querySelectorAll('.copy-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    const code = btn.closest('.code-block').querySelector('code').textContent.trim();
    navigator.clipboard.writeText(code).then(() => {
      btn.classList.add('copied');
      setTimeout(() => btn.classList.remove('copied'), 1800);
    });
  });
});

const REPO = 'weversonl/gnome-quick-share';

async function fetchLatestRelease() {
  try {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`);
    if (!res.ok) return;
    const data = await res.json();
    document.querySelectorAll('#hero-version, #footer-version').forEach(el => {
      el.textContent = data.tag_name;
    });
  } catch (_) {}
}

setTheme(detectTheme());
setLanguage(detectLang());
fetchLatestRelease();

document.getElementById('theme-toggle').addEventListener('click', toggleTheme);
document.getElementById('lang-toggle').addEventListener('click', () => {
  setLanguage(currentLang === 'en' ? 'pt' : 'en');
});

// ── LIVE DEMO ────────────────────────────────────────
function initDemo() {
  const stateEls = {
    idle:      document.getElementById('state-idle'),
    incoming:  document.getElementById('state-incoming'),
    receiving: document.getElementById('state-receiving'),
    complete:  document.getElementById('state-complete'),
  };
  const sectionLabel = document.getElementById('app-section-label');
  const progressBar  = document.getElementById('demo-progress');
  const statsEl      = document.getElementById('demo-stats');
  const replayBtn    = document.getElementById('demo-replay');
  const acceptBtn    = document.getElementById('btn-accept');

  let animating        = false;
  let progressInterval = null;
  let autoAcceptTimer  = null;

  function showState(name) {
    Object.values(stateEls).forEach(s => s.classList.add('hidden'));
    stateEls[name].classList.remove('hidden');
    const showLabel = name === 'incoming' || name === 'receiving' || name === 'complete';
    sectionLabel.classList.toggle('hidden', !showLabel);
  }

  function startTransfer() {
    clearTimeout(autoAcceptTimer);
    showState('receiving');
    progressBar.style.width = '0%';

    let pct = 0;
    progressInterval = setInterval(() => {
      pct += Math.random() * 5 + 2;
      if (pct >= 100) {
        pct = 100;
        clearInterval(progressInterval);
        progressBar.style.width = '100%';
        if (statsEl) statsEl.textContent = '100%';
        setTimeout(() => {
          showState('complete');
          replayBtn.classList.add('visible');
          animating = false;
        }, 500);
        return;
      }
      progressBar.style.width = pct.toFixed(0) + '%';
      if (statsEl) statsEl.textContent = Math.floor(pct) + '%';
    }, 90);
  }

  function runDemo() {
    if (animating) return;
    animating = true;
    if (progressInterval) clearInterval(progressInterval);
    clearTimeout(autoAcceptTimer);
    replayBtn.classList.remove('visible');

    showState('idle');

    setTimeout(() => {
      showState('incoming');
      autoAcceptTimer = setTimeout(startTransfer, 3200);
    }, 1800);
  }

  // Visibility toggle
  const visBtn   = document.getElementById('demo-visibility-btn');
  const visLabel = document.getElementById('demo-visibility-label');
  const visIcon  = document.getElementById('demo-visibility-icon');
  const eyeOpen  = '<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>';
  const eyeClosed= '<path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/><path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/><line x1="1" y1="1" x2="23" y2="23"/>';
  let visible = true;

  visBtn.addEventListener('click', () => {
    visible = !visible;
    visBtn.classList.toggle('hidden-state', !visible);
    visIcon.innerHTML = visible ? eyeOpen : eyeClosed;
    visLabel.dataset.i18n = visible ? 'demo_visible' : 'demo_hidden';
    visLabel.textContent = translations[currentLang][visLabel.dataset.i18n];
  });

  acceptBtn.addEventListener('click', startTransfer);

  document.getElementById('btn-decline').addEventListener('click', () => {
    clearTimeout(autoAcceptTimer);
    showState('idle');
    animating = false;
    setTimeout(runDemo, 1200);
  });

  replayBtn.addEventListener('click', runDemo);

  document.getElementById('btn-clear').addEventListener('click', () => {
    replayBtn.classList.remove('visible');
    showState('idle');
    animating = false;
    setTimeout(runDemo, 800);
  });

  const section = document.getElementById('screenshots');
  if (section) {
    const obs = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting) { runDemo(); obs.disconnect(); }
    }, { threshold: 0.2 });
    obs.observe(section);
  }
}

initDemo();

// ── SEND DEMO ────────────────────────────────────────
function initSendDemo() {
  const sendStates = {
    idle:    document.getElementById('send-state-idle'),
    file:    document.getElementById('send-state-file'),
    sending: document.getElementById('send-state-sending'),
    done:    document.getElementById('send-state-done'),
  };
  const sendProgress = document.getElementById('send-progress');
  const sendStats    = document.getElementById('send-stats');
  const sendReplay   = document.getElementById('send-replay');
  const btnAddFiles  = document.getElementById('btn-add-files');
  const btnDeviceS26 = document.getElementById('send-device-s26');
  const btnSendClear = document.getElementById('send-btn-clear');

  let animating        = false;
  let progressInterval = null;
  let autoTimers       = [];

  function clearTimers() { autoTimers.forEach(clearTimeout); autoTimers = []; }

  function showSendState(name) {
    Object.values(sendStates).forEach(s => s.classList.add('hidden'));
    sendStates[name].classList.remove('hidden');
  }

  function startSending() {
    clearTimers();
    showSendState('sending');
    sendProgress.style.width = '0%';
    let pct = 0;
    progressInterval = setInterval(() => {
      pct += Math.random() * 5 + 2;
      if (pct >= 100) {
        pct = 100;
        clearInterval(progressInterval);
        sendProgress.style.width = '100%';
        if (sendStats) sendStats.textContent = '100%';
        autoTimers.push(setTimeout(() => {
          showSendState('done');
          sendReplay.classList.add('visible');
          animating = false;
        }, 500));
        return;
      }
      sendProgress.style.width = pct.toFixed(0) + '%';
      if (sendStats) sendStats.textContent = Math.floor(pct) + '%';
    }, 90);
  }

  function runSendDemo() {
    if (animating) return;
    animating = true;
    clearTimers();
    if (progressInterval) clearInterval(progressInterval);
    sendReplay.classList.remove('visible');

    showSendState('idle');

    // auto: add file after 2s
    autoTimers.push(setTimeout(() => {
      showSendState('file');
      // auto: click device after 2.5s
      autoTimers.push(setTimeout(startSending, 2500));
    }, 2000));
  }

  btnAddFiles.addEventListener('click', () => {
    clearTimers();
    showSendState('file');
    autoTimers.push(setTimeout(startSending, 2500));
  });

  btnDeviceS26.addEventListener('click', startSending);

  btnSendClear.addEventListener('click', () => {
    sendReplay.classList.remove('visible');
    showSendState('idle');
    animating = false;
    autoTimers.push(setTimeout(runSendDemo, 800));
  });

  sendReplay.addEventListener('click', runSendDemo);

  // Start when receive demo starts (same viewport trigger)
  const section = document.getElementById('screenshots');
  if (section) {
    const obs = new IntersectionObserver(entries => {
      if (entries[0].isIntersecting) { runSendDemo(); obs.disconnect(); }
    }, { threshold: 0.2 });
    obs.observe(section);
  }
}

initSendDemo();
