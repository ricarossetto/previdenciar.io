const token = localStorage.getItem('previdenciar.web.idToken');
const status = document.getElementById('status');

async function call(path, options = {}) {
  const r = await fetch(path, {
    ...options,
    headers: {
      ...(options.headers || {}),
      Authorization: 'Bearer ' + token,
      'Content-Type': 'application/json',
    },
  });
  if (!r.ok) throw new Error('HTTP ' + r.status);
  return r.json();
}

async function load() {
  if (!token) {
    status.textContent = 'Entre no app para acessar.';
    return;
  }
  try {
    const data = await call('/api/admin/users');
    status.textContent = 'Usuários: ' + data.users.length;
    const body = document.getElementById('users');
    body.textContent = '';
    for (const u of data.users) {
      const tr = document.createElement('tr');
      const td1 = document.createElement('td');
      td1.textContent = u.email;
      const td2 = document.createElement('td');
      td2.textContent = u.advanced ? 'sim' : 'não';
      const td3 = document.createElement('td');
      const b = document.createElement('button');
      b.textContent = u.advanced ? 'Desligar avançado' : 'Ligar avançado';
      b.onclick = async () => {
        await call('/api/admin/entitlement', {
          method: 'POST',
          body: JSON.stringify({ email: u.email, advanced: !u.advanced }),
        });
        load();
      };
      td3.appendChild(b);
      tr.append(td1, td2, td3);
      body.appendChild(tr);
    }
    const audit = await call('/api/admin/audit');
    const ab = document.getElementById('audit');
    ab.textContent = '';
    for (const e of audit.events) {
      const tr = document.createElement('tr');
      for (const v of [
        new Date(e.created_at).toLocaleString('pt-BR'),
        e.action,
        e.entity_type + (e.entity_id ? ' #' + e.entity_id : ''),
        e.actor_id || '—',
      ]) {
        const td = document.createElement('td');
        td.textContent = v;
        tr.appendChild(td);
      }
      ab.appendChild(tr);
    }
  } catch (e) {
    status.textContent = 'Acesso negado ou sessão expirada.';
    status.className = 'error';
  }
}

load();
