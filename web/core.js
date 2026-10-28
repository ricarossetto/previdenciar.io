const ICONS = {
  home: '<path d="M4 10.5 12 4l8 6.5V20h-5v-5.5H9V20H4z"/>',
  users: '<circle cx="9" cy="8" r="3.2"/><path d="M3.5 19c.6-3.1 2.8-5 5.5-5s4.9 1.9 5.5 5"/><path d="M16 8.5a2.8 2.8 0 1 1 0 5.6M17 14.4c2 .5 3.3 2 3.6 4.6"/>',
  calc: '<rect x="5" y="3.5" width="14" height="17" rx="1.5"/><path d="M8 7.5h8M8 11h2M12 11h2M16 11h.01M8 14.5h2M12 14.5h2M16 14.5h.01M8 18h2M12 18h2M16 18h.01"/>',
  scale: '<path d="M12 4v16M7 20h10M5 7h14"/><path d="M8 7 5 13h6L8 7ZM16 7l-3 6h6l-3-6Z"/>',
  chip: '<rect x="7" y="7" width="10" height="10" rx="1"/><path d="M9 3.5v3M15 3.5v3M9 17.5v3M15 17.5v3M3.5 9h3M3.5 15h3M17.5 9h3M17.5 15h3"/>',
  compass: '<circle cx="12" cy="12" r="8"/><path d="m15.5 8.5-2 5-5 2 2-5z"/>',
  file: '<path d="M6 3.5h8l4 4v13H6z"/><path d="M14 3.5v4h4M9 12h6M9 15.5h6"/>',
  table: '<rect x="4" y="4.5" width="16" height="15" rx="1"/><path d="M4 9.5h16M4 14.5h16M9 4.5v15"/>',
  tool: '<path d="M14.5 6.5a4 4 0 0 0-5.3 5L4 16.7 7.3 20l5.2-5.2a4 4 0 0 0 5-5.3l-2.6 2.6-2.4-2.4z"/>',
  book: '<path d="M5 4.5h6a3 3 0 0 1 3 3V20a2.5 2.5 0 0 0-2.5-2.5H5z"/><path d="M19 4.5h-2a3 3 0 0 0-3 3V20a2.5 2.5 0 0 1 2.5-2.5H19z"/>',
  report: '<path d="M6 3.5h8l4 4v13H6z"/><path d="M14 3.5v4h4M9 12h6M9 15.5h4"/>',
  calendar: '<rect x="4" y="5.5" width="16" height="14" rx="1.5"/><path d="M4 10h16M8 3.5v4M16 3.5v4"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M12 3.5v2M12 18.5v2M3.5 12h2M18.5 12h2M5.9 5.9l1.4 1.4M16.7 16.7l1.4 1.4M18.1 5.9l-1.4 1.4M7.3 16.7l-1.4 1.4"/>',
  shield: '<path d="M12 3.5 19 6v6c0 4-3 6.8-7 8.5-4-1.7-7-4.5-7-8.5V6z"/><path d="m9 12 2 2 4-4"/>',
  search: '<circle cx="11" cy="11" r="6.5"/><path d="m16 16 4 4"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  arrow: '<path d="M5 12h14M14 7l5 5-5 5"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  alert: '<path d="M12 4 21 20H3z"/><path d="M12 9.5v4M12 16.5h.01"/>',
  lock: '<rect x="5" y="10" width="14" height="10" rx="1.5"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/>',
  clock: '<circle cx="12" cy="12" r="8"/><path d="M12 7.5V12l3 2"/>',
  download: '<path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19.5h14"/>',
  close: '<path d="M6 6l12 12M18 6 6 18"/>',
  menu: '<path d="M4 7h16M4 12h16M4 17h16"/>',
  filter: '<path d="M4 5h16l-6 7v6l-4 2v-8z"/>',
  chevron: '<path d="m9 6 6 6-6 6"/>'
};

const state = {
  view: 'home',
  caseId: 1,
  caseTab: 'overview',
  seriesId: 'inpc',
  docId: 1,
  reportCase: 1,
  rgpsScenario: 2,
  assistantStep: 2,
  assistantChoice: null,
  workspaceSource: 'demo',
  theme: 'dark',
  motion: 'full',
  railOpen: false
};

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));
const esc = (s) => String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const brl = (n) => n == null ? '—' : n.toLocaleString('pt-BR', { style: 'currency', currency: 'BRL' });
const dateBR = (iso) => {
  if (!iso || typeof iso !== 'string') return '—';
  const d = new Date(iso + 'T12:00:00');
  if (Number.isNaN(d.getTime())) return '—';
  return d.toLocaleDateString('pt-BR');
};
const ageAt = (birth, ref) => {
  if (!birth || typeof birth !== 'string') return null;
  const b = new Date(birth + 'T12:00:00');
  const r = new Date(ref + 'T12:00:00');
  if (Number.isNaN(b.getTime()) || Number.isNaN(r.getTime())) return null;
  let age = r.getFullYear() - b.getFullYear();
  const m = r.getMonth() - b.getMonth();
  if (m < 0 || (m === 0 && r.getDate() < b.getDate())) age--;
  return age;
};
const months = (n) => n == null ? '—' : Math.floor(n / 12) + 'a ' + (n % 12) + 'm';
const clientById = (id) => CLIENTS.find((c) => c.id === id) || CLIENTS[0];
const caseById = (id) => CASES.find((c) => c.id === id) || CASES[0];
const activeCases = () => CASES;
const caseCount = (clientId) => activeCases().filter((c) => c.client === clientId).length;
const statusTone = (s) => s === 'Calculado' ? 'ok' : s === 'Bloqueado' ? 'bad' : (s === 'Em revisão' || s === 'Pendente') ? 'warn' : 'quiet';
const periodTone = (s) => s === 'pendente' ? 'pendente' : s === 'parcial' ? 'parcial' : s === 'excluido' ? 'excluido' : s;
const kindTone = (k) => k === 'Autônomo' ? 'individual' : k === 'Especial' ? 'especial' : 'empregado';

function ico(name, size = 17) {
  return '<svg viewBox="0 0 24 24" width="' + size + '" height="' + size + '" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' + (ICONS[name] || ICONS.file) + '</svg>';
}

function stateTag(label, tone) {
  return '<span class="state state--' + tone + '">' + esc(label) + '</span>';
}

function pageHead(title, desc, actions) {
  return '<header class="page-head"><div class="page-head__copy"><h1 tabindex="-1">' + title + '</h1><p>' + desc + '</p></div>' + (actions ? '<div class="page-head__actions">' + actions + '</div>' : '') + '</header>';
}

function navCount(id) {
  return (id === 'cases' || id === 'rgps') ? '<span class="nav__count">' + (id === 'rgps' ? activeCases().filter((c) => c.kind === 'RGPS').length : activeCases().length) + '</span>' : '';
}
