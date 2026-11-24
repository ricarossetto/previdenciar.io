-- Identidades extras de um mesmo usuário. A entrada pelo Atrium chega com um
-- identificador próprio (atrium:<workspace>:<usuário>). Quando o e-mail já tem
-- conta, essa identidade passa a apontar para a conta existente, em vez de
-- tentar criar um segundo usuário com o mesmo e-mail.
CREATE TABLE IF NOT EXISTS user_identities (
  subject text PRIMARY KEY,
  user_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS user_identities_user_idx ON user_identities (user_id);
