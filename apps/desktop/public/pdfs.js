const token = localStorage.getItem('previdenciar.web.idToken');
const status = document.getElementById('status');

async function call(path, opt = {}) {
  const r = await fetch(path, {
    ...opt,
    headers: {
      ...(opt.headers || {}),
      Authorization: 'Bearer ' + token,
      'Content-Type': 'application/json',
    },
  });
  if (!r.ok) throw new Error('HTTP ' + r.status);
  return r.json();
}

async function openSavedPdf(id, format) {
  const r = await fetch('/api/reports/saved/' + encodeURIComponent(id) + '/file', {
    headers: { Authorization: 'Bearer ' + token },
  });
  if (!r.ok) {
    const j = await call('/api/reports/saved/' + encodeURIComponent(id));
    window.open(j.url, '_blank', 'noopener');
    return;
  }
  const blob = await r.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = 'relatorio-' + format + '-' + id + '.pdf';
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 5000);
}

async function load() {
  if (!token) {
    status.textContent = 'Entre no app para acessar.';
    return;
  }
  try {
    const data = await call('/api/reports/saved');
    const rows = document.getElementById('rows');
    rows.textContent = '';
    status.textContent = data.reports.length + ' PDF(s).';
    for (const x of data.reports) {
      const tr = document.createElement('tr');
      for (const v of [
        x.format,
        new Date(x.createdAt).toLocaleString('pt-BR'),
        new Date(x.expiresAt).toLocaleDateString('pt-BR'),
        (x.sizeBytes / 1024).toFixed(0) + ' KB',
      ]) {
        const td = document.createElement('td');
        td.textContent = v;
        tr.appendChild(td);
      }
      const td = document.createElement('td');
      const a = document.createElement('button');
      a.textContent = 'Baixar';
      a.onclick = () => openSavedPdf(x.id, x.format);
      const d = document.createElement('button');
      d.textContent = 'Excluir';
      d.onclick = async () => {
        if (confirm('Excluir este PDF?')) {
          await call('/api/reports/saved/' + encodeURIComponent(x.id), { method: 'DELETE' });
          load();
        }
      };
      td.append(a, d);
      tr.appendChild(td);
      rows.appendChild(tr);
    }
  } catch (e) {
    status.textContent = 'Sessão expirada ou acesso negado.';
    status.className = 'error';
  }
}

load();
